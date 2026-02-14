#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::{Context, Result};
use chrono::Utc;
use qa_orchestrator::qa_utils::{new_ticket_diff, render_template, validate_workspace_rel_path};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use tauri::{AppHandle, Manager, State};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use uuid::Uuid;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OrchestratorConfig {
    runner: RunnerConfig,
    resume: ResumeConfig,
    defaults: ConfigDefaults,
    workspaces: HashMap<String, WorkspaceConfig>,
    agents: HashMap<String, AgentConfig>,
    workflows: HashMap<String, WorkflowConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConfigDefaults {
    workspace: String,
    workflow: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RunnerConfig {
    shell: String,
    shell_arg: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResumeConfig {
    auto: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkspaceConfig {
    root_path: String,
    qa_targets: Vec<String>,
    ticket_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AgentConfig {
    templates: AgentTemplates,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AgentTemplates {
    init_once: Option<String>,
    qa: Option<String>,
    fix: Option<String>,
    retest: Option<String>,
    loop_guard: Option<String>,
}

impl AgentTemplates {
    fn phase_template(&self, phase: &str) -> Option<&str> {
        match phase {
            "init_once" => self.init_once.as_deref(),
            "qa" => self.qa.as_deref(),
            "fix" => self.fix.as_deref(),
            "retest" => self.retest.as_deref(),
            "loop_guard" => self.loop_guard.as_deref(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum WorkflowStepType {
    InitOnce,
    Qa,
    Fix,
    Retest,
}

impl WorkflowStepType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::InitOnce => "init_once",
            Self::Qa => "qa",
            Self::Fix => "fix",
            Self::Retest => "retest",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LoopMode {
    Once,
    Infinite,
}

impl Default for LoopMode {
    fn default() -> Self {
        Self::Once
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkflowLoopGuardConfig {
    enabled: bool,
    stop_when_no_unresolved: bool,
    max_cycles: Option<u32>,
    agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    agent_template: Option<String>,
}

impl Default for WorkflowLoopGuardConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            stop_when_no_unresolved: true,
            max_cycles: None,
            agent_id: None,
            agent_template: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct WorkflowLoopConfig {
    mode: LoopMode,
    #[serde(default)]
    guard: WorkflowLoopGuardConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkflowStepConfig {
    id: String,
    #[serde(rename = "type")]
    step_type: WorkflowStepType,
    enabled: bool,
    agent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskExecutionStep {
    id: String,
    #[serde(rename = "type")]
    step_type: WorkflowStepType,
    agent_id: String,
    template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskExecutionPlan {
    steps: Vec<TaskExecutionStep>,
    #[serde(rename = "loop")]
    loop_policy: WorkflowLoopConfig,
}

impl TaskExecutionPlan {
    fn step(&self, step_type: WorkflowStepType) -> Option<&TaskExecutionStep> {
        self.steps.iter().find(|step| step.step_type == step_type)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkflowConfig {
    #[serde(default)]
    steps: Vec<WorkflowStepConfig>,
    #[serde(rename = "loop", default)]
    loop_policy: WorkflowLoopConfig,
    #[serde(default)]
    qa: Option<String>,
    #[serde(default)]
    fix: Option<String>,
    #[serde(default)]
    retest: Option<String>,
}

impl WorkflowConfig {
    fn uses_agent(&self, agent_id: &str) -> bool {
        self.steps
            .iter()
            .any(|step| step.enabled && step.agent_id.as_deref() == Some(agent_id))
            || self.loop_policy.guard.agent_id.as_deref() == Some(agent_id)
    }
}

fn default_workflow_steps(
    qa: Option<&str>,
    fix: Option<&str>,
    retest: Option<&str>,
) -> Vec<WorkflowStepConfig> {
    vec![
        WorkflowStepConfig {
            id: "init_once".to_string(),
            step_type: WorkflowStepType::InitOnce,
            enabled: false,
            agent_id: None,
        },
        WorkflowStepConfig {
            id: "qa".to_string(),
            step_type: WorkflowStepType::Qa,
            enabled: qa.is_some(),
            agent_id: qa.map(str::to_string),
        },
        WorkflowStepConfig {
            id: "fix".to_string(),
            step_type: WorkflowStepType::Fix,
            enabled: fix.is_some(),
            agent_id: fix.map(str::to_string),
        },
        WorkflowStepConfig {
            id: "retest".to_string(),
            step_type: WorkflowStepType::Retest,
            enabled: retest.is_some(),
            agent_id: retest.map(str::to_string),
        },
    ]
}

#[derive(Debug, Clone)]
struct ResolvedWorkspace {
    root_path: PathBuf,
    qa_targets: Vec<String>,
    ticket_dir: String,
}

#[derive(Debug, Clone)]
struct ActiveConfig {
    config: OrchestratorConfig,
    workspaces: HashMap<String, ResolvedWorkspace>,
    default_workspace_id: String,
    default_workflow_id: String,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        let mut workspaces = HashMap::new();
        workspaces.insert(
            "auth9".to_string(),
            WorkspaceConfig {
                root_path: "../..".to_string(),
                qa_targets: vec!["docs/qa".to_string(), "docs/security".to_string()],
                ticket_dir: "docs/ticket".to_string(),
            },
        );

        let mut agents = HashMap::new();
        agents.insert(
            "opencode".to_string(),
            AgentConfig {
                templates: AgentTemplates {
                    init_once: Some("echo \"qa-orchestrator init_once\"".to_string()),
                    qa: Some(
                        "opencode run \"读取文档：{rel_path}，执行QA测试\" -m \"deepseek/deepseek-chat\""
                            .to_string(),
                    ),
                    fix: None,
                    retest: Some(
                        "opencode run \"读取文档：{rel_path}，执行QA测试\" -m \"deepseek/deepseek-chat\""
                            .to_string(),
                    ),
                    loop_guard: Some(
                        "if [ \"{unresolved_items}\" -eq 0 ]; then echo stop; else echo continue; fi"
                            .to_string(),
                    ),
                },
            },
        );
        agents.insert(
            "claudecode".to_string(),
            AgentConfig {
                templates: AgentTemplates {
                    init_once: None,
                    qa: None,
                    fix: Some("claude -p --dangerously-skip-permissions --verbose --model opus --output-format stream-json \"/ticket-fix {ticket_paths}\"".to_string()),
                    retest: None,
                    loop_guard: None,
                },
            },
        );

        let mut workflows = HashMap::new();
        workflows.insert(
            "qa_only".to_string(),
            WorkflowConfig {
                steps: default_workflow_steps(Some("opencode"), None, None),
                loop_policy: WorkflowLoopConfig::default(),
                qa: None,
                fix: None,
                retest: None,
            },
        );
        workflows.insert(
            "qa_fix".to_string(),
            WorkflowConfig {
                steps: default_workflow_steps(Some("opencode"), Some("claudecode"), None),
                loop_policy: WorkflowLoopConfig::default(),
                qa: None,
                fix: None,
                retest: None,
            },
        );
        workflows.insert(
            "only-fix".to_string(),
            WorkflowConfig {
                steps: default_workflow_steps(None, Some("claudecode"), None),
                loop_policy: WorkflowLoopConfig::default(),
                qa: None,
                fix: None,
                retest: None,
            },
        );
        workflows.insert(
            "qa_fix_retest".to_string(),
            WorkflowConfig {
                steps: default_workflow_steps(
                    Some("opencode"),
                    Some("claudecode"),
                    Some("opencode"),
                ),
                loop_policy: WorkflowLoopConfig::default(),
                qa: None,
                fix: None,
                retest: None,
            },
        );

        Self {
            runner: RunnerConfig {
                shell: "/bin/zsh".to_string(),
                shell_arg: "-lc".to_string(),
            },
            resume: ResumeConfig { auto: true },
            defaults: ConfigDefaults {
                workspace: "auth9".to_string(),
                workflow: "qa_fix_retest".to_string(),
            },
            workspaces,
            agents,
            workflows,
        }
    }
}

#[derive(Clone)]
struct ManagedState {
    inner: Arc<InnerState>,
}

struct InnerState {
    app_root: PathBuf,
    db_path: PathBuf,
    logs_dir: PathBuf,
    config_path: PathBuf,
    active_config: RwLock<ActiveConfig>,
    running: Mutex<HashMap<String, RunningTask>>,
}

#[derive(Clone)]
struct RunningTask {
    stop_flag: Arc<AtomicBool>,
    child: Arc<Mutex<Option<Child>>>,
}

impl RunningTask {
    fn new() -> Self {
        Self {
            stop_flag: Arc::new(AtomicBool::new(false)),
            child: Arc::new(Mutex::new(None)),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct CreateTaskPayload {
    name: Option<String>,
    goal: Option<String>,
    workspace_id: Option<String>,
    workflow_id: Option<String>,
    target_files: Option<Vec<String>>,
}

impl Default for CreateTaskPayload {
    fn default() -> Self {
        Self {
            name: None,
            goal: None,
            workspace_id: None,
            workflow_id: None,
            target_files: None,
        }
    }
}

#[derive(Debug, Serialize)]
struct BootstrapResponse {
    resumed_task_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct NamedOption {
    id: String,
}

#[derive(Debug, Serialize)]
struct CreateTaskDefaults {
    workspace_id: String,
    workflow_id: String,
}

#[derive(Debug, Serialize)]
struct CreateTaskOptions {
    defaults: CreateTaskDefaults,
    workspaces: Vec<NamedOption>,
    workflows: Vec<NamedOption>,
}

#[derive(Debug, Serialize)]
struct ConfigOverview {
    config: OrchestratorConfig,
    yaml: String,
    version: i64,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
struct SaveConfigFormPayload {
    config: OrchestratorConfig,
}

#[derive(Debug, Deserialize)]
struct SaveConfigYamlPayload {
    yaml: String,
}

#[derive(Debug, Serialize)]
struct ConfigVersionSummary {
    version: i64,
    created_at: String,
    author: String,
}

#[derive(Debug, Serialize)]
struct ConfigVersionDetail {
    version: i64,
    created_at: String,
    author: String,
    yaml: String,
}

#[derive(Debug, Serialize)]
struct ConfigValidationResult {
    valid: bool,
    normalized_yaml: String,
}

#[derive(Debug, Serialize)]
struct TaskSummary {
    id: String,
    name: String,
    status: String,
    started_at: Option<String>,
    completed_at: Option<String>,
    goal: String,
    workspace_id: String,
    workflow_id: String,
    target_files: Vec<String>,
    total_items: i64,
    finished_items: i64,
    failed_items: i64,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
struct TaskItemDto {
    id: String,
    task_id: String,
    order_no: i64,
    qa_file_path: String,
    status: String,
    ticket_files: Vec<String>,
    ticket_content: Vec<Value>,
    fix_required: bool,
    fixed: bool,
    last_error: String,
    started_at: Option<String>,
    completed_at: Option<String>,
    updated_at: String,
}

#[derive(Debug, Serialize)]
struct CommandRunDto {
    id: String,
    task_item_id: String,
    phase: String,
    command: String,
    cwd: String,
    workspace_id: String,
    agent_id: String,
    exit_code: Option<i64>,
    stdout_path: String,
    stderr_path: String,
    started_at: String,
    ended_at: Option<String>,
    interrupted: bool,
}

#[derive(Debug, Serialize)]
struct EventDto {
    id: i64,
    task_id: String,
    task_item_id: Option<String>,
    event_type: String,
    payload: Value,
    created_at: String,
}

#[derive(Debug, Serialize)]
struct TaskDetail {
    task: TaskSummary,
    items: Vec<TaskItemDto>,
    runs: Vec<CommandRunDto>,
    events: Vec<EventDto>,
}

#[derive(Debug, Serialize)]
struct LogChunk {
    run_id: String,
    phase: String,
    content: String,
    stdout_path: String,
    stderr_path: String,
}

#[derive(Debug, Clone)]
struct TaskItemRow {
    id: String,
    qa_file_path: String,
}

#[derive(Debug)]
struct RunResult {
    success: bool,
    exit_code: i64,
    stdout_path: String,
    stderr_path: String,
}

#[derive(Debug, Clone)]
struct TaskRuntimeContext {
    workspace_id: String,
    workspace_root: PathBuf,
    ticket_dir: String,
    execution_plan: TaskExecutionPlan,
    current_cycle: u32,
    init_done: bool,
}

#[derive(Debug, Default, Clone)]
struct CliOptions {
    cli: bool,
    show_help: bool,
    no_auto_resume: bool,
    task_id: Option<String>,
    workspace_id: Option<String>,
    workflow_id: Option<String>,
    name: Option<String>,
    goal: Option<String>,
    target_files: Vec<String>,
}

#[tauri::command]
async fn bootstrap(state: State<'_, ManagedState>) -> Result<BootstrapResponse, String> {
    let active = read_active_config(&state.inner).map_err(err_to_string)?;
    if !active.config.resume.auto {
        return Ok(BootstrapResponse {
            resumed_task_id: None,
        });
    }
    let resumed_task_id =
        find_latest_resumable_task_id(&state.inner, false).map_err(err_to_string)?;
    Ok(BootstrapResponse { resumed_task_id })
}

#[tauri::command]
async fn get_create_task_options(
    state: State<'_, ManagedState>,
) -> Result<CreateTaskOptions, String> {
    let active = read_active_config(&state.inner).map_err(err_to_string)?;
    let mut workspaces: Vec<NamedOption> = active
        .config
        .workspaces
        .keys()
        .cloned()
        .map(|id| NamedOption { id })
        .collect();
    workspaces.sort_by(|a, b| a.id.cmp(&b.id));

    let mut workflows: Vec<NamedOption> = active
        .config
        .workflows
        .keys()
        .cloned()
        .map(|id| NamedOption { id })
        .collect();
    workflows.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(CreateTaskOptions {
        defaults: CreateTaskDefaults {
            workspace_id: active.default_workspace_id.clone(),
            workflow_id: active.default_workflow_id.clone(),
        },
        workspaces,
        workflows,
    })
}

#[tauri::command]
async fn get_config_overview(state: State<'_, ManagedState>) -> Result<ConfigOverview, String> {
    load_config_overview(&state.inner).map_err(err_to_string)
}

#[tauri::command]
async fn save_config_from_form(
    state: State<'_, ManagedState>,
    payload: SaveConfigFormPayload,
) -> Result<ConfigOverview, String> {
    let yaml = serde_yaml::to_string(&payload.config).map_err(err_to_string)?;
    persist_config_and_reload(&state.inner, payload.config, yaml, "ui-form").map_err(err_to_string)
}

#[tauri::command]
async fn save_config_from_yaml(
    state: State<'_, ManagedState>,
    payload: SaveConfigYamlPayload,
) -> Result<ConfigOverview, String> {
    let config =
        serde_yaml::from_str::<OrchestratorConfig>(&payload.yaml).map_err(err_to_string)?;
    persist_config_and_reload(&state.inner, config, payload.yaml, "ui-yaml").map_err(err_to_string)
}

#[tauri::command]
async fn validate_config_yaml(
    state: State<'_, ManagedState>,
    payload: SaveConfigYamlPayload,
) -> Result<ConfigValidationResult, String> {
    let config =
        serde_yaml::from_str::<OrchestratorConfig>(&payload.yaml).map_err(err_to_string)?;
    let candidate = build_active_config(&state.inner.app_root, config).map_err(err_to_string)?;
    let current = read_active_config(&state.inner)
        .map_err(err_to_string)?
        .config
        .clone();
    let conn = open_conn(&state.inner.db_path).map_err(err_to_string)?;
    enforce_deletion_guards(&conn, &current, &candidate.config).map_err(err_to_string)?;
    let normalized_yaml = serde_yaml::to_string(&candidate.config).map_err(err_to_string)?;
    Ok(ConfigValidationResult {
        valid: true,
        normalized_yaml,
    })
}

#[tauri::command]
async fn list_config_versions(
    state: State<'_, ManagedState>,
) -> Result<Vec<ConfigVersionSummary>, String> {
    let conn = open_conn(&state.inner.db_path).map_err(err_to_string)?;
    let mut stmt = conn
        .prepare(
            "SELECT version, created_at, author FROM orchestrator_config_versions ORDER BY version DESC LIMIT 200",
        )
        .map_err(err_to_string)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ConfigVersionSummary {
                version: row.get(0)?,
                created_at: row.get(1)?,
                author: row.get(2)?,
            })
        })
        .map_err(err_to_string)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(err_to_string)?;
    Ok(rows)
}

#[tauri::command]
async fn get_config_version(
    state: State<'_, ManagedState>,
    version: i64,
) -> Result<ConfigVersionDetail, String> {
    let conn = open_conn(&state.inner.db_path).map_err(err_to_string)?;
    let detail = conn
        .query_row(
            "SELECT version, created_at, author, config_yaml FROM orchestrator_config_versions WHERE version = ?1",
            params![version],
            |row| {
                Ok(ConfigVersionDetail {
                    version: row.get(0)?,
                    created_at: row.get(1)?,
                    author: row.get(2)?,
                    yaml: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(err_to_string)?;
    detail.ok_or_else(|| format!("config version not found: {}", version))
}

#[tauri::command]
async fn create_task(
    state: State<'_, ManagedState>,
    payload: CreateTaskPayload,
) -> Result<TaskSummary, String> {
    create_task_impl(&state.inner, payload).map_err(err_to_string)
}

#[tauri::command]
async fn list_tasks(state: State<'_, ManagedState>) -> Result<Vec<TaskSummary>, String> {
    list_tasks_impl(&state.inner).map_err(err_to_string)
}

#[tauri::command]
async fn get_task_details(
    state: State<'_, ManagedState>,
    task_id: String,
) -> Result<TaskDetail, String> {
    get_task_details_impl(&state.inner, &task_id).map_err(err_to_string)
}

#[tauri::command]
async fn start_task(
    state: State<'_, ManagedState>,
    app: AppHandle,
    task_id: String,
) -> Result<TaskSummary, String> {
    prepare_task_for_start(&state.inner, &task_id).map_err(err_to_string)?;
    spawn_task_runner(state.inner.clone(), app, task_id.clone())
        .await
        .map_err(err_to_string)?;
    load_task_summary(&state.inner, &task_id).map_err(err_to_string)
}

#[tauri::command]
async fn pause_task(
    state: State<'_, ManagedState>,
    task_id: String,
) -> Result<TaskSummary, String> {
    stop_task_runtime(state.inner.clone(), &task_id, "paused")
        .await
        .map_err(err_to_string)?;
    load_task_summary(&state.inner, &task_id).map_err(err_to_string)
}

#[tauri::command]
async fn resume_task(
    state: State<'_, ManagedState>,
    app: AppHandle,
    task_id: String,
) -> Result<TaskSummary, String> {
    prepare_task_for_start(&state.inner, &task_id).map_err(err_to_string)?;
    spawn_task_runner(state.inner.clone(), app, task_id.clone())
        .await
        .map_err(err_to_string)?;
    load_task_summary(&state.inner, &task_id).map_err(err_to_string)
}

#[tauri::command]
async fn retry_task_item(
    state: State<'_, ManagedState>,
    app: AppHandle,
    task_item_id: String,
) -> Result<TaskSummary, String> {
    let task_id = reset_task_item_for_retry(&state.inner, &task_item_id).map_err(err_to_string)?;
    prepare_task_for_start(&state.inner, &task_id).map_err(err_to_string)?;
    spawn_task_runner(state.inner.clone(), app, task_id.clone())
        .await
        .map_err(err_to_string)?;
    load_task_summary(&state.inner, &task_id).map_err(err_to_string)
}

#[tauri::command]
async fn stream_task_logs(
    state: State<'_, ManagedState>,
    task_id: String,
    limit: Option<usize>,
) -> Result<Vec<LogChunk>, String> {
    stream_task_logs_impl(&state.inner, &task_id, limit.unwrap_or(300)).map_err(err_to_string)
}

fn now_ts() -> String {
    Utc::now().to_rfc3339()
}

fn err_to_string(err: impl std::fmt::Display) -> String {
    err.to_string()
}

fn detect_app_root() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if cwd.join("src-tauri").exists() && cwd.join("src").exists() {
        return cwd;
    }
    if cwd.ends_with("src-tauri") {
        return cwd.parent().unwrap_or(&cwd).to_path_buf();
    }
    let candidate = cwd.join("tools/qa-orchestrator");
    if candidate.exists() {
        return candidate;
    }
    cwd
}

fn load_config(config_path: &Path) -> Result<OrchestratorConfig> {
    match std::fs::read_to_string(config_path) {
        Ok(content) => serde_yaml::from_str::<OrchestratorConfig>(&content)
            .with_context(|| format!("failed to parse {}", config_path.display())),
        Err(_) => Ok(OrchestratorConfig::default()),
    }
}

fn open_conn(db_path: &Path) -> Result<Connection> {
    Connection::open(db_path).context("failed to open sqlite db")
}

fn read_active_config<'a>(
    state: &'a InnerState,
) -> Result<std::sync::RwLockReadGuard<'a, ActiveConfig>> {
    state
        .active_config
        .read()
        .map_err(|_| anyhow::anyhow!("active config lock is poisoned"))
}

fn write_active_config<'a>(
    state: &'a InnerState,
) -> Result<std::sync::RwLockWriteGuard<'a, ActiveConfig>> {
    state
        .active_config
        .write()
        .map_err(|_| anyhow::anyhow!("active config lock is poisoned"))
}

fn ensure_within_root(root: &Path, target: &Path, field: &str) -> Result<()> {
    let root_canon = root
        .canonicalize()
        .with_context(|| format!("failed to canonicalize workspace root {}", root.display()))?;
    let target_canon = target.canonicalize().with_context(|| {
        format!(
            "failed to canonicalize path {} for {}",
            target.display(),
            field
        )
    })?;
    if !target_canon.starts_with(&root_canon) {
        anyhow::bail!(
            "{} resolves outside workspace root: {}",
            field,
            target_canon.display()
        );
    }
    Ok(())
}

fn resolve_workspace_path(workspace_root: &Path, rel_path: &str, field: &str) -> Result<PathBuf> {
    validate_workspace_rel_path(rel_path, field)?;
    let joined = workspace_root.join(rel_path);
    if joined.exists() {
        ensure_within_root(workspace_root, &joined, field)?;
    } else if let Some(parent) = joined.parent() {
        if parent.exists() {
            ensure_within_root(workspace_root, parent, field)?;
        }
    }
    Ok(joined)
}

fn normalize_workflow_config(workflow: &mut WorkflowConfig) {
    if workflow.steps.is_empty() {
        workflow.steps = default_workflow_steps(
            workflow.qa.as_deref(),
            workflow.fix.as_deref(),
            workflow.retest.as_deref(),
        );
    }
    for step in &mut workflow.steps {
        if step.id.trim().is_empty() {
            step.id = step.step_type.as_str().to_string();
        }
    }
    workflow.qa = None;
    workflow.fix = None;
    workflow.retest = None;
    workflow.loop_policy.guard.agent_template = None;
}

fn normalize_config(mut config: OrchestratorConfig) -> OrchestratorConfig {
    for workflow in config.workflows.values_mut() {
        normalize_workflow_config(workflow);
    }
    config
}

fn validate_workflow_config(
    config: &OrchestratorConfig,
    workflow: &WorkflowConfig,
    workflow_id: &str,
) -> Result<()> {
    if workflow.steps.is_empty() {
        anyhow::bail!("workflow '{}' must define at least one step", workflow_id);
    }

    let mut enabled_count = 0usize;
    let mut seen: HashMap<&'static str, bool> = HashMap::new();
    for step in &workflow.steps {
        let key = step.step_type.as_str();
        if seen.insert(key, true).is_some() {
            anyhow::bail!(
                "workflow '{}' has duplicate step type '{}'",
                workflow_id,
                key
            );
        }
        if !step.enabled {
            continue;
        }
        enabled_count += 1;
        let agent_id = step.agent_id.as_deref().with_context(|| {
            format!("workflow '{}' step '{}' missing agent_id", workflow_id, key)
        })?;
        let agent = config.agents.get(agent_id).with_context(|| {
            format!(
                "workflow '{}' step '{}' references unknown agent '{}'",
                workflow_id, key, agent_id
            )
        })?;
        if agent.templates.phase_template(key).is_none() {
            anyhow::bail!(
                "agent '{}' is missing template for step '{}' used by workflow '{}'",
                agent_id,
                key,
                workflow_id
            );
        }
    }
    if enabled_count == 0 {
        anyhow::bail!("workflow '{}' has no enabled steps", workflow_id);
    }
    if let Some(max_cycles) = workflow.loop_policy.guard.max_cycles {
        if max_cycles == 0 {
            anyhow::bail!(
                "workflow '{}' loop.guard.max_cycles must be > 0",
                workflow_id
            );
        }
    }
    if workflow.loop_policy.guard.enabled {
        if let Some(agent_id) = workflow.loop_policy.guard.agent_id.as_deref() {
            let agent = config.agents.get(agent_id).with_context(|| {
                format!(
                    "workflow '{}' loop.guard references unknown agent '{}'",
                    workflow_id, agent_id
                )
            })?;
            if agent.templates.phase_template("loop_guard").is_none() {
                anyhow::bail!(
                    "workflow '{}' loop.guard agent '{}' has no loop_guard template",
                    workflow_id,
                    agent_id
                );
            }
        }
    }
    Ok(())
}

fn build_execution_plan(
    config: &OrchestratorConfig,
    workflow: &WorkflowConfig,
    workflow_id: &str,
) -> Result<TaskExecutionPlan> {
    validate_workflow_config(config, workflow, workflow_id)?;
    let mut steps = Vec::new();
    for step in &workflow.steps {
        if !step.enabled {
            continue;
        }
        let phase = step.step_type.as_str();
        let agent_id = step.agent_id.as_deref().with_context(|| {
            format!(
                "workflow '{}' step '{}' missing agent_id",
                workflow_id, phase
            )
        })?;
        let agent = config
            .agents
            .get(agent_id)
            .with_context(|| format!("unknown agent '{}'", agent_id))?;
        let template = agent
            .templates
            .phase_template(phase)
            .with_context(|| format!("agent '{}' missing template '{}'", agent_id, phase))?;
        steps.push(TaskExecutionStep {
            id: step.id.clone(),
            step_type: step.step_type.clone(),
            agent_id: agent_id.to_string(),
            template: template.to_string(),
        });
    }
    let mut loop_policy = workflow.loop_policy.clone();
    if let Some(agent_id) = loop_policy.guard.agent_id.clone() {
        let agent = config
            .agents
            .get(&agent_id)
            .with_context(|| format!("unknown loop guard agent '{}'", agent_id))?;
        let template = agent
            .templates
            .phase_template("loop_guard")
            .with_context(|| format!("agent '{}' missing template 'loop_guard'", agent_id))?;
        loop_policy.guard.agent_template = Some(template.to_string());
    } else {
        loop_policy.guard.agent_template = None;
    }
    Ok(TaskExecutionPlan { steps, loop_policy })
}

fn resolve_and_validate_workspaces(
    app_root: &Path,
    config: &OrchestratorConfig,
) -> Result<HashMap<String, ResolvedWorkspace>> {
    if config.workspaces.is_empty() {
        anyhow::bail!("config.workspaces cannot be empty");
    }
    if config.agents.is_empty() {
        anyhow::bail!("config.agents cannot be empty");
    }
    if config.workflows.is_empty() {
        anyhow::bail!("config.workflows cannot be empty");
    }

    let mut resolved = HashMap::new();
    for (id, entry) in &config.workspaces {
        if id.trim().is_empty() {
            anyhow::bail!("workspace id cannot be empty");
        }
        if entry.qa_targets.is_empty() {
            anyhow::bail!("workspace '{}' qa_targets cannot be empty", id);
        }

        let root_path = app_root
            .join(&entry.root_path)
            .canonicalize()
            .with_context(|| {
                format!(
                    "workspace '{}' root_path not found: {}",
                    id, entry.root_path
                )
            })?;

        for (idx, target) in entry.qa_targets.iter().enumerate() {
            let field = format!("workspace '{}' qa_targets[{}]", id, idx);
            let resolved_target = resolve_workspace_path(&root_path, target, &field)?;
            if resolved_target.exists() && !resolved_target.is_dir() {
                anyhow::bail!(
                    "{} must be a directory: {}",
                    field,
                    resolved_target.display()
                );
            }
        }
        let ticket_field = format!("workspace '{}' ticket_dir", id);
        let resolved_ticket = resolve_workspace_path(&root_path, &entry.ticket_dir, &ticket_field)?;
        if resolved_ticket.exists() && !resolved_ticket.is_dir() {
            anyhow::bail!(
                "{} must be a directory: {}",
                ticket_field,
                resolved_ticket.display()
            );
        }

        resolved.insert(
            id.clone(),
            ResolvedWorkspace {
                root_path,
                qa_targets: entry.qa_targets.clone(),
                ticket_dir: entry.ticket_dir.clone(),
            },
        );
    }

    if !resolved.contains_key(&config.defaults.workspace) {
        anyhow::bail!(
            "defaults.workspace '{}' does not exist",
            config.defaults.workspace
        );
    }
    if !config.workflows.contains_key(&config.defaults.workflow) {
        anyhow::bail!(
            "defaults.workflow '{}' does not exist",
            config.defaults.workflow
        );
    }

    for (workflow_id, workflow) in &config.workflows {
        validate_workflow_config(config, workflow, workflow_id)?;
    }

    Ok(resolved)
}

fn build_active_config(app_root: &Path, config: OrchestratorConfig) -> Result<ActiveConfig> {
    let config = normalize_config(config);
    let workspaces = resolve_and_validate_workspaces(app_root, &config)?;
    Ok(ActiveConfig {
        default_workspace_id: config.defaults.workspace.clone(),
        default_workflow_id: config.defaults.workflow.clone(),
        workspaces,
        config,
    })
}

fn atomic_write_string(path: &Path, content: &str) -> Result<()> {
    let parent = path
        .parent()
        .with_context(|| format!("invalid file path: {}", path.display()))?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("failed to create dir {}", parent.display()))?;
    let tmp_path = path.with_extension("yaml.tmp");
    std::fs::write(&tmp_path, content)
        .with_context(|| format!("failed writing temp config {}", tmp_path.display()))?;
    std::fs::rename(&tmp_path, path)
        .with_context(|| format!("failed replacing config {}", path.display()))?;
    Ok(())
}

fn load_or_seed_config(
    db_path: &Path,
    config_path: &Path,
) -> Result<(OrchestratorConfig, String, i64, String)> {
    let conn = open_conn(db_path)?;
    let row: Option<(String, String, i64, String)> = conn
        .query_row(
            "SELECT config_yaml, config_json, version, updated_at FROM orchestrator_config WHERE id = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;

    if let Some((yaml, json_raw, version, updated_at)) = row {
        let config = serde_json::from_str::<OrchestratorConfig>(&json_raw)
            .or_else(|_| serde_yaml::from_str::<OrchestratorConfig>(&yaml))
            .context("failed to parse config from sqlite")?;
        let config = normalize_config(config);
        let yaml = serde_yaml::to_string(&config).context("failed to normalize config yaml")?;
        return Ok((config, yaml, version, updated_at));
    }

    let config = normalize_config(load_config(config_path)?);
    let yaml =
        serde_yaml::to_string(&config).context("failed to serialize initial config to yaml")?;
    let json_raw = serde_json::to_string(&config).context("failed to serialize initial config")?;
    let now = now_ts();
    conn.execute(
        "INSERT INTO orchestrator_config (id, config_yaml, config_json, version, updated_at) VALUES (1, ?1, ?2, 1, ?3)",
        params![yaml, json_raw, now],
    )?;
    conn.execute(
        "INSERT INTO orchestrator_config_versions (version, config_yaml, config_json, created_at, author) VALUES (1, ?1, ?2, ?3, 'bootstrap')",
        params![yaml, serde_json::to_string(&config)?, now],
    )?;
    atomic_write_string(config_path, &yaml)?;
    Ok((config, yaml, 1, now))
}

fn count_tasks_by_workspace(conn: &Connection, workspace_id: &str) -> Result<i64> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE workspace_id = ?1",
        params![workspace_id],
        |row| row.get(0),
    )?;
    Ok(count)
}

fn count_tasks_by_workflow(conn: &Connection, workflow_id: &str) -> Result<i64> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE workflow_id = ?1",
        params![workflow_id],
        |row| row.get(0),
    )?;
    Ok(count)
}

fn agent_is_referenced(workflows: &HashMap<String, WorkflowConfig>, agent_id: &str) -> bool {
    workflows.values().any(|wf| wf.uses_agent(agent_id))
}

fn enforce_deletion_guards(
    conn: &Connection,
    previous: &OrchestratorConfig,
    candidate: &OrchestratorConfig,
) -> Result<()> {
    let removed_workspaces: Vec<String> = previous
        .workspaces
        .keys()
        .filter(|id| !candidate.workspaces.contains_key(*id))
        .cloned()
        .collect();
    for workspace_id in removed_workspaces {
        let task_count = count_tasks_by_workspace(conn, &workspace_id)?;
        if task_count > 0 {
            anyhow::bail!(
                "cannot delete workspace '{}' because {} tasks reference it",
                workspace_id,
                task_count
            );
        }
    }

    let removed_workflows: Vec<String> = previous
        .workflows
        .keys()
        .filter(|id| !candidate.workflows.contains_key(*id))
        .cloned()
        .collect();
    for workflow_id in removed_workflows {
        let task_count = count_tasks_by_workflow(conn, &workflow_id)?;
        if task_count > 0 {
            anyhow::bail!(
                "cannot delete workflow '{}' because {} tasks reference it",
                workflow_id,
                task_count
            );
        }
    }

    let removed_agents: Vec<String> = previous
        .agents
        .keys()
        .filter(|id| !candidate.agents.contains_key(*id))
        .cloned()
        .collect();
    for agent_id in removed_agents {
        if agent_is_referenced(&candidate.workflows, &agent_id) {
            anyhow::bail!(
                "cannot delete agent '{}' because workflows still reference it",
                agent_id
            );
        }
    }

    Ok(())
}

fn persist_config_and_reload(
    state: &InnerState,
    config: OrchestratorConfig,
    _yaml: String,
    author: &str,
) -> Result<ConfigOverview> {
    let candidate = build_active_config(&state.app_root, config.clone())?;
    let normalized = candidate.config.clone();
    let yaml = serde_yaml::to_string(&normalized).context("failed to serialize config yaml")?;
    let json_raw = serde_json::to_string(&normalized).context("failed to serialize config json")?;

    let previous_config = {
        let active = read_active_config(state)?;
        active.config.clone()
    };

    let conn = open_conn(&state.db_path)?;
    let tx = conn.unchecked_transaction()?;
    enforce_deletion_guards(&tx, &previous_config, &normalized)?;
    let current_version: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM orchestrator_config_versions",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    let next_version = current_version + 1;
    let now = now_ts();

    tx.execute(
        "INSERT INTO orchestrator_config (id, config_yaml, config_json, version, updated_at) VALUES (1, ?1, ?2, ?3, ?4)
         ON CONFLICT(id) DO UPDATE SET config_yaml=excluded.config_yaml, config_json=excluded.config_json, version=excluded.version, updated_at=excluded.updated_at",
        params![yaml, json_raw, next_version, now],
    )?;
    tx.execute(
        "INSERT INTO orchestrator_config_versions (version, config_yaml, config_json, created_at, author) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![next_version, yaml, serde_json::to_string(&normalized)?, now, author],
    )?;

    atomic_write_string(&state.config_path, &yaml)?;
    tx.commit()?;

    {
        let mut active = write_active_config(state)?;
        *active = candidate;
    }

    Ok(ConfigOverview {
        config: normalized,
        yaml,
        version: next_version,
        updated_at: now,
    })
}

fn load_config_overview(state: &InnerState) -> Result<ConfigOverview> {
    let conn = open_conn(&state.db_path)?;
    let (yaml, version, updated_at): (String, i64, String) = conn.query_row(
        "SELECT config_yaml, version, updated_at FROM orchestrator_config WHERE id = 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;

    let active = read_active_config(state)?;
    Ok(ConfigOverview {
        config: active.config.clone(),
        yaml,
        version,
        updated_at,
    })
}

fn init_state() -> Result<ManagedState> {
    let app_root = detect_app_root();
    let config_path = app_root.join("config/default.yaml");
    let data_dir = app_root.join("data");
    let logs_dir = data_dir.join("logs");
    std::fs::create_dir_all(&logs_dir)
        .with_context(|| format!("failed to create logs dir {}", logs_dir.display()))?;

    let db_path = data_dir.join("qa_orchestrator.db");
    init_schema(&db_path)?;

    let (config, _yaml, _version, _updated_at) = load_or_seed_config(&db_path, &config_path)?;
    let active = build_active_config(&app_root, config)?;
    let default_workspace = active
        .workspaces
        .get(&active.default_workspace_id)
        .context("default workspace is missing after config validation")?;
    backfill_legacy_data(
        &db_path,
        &active.default_workspace_id,
        &active.default_workflow_id,
        default_workspace,
    )?;

    Ok(ManagedState {
        inner: Arc::new(InnerState {
            app_root,
            db_path,
            logs_dir,
            config_path,
            active_config: RwLock::new(active),
            running: Mutex::new(HashMap::new()),
        }),
    })
}

fn ensure_column(conn: &Connection, table: &str, column: &str, ddl: &str) -> Result<()> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({})", table))
        .with_context(|| format!("failed to read schema for {}", table))?;
    let cols = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if !cols.iter().any(|c| c == column) {
        conn.execute(ddl, [])
            .with_context(|| format!("failed to add column {}.{}", table, column))?;
    }
    Ok(())
}

fn init_schema(db_path: &Path) -> Result<()> {
    let conn = open_conn(db_path)?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            status TEXT NOT NULL,
            started_at TEXT,
            completed_at TEXT,
            goal TEXT NOT NULL,
            target_files_json TEXT NOT NULL,
            mode TEXT NOT NULL,
            workspace_id TEXT NOT NULL,
            workflow_id TEXT NOT NULL,
            workspace_root TEXT NOT NULL,
            qa_targets_json TEXT NOT NULL,
            ticket_dir TEXT NOT NULL,
            execution_plan_json TEXT NOT NULL DEFAULT '{}',
            loop_mode TEXT NOT NULL DEFAULT 'once',
            current_cycle INTEGER NOT NULL DEFAULT 0,
            init_done INTEGER NOT NULL DEFAULT 0,
            resume_token TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS task_items (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            order_no INTEGER NOT NULL,
            qa_file_path TEXT NOT NULL,
            status TEXT NOT NULL,
            ticket_files_json TEXT NOT NULL,
            ticket_content_json TEXT NOT NULL,
            fix_required INTEGER NOT NULL DEFAULT 0,
            fixed INTEGER NOT NULL DEFAULT 0,
            last_error TEXT NOT NULL DEFAULT '',
            started_at TEXT,
            completed_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY(task_id) REFERENCES tasks(id)
        );

        CREATE TABLE IF NOT EXISTS command_runs (
            id TEXT PRIMARY KEY,
            task_item_id TEXT NOT NULL,
            phase TEXT NOT NULL,
            command TEXT NOT NULL,
            cwd TEXT NOT NULL,
            workspace_id TEXT NOT NULL,
            agent_id TEXT NOT NULL,
            exit_code INTEGER,
            stdout_path TEXT NOT NULL,
            stderr_path TEXT NOT NULL,
            started_at TEXT NOT NULL,
            ended_at TEXT,
            interrupted INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY(task_item_id) REFERENCES task_items(id)
        );

        CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id TEXT NOT NULL,
            task_item_id TEXT,
            event_type TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS orchestrator_config (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            config_yaml TEXT NOT NULL,
            config_json TEXT NOT NULL,
            version INTEGER NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS orchestrator_config_versions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            version INTEGER NOT NULL,
            config_yaml TEXT NOT NULL,
            config_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            author TEXT NOT NULL DEFAULT 'system'
        );

        CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
        CREATE INDEX IF NOT EXISTS idx_task_items_task_order ON task_items(task_id, order_no);
        CREATE INDEX IF NOT EXISTS idx_task_items_status ON task_items(status);
        CREATE INDEX IF NOT EXISTS idx_command_runs_task_item_phase ON command_runs(task_item_id, phase);
        CREATE INDEX IF NOT EXISTS idx_events_task_created_at ON events(task_id, created_at);
        CREATE INDEX IF NOT EXISTS idx_cfg_versions_version ON orchestrator_config_versions(version DESC);
        "#,
    )
    .context("failed to initialize schema")?;

    ensure_column(
        &conn,
        "tasks",
        "workspace_id",
        "ALTER TABLE tasks ADD COLUMN workspace_id TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        &conn,
        "tasks",
        "workflow_id",
        "ALTER TABLE tasks ADD COLUMN workflow_id TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        &conn,
        "tasks",
        "workspace_root",
        "ALTER TABLE tasks ADD COLUMN workspace_root TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        &conn,
        "tasks",
        "qa_targets_json",
        "ALTER TABLE tasks ADD COLUMN qa_targets_json TEXT NOT NULL DEFAULT '[]'",
    )?;
    ensure_column(
        &conn,
        "tasks",
        "ticket_dir",
        "ALTER TABLE tasks ADD COLUMN ticket_dir TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        &conn,
        "tasks",
        "execution_plan_json",
        "ALTER TABLE tasks ADD COLUMN execution_plan_json TEXT NOT NULL DEFAULT '{}'",
    )?;
    ensure_column(
        &conn,
        "tasks",
        "loop_mode",
        "ALTER TABLE tasks ADD COLUMN loop_mode TEXT NOT NULL DEFAULT 'once'",
    )?;
    ensure_column(
        &conn,
        "tasks",
        "current_cycle",
        "ALTER TABLE tasks ADD COLUMN current_cycle INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(
        &conn,
        "tasks",
        "init_done",
        "ALTER TABLE tasks ADD COLUMN init_done INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(
        &conn,
        "command_runs",
        "workspace_id",
        "ALTER TABLE command_runs ADD COLUMN workspace_id TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        &conn,
        "command_runs",
        "agent_id",
        "ALTER TABLE command_runs ADD COLUMN agent_id TEXT NOT NULL DEFAULT ''",
    )?;
    Ok(())
}

fn backfill_legacy_data(
    db_path: &Path,
    default_workspace_id: &str,
    default_workflow_id: &str,
    workspace: &ResolvedWorkspace,
) -> Result<()> {
    let conn = open_conn(db_path)?;
    let workspace_root = workspace.root_path.to_string_lossy().to_string();
    let qa_targets = serde_json::to_string(&workspace.qa_targets)?;
    conn.execute(
        "UPDATE tasks SET workspace_id = ?1 WHERE workspace_id = ''",
        params![default_workspace_id],
    )?;
    conn.execute(
        "UPDATE tasks SET workflow_id = ?1 WHERE workflow_id = ''",
        params![default_workflow_id],
    )?;
    conn.execute(
        "UPDATE tasks SET workspace_root = ?1 WHERE workspace_root = ''",
        params![workspace_root],
    )?;
    conn.execute(
        "UPDATE tasks SET qa_targets_json = ?1 WHERE qa_targets_json = '' OR qa_targets_json = '[]'",
        params![qa_targets],
    )?;
    conn.execute(
        "UPDATE tasks SET ticket_dir = ?1 WHERE ticket_dir = ''",
        params![workspace.ticket_dir],
    )?;
    conn.execute(
        "UPDATE command_runs SET workspace_id = ?1 WHERE workspace_id = ''",
        params![default_workspace_id],
    )?;
    conn.execute(
        "UPDATE command_runs SET agent_id = 'legacy' WHERE agent_id = ''",
        [],
    )?;
    Ok(())
}

fn create_task_impl(state: &InnerState, payload: CreateTaskPayload) -> Result<TaskSummary> {
    let active = read_active_config(state)?;

    let workspace_id = payload
        .workspace_id
        .clone()
        .unwrap_or_else(|| active.default_workspace_id.clone());
    let workspace = active
        .workspaces
        .get(&workspace_id)
        .with_context(|| format!("workspace not found: {}", workspace_id))?;

    let workflow_id = payload
        .workflow_id
        .clone()
        .unwrap_or_else(|| active.default_workflow_id.clone());
    let workflow = active
        .config
        .workflows
        .get(&workflow_id)
        .with_context(|| format!("workflow not found: {}", workflow_id))?;
    let execution_plan = build_execution_plan(&active.config, workflow, &workflow_id)?;
    let execution_plan_json =
        serde_json::to_string(&execution_plan).context("serialize execution plan")?;
    let loop_mode = match execution_plan.loop_policy.mode {
        LoopMode::Once => "once",
        LoopMode::Infinite => "infinite",
    };

    let target_files = collect_target_files(
        &workspace.root_path,
        &workspace.qa_targets,
        payload.target_files,
    )?;
    if target_files.is_empty() {
        anyhow::bail!("No QA/Security markdown files found");
    }

    let task_id = Uuid::new_v4().to_string();
    let created_at = now_ts();
    let task_name = payload
        .name
        .unwrap_or_else(|| format!("QA Sprint {}", Utc::now().format("%Y-%m-%d %H:%M:%S")));
    let goal = payload
        .goal
        .unwrap_or_else(|| "Automated QA workflow with fix and resume".to_string());

    let conn = open_conn(&state.db_path)?;
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO tasks (id, name, status, started_at, completed_at, goal, target_files_json, mode, workspace_id, workflow_id, workspace_root, qa_targets_json, ticket_dir, execution_plan_json, loop_mode, current_cycle, init_done, resume_token, created_at, updated_at) VALUES (?1, ?2, 'pending', NULL, NULL, ?3, ?4, '', ?5, ?6, ?7, ?8, ?9, ?10, ?11, 0, 0, NULL, ?12, ?12)",
        params![
            task_id,
            task_name,
            goal,
            serde_json::to_string(&target_files)?,
            workspace_id,
            workflow_id,
            workspace.root_path.to_string_lossy().to_string(),
            serde_json::to_string(&workspace.qa_targets)?,
            workspace.ticket_dir,
            execution_plan_json,
            loop_mode,
            created_at
        ],
    )?;

    for (idx, path) in target_files.iter().enumerate() {
        let item_id = Uuid::new_v4().to_string();
        tx.execute(
            "INSERT INTO task_items (id, task_id, order_no, qa_file_path, status, ticket_files_json, ticket_content_json, fix_required, fixed, last_error, started_at, completed_at, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, 'pending', '[]', '[]', 0, 0, '', NULL, NULL, ?5, ?5)",
            params![item_id, task_id, (idx as i64) + 1, path, created_at],
        )?;
    }
    tx.commit()?;

    load_task_summary(state, &task_id)
}

fn collect_target_files(
    workspace_root: &Path,
    qa_targets: &[String],
    input: Option<Vec<String>>,
) -> Result<Vec<String>> {
    if let Some(list) = input {
        let mut result = Vec::new();
        for entry in list {
            let trimmed = entry.trim();
            if trimmed.is_empty() {
                continue;
            }
            let abs = resolve_workspace_path(workspace_root, trimmed, "target_files")?;
            if abs.is_file() {
                result.push(trimmed.to_string());
            }
        }
        result.sort();
        result.dedup();
        return Ok(result);
    }

    let mut files = Vec::new();
    for target in qa_targets {
        let base = resolve_workspace_path(workspace_root, target, "qa_targets")?;
        if !base.exists() {
            continue;
        }
        for entry in WalkDir::new(base)
            .into_iter()
            .filter_map(|value| value.ok())
        {
            if !entry.path().is_file() {
                continue;
            }
            if entry.path().extension().and_then(|v| v.to_str()) != Some("md") {
                continue;
            }
            if entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case("README.md")
            {
                continue;
            }
            let rel = pathdiff::diff_paths(entry.path(), workspace_root)
                .unwrap_or_else(|| entry.path().to_path_buf())
                .to_string_lossy()
                .to_string();
            files.push(rel);
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn load_task_summary(state: &InnerState, task_id: &str) -> Result<TaskSummary> {
    let conn = open_conn(&state.db_path)?;
    let mut stmt = conn.prepare(
        "SELECT id, name, status, started_at, completed_at, goal, target_files_json, workspace_id, workflow_id, created_at, updated_at FROM tasks WHERE id = ?1",
    )?;
    let mut summary = stmt.query_row(params![task_id], |row| {
        let target_raw: String = row.get(6)?;
        let target_files = serde_json::from_str::<Vec<String>>(&target_raw).unwrap_or_default();
        Ok(TaskSummary {
            id: row.get(0)?,
            name: row.get(1)?,
            status: row.get(2)?,
            started_at: row.get(3)?,
            completed_at: row.get(4)?,
            goal: row.get(5)?,
            workspace_id: row.get(7)?,
            workflow_id: row.get(8)?,
            target_files,
            total_items: 0,
            finished_items: 0,
            failed_items: 0,
            created_at: row.get(9)?,
            updated_at: row.get(10)?,
        })
    })?;

    let (total, finished, failed): (i64, i64, i64) = conn.query_row(
        "SELECT COUNT(*), SUM(CASE WHEN status IN ('qa_passed','fixed','verified','skipped','unresolved') THEN 1 ELSE 0 END), SUM(CASE WHEN status IN ('qa_failed','unresolved') THEN 1 ELSE 0 END) FROM task_items WHERE task_id = ?1",
        params![task_id],
        |row| {
            Ok((
                row.get(0)?,
                row.get::<_, Option<i64>>(1)?.unwrap_or(0),
                row.get::<_, Option<i64>>(2)?.unwrap_or(0),
            ))
        },
    )?;

    summary.total_items = total;
    summary.finished_items = finished;
    summary.failed_items = failed;
    Ok(summary)
}

fn list_tasks_impl(state: &InnerState) -> Result<Vec<TaskSummary>> {
    let conn = open_conn(&state.db_path)?;
    let mut stmt = conn.prepare("SELECT id FROM tasks ORDER BY created_at DESC")?;
    let ids = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let mut result = Vec::new();
    for id in ids {
        result.push(load_task_summary(state, &id)?);
    }
    Ok(result)
}

fn get_task_details_impl(state: &InnerState, task_id: &str) -> Result<TaskDetail> {
    let task = load_task_summary(state, task_id)?;
    let conn = open_conn(&state.db_path)?;

    let mut items_stmt = conn.prepare(
        "SELECT id, task_id, order_no, qa_file_path, status, ticket_files_json, ticket_content_json, fix_required, fixed, last_error, started_at, completed_at, updated_at FROM task_items WHERE task_id = ?1 ORDER BY order_no",
    )?;
    let items = items_stmt
        .query_map(params![task_id], |row| {
            let ticket_files_raw: String = row.get(5)?;
            let ticket_content_raw: String = row.get(6)?;
            Ok(TaskItemDto {
                id: row.get(0)?,
                task_id: row.get(1)?,
                order_no: row.get(2)?,
                qa_file_path: row.get(3)?,
                status: row.get(4)?,
                ticket_files: serde_json::from_str(&ticket_files_raw).unwrap_or_default(),
                ticket_content: serde_json::from_str(&ticket_content_raw).unwrap_or_default(),
                fix_required: row.get::<_, i64>(7)? == 1,
                fixed: row.get::<_, i64>(8)? == 1,
                last_error: row.get(9)?,
                started_at: row.get(10)?,
                completed_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let mut runs_stmt = conn.prepare(
        "SELECT cr.id, cr.task_item_id, cr.phase, cr.command, cr.cwd, cr.workspace_id, cr.agent_id, cr.exit_code, cr.stdout_path, cr.stderr_path, cr.started_at, cr.ended_at, cr.interrupted
         FROM command_runs cr
         JOIN task_items ti ON ti.id = cr.task_item_id
         WHERE ti.task_id = ?1
         ORDER BY cr.started_at DESC
         LIMIT 120",
    )?;
    let runs = runs_stmt
        .query_map(params![task_id], |row| {
            Ok(CommandRunDto {
                id: row.get(0)?,
                task_item_id: row.get(1)?,
                phase: row.get(2)?,
                command: row.get(3)?,
                cwd: row.get(4)?,
                workspace_id: row.get(5)?,
                agent_id: row.get(6)?,
                exit_code: row.get(7)?,
                stdout_path: row.get(8)?,
                stderr_path: row.get(9)?,
                started_at: row.get(10)?,
                ended_at: row.get(11)?,
                interrupted: row.get::<_, i64>(12)? == 1,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let mut events_stmt = conn.prepare(
        "SELECT id, task_id, task_item_id, event_type, payload_json, created_at FROM events WHERE task_id = ?1 ORDER BY id DESC LIMIT 200",
    )?;
    let events = events_stmt
        .query_map(params![task_id], |row| {
            let payload_raw: String = row.get(4)?;
            Ok(EventDto {
                id: row.get(0)?,
                task_id: row.get(1)?,
                task_item_id: row.get(2)?,
                event_type: row.get(3)?,
                payload: serde_json::from_str(&payload_raw).unwrap_or_else(|_| json!({})),
                created_at: row.get(5)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(TaskDetail {
        task,
        items,
        runs,
        events,
    })
}

fn stream_task_logs_impl(
    state: &InnerState,
    task_id: &str,
    line_limit: usize,
) -> Result<Vec<LogChunk>> {
    let conn = open_conn(&state.db_path)?;
    let mut stmt = conn.prepare(
        "SELECT cr.id, cr.phase, cr.stdout_path, cr.stderr_path
         FROM command_runs cr
         JOIN task_items ti ON ti.id = cr.task_item_id
         WHERE ti.task_id = ?1
         ORDER BY cr.started_at DESC
         LIMIT 14",
    )?;

    let mut chunks = Vec::new();
    for row in stmt.query_map(params![task_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })? {
        let (run_id, phase, stdout_path, stderr_path) = row?;
        let stdout_tail = tail_lines(Path::new(&stdout_path), line_limit / 2).unwrap_or_default();
        let stderr_tail = tail_lines(Path::new(&stderr_path), line_limit / 2).unwrap_or_default();
        let content = format!(
            "[{}][{}]\n{}\n{}",
            run_id,
            phase,
            stdout_tail,
            if stderr_tail.is_empty() {
                String::new()
            } else {
                format!("\n[stderr]\n{}", stderr_tail)
            }
        );
        chunks.push(LogChunk {
            run_id,
            phase,
            content,
            stdout_path,
            stderr_path,
        });
    }
    chunks.reverse();
    Ok(chunks)
}

fn tail_lines(path: &Path, limit: usize) -> Result<String> {
    let content = std::fs::read_to_string(path).unwrap_or_default();
    let lines: Vec<&str> = content.lines().collect();
    let start = lines.len().saturating_sub(limit);
    Ok(lines[start..].join("\n"))
}

fn prepare_task_for_start(state: &InnerState, task_id: &str) -> Result<()> {
    let conn = open_conn(&state.db_path)?;
    let status: Option<String> = conn
        .query_row(
            "SELECT status FROM tasks WHERE id = ?1",
            params![task_id],
            |row| row.get(0),
        )
        .optional()?;

    if status.is_none() {
        anyhow::bail!("task not found: {}", task_id);
    }

    if matches!(status.as_deref(), Some("failed")) {
        conn.execute(
            "UPDATE task_items SET status='pending', ticket_files_json='[]', ticket_content_json='[]', fix_required=0, fixed=0, last_error='', completed_at=NULL, updated_at=?2 WHERE task_id=?1 AND status='unresolved'",
            params![task_id, now_ts()],
        )?;
    }

    set_task_status(state, task_id, "running", false)?;
    insert_event(
        state,
        task_id,
        None,
        "task_started",
        json!({"reason":"manual_or_resume"}),
    )?;
    Ok(())
}

fn set_task_status(
    state: &InnerState,
    task_id: &str,
    status: &str,
    set_completed: bool,
) -> Result<()> {
    let conn = open_conn(&state.db_path)?;
    let now = now_ts();
    if set_completed {
        conn.execute(
            "UPDATE tasks SET status = ?2, completed_at = ?3, updated_at = ?4 WHERE id = ?1",
            params![task_id, status, now.clone(), now],
        )?;
    } else if matches!(status, "pending" | "running" | "paused" | "interrupted") {
        conn.execute(
            "UPDATE tasks SET status = ?2, completed_at = NULL, updated_at = ?3 WHERE id = ?1",
            params![task_id, status, now],
        )?;
    } else {
        conn.execute(
            "UPDATE tasks SET status = ?2, updated_at = ?3 WHERE id = ?1",
            params![task_id, status, now],
        )?;
    }
    Ok(())
}

fn insert_event(
    state: &InnerState,
    task_id: &str,
    task_item_id: Option<&str>,
    event_type: &str,
    payload: Value,
) -> Result<()> {
    let conn = open_conn(&state.db_path)?;
    conn.execute(
        "INSERT INTO events (task_id, task_item_id, event_type, payload_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            task_id,
            task_item_id,
            event_type,
            serde_json::to_string(&payload)?,
            now_ts()
        ],
    )?;
    Ok(())
}

fn emit_event(
    app: &AppHandle,
    task_id: &str,
    task_item_id: Option<&str>,
    event_type: &str,
    payload: Value,
) {
    let _ = app.emit_all(
        "task-event",
        json!({
            "task_id": task_id,
            "task_item_id": task_item_id,
            "event_type": event_type,
            "payload": payload,
            "ts": now_ts()
        }),
    );
}

fn list_task_items_for_cycle(state: &InnerState, task_id: &str) -> Result<Vec<TaskItemRow>> {
    let conn = open_conn(&state.db_path)?;
    let mut stmt = conn.prepare(
        "SELECT id, qa_file_path
         FROM task_items
         WHERE task_id = ?1
         ORDER BY order_no
        ",
    )?;

    let rows = stmt
        .query_map(params![task_id], |row| {
            Ok(TaskItemRow {
                id: row.get(0)?,
                qa_file_path: row.get(1)?,
            })
        })
        .context("query task items")?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn first_task_item_id(state: &InnerState, task_id: &str) -> Result<Option<String>> {
    let conn = open_conn(&state.db_path)?;
    conn.query_row(
        "SELECT id FROM task_items WHERE task_id = ?1 ORDER BY order_no LIMIT 1",
        params![task_id],
        |row| row.get(0),
    )
    .optional()
    .context("query first task item")
}

fn count_unresolved_items(state: &InnerState, task_id: &str) -> Result<i64> {
    let conn = open_conn(&state.db_path)?;
    conn.query_row(
        "SELECT COUNT(*) FROM task_items WHERE task_id = ?1 AND status IN ('unresolved','qa_failed')",
        params![task_id],
        |row| row.get(0),
    )
    .context("count unresolved items")
}

fn update_task_cycle_state(
    state: &InnerState,
    task_id: &str,
    current_cycle: u32,
    init_done: bool,
) -> Result<()> {
    let conn = open_conn(&state.db_path)?;
    conn.execute(
        "UPDATE tasks SET current_cycle = ?2, init_done = ?3, updated_at = ?4 WHERE id = ?1",
        params![
            task_id,
            current_cycle as i64,
            if init_done { 1 } else { 0 },
            now_ts()
        ],
    )?;
    Ok(())
}

fn update_task_item(
    state: &InnerState,
    item_id: &str,
    status: &str,
    ticket_files: Option<&[String]>,
    ticket_content: Option<&[Value]>,
    fix_required: Option<bool>,
    fixed: Option<bool>,
    last_error: Option<&str>,
    set_started: bool,
    set_completed: bool,
) -> Result<()> {
    let conn = open_conn(&state.db_path)?;
    let now = now_ts();
    let mut current = conn.prepare(
        "SELECT ticket_files_json, ticket_content_json, fix_required, fixed, last_error, started_at, completed_at FROM task_items WHERE id = ?1",
    )?;
    let old = current.query_row(params![item_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, Option<String>>(6)?,
        ))
    })?;

    let ticket_files_json = ticket_files
        .map(serde_json::to_string)
        .transpose()?
        .unwrap_or(old.0);
    let ticket_content_json = ticket_content
        .map(serde_json::to_string)
        .transpose()?
        .unwrap_or(old.1);
    let fix_required_value = fix_required.unwrap_or(old.2 == 1);
    let fixed_value = fixed.unwrap_or(old.3 == 1);
    let last_error_value = last_error.unwrap_or(&old.4).to_string();
    let started_at = if set_started {
        Some(now.clone())
    } else {
        old.5
    };
    let completed_at = if set_completed {
        Some(now.clone())
    } else {
        old.6
    };

    conn.execute(
        "UPDATE task_items
         SET status = ?2,
             ticket_files_json = ?3,
             ticket_content_json = ?4,
             fix_required = ?5,
             fixed = ?6,
             last_error = ?7,
             started_at = ?8,
             completed_at = ?9,
             updated_at = ?10
         WHERE id = ?1",
        params![
            item_id,
            status,
            ticket_files_json,
            ticket_content_json,
            if fix_required_value { 1 } else { 0 },
            if fixed_value { 1 } else { 0 },
            last_error_value,
            started_at,
            completed_at,
            now,
        ],
    )?;
    Ok(())
}

fn list_ticket_files(task_ctx: &TaskRuntimeContext) -> Result<Vec<String>> {
    let ticket_dir = resolve_workspace_path(
        &task_ctx.workspace_root,
        &task_ctx.ticket_dir,
        "task.ticket_dir",
    )?;
    if !ticket_dir.exists() {
        return Ok(Vec::new());
    }
    let mut result = Vec::new();
    for entry in WalkDir::new(ticket_dir)
        .min_depth(1)
        .into_iter()
        .filter_map(|value| value.ok())
    {
        if !entry.path().is_file() {
            continue;
        }
        if entry.path().extension().and_then(|value| value.to_str()) != Some("md") {
            continue;
        }
        if entry
            .file_name()
            .to_string_lossy()
            .eq_ignore_ascii_case("README.md")
        {
            continue;
        }
        let rel = pathdiff::diff_paths(entry.path(), &task_ctx.workspace_root)
            .unwrap_or_else(|| entry.path().to_path_buf())
            .to_string_lossy()
            .to_string();
        result.push(rel);
    }
    result.sort();
    Ok(result)
}

fn list_existing_tickets_for_item(
    task_ctx: &TaskRuntimeContext,
    qa_file_path: &str,
) -> Result<Vec<String>> {
    let mut matched = Vec::new();
    for ticket in list_ticket_files(task_ctx)? {
        let preview = read_ticket_preview(task_ctx, &ticket);
        let qa_doc = preview
            .get("qa_document")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        if qa_doc == qa_file_path {
            matched.push(ticket);
        }
    }
    matched.sort();
    Ok(matched)
}

fn read_ticket_preview(task_ctx: &TaskRuntimeContext, rel_path: &str) -> Value {
    let abs =
        match resolve_workspace_path(&task_ctx.workspace_root, rel_path, "ticket preview path") {
            Ok(value) => value,
            Err(_) => {
                return json!({"path": rel_path, "title": "", "status": "", "qa_document": ""})
            }
        };
    let content = std::fs::read_to_string(abs).unwrap_or_default();
    let mut title = String::new();
    let mut status = String::new();
    let mut qa_doc = String::new();
    for line in content.lines().take(50) {
        if line.starts_with("# Ticket:") {
            title = line.trim_start_matches("# Ticket:").trim().to_string();
        }
        if line.starts_with("**Status**:") {
            status = line.trim_start_matches("**Status**:").trim().to_string();
        }
        if line.starts_with("**QA Document**:") {
            qa_doc = line
                .trim_start_matches("**QA Document**:")
                .trim()
                .trim_matches('`')
                .to_string();
        }
    }
    json!({
        "path": rel_path,
        "title": title,
        "status": status,
        "qa_document": qa_doc
    })
}

fn load_task_runtime_context(state: &InnerState, task_id: &str) -> Result<TaskRuntimeContext> {
    let conn = open_conn(&state.db_path)?;
    let (
        workspace_id,
        workflow_id,
        workspace_root_raw,
        ticket_dir,
        execution_plan_json,
        current_cycle,
        init_done,
    ): (String, String, String, String, String, i64, i64) = conn.query_row(
        "SELECT workspace_id, workflow_id, workspace_root, ticket_dir, execution_plan_json, current_cycle, init_done FROM tasks WHERE id = ?1",
        params![task_id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        },
    )?;

    let active = read_active_config(state)?;
    let workflow = active
        .config
        .workflows
        .get(&workflow_id)
        .with_context(|| format!("workflow not found for task {}: {}", task_id, workflow_id))?;

    let execution_plan = serde_json::from_str::<TaskExecutionPlan>(&execution_plan_json)
        .ok()
        .filter(|plan| !plan.steps.is_empty())
        .unwrap_or_else(|| {
            build_execution_plan(&active.config, workflow, &workflow_id).unwrap_or(
                TaskExecutionPlan {
                    steps: Vec::new(),
                    loop_policy: WorkflowLoopConfig::default(),
                },
            )
        });
    if execution_plan.steps.is_empty() {
        anyhow::bail!("task '{}' has empty execution plan", task_id);
    }

    let workspace_root = PathBuf::from(workspace_root_raw);
    if !workspace_root.exists() {
        anyhow::bail!(
            "workspace root does not exist for task {}: {}",
            task_id,
            workspace_root.display()
        );
    }
    let workspace_root = workspace_root
        .canonicalize()
        .with_context(|| format!("failed to canonicalize workspace root for task {}", task_id))?;
    resolve_workspace_path(&workspace_root, &ticket_dir, "task.ticket_dir")?;

    Ok(TaskRuntimeContext {
        workspace_id,
        workspace_root,
        ticket_dir,
        execution_plan,
        current_cycle: current_cycle.max(0) as u32,
        init_done: init_done == 1,
    })
}

fn build_phase_command(
    step: &TaskExecutionStep,
    rel_path: &str,
    ticket_paths: &[String],
) -> Result<(String, String)> {
    Ok((
        step.agent_id.clone(),
        render_template(&step.template, rel_path, ticket_paths),
    ))
}

fn find_latest_resumable_task_id(
    state: &InnerState,
    include_pending: bool,
) -> Result<Option<String>> {
    let conn = open_conn(&state.db_path)?;
    let mut stmt = conn.prepare("SELECT id, status FROM tasks ORDER BY updated_at DESC")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    for row in rows {
        let (id, status) = row?;
        let resumable = matches!(status.as_str(), "running" | "interrupted" | "paused")
            || (include_pending && status == "pending");
        if resumable {
            return Ok(Some(id));
        }
    }
    Ok(None)
}

fn print_cli_help(binary_name: &str) {
    println!("Auth9 QA Orchestrator CLI");
    println!();
    println!(
        "Usage: {} --cli [--task-id ID] [--workspace ID] [--workflow ID] [--name NAME] [--goal GOAL] [--target-file PATH]... [--no-auto-resume]",
        binary_name
    );
}

fn parse_cli_options(args: &[String]) -> Result<CliOptions> {
    let mut options = CliOptions::default();
    let mut idx = 0usize;

    while idx < args.len() {
        match args[idx].as_str() {
            "--cli" => {
                options.cli = true;
                idx += 1;
            }
            "--help" | "-h" => {
                options.show_help = true;
                idx += 1;
            }
            "--no-auto-resume" => {
                options.no_auto_resume = true;
                idx += 1;
            }
            "--task-id" => {
                let value = args.get(idx + 1).context("missing value for --task-id")?;
                options.task_id = Some(value.clone());
                idx += 2;
            }
            "--workspace" => {
                let value = args.get(idx + 1).context("missing value for --workspace")?;
                options.workspace_id = Some(value.clone());
                idx += 2;
            }
            "--workflow" => {
                let value = args.get(idx + 1).context("missing value for --workflow")?;
                options.workflow_id = Some(value.clone());
                idx += 2;
            }
            "--name" => {
                let value = args.get(idx + 1).context("missing value for --name")?;
                options.name = Some(value.clone());
                idx += 2;
            }
            "--goal" => {
                let value = args.get(idx + 1).context("missing value for --goal")?;
                options.goal = Some(value.clone());
                idx += 2;
            }
            "--target-file" => {
                let value = args
                    .get(idx + 1)
                    .context("missing value for --target-file")?;
                options.target_files.push(value.clone());
                idx += 2;
            }
            unknown => {
                if options.cli {
                    anyhow::bail!("unknown argument: {}", unknown);
                }
                idx += 1;
            }
        }
    }

    Ok(options)
}

async fn spawn_task_runner(state: Arc<InnerState>, app: AppHandle, task_id: String) -> Result<()> {
    {
        let mut running = state.running.lock().await;
        if running.contains_key(&task_id) {
            return Ok(());
        }
        running.insert(task_id.clone(), RunningTask::new());
    }

    tokio::spawn(async move {
        let runtime = {
            let running = state.running.lock().await;
            running.get(&task_id).cloned()
        };

        if let Some(runtime) = runtime {
            let run_result =
                run_task_loop(state.clone(), Some(&app), &task_id, runtime.clone()).await;
            if let Err(err) = run_result {
                let _ = set_task_status(&state, &task_id, "failed", false);
                let _ = insert_event(
                    &state,
                    &task_id,
                    None,
                    "task_failed",
                    json!({"error": err.to_string()}),
                );
                emit_event(
                    &app,
                    &task_id,
                    None,
                    "task_failed",
                    json!({"error": err.to_string()}),
                );
            }
        }

        let mut running = state.running.lock().await;
        running.remove(&task_id);
    });

    Ok(())
}

async fn stop_task_runtime(state: Arc<InnerState>, task_id: &str, status: &str) -> Result<()> {
    let runtime = {
        let running = state.running.lock().await;
        running.get(task_id).cloned()
    };

    if let Some(runtime) = runtime {
        runtime.stop_flag.store(true, Ordering::SeqCst);
        kill_current_child(&runtime).await;
    }

    set_task_status(&state, task_id, status, false)?;
    insert_event(
        &state,
        task_id,
        None,
        "task_control",
        json!({"status": status}),
    )?;
    Ok(())
}

async fn run_task_loop(
    state: Arc<InnerState>,
    app: Option<&AppHandle>,
    task_id: &str,
    runtime: RunningTask,
) -> Result<()> {
    set_task_status(&state, task_id, "running", false)?;
    let mut task_ctx = load_task_runtime_context(&state, task_id)?;

    if !task_ctx.init_done {
        if let Some(step) = task_ctx.execution_plan.step(WorkflowStepType::InitOnce) {
            if let Some(anchor_item_id) = first_task_item_id(&state, task_id)? {
                insert_event(
                    &state,
                    task_id,
                    Some(&anchor_item_id),
                    "step_started",
                    json!({"step":"init_once"}),
                )?;
                let (_, init_cmd) = build_phase_command(step, ".", &[])?;
                let init_result = run_phase(
                    &state,
                    app,
                    task_id,
                    &anchor_item_id,
                    "init_once",
                    init_cmd,
                    &task_ctx.workspace_root,
                    &task_ctx.workspace_id,
                    &step.agent_id,
                    &runtime,
                )
                .await?;
                if !init_result.success {
                    anyhow::bail!("init_once failed: exit={}", init_result.exit_code);
                }
                insert_event(
                    &state,
                    task_id,
                    Some(&anchor_item_id),
                    "step_finished",
                    json!({"step":"init_once","exit_code":init_result.exit_code}),
                )?;
            }
        }
        task_ctx.init_done = true;
        update_task_cycle_state(&state, task_id, task_ctx.current_cycle, true)?;
    }

    'cycle: loop {
        if runtime.stop_flag.load(Ordering::SeqCst) {
            set_task_status(&state, task_id, "paused", false)?;
            insert_event(
                &state,
                task_id,
                None,
                "task_paused",
                json!({"reason":"stop_flag"}),
            )?;
            if let Some(app) = app {
                emit_event(app, task_id, None, "task_paused", json!({}));
            }
            return Ok(());
        }

        task_ctx.current_cycle += 1;
        update_task_cycle_state(&state, task_id, task_ctx.current_cycle, task_ctx.init_done)?;
        insert_event(
            &state,
            task_id,
            None,
            "cycle_started",
            json!({"cycle": task_ctx.current_cycle}),
        )?;
        if let Some(app) = app {
            emit_event(
                app,
                task_id,
                None,
                "cycle_started",
                json!({"cycle": task_ctx.current_cycle}),
            );
        }

        let items = list_task_items_for_cycle(&state, task_id)?;
        for item in items {
            process_item(&state, app, task_id, &item, &task_ctx, &runtime).await?;
            if runtime.stop_flag.load(Ordering::SeqCst) {
                continue 'cycle;
            }
        }

        let unresolved = count_unresolved_items(&state, task_id)?;
        let (should_continue, reason) = if let Some((decision, reason)) = evaluate_loop_guard_rules(
            &task_ctx.execution_plan.loop_policy,
            task_ctx.current_cycle,
            unresolved,
        ) {
            (decision, reason)
        } else if let Some(agent_id) = task_ctx
            .execution_plan
            .loop_policy
            .guard
            .agent_id
            .as_deref()
        {
            run_guard_agent_decision(
                &state,
                app,
                task_id,
                &task_ctx,
                &runtime,
                task_ctx.current_cycle,
                unresolved,
                agent_id,
            )
            .await?
        } else if task_ctx
            .execution_plan
            .loop_policy
            .guard
            .stop_when_no_unresolved
            && unresolved == 0
        {
            (false, "no_unresolved".to_string())
        } else {
            (true, "continue".to_string())
        };
        insert_event(
            &state,
            task_id,
            None,
            "loop_guard_decision",
            json!({
                "cycle": task_ctx.current_cycle,
                "continue": should_continue,
                "reason": reason,
                "unresolved_items": unresolved
            }),
        )?;
        if let Some(app) = app {
            emit_event(
                app,
                task_id,
                None,
                "loop_guard_decision",
                json!({
                    "cycle": task_ctx.current_cycle,
                    "continue": should_continue,
                    "reason": reason,
                    "unresolved_items": unresolved
                }),
            );
        }
        if !should_continue {
            break;
        }
    }

    let unresolved = count_unresolved_items(&state, task_id)?;

    if unresolved > 0 {
        set_task_status(&state, task_id, "failed", true)?;
        insert_event(
            &state,
            task_id,
            None,
            "task_failed",
            json!({"unresolved_items": unresolved}),
        )?;
        if let Some(app) = app {
            emit_event(
                app,
                task_id,
                None,
                "task_failed",
                json!({"unresolved_items": unresolved}),
            );
        }
    } else {
        set_task_status(&state, task_id, "completed", true)?;
        insert_event(&state, task_id, None, "task_completed", json!({}))?;
        if let Some(app) = app {
            emit_event(app, task_id, None, "task_completed", json!({}));
        }
    }

    Ok(())
}

fn evaluate_loop_guard_rules(
    loop_policy: &WorkflowLoopConfig,
    current_cycle: u32,
    _unresolved: i64,
) -> Option<(bool, String)> {
    match loop_policy.mode {
        LoopMode::Once => Some((false, "once_mode".to_string())),
        LoopMode::Infinite => {
            if !loop_policy.guard.enabled {
                return Some((true, "guard_disabled".to_string()));
            }
            if let Some(max_cycles) = loop_policy.guard.max_cycles {
                if current_cycle >= max_cycles {
                    return Some((false, "max_cycles_reached".to_string()));
                }
            }
            None
        }
    }
}

fn parse_guard_agent_decision(output: &str) -> Option<bool> {
    for line in output.lines().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(json_value) = serde_json::from_str::<Value>(trimmed) {
            if let Some(decision) = json_value.get("decision").and_then(Value::as_str) {
                let decision = decision.trim().to_ascii_lowercase();
                if matches!(
                    decision.as_str(),
                    "continue" | "cont" | "true" | "yes" | "1"
                ) {
                    return Some(true);
                }
                if matches!(decision.as_str(), "stop" | "halt" | "false" | "no" | "0") {
                    return Some(false);
                }
            }
        }
        let normalized = trimmed.to_ascii_lowercase();
        if matches!(
            normalized.as_str(),
            "continue" | "cont" | "true" | "yes" | "1"
        ) {
            return Some(true);
        }
        if matches!(normalized.as_str(), "stop" | "halt" | "false" | "no" | "0") {
            return Some(false);
        }
    }
    None
}

fn render_loop_guard_template(
    template: &str,
    task_id: &str,
    cycle: u32,
    unresolved_items: i64,
) -> String {
    template
        .replace("{task_id}", task_id)
        .replace("{cycle}", &cycle.to_string())
        .replace("{unresolved_items}", &unresolved_items.to_string())
}

async fn run_guard_agent_decision(
    state: &Arc<InnerState>,
    app: Option<&AppHandle>,
    task_id: &str,
    task_ctx: &TaskRuntimeContext,
    runtime: &RunningTask,
    current_cycle: u32,
    unresolved: i64,
    agent_id: &str,
) -> Result<(bool, String)> {
    let Some(anchor_item_id) = first_task_item_id(state, task_id)? else {
        anyhow::bail!(
            "task '{}' has no task items for loop guard decision",
            task_id
        );
    };
    let template = task_ctx
        .execution_plan
        .loop_policy
        .guard
        .agent_template
        .as_deref()
        .with_context(|| {
            format!(
                "workflow loop guard template is missing for agent '{}'",
                agent_id
            )
        })?;
    let command = render_loop_guard_template(template, task_id, current_cycle, unresolved);
    let result = run_phase(
        state,
        app,
        task_id,
        &anchor_item_id,
        "loop_guard",
        command,
        &task_ctx.workspace_root,
        &task_ctx.workspace_id,
        agent_id,
        runtime,
    )
    .await?;
    if !result.success {
        anyhow::bail!("loop_guard failed: exit={}", result.exit_code);
    }
    let stdout = std::fs::read_to_string(&result.stdout_path).unwrap_or_default();
    let stderr = std::fs::read_to_string(&result.stderr_path).unwrap_or_default();
    let combined = format!("{}\n{}", stdout, stderr);
    let decision = parse_guard_agent_decision(&combined).with_context(|| {
        format!(
            "loop_guard output is invalid, expected continue/stop in {} or {}",
            result.stdout_path, result.stderr_path
        )
    })?;
    let reason = if decision {
        "guard_agent_continue"
    } else {
        "guard_agent_stop"
    };
    Ok((decision, reason.to_string()))
}

async fn process_item(
    state: &Arc<InnerState>,
    app: Option<&AppHandle>,
    task_id: &str,
    item: &TaskItemRow,
    task_ctx: &TaskRuntimeContext,
    runtime: &RunningTask,
) -> Result<()> {
    let item_id = item.id.as_str();
    let qa_step = task_ctx.execution_plan.step(WorkflowStepType::Qa);
    let fix_step = task_ctx.execution_plan.step(WorkflowStepType::Fix);
    let retest_step = task_ctx.execution_plan.step(WorkflowStepType::Retest);
    let mut active_tickets: Vec<String>;
    let mut qa_failed = false;

    if let Some(qa_step) = qa_step {
        update_task_item(
            state,
            item_id,
            "qa_running",
            None,
            None,
            Some(false),
            Some(false),
            Some(""),
            true,
            false,
        )?;
        let before_tickets = list_ticket_files(task_ctx)?;
        let (qa_agent_id, qa_cmd) = build_phase_command(qa_step, &item.qa_file_path, &[])?;
        let qa_result = run_phase(
            state,
            app,
            task_id,
            item_id,
            "qa",
            qa_cmd,
            &task_ctx.workspace_root,
            &task_ctx.workspace_id,
            &qa_agent_id,
            runtime,
        )
        .await?;
        let after_tickets = list_ticket_files(task_ctx)?;
        active_tickets = new_ticket_diff(&before_tickets, &after_tickets);
        if qa_result.success && active_tickets.is_empty() {
            update_task_item(
                state,
                item_id,
                "qa_passed",
                Some(&[]),
                Some(&[]),
                Some(false),
                Some(false),
                Some(""),
                false,
                true,
            )?;
            if fix_step.is_none() {
                return Ok(());
            }
        } else {
            qa_failed = true;
            if active_tickets.is_empty() {
                active_tickets = list_existing_tickets_for_item(task_ctx, &item.qa_file_path)?;
            }
            let ticket_content: Vec<Value> = active_tickets
                .iter()
                .map(|path| read_ticket_preview(task_ctx, path))
                .collect();
            update_task_item(
                state,
                item_id,
                "qa_failed",
                Some(&active_tickets),
                Some(&ticket_content),
                Some(true),
                Some(false),
                Some(&format!("qa failed: exit={}", qa_result.exit_code)),
                false,
                false,
            )?;
        }
    } else {
        active_tickets = list_existing_tickets_for_item(task_ctx, &item.qa_file_path)?;
        if active_tickets.is_empty() {
            update_task_item(
                state,
                item_id,
                "skipped",
                Some(&[]),
                Some(&[]),
                Some(false),
                Some(false),
                Some("qa disabled and no existing tickets"),
                true,
                true,
            )?;
            return Ok(());
        }
        qa_failed = true;
        let ticket_content: Vec<Value> = active_tickets
            .iter()
            .map(|path| read_ticket_preview(task_ctx, path))
            .collect();
        update_task_item(
            state,
            item_id,
            "qa_failed",
            Some(&active_tickets),
            Some(&ticket_content),
            Some(true),
            Some(false),
            Some("qa disabled; using existing tickets"),
            true,
            false,
        )?;
    }

    let Some(fix_step) = fix_step else {
        if qa_failed || !active_tickets.is_empty() {
            update_task_item(
                state,
                item_id,
                "unresolved",
                None,
                None,
                Some(true),
                Some(false),
                Some("fix disabled by workflow"),
                false,
                true,
            )?;
        }
        return Ok(());
    };
    if active_tickets.is_empty() {
        return Ok(());
    }

    update_task_item(
        state,
        item_id,
        "fix_running",
        None,
        None,
        Some(true),
        Some(false),
        Some(""),
        false,
        false,
    )?;
    let (fix_agent_id, fix_cmd) =
        build_phase_command(fix_step, &item.qa_file_path, &active_tickets)?;
    let fix_result = run_phase(
        state,
        app,
        task_id,
        item_id,
        "fix",
        fix_cmd,
        &task_ctx.workspace_root,
        &task_ctx.workspace_id,
        &fix_agent_id,
        runtime,
    )
    .await?;

    if !fix_result.success {
        update_task_item(
            state,
            item_id,
            "unresolved",
            None,
            None,
            Some(true),
            Some(false),
            Some(&format!("fix failed: exit={}", fix_result.exit_code)),
            false,
            true,
        )?;
        return Ok(());
    }

    let Some(retest_step) = retest_step else {
        update_task_item(
            state,
            item_id,
            "fixed",
            None,
            None,
            Some(true),
            Some(true),
            Some(""),
            false,
            true,
        )?;
        return Ok(());
    };

    update_task_item(
        state,
        item_id,
        "retest_running",
        None,
        None,
        Some(true),
        Some(true),
        Some(""),
        false,
        false,
    )?;

    let before_retest_tickets = list_ticket_files(task_ctx)?;
    let (retest_agent_id, retest_cmd) = build_phase_command(retest_step, &item.qa_file_path, &[])?;
    let retest_result = run_phase(
        state,
        app,
        task_id,
        item_id,
        "retest",
        retest_cmd,
        &task_ctx.workspace_root,
        &task_ctx.workspace_id,
        &retest_agent_id,
        runtime,
    )
    .await?;
    let after_retest_tickets = list_ticket_files(task_ctx)?;
    let new_retest_tickets = new_ticket_diff(&before_retest_tickets, &after_retest_tickets);

    if retest_result.success && new_retest_tickets.is_empty() {
        update_task_item(
            state,
            item_id,
            "verified",
            None,
            None,
            Some(true),
            Some(true),
            Some(""),
            false,
            true,
        )?;
    } else {
        let previews: Vec<Value> = new_retest_tickets
            .iter()
            .map(|path| read_ticket_preview(task_ctx, path))
            .collect();
        update_task_item(
            state,
            item_id,
            "unresolved",
            Some(&new_retest_tickets),
            Some(&previews),
            Some(true),
            Some(false),
            Some("retest still failing"),
            false,
            true,
        )?;
    }

    Ok(())
}

async fn run_phase(
    state: &Arc<InnerState>,
    app: Option<&AppHandle>,
    task_id: &str,
    task_item_id: &str,
    phase: &str,
    command: String,
    workspace_root: &Path,
    workspace_id: &str,
    agent_id: &str,
    runtime: &RunningTask,
) -> Result<RunResult> {
    let run_id = Uuid::new_v4().to_string();
    let started_at = now_ts();
    let stdout_path = state
        .logs_dir
        .join(format!("{}-{}-stdout.log", phase, &run_id))
        .to_string_lossy()
        .to_string();
    let stderr_path = state
        .logs_dir
        .join(format!("{}-{}-stderr.log", phase, &run_id))
        .to_string_lossy()
        .to_string();

    {
        let conn = open_conn(&state.db_path)?;
        conn.execute(
            "INSERT INTO command_runs (id, task_item_id, phase, command, cwd, workspace_id, agent_id, exit_code, stdout_path, stderr_path, started_at, ended_at, interrupted)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, ?8, ?9, ?10, NULL, 0)",
            params![
                run_id,
                task_item_id,
                phase,
                command,
                workspace_root.to_string_lossy().to_string(),
                workspace_id,
                agent_id,
                stdout_path,
                stderr_path,
                started_at,
            ],
        )?;
    }

    insert_event(
        state,
        task_id,
        Some(task_item_id),
        "command_started",
        json!({"phase": phase, "run_id": run_id, "command": command, "workspace_id": workspace_id, "agent_id": agent_id}),
    )?;
    if let Some(app) = app {
        emit_event(
            app,
            task_id,
            Some(task_item_id),
            "command_started",
            json!({"phase": phase, "run_id": run_id, "workspace_id": workspace_id, "agent_id": agent_id}),
        );
    }

    let (shell, shell_arg) = {
        let active = read_active_config(state)?;
        (
            active.config.runner.shell.clone(),
            active.config.runner.shell_arg.clone(),
        )
    };
    let mut child = Command::new(&shell)
        .arg(&shell_arg)
        .arg(&command)
        .current_dir(workspace_root)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn command: {}", command))?;

    let stdout_stream = child
        .stdout
        .take()
        .context("failed to capture stdout stream")?;
    let stderr_stream = child
        .stderr
        .take()
        .context("failed to capture stderr stream")?;

    let stdout_file_path = stdout_path.clone();
    let stderr_file_path = stderr_path.clone();
    let app_for_stdout = app.cloned();
    let app_for_stderr = app.cloned();
    let task_id_for_stdout = task_id.to_string();
    let task_id_for_stderr = task_id.to_string();
    let task_item_id_for_stdout = task_item_id.to_string();
    let task_item_id_for_stderr = task_item_id.to_string();
    let phase_for_stdout = phase.to_string();
    let phase_for_stderr = phase.to_string();
    let run_id_for_stdout = run_id.clone();
    let run_id_for_stderr = run_id.clone();

    let stdout_task = tokio::spawn(async move {
        let mut out_file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(stdout_file_path)
            .await
            .context("failed opening stdout log file")?;
        let mut lines = BufReader::new(stdout_stream).lines();
        while let Some(line) = lines.next_line().await.context("reading stdout line")? {
            use tokio::io::AsyncWriteExt;
            out_file
                .write_all(format!("{}\n", line).as_bytes())
                .await
                .context("writing stdout line")?;
            if let Some(app_handle) = &app_for_stdout {
                emit_event(
                    app_handle,
                    &task_id_for_stdout,
                    Some(&task_item_id_for_stdout),
                    "log_chunk",
                    json!({
                        "run_id": run_id_for_stdout.clone(),
                        "phase": phase_for_stdout.clone(),
                        "stream": "stdout",
                        "line": line,
                    }),
                );
            }
        }
        Result::<()>::Ok(())
    });

    let stderr_task = tokio::spawn(async move {
        let mut err_file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(stderr_file_path)
            .await
            .context("failed opening stderr log file")?;
        let mut lines = BufReader::new(stderr_stream).lines();
        while let Some(line) = lines.next_line().await.context("reading stderr line")? {
            use tokio::io::AsyncWriteExt;
            err_file
                .write_all(format!("{}\n", line).as_bytes())
                .await
                .context("writing stderr line")?;
            if let Some(app_handle) = &app_for_stderr {
                emit_event(
                    app_handle,
                    &task_id_for_stderr,
                    Some(&task_item_id_for_stderr),
                    "log_chunk",
                    json!({
                        "run_id": run_id_for_stderr.clone(),
                        "phase": phase_for_stderr.clone(),
                        "stream": "stderr",
                        "line": line,
                    }),
                );
            }
        }
        Result::<()>::Ok(())
    });

    {
        let mut slot = runtime.child.lock().await;
        *slot = Some(child);
    }

    let mut interrupted = false;
    let exit_code = loop {
        if runtime.stop_flag.load(Ordering::SeqCst) {
            interrupted = true;
            kill_current_child(runtime).await;
        }

        let status = {
            let mut slot = runtime.child.lock().await;
            if let Some(child) = slot.as_mut() {
                child.try_wait().context("query command status")?
            } else {
                None
            }
        };

        if let Some(status) = status {
            break status.code().unwrap_or(1) as i64;
        }

        let is_empty = {
            let slot = runtime.child.lock().await;
            slot.is_none()
        };
        if is_empty {
            break 1;
        }

        sleep(Duration::from_millis(350)).await;
    };

    {
        let mut slot = runtime.child.lock().await;
        *slot = None;
    }

    let _ = stdout_task.await;
    let _ = stderr_task.await;

    let ended_at = now_ts();
    {
        let conn = open_conn(&state.db_path)?;
        conn.execute(
            "UPDATE command_runs SET exit_code=?2, ended_at=?3, interrupted=?4 WHERE id=?1",
            params![run_id, exit_code, ended_at, if interrupted { 1 } else { 0 }],
        )?;
    }

    insert_event(
        state,
        task_id,
        Some(task_item_id),
        "command_finished",
        json!({
            "phase": phase,
            "run_id": run_id,
            "exit_code": exit_code,
            "interrupted": interrupted
        }),
    )?;
    if let Some(app) = app {
        emit_event(
            app,
            task_id,
            Some(task_item_id),
            "command_finished",
            json!({"phase": phase, "run_id": run_id, "exit_code": exit_code}),
        );
    }

    Ok(RunResult {
        success: exit_code == 0 && !interrupted,
        exit_code,
        stdout_path,
        stderr_path,
    })
}

async fn kill_current_child(runtime: &RunningTask) {
    let mut slot = runtime.child.lock().await;
    if let Some(child) = slot.as_mut() {
        let _ = child.kill().await;
    }
}

fn reset_task_item_for_retry(state: &InnerState, task_item_id: &str) -> Result<String> {
    let conn = open_conn(&state.db_path)?;
    let task_id: String = conn.query_row(
        "SELECT task_id FROM task_items WHERE id = ?1",
        params![task_item_id],
        |row| row.get(0),
    )?;

    conn.execute(
        "UPDATE task_items SET status='pending', ticket_files_json='[]', ticket_content_json='[]', fix_required=0, fixed=0, last_error='', completed_at=NULL, updated_at=?2 WHERE id=?1",
        params![task_item_id, now_ts()],
    )?;

    conn.execute(
        "UPDATE tasks SET status='running', completed_at=NULL, updated_at=?2 WHERE id=?1",
        params![task_id, now_ts()],
    )?;

    Ok(task_id)
}

fn resolve_cli_task_id(state: &InnerState, options: &CliOptions) -> Result<String> {
    if let Some(task_id) = &options.task_id {
        let _ = load_task_summary(state, task_id)?;
        return Ok(task_id.clone());
    }

    if !options.no_auto_resume {
        if let Some(task_id) = find_latest_resumable_task_id(state, true)? {
            println!(
                "[qa-orchestrator][cli] auto-selected existing task: {}",
                task_id
            );
            return Ok(task_id);
        }
    }

    let payload = CreateTaskPayload {
        name: options.name.clone(),
        goal: options.goal.clone(),
        workspace_id: options.workspace_id.clone(),
        workflow_id: options.workflow_id.clone(),
        target_files: if options.target_files.is_empty() {
            None
        } else {
            Some(options.target_files.clone())
        },
    };
    let created = create_task_impl(state, payload)?;
    println!("[qa-orchestrator][cli] created task: {}", created.id);
    Ok(created.id)
}

async fn run_cli_mode_async(state: Arc<InnerState>, options: CliOptions) -> Result<i32> {
    let task_id = resolve_cli_task_id(&state, &options)?;
    prepare_task_for_start(&state, &task_id)?;

    let runtime = RunningTask::new();
    run_task_loop(state.clone(), None, &task_id, runtime).await?;

    let summary = load_task_summary(&state, &task_id)?;
    println!(
        "[qa-orchestrator][cli] finished task={} status={} finished={}/{} failed={}",
        summary.id,
        summary.status,
        summary.finished_items,
        summary.total_items,
        summary.failed_items
    );
    Ok(if summary.status == "completed" { 0 } else { 1 })
}

fn run_cli_mode_blocking(state: Arc<InnerState>, options: CliOptions) -> Result<i32> {
    let runtime = tokio::runtime::Runtime::new().context("failed to create tokio runtime")?;
    runtime.block_on(run_cli_mode_async(state, options))
}

fn print_startup_banner(state: &InnerState) {
    println!("[qa-orchestrator] app_root={}", state.app_root.display());
    if let Ok(active) = read_active_config(state) {
        println!(
            "[qa-orchestrator] default_workspace={}",
            active.default_workspace_id
        );
        println!(
            "[qa-orchestrator] default_workflow={}",
            active.default_workflow_id
        );
    }
    println!("[qa-orchestrator] db_path={}", state.db_path.display());
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cli_options = match parse_cli_options(&args[1..]) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("invalid arguments: {}", err);
            print_cli_help(
                args.first()
                    .map(String::as_str)
                    .unwrap_or("qa-orchestrator"),
            );
            std::process::exit(2);
        }
    };

    if cli_options.show_help {
        print_cli_help(
            args.first()
                .map(String::as_str)
                .unwrap_or("qa-orchestrator"),
        );
        std::process::exit(0);
    }

    let state = match init_state() {
        Ok(value) => value,
        Err(err) => {
            eprintln!("failed to initialize orchestrator: {}", err);
            std::process::exit(1);
        }
    };
    print_startup_banner(&state.inner);

    if cli_options.cli {
        match run_cli_mode_blocking(state.inner.clone(), cli_options) {
            Ok(code) => std::process::exit(code),
            Err(err) => {
                eprintln!("cli execution failed: {}", err);
                std::process::exit(1);
            }
        }
    }

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            get_config_overview,
            save_config_from_form,
            save_config_from_yaml,
            validate_config_yaml,
            list_config_versions,
            get_config_version,
            get_create_task_options,
            create_task,
            list_tasks,
            get_task_details,
            start_task,
            pause_task,
            resume_task,
            retry_task_item,
            stream_task_logs
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
