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
  phase: 'init_once' | 'qa' | 'fix' | 'retest' | 'loop_guard' | 'custom';
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

export interface TaskEventEnvelope {
  task_id: string;
  task_item_id: string | null;
  event_type: string;
  payload: Record<string, unknown>;
  ts: string;
}

export interface LogChunkEventPayload {
  run_id: string;
  phase: string;
  stream: 'stdout' | 'stderr';
  line: string;
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
  init_once?: string;
  qa?: string;
  fix?: string;
  retest?: string;
  loop_guard?: string;
}

export interface AgentConfig {
  templates: AgentTemplates;
}

export type WorkflowStepType = 'init_once' | 'qa' | 'fix' | 'retest';
export type StepHookEngine = 'cel';
export type StepPrehookUiMode = 'visual' | 'cel';
export type StepPrehookVisualOp = 'all' | 'any';
export type StepPrehookVisualField =
  | 'cycle'
  | 'active_ticket_count'
  | 'new_ticket_count'
  | 'qa_exit_code'
  | 'fix_exit_code'
  | 'retest_exit_code'
  | 'qa_failed'
  | 'fix_required';
export type StepPrehookVisualComparator = '>' | '>=' | '==' | '!=' | '<' | '<=';

export interface StepPrehookVisualRule {
  field: StepPrehookVisualField;
  cmp: StepPrehookVisualComparator;
  value: number | boolean;
}

export interface StepPrehookVisualExpression {
  op: StepPrehookVisualOp;
  rules: StepPrehookVisualRule[];
}

export interface StepPrehookUiConfig {
  mode?: StepPrehookUiMode;
  preset_id?: string;
  expr?: StepPrehookVisualExpression;
}

export interface StepPrehookConfig {
  engine: StepHookEngine;
  when: string;
  reason?: string;
  ui?: StepPrehookUiConfig;
}

export type WorkflowLoopMode = 'once' | 'infinite';

export interface WorkflowStepConfig {
  id: string;
  type: WorkflowStepType;
  enabled: boolean;
  agent_id?: string;
  prehook?: StepPrehookConfig;
}

export interface WorkflowLoopGuardConfig {
  enabled: boolean;
  stop_when_no_unresolved: boolean;
  max_cycles?: number;
  agent_id?: string;
}

export interface WorkflowLoopConfig {
  mode: WorkflowLoopMode;
  guard: WorkflowLoopGuardConfig;
}

export interface WorkflowFinalizeRule {
  id: string;
  engine: StepHookEngine;
  when: string;
  status: string;
  reason?: string;
}

export interface WorkflowFinalizeConfig {
  rules: WorkflowFinalizeRule[];
}

export interface WorkflowConfig {
  steps: WorkflowStepConfig[];
  loop: WorkflowLoopConfig;
  finalize?: WorkflowFinalizeConfig;
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

export interface SimulatePrehookRequest {
  expression: string;
  step?: string;
  context?: {
    cycle: number;
    active_ticket_count: number;
    new_ticket_count: number;
    qa_exit_code?: number | null;
    fix_exit_code?: number | null;
    retest_exit_code?: number | null;
    qa_failed: boolean;
    fix_required: boolean;
  };
}

export interface SimulatePrehookResult {
  result: boolean;
  expression: string;
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
