export type TaskMode = 'qa_only' | 'qa_fix' | 'qa_fix_retest';

export type TaskStatus =
  | 'pending'
  | 'running'
  | 'paused'
  | 'failed'
  | 'completed'
  | 'cancelled'
  | 'interrupted';

export type TaskItemStatus =
  | 'pending'
  | 'qa_running'
  | 'qa_passed'
  | 'qa_failed'
  | 'fix_running'
  | 'fixed'
  | 'retest_running'
  | 'verified'
  | 'unresolved'
  | 'skipped';

export interface TaskSummary {
  id: string;
  name: string;
  status: TaskStatus;
  started_at: string | null;
  completed_at: string | null;
  goal: string;
  mode: TaskMode;
  workspace_id: string;
  workflow_id: string;
  target_files: string[];
  total_items: number;
  finished_items: number;
  failed_items: number;
  created_at: string;
  updated_at: string;
}

export interface TaskItem {
  id: string;
  task_id: string;
  order_no: number;
  qa_file_path: string;
  status: TaskItemStatus;
  ticket_files: string[];
  ticket_content: Record<string, unknown>[];
  fix_required: boolean;
  fixed: boolean;
  last_error: string;
  started_at: string | null;
  completed_at: string | null;
  updated_at: string;
}

export interface CommandRun {
  id: string;
  task_item_id: string;
  phase: 'qa' | 'fix' | 'retest' | 'custom';
  command: string;
  cwd: string;
  workspace_id: string;
  agent_id: string;
  exit_code: number | null;
  stdout_path: string;
  stderr_path: string;
  started_at: string;
  ended_at: string | null;
  interrupted: boolean;
}

export interface EventRow {
  id: number;
  task_id: string;
  task_item_id: string | null;
  event_type: string;
  payload: Record<string, unknown>;
  created_at: string;
}

export interface TaskDetail {
  task: TaskSummary;
  items: TaskItem[];
  runs: CommandRun[];
  events: EventRow[];
}

export interface LogChunk {
  run_id: string;
  phase: string;
  content: string;
  stdout_path: string;
  stderr_path: string;
}

export interface CreateTaskRequest {
  name?: string;
  goal?: string;
  mode?: TaskMode;
  workspace_id?: string;
  workflow_id?: string;
  target_files?: string[];
}

export interface NamedOption {
  id: string;
}

export interface CreateTaskOptions {
  defaults: {
    workspace_id: string;
    workflow_id: string;
  };
  workspaces: NamedOption[];
  workflows: NamedOption[];
}

export interface WorkspaceConfig {
  root_path: string;
  qa_targets: string[];
  ticket_dir: string;
}

export interface AgentTemplates {
  qa?: string;
  fix?: string;
  retest?: string;
}

export interface AgentConfig {
  templates: AgentTemplates;
}

export interface WorkflowConfig {
  qa: string;
  fix?: string;
  retest?: string;
}

export interface OrchestratorConfigModel {
  runner: {
    shell: string;
    shell_arg: string;
  };
  resume: {
    auto: boolean;
  };
  defaults: {
    workspace: string;
    workflow: string;
  };
  workspaces: Record<string, WorkspaceConfig>;
  agents: Record<string, AgentConfig>;
  workflows: Record<string, WorkflowConfig>;
}

export interface ConfigOverview {
  config: OrchestratorConfigModel;
  yaml: string;
  version: number;
  updated_at: string;
}

export interface SaveConfigFormRequest {
  config: OrchestratorConfigModel;
}

export interface SaveConfigYamlRequest {
  yaml: string;
}

export interface ConfigValidationResult {
  valid: boolean;
  normalized_yaml: string;
}

export interface ConfigVersionSummary {
  version: number;
  created_at: string;
  author: string;
}

export interface ConfigVersionDetail {
  version: number;
  created_at: string;
  author: string;
  yaml: string;
}
