import { useEffect, useMemo, useRef, useState } from 'react';
import { api } from './api';
import type {
  AgentHealthInfo,
  ConfigOverview,
  ConfigVersionSummary,
  CreateTaskOptions,
  EventRow,
  LogChunkEventPayload,
  OrchestratorConfigModel,
  StepPrehookConfig,
  StepPrehookUiConfig,
  StepPrehookVisualComparator,
  StepPrehookVisualExpression,
  StepPrehookVisualField,
  StepPrehookVisualRule,
  TaskDetail,
  TaskEventEnvelope,
  TaskSummary,
  WorkflowStepConfig,
  WorkflowStepType
} from './types';

const STATUS_CLASS: Record<string, string> = {
  pending: 'badge gray',
  running: 'badge blue',
  paused: 'badge amber',
  failed: 'badge red',
  completed: 'badge green',
  interrupted: 'badge amber',
  cancelled: 'badge gray',
  qa_running: 'badge blue',
  qa_passed: 'badge green',
  qa_failed: 'badge red',
  fix_running: 'badge blue',
  fixed: 'badge green',
  retest_running: 'badge blue',
  verified: 'badge green',
  unresolved: 'badge red',
  skipped: 'badge gray'
};

type ItemFilter = 'all' | 'active' | 'unresolved' | 'completed';
type Theme = 'light' | 'dark';
type ViewTab = 'tasks' | 'config';
type ConfigFormTab = 'overview' | 'workspace' | 'agent' | 'agent_group' | 'workflow' | 'yaml';
type ConfigEntityKind = 'workspace' | 'agent' | 'agent_group' | 'workflow';
interface PendingEntitySwitch {
  kind: ConfigEntityKind;
  id: string;
}
const WORKFLOW_STEP_ORDER: WorkflowStepType[] = ['init_once', 'qa', 'ticket_scan', 'fix', 'retest'];
const CONFIG_FORM_TABS: Array<{ id: ConfigFormTab; label: string; hint: string }> = [
  { id: 'overview', label: '总览', hint: '默认入口与配置状态' },
  { id: 'workspace', label: 'Workspace', hint: '目录、目标与工单路径' },
  { id: 'agent', label: 'Agent', hint: '阶段模板与执行角色' },
  { id: 'agent_group', label: 'Agent Group', hint: 'Agent 分组与轮转策略' },
  { id: 'workflow', label: 'Workflow', hint: '步骤、Prehook 与循环策略' },
  { id: 'yaml', label: 'YAML', hint: '高级配置编辑' }
];
const ANSI_CSI_REGEX = /\u001b\[[0-?]*[ -/]*[@-~]/g;
const ANSI_OSC_REGEX = /\u001b\][^\u0007]*(?:\u0007|\u001b\\)/g;
const MAX_LOG_LINE_LENGTH = 320;

type PrehookFieldValueType = 'number' | 'boolean';
type PrehookDecision = 'run' | 'skip' | 'error';

interface PrehookFieldMeta {
  id: StepPrehookVisualField;
  label: string;
  valueType: PrehookFieldValueType;
}

interface PrehookPreset {
  id: string;
  label: string;
  description: string;
  reason?: string;
  expr: StepPrehookVisualExpression;
}

type PrehookSimulationContext = Record<StepPrehookVisualField, number | boolean>;

interface PrehookSimulationResult {
  result: boolean;
  explanation: string;
}

const PREHOOK_FIELDS: PrehookFieldMeta[] = [
  { id: 'active_ticket_count', label: 'Active Ticket Count', valueType: 'number' },
  { id: 'new_ticket_count', label: 'New Ticket Count', valueType: 'number' },
  { id: 'cycle', label: 'Cycle', valueType: 'number' },
  { id: 'qa_exit_code', label: 'QA Exit Code', valueType: 'number' },
  { id: 'fix_exit_code', label: 'Fix Exit Code', valueType: 'number' },
  { id: 'retest_exit_code', label: 'Retest Exit Code', valueType: 'number' },
  { id: 'qa_failed', label: 'QA Failed', valueType: 'boolean' },
  { id: 'fix_required', label: 'Fix Required', valueType: 'boolean' }
];

const PREHOOK_FIELD_SET = new Set(PREHOOK_FIELDS.map((field) => field.id));
const NUMBER_COMPARATORS: StepPrehookVisualComparator[] = ['>', '>=', '==', '!=', '<', '<='];
const BOOLEAN_COMPARATORS: StepPrehookVisualComparator[] = ['==', '!='];

const DEFAULT_PREHOOK_SIMULATION_CONTEXT: PrehookSimulationContext = {
  cycle: 1,
  active_ticket_count: 0,
  new_ticket_count: 0,
  qa_exit_code: 0,
  fix_exit_code: 0,
  retest_exit_code: 0,
  qa_failed: false,
  fix_required: false
};

const STEP_PREHOOK_PRESETS: Record<WorkflowStepType, PrehookPreset[]> = {
  init_once: [
    {
      id: 'always_run',
      label: 'Always run',
      description: 'No condition, this step always runs.',
      expr: { op: 'all', rules: [] },
      reason: 'run init_once by default'
    }
  ],
  qa: [
    {
      id: 'always_run',
      label: 'Always run',
      description: 'No condition, this step always runs.',
      expr: { op: 'all', rules: [] },
      reason: 'run qa by default'
    },
    {
      id: 'only_when_fix_required',
      label: 'Only if fix required',
      description: 'Run QA only when fix is required in context.',
      expr: {
        op: 'all',
        rules: [{ field: 'fix_required', cmp: '==', value: true }]
      },
      reason: 'skip qa when fix is not required'
    }
  ],
  ticket_scan: [
    {
      id: 'always_run',
      label: 'Always run',
      description: 'Scan ticket directory and map active tickets to task items.',
      expr: { op: 'all', rules: [] },
      reason: 'run ticket scan by default'
    },
    {
      id: 'has_active_tickets',
      label: 'Has active tickets',
      description: 'Run scan only when known active tickets already exist in context.',
      expr: {
        op: 'all',
        rules: [{ field: 'active_ticket_count', cmp: '>', value: 0 }]
      },
      reason: 'skip scan when no active tickets in context'
    }
  ],
  fix: [
    {
      id: 'has_active_tickets',
      label: 'Has active tickets',
      description: 'Run fix only when active tickets exist.',
      expr: {
        op: 'all',
        rules: [{ field: 'active_ticket_count', cmp: '>', value: 0 }]
      },
      reason: 'skip fix when no active tickets'
    },
    {
      id: 'qa_failed_only',
      label: 'QA failed only',
      description: 'Run fix only when QA failed.',
      expr: {
        op: 'all',
        rules: [{ field: 'qa_failed', cmp: '==', value: true }]
      },
      reason: 'skip fix when qa is not failed'
    }
  ],
  retest: [
    {
      id: 'retest_after_fix_success',
      label: 'After fix success',
      description: 'Run retest only when tickets exist and fix succeeds.',
      expr: {
        op: 'all',
        rules: [
          { field: 'active_ticket_count', cmp: '>', value: 0 },
          { field: 'fix_exit_code', cmp: '==', value: 0 }
        ]
      },
      reason: 'skip retest when fix did not run successfully'
    },
    {
      id: 'has_active_tickets',
      label: 'Has active tickets',
      description: 'Run retest only when active tickets exist.',
      expr: {
        op: 'all',
        rules: [{ field: 'active_ticket_count', cmp: '>', value: 0 }]
      },
      reason: 'skip retest when no active tickets'
    }
  ]
};

function clipLogText(value: string): string {
  if (value.length <= MAX_LOG_LINE_LENGTH) {
    return value;
  }
  return `${value.slice(0, MAX_LOG_LINE_LENGTH - 3)}...`;
}

function getFieldMeta(field: StepPrehookVisualField): PrehookFieldMeta {
  return PREHOOK_FIELDS.find((entry) => entry.id === field) ?? PREHOOK_FIELDS[0];
}

function cloneVisualExpression(expr?: StepPrehookVisualExpression): StepPrehookVisualExpression {
  if (!expr) {
    return { op: 'all', rules: [] };
  }
  return {
    op: expr.op,
    rules: expr.rules.map((rule) => ({ ...rule }))
  };
}

function toCelLiteral(value: number | boolean): string {
  if (typeof value === 'boolean') {
    return value ? 'true' : 'false';
  }
  return Number.isFinite(value) ? String(value) : '0';
}

function compileVisualExpressionToCel(expr?: StepPrehookVisualExpression): string {
  if (!expr || expr.rules.length === 0) {
    return 'true';
  }
  const connector = expr.op === 'any' ? ' || ' : ' && ';
  return expr.rules
    .map((rule) => `(${rule.field} ${rule.cmp} ${toCelLiteral(rule.value)})`)
    .join(connector);
}

function resolvePrehookMode(prehook?: StepPrehookConfig): 'visual' | 'cel' {
  if (!prehook) {
    return 'cel';
  }
  return prehook.ui?.mode === 'visual' ? 'visual' : 'cel';
}

function buildPrehookFromPreset(stepType: WorkflowStepType, presetId?: string): StepPrehookConfig {
  const presets = STEP_PREHOOK_PRESETS[stepType] ?? [];
  const preset = presets.find((entry) => entry.id === presetId) ?? presets[0];
  const expr = cloneVisualExpression(preset?.expr);
  return {
    engine: 'cel',
    when: compileVisualExpressionToCel(expr),
    reason: preset?.reason,
    ui: {
      mode: 'visual',
      preset_id: preset?.id,
      expr
    }
  };
}

function normalizePrehook(step: WorkflowStepConfig): WorkflowStepConfig {
  if (!step.prehook) {
    return step;
  }
  if (step.prehook.ui?.mode === 'visual' && step.prehook.ui.expr) {
    const normalizedExpr = cloneVisualExpression(step.prehook.ui.expr);
    return {
      ...step,
      prehook: {
        ...step.prehook,
        when: compileVisualExpressionToCel(normalizedExpr),
        ui: {
          ...(step.prehook.ui as StepPrehookUiConfig),
          expr: normalizedExpr
        }
      }
    };
  }
  return {
    ...step,
    prehook: {
      ...step.prehook,
      ui: {
        ...(step.prehook.ui as StepPrehookUiConfig),
        mode: step.prehook.ui?.mode ?? 'cel'
      }
    }
  };
}

function extractReferencedFields(
  expression: string,
  context: Record<string, unknown>
): StepPrehookVisualField[] {
  const result = new Set<StepPrehookVisualField>();

  for (const match of expression.matchAll(/\b([a-zA-Z_][a-zA-Z0-9_]*)\b/g)) {
    const token = match[1] as StepPrehookVisualField;
    if (PREHOOK_FIELD_SET.has(token) && token in context) {
      result.add(token);
    }
  }

  for (const match of expression.matchAll(/\bcontext\.([a-zA-Z_][a-zA-Z0-9_]*)\b/g)) {
    const token = match[1] as StepPrehookVisualField;
    if (PREHOOK_FIELD_SET.has(token) && token in context) {
      result.add(token);
    }
  }

  return [...result];
}

function formatSimulationExplanation(
  expression: string,
  context: PrehookSimulationContext
): string {
  const fields = extractReferencedFields(expression, context as Record<string, unknown>);
  if (fields.length === 0) {
    return 'No known context variables referenced.';
  }
  return fields
    .map((field) => `${field}=${String(context[field])}`)
    .join(', ');
}

function formatPrehookEventLine(
  payload: Record<string, unknown>,
  createdAt?: string
): string | null {
  const step = typeof payload.step === 'string' ? payload.step : null;
  const decisionRaw = typeof payload.decision === 'string' ? payload.decision : null;
  const reason = typeof payload.reason === 'string' ? payload.reason : '';
  const when = typeof payload.when === 'string' ? payload.when : '';
  const context =
    payload.context && typeof payload.context === 'object'
      ? (payload.context as Record<string, unknown>)
      : {};

  if (!step || !decisionRaw) {
    return null;
  }
  const decision = (decisionRaw as PrehookDecision) || 'error';
  const prefix = createdAt ? `${createdAt.slice(11, 19)} ` : '';
  const fields = extractReferencedFields(when, context);
  const values = fields.map((field) => `${field}=${String(context[field])}`).join(', ');
  const detail = [
    `${prefix}[prehook][${step}] ${decision.toUpperCase()}: ${reason || 'no reason provided'}`,
    when ? `when: ${when}` : '',
    values ? `values: ${values}` : ''
  ]
    .filter(Boolean)
    .join(' | ');

  return clipLogText(detail);
}

function formatFinalizeEventLine(
  payload: Record<string, unknown>,
  createdAt?: string
): string | null {
  const status = typeof payload.status === 'string' ? payload.status : null;
  const reason = typeof payload.reason === 'string' ? payload.reason : '';
  const ruleId = typeof payload.rule_id === 'string' ? payload.rule_id : '';
  if (!status) {
    return null;
  }
  const prefix = createdAt ? `${createdAt.slice(11, 19)} ` : '';
  return clipLogText(
    `${prefix}[finalize] status=${status}${ruleId ? ` rule=${ruleId}` : ''}${
      reason ? ` reason=${reason}` : ''
    }`
  );
}

function formatRuleEventRows(events: EventRow[]): string[] {
  return events
    .map((event) => {
      if (event.event_type === 'step_prehook_evaluated') {
        return formatPrehookEventLine(event.payload, event.created_at);
      }
      if (event.event_type === 'item_finalize_evaluated') {
        return formatFinalizeEventLine(event.payload, event.created_at);
      }
      return null;
    })
    .filter((line): line is string => Boolean(line));
}

function cloneConfig(config: OrchestratorConfigModel): OrchestratorConfigModel {
  return JSON.parse(JSON.stringify(config)) as OrchestratorConfigModel;
}

function defaultWorkflowSteps(firstGroupId: string) {
  return [
    { id: 'init_once', type: 'init_once' as const, enabled: false, agent_group_id: undefined },
    { id: 'qa', type: 'qa' as const, enabled: Boolean(firstGroupId), agent_group_id: firstGroupId || undefined },
    { id: 'ticket_scan', type: 'ticket_scan' as const, enabled: false, agent_group_id: undefined },
    { id: 'fix', type: 'fix' as const, enabled: false, agent_group_id: undefined },
    { id: 'retest', type: 'retest' as const, enabled: false, agent_group_id: undefined }
  ];
}

function ensureWorkflowShape(config: OrchestratorConfigModel) {
  if (!config.agent_groups) {
    config.agent_groups = {};
  }
  for (const workflow of Object.values(config.workflows)) {
    if (!workflow.steps) {
      workflow.steps = defaultWorkflowSteps('');
    }
    const byType = new Map(workflow.steps.map((step) => [step.type, step]));
    workflow.steps = WORKFLOW_STEP_ORDER.map((stepType) => {
      const existing = byType.get(stepType);
      if (existing) {
        return normalizePrehook({ ...existing, id: existing.id || stepType });
      }
      return { id: stepType, type: stepType, enabled: false, agent_group_id: undefined };
    });
    workflow.loop = workflow.loop ?? {
      mode: 'once',
      guard: { enabled: true, stop_when_no_unresolved: true, agent_group_id: undefined }
    };
    workflow.loop.guard = workflow.loop.guard ?? {
      enabled: true,
      stop_when_no_unresolved: true,
      agent_group_id: undefined
    };
  }
}

function parseLogChunkPayload(payload: Record<string, unknown>): LogChunkEventPayload | null {
  const runId = typeof payload.run_id === 'string' ? payload.run_id : null;
  const phase = typeof payload.phase === 'string' ? payload.phase : null;
  const stream = payload.stream === 'stdout' || payload.stream === 'stderr' ? payload.stream : null;
  const line = typeof payload.line === 'string' ? payload.line : null;

  if (!runId || !phase || !stream || line === null) {
    return null;
  }

  return {
    run_id: runId,
    phase,
    stream,
    line
  };
}

function stripAnsi(value: string): string {
  return value.replace(ANSI_OSC_REGEX, '').replace(ANSI_CSI_REGEX, '').replace(/\r/g, '');
}

function formatLogLine({
  runId,
  phase,
  stream,
  line
}: {
  runId: string;
  phase: string;
  stream: 'stdout' | 'stderr';
  line: string;
}): string | null {
  const clean = stripAnsi(line).trimEnd();
  if (!clean.trim()) {
    return null;
  }
  const clipped = clipLogText(clean);
  const shortRunId = runId.slice(0, 8);
  const streamMark = stream === 'stderr' ? ' !' : '';
  return `[${shortRunId}][${phase}${streamMark}] ${clipped}`;
}

function formatLogChunkForDisplay(chunk: { run_id: string; phase: string; content: string }): string {
  const lines = chunk.content.split(/\r?\n/);
  const rendered: string[] = [];
  let currentStream: 'stdout' | 'stderr' = 'stdout';

  for (const rawLine of lines) {
    const normalized = stripAnsi(rawLine).trimEnd();
    if (!normalized.trim()) {
      continue;
    }
    if (normalized === `[${chunk.run_id}][${chunk.phase}]`) {
      continue;
    }
    if (normalized === '[stderr]') {
      currentStream = 'stderr';
      continue;
    }
    const formatted = formatLogLine({
      runId: chunk.run_id,
      phase: chunk.phase,
      stream: currentStream,
      line: normalized
    });
    if (formatted) {
      rendered.push(formatted);
    }
  }

  return rendered.join('\n');
}

export function App() {
  const [viewTab, setViewTab] = useState<ViewTab>('tasks');

  const [tasks, setTasks] = useState<TaskSummary[]>([]);
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null);
  const [detail, setDetail] = useState<TaskDetail | null>(null);
  const [logs, setLogs] = useState<string>('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>('');

  const [name, setName] = useState('');
  const [goal, setGoal] = useState('Automate QA sprint with auto-fix and restart');
  const [workspaceId, setWorkspaceId] = useState('');
  const [workflowId, setWorkflowId] = useState('');
  const [targets, setTargets] = useState('');
  const [createOptions, setCreateOptions] = useState<CreateTaskOptions | null>(null);

  const [itemFilter, setItemFilter] = useState<ItemFilter>('all');
  const [itemQuery, setItemQuery] = useState('');
  const [theme, setTheme] = useState<Theme>('light');
  const [isCreateModalOpen, setIsCreateModalOpen] = useState(false);
  const [isInfoOverlayOpen, setIsInfoOverlayOpen] = useState(false);
  const [isItemsPanelOpen, setIsItemsPanelOpen] = useState(false);

  const [configTab, setConfigTab] = useState<ConfigFormTab>('overview');
  const [configOverview, setConfigOverview] = useState<ConfigOverview | null>(null);
  const [configDraft, setConfigDraft] = useState<OrchestratorConfigModel | null>(null);
  const [configSavedSnapshot, setConfigSavedSnapshot] = useState<OrchestratorConfigModel | null>(null);
  const [yamlDraft, setYamlDraft] = useState('');
  const [configVersions, setConfigVersions] = useState<ConfigVersionSummary[]>([]);
  const [isConfigVersionsOpen, setIsConfigVersionsOpen] = useState(false);
  const [selectedWorkspaceId, setSelectedWorkspaceId] = useState<string | null>(null);
  const [selectedAgentId, setSelectedAgentId] = useState<string | null>(null);
  const [selectedWorkflowId, setSelectedWorkflowId] = useState<string | null>(null);
  const [selectedAgentGroupId, setSelectedAgentGroupId] = useState<string | null>(null);
  const [agentHealthList, setAgentHealthList] = useState<AgentHealthInfo[]>([]);
  const [agentGroupAddFilter, setAgentGroupAddFilter] = useState('');
  const [agentGroupDropdownOpen, setAgentGroupDropdownOpen] = useState(false);
  const [pendingRemoveAgentId, setPendingRemoveAgentId] = useState<string | null>(null);
  const agentGroupDropdownRef = useRef<HTMLDivElement>(null);
  const [pendingEntitySwitch, setPendingEntitySwitch] = useState<PendingEntitySwitch | null>(null);
  const [entitySwitchBusy, setEntitySwitchBusy] = useState(false);
  const [configBusy, setConfigBusy] = useState(false);
  const [configMessage, setConfigMessage] = useState('');
  const [prehookSimulationInputs, setPrehookSimulationInputs] = useState<
    Record<string, PrehookSimulationContext>
  >({});
  const [prehookSimulationResults, setPrehookSimulationResults] = useState<
    Record<string, PrehookSimulationResult | { error: string }>
  >({});

  const selectedTaskIdRef = useRef<string | null>(null);
  const viewTabRef = useRef<ViewTab>('tasks');
  const realtimeRefreshTimerRef = useRef<number | null>(null);
  const realtimeRefreshPendingRef = useRef<{ tasks: boolean; detail: boolean }>({
    tasks: false,
    detail: false
  });

  const selectedTask = useMemo(
    () => tasks.find((task) => task.id === selectedTaskId) ?? null,
    [tasks, selectedTaskId]
  );

  const taskStats = useMemo(() => {
    const running = tasks.filter((task) => task.status === 'running').length;
    const failed = tasks.filter((task) => task.status === 'failed').length;
    const completed = tasks.filter((task) => task.status === 'completed').length;
    return { total: tasks.length, running, failed, completed };
  }, [tasks]);

  const itemStats = useMemo(() => {
    const items = detail?.items ?? [];
    const unresolved = items.filter((item) => ['unresolved', 'qa_failed'].includes(item.status)).length;
    const active = items.filter((item) => item.status.endsWith('_running') || item.status === 'pending').length;
    const completed = items.filter((item) => ['qa_passed', 'fixed', 'verified'].includes(item.status)).length;
    return { total: items.length, unresolved, active, completed };
  }, [detail]);

  const runStats = useMemo(() => {
    const counts = { init_once: 0, qa: 0, fix: 0, retest: 0, loop_guard: 0, custom: 0 };
    for (const run of detail?.runs ?? []) {
      if (run.phase in counts) {
        counts[run.phase as keyof typeof counts] += 1;
      }
    }
    return counts;
  }, [detail]);

  const ticketList = useMemo(() => {
    const map = new Map<string, { path: string; status: string; source: string }>();
    for (const item of detail?.items ?? []) {
      for (const ticketFile of item.ticket_files) {
        map.set(ticketFile, {
          path: ticketFile,
          status: item.status,
          source: item.qa_file_path
        });
      }
    }
    return [...map.values()];
  }, [detail]);

  const filteredItems = useMemo(() => {
    let items = detail?.items ?? [];

    if (itemFilter === 'active') {
      items = items.filter((item) => item.status.endsWith('_running') || item.status === 'pending');
    } else if (itemFilter === 'unresolved') {
      items = items.filter((item) => ['unresolved', 'qa_failed'].includes(item.status));
    } else if (itemFilter === 'completed') {
      items = items.filter((item) => ['qa_passed', 'fixed', 'verified'].includes(item.status));
    }

    const query = itemQuery.trim().toLowerCase();
    if (query) {
      items = items.filter((item) => item.qa_file_path.toLowerCase().includes(query));
    }

    return items;
  }, [detail, itemFilter, itemQuery]);

  const workspaceKeys = useMemo(
    () => Object.keys(configDraft?.workspaces ?? {}).sort(),
    [configDraft]
  );
  const workflowKeys = useMemo(
    () => Object.keys(configDraft?.workflows ?? {}).sort(),
    [configDraft]
  );
  const agentKeys = useMemo(
    () => Object.keys(configDraft?.agents ?? {}).sort(),
    [configDraft]
  );
  const agentGroupKeys = useMemo(
    () => Object.keys(configDraft?.agent_groups ?? {}).sort(),
    [configDraft]
  );
  const selectedAgentIndex = useMemo(
    () => (selectedAgentId ? agentKeys.indexOf(selectedAgentId) : -1),
    [agentKeys, selectedAgentId]
  );
  const selectedAgentGroupIndex = useMemo(
    () => (selectedAgentGroupId ? agentGroupKeys.indexOf(selectedAgentGroupId) : -1),
    [agentGroupKeys, selectedAgentGroupId]
  );
  const selectedWorkspaceIndex = useMemo(
    () => (selectedWorkspaceId ? workspaceKeys.indexOf(selectedWorkspaceId) : -1),
    [selectedWorkspaceId, workspaceKeys]
  );
  const selectedWorkflowIndex = useMemo(
    () => (selectedWorkflowId ? workflowKeys.indexOf(selectedWorkflowId) : -1),
    [selectedWorkflowId, workflowKeys]
  );
  const selectedWorkspaceDirty = useMemo(() => {
    if (!selectedWorkspaceId || !configDraft || !configSavedSnapshot) {
      return false;
    }
    return JSON.stringify(configDraft.workspaces[selectedWorkspaceId] ?? null) !==
      JSON.stringify(configSavedSnapshot.workspaces[selectedWorkspaceId] ?? null);
  }, [configDraft, configSavedSnapshot, selectedWorkspaceId]);
  const selectedAgentDirty = useMemo(() => {
    if (!selectedAgentId || !configDraft || !configSavedSnapshot) {
      return false;
    }
    return JSON.stringify(configDraft.agents[selectedAgentId] ?? null) !==
      JSON.stringify(configSavedSnapshot.agents[selectedAgentId] ?? null);
  }, [configDraft, configSavedSnapshot, selectedAgentId]);
  const selectedAgentGroupDirty = useMemo(() => {
    if (!selectedAgentGroupId || !configDraft || !configSavedSnapshot) {
      return false;
    }
    return JSON.stringify(configDraft.agent_groups[selectedAgentGroupId] ?? null) !==
      JSON.stringify(configSavedSnapshot.agent_groups[selectedAgentGroupId] ?? null);
  }, [configDraft, configSavedSnapshot, selectedAgentGroupId]);
  const selectedWorkflowDirty = useMemo(() => {
    if (!selectedWorkflowId || !configDraft || !configSavedSnapshot) {
      return false;
    }
    return JSON.stringify(configDraft.workflows[selectedWorkflowId] ?? null) !==
      JSON.stringify(configSavedSnapshot.workflows[selectedWorkflowId] ?? null);
  }, [configDraft, configSavedSnapshot, selectedWorkflowId]);

  async function withTimeout<T>(promise: Promise<T>, ms: number, label: string): Promise<T> {
    return new Promise<T>((resolve, reject) => {
      const timer = window.setTimeout(() => {
        reject(new Error(`${label} timed out after ${ms}ms`));
      }, ms);
      promise
        .then((value) => {
          window.clearTimeout(timer);
          resolve(value);
        })
        .catch((err) => {
          window.clearTimeout(timer);
          reject(err);
        });
    });
  }

  async function loadTasks() {
    const data = await api.listTasks();
    setTasks(data);
    if (!selectedTaskIdRef.current && data.length > 0) {
      setSelectedTaskId(data[0].id);
    }
  }

  async function loadTaskLogs(taskId: string, prehookLines: string[] = []) {
    const chunks = await api.streamTaskLogs(taskId, 350);
    const commandLines = chunks
      .map((chunk) => formatLogChunkForDisplay(chunk))
      .filter((block) => block.length > 0)
      .join('\n');
    const mergedLines = [commandLines, ...prehookLines].filter(Boolean).join('\n');
    setLogs(mergedLines);
  }

  async function loadTaskDetails(taskId: string, includeLogs = true) {
    const data = await api.getTaskDetails(taskId);
    setDetail(data);
    if (includeLogs) {
      await loadTaskLogs(taskId, formatRuleEventRows(data.events));
    }
  }

  async function refreshSnapshot(forceTaskId?: string, includeLogs = true) {
    const latest = await api.listTasks();
    setTasks(latest);

    const currentSelectedTaskId = selectedTaskIdRef.current;
    const focusId = forceTaskId ?? currentSelectedTaskId ?? latest[0]?.id ?? null;
    if (focusId) {
      if (!currentSelectedTaskId) {
        setSelectedTaskId(focusId);
      }
      await loadTaskDetails(focusId, includeLogs);
    }
  }

  function scheduleRealtimeRefresh(options: { tasks?: boolean; detail?: boolean }) {
    const pending = realtimeRefreshPendingRef.current;
    if (options.tasks) {
      pending.tasks = true;
    }
    if (options.detail) {
      pending.detail = true;
    }
    if (realtimeRefreshTimerRef.current !== null) {
      return;
    }
    realtimeRefreshTimerRef.current = window.setTimeout(() => {
      realtimeRefreshTimerRef.current = null;
      const run = realtimeRefreshPendingRef.current;
      realtimeRefreshPendingRef.current = { tasks: false, detail: false };

      Promise.resolve()
        .then(async () => {
          if (run.tasks) {
            await loadTasks();
          }
          const currentTaskId = selectedTaskIdRef.current;
          if (run.detail && currentTaskId && viewTabRef.current === 'tasks') {
            await loadTaskDetails(currentTaskId, false);
          }
        })
        .catch((err) => setError(String(err)));
    }, 150);
  }

  function appendRealtimeLog(event: TaskEventEnvelope) {
    if (!selectedTaskIdRef.current || event.task_id !== selectedTaskIdRef.current) {
      return;
    }
    const payload = parseLogChunkPayload(event.payload);
    if (!payload) {
      return;
    }
    const nextLine = formatLogLine({
      runId: payload.run_id,
      phase: payload.phase,
      stream: payload.stream,
      line: payload.line
    });
    if (!nextLine) {
      return;
    }
    setLogs((current) => (current ? `${current}\n${nextLine}` : nextLine));
  }

  function appendRealtimeRuleEvent(event: TaskEventEnvelope) {
    if (!selectedTaskIdRef.current || event.task_id !== selectedTaskIdRef.current) {
      return;
    }
    const line =
      event.event_type === 'step_prehook_evaluated'
        ? formatPrehookEventLine(event.payload)
        : formatFinalizeEventLine(event.payload);
    if (!line) {
      return;
    }
    setLogs((current) => (current ? `${current}\n${line}` : line));
  }

  function handleRealtimeEvent(event: TaskEventEnvelope) {
    if (event.event_type === 'log_chunk') {
      if (viewTabRef.current === 'tasks') {
        appendRealtimeLog(event);
      }
      return;
    }
    if (event.event_type === 'task_deleted') {
      const selectedId = selectedTaskIdRef.current;
      if (selectedId && selectedId === event.task_id) {
        setSelectedTaskId(null);
        setDetail(null);
        setLogs('');
      }
      scheduleRealtimeRefresh({ tasks: true, detail: false });
      return;
    }
    if (event.event_type === 'step_prehook_evaluated') {
      if (viewTabRef.current === 'tasks') {
        appendRealtimeRuleEvent(event);
      }
    }
    if (event.event_type === 'item_finalize_evaluated') {
      if (viewTabRef.current === 'tasks') {
        appendRealtimeRuleEvent(event);
      }
    }
    if (event.event_type === 'agent_health_changed') {
      const p = event.payload as { agent_id: string; healthy: boolean; diseased_until: string | null; consecutive_errors: number };
      setAgentHealthList((prev) => {
        const idx = prev.findIndex((h) => h.agent_id === p.agent_id);
        const updated: AgentHealthInfo = { agent_id: p.agent_id, healthy: p.healthy, diseased_until: p.diseased_until, consecutive_errors: p.consecutive_errors };
        if (idx >= 0) {
          const next = [...prev];
          next[idx] = updated;
          return next;
        }
        return [...prev, updated];
      });
      return;
    }
    const selectedId = selectedTaskIdRef.current;
    const isSelectedTask = Boolean(selectedId && event.task_id === selectedId);
    scheduleRealtimeRefresh({
      tasks: true,
      detail: isSelectedTask
    });
  }

  async function loadCreateTaskOptions() {
    const options = await api.getCreateTaskOptions();
    setCreateOptions(options);
    setWorkspaceId((current) => current || options.defaults.workspace_id);
    setWorkflowId((current) => current || options.defaults.workflow_id);
  }

  function resetCreateTaskForm(options?: CreateTaskOptions | null) {
    const resolved = options ?? createOptions;
    setName('');
    setGoal('Automate QA sprint with auto-fix and restart');
    setTargets('');
    setWorkspaceId(resolved?.defaults.workspace_id ?? '');
    setWorkflowId(resolved?.defaults.workflow_id ?? '');
  }

  function closeCreateModal() {
    resetCreateTaskForm();
    setIsCreateModalOpen(false);
  }

  async function loadConfigOverview() {
    const overview = await api.getConfigOverview();
    const normalized = cloneConfig(overview.config);
    ensureWorkflowShape(normalized);
    setConfigOverview(overview);
    setConfigDraft(normalized);
    setConfigSavedSnapshot(cloneConfig(normalized));
    setYamlDraft(overview.yaml);
  }

  async function loadConfigVersions() {
    const versions = await api.listConfigVersions();
    setConfigVersions(versions);
  }

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (agentGroupDropdownRef.current && !agentGroupDropdownRef.current.contains(e.target as Node)) {
        setAgentGroupDropdownOpen(false);
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, []);

  useEffect(() => {
    const saved = (localStorage.getItem('auth9-theme') as Theme | null) ?? 'dark';
    const nextTheme: Theme = saved === 'dark' ? 'dark' : 'light';
    setTheme(nextTheme);
    if (nextTheme === 'dark') {
      document.documentElement.setAttribute('data-theme', 'dark');
    } else {
      document.documentElement.removeAttribute('data-theme');
    }
  }, []);

  useEffect(() => {
    localStorage.setItem('auth9-theme', theme);
    if (theme === 'dark') {
      document.documentElement.setAttribute('data-theme', 'dark');
    } else {
      document.documentElement.removeAttribute('data-theme');
    }
  }, [theme]);

  useEffect(() => {
    api
      .bootstrap()
      .then(async () => {
        await Promise.all([
          refreshSnapshot(),
          loadCreateTaskOptions(),
          loadConfigOverview(),
          loadConfigVersions()
        ]);
        setError('');
      })
      .catch((err) => setError(String(err)));
  }, []);

  useEffect(() => {
    selectedTaskIdRef.current = selectedTaskId;
  }, [selectedTaskId]);

  useEffect(() => {
    viewTabRef.current = viewTab;
  }, [viewTab]);

  useEffect(() => {
    if (!isCreateModalOpen) {
      return;
    }

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        closeCreateModal();
      }
    };

    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [isCreateModalOpen, createOptions]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;

    api
      .subscribeTaskEvents((event) => {
        if (!disposed) {
          handleRealtimeEvent(event);
        }
      })
      .then((fn) => {
        if (disposed) {
          fn();
          return;
        }
        unlisten = fn;
      })
      .catch((err) => setError(String(err)));

    return () => {
      disposed = true;
      if (realtimeRefreshTimerRef.current !== null) {
        window.clearTimeout(realtimeRefreshTimerRef.current);
        realtimeRefreshTimerRef.current = null;
      }
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  useEffect(() => {
    if (!selectedTaskId || viewTab !== 'tasks') {
      return;
    }
    loadTaskDetails(selectedTaskId).catch((err) => setError(String(err)));
  }, [selectedTaskId, viewTab]);

  useEffect(() => {
    if (workspaceKeys.length === 0) {
      setSelectedWorkspaceId(null);
      return;
    }
    setSelectedWorkspaceId((current) =>
      current && workspaceKeys.includes(current) ? current : workspaceKeys[0]
    );
  }, [workspaceKeys]);

  useEffect(() => {
    if (agentKeys.length === 0) {
      setSelectedAgentId(null);
      return;
    }
    setSelectedAgentId((current) => (current && agentKeys.includes(current) ? current : agentKeys[0]));
  }, [agentKeys]);

  useEffect(() => {
    if (agentGroupKeys.length === 0) {
      setSelectedAgentGroupId(null);
      return;
    }
    setSelectedAgentGroupId((current) =>
      current && agentGroupKeys.includes(current) ? current : agentGroupKeys[0]
    );
  }, [agentGroupKeys]);

  useEffect(() => {
    if (workflowKeys.length === 0) {
      setSelectedWorkflowId(null);
      return;
    }
    setSelectedWorkflowId((current) =>
      current && workflowKeys.includes(current) ? current : workflowKeys[0]
    );
  }, [workflowKeys]);

  async function createTask() {
    setBusy(true);
    try {
      if (!workspaceId || !workflowId) {
        throw new Error('Workspace and workflow are required');
      }
      const targetFiles = targets
        .split('\n')
        .map((value) => value.trim())
        .filter(Boolean);
      const created = await api.createTask({
        name: name.trim() || undefined,
        goal: goal.trim() || undefined,
        workspace_id: workspaceId,
        workflow_id: workflowId,
        target_files: targetFiles.length > 0 ? targetFiles : undefined
      });
      await refreshSnapshot(created.id);
      resetCreateTaskForm();
      setIsCreateModalOpen(false);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function withTaskAction(action: () => Promise<unknown>) {
    if (!selectedTaskId) {
      return;
    }
    setBusy(true);
    try {
      await action();
      await refreshSnapshot(selectedTaskId);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function deleteSelectedTask() {
    if (!selectedTaskId) {
      return;
    }
    const ok = window.confirm(
      'Delete this task permanently? This removes task records and run logs, but keeps ticket markdown files.'
    );
    if (!ok) {
      return;
    }
    setBusy(true);
    try {
      await withTimeout(api.deleteTask(selectedTaskId), 15000, 'delete task');
      const latest = await api.listTasks();
      setTasks(latest);
      const nextTaskId = latest[0]?.id ?? null;
      setSelectedTaskId(nextTaskId);
      if (nextTaskId) {
        await loadTaskDetails(nextTaskId);
      } else {
        setDetail(null);
        setLogs('');
      }
      setError('');
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  function updateConfig(mutator: (draft: OrchestratorConfigModel) => void) {
    setConfigDraft((prev) => {
      if (!prev) {
        return prev;
      }
      const draft = cloneConfig(prev);
      mutator(draft);
      return draft;
    });
    setConfigMessage('');
  }

  function isEntityDirty(kind: ConfigEntityKind, id: string): boolean {
    if (!configDraft || !configSavedSnapshot) {
      return false;
    }
    const lookupMap: Record<ConfigEntityKind, Record<string, unknown>> = {
      workspace: configDraft.workspaces,
      agent: configDraft.agents,
      agent_group: configDraft.agent_groups,
      workflow: configDraft.workflows,
    };
    const savedMap: Record<ConfigEntityKind, Record<string, unknown>> = {
      workspace: configSavedSnapshot.workspaces,
      agent: configSavedSnapshot.agents,
      agent_group: configSavedSnapshot.agent_groups,
      workflow: configSavedSnapshot.workflows,
    };
    return JSON.stringify(lookupMap[kind]?.[id] ?? null) !== JSON.stringify(savedMap[kind]?.[id] ?? null);
  }

  function applyEntitySwitch(target: PendingEntitySwitch) {
    if (target.kind === 'workspace') {
      setSelectedWorkspaceId(target.id);
    } else if (target.kind === 'agent') {
      setSelectedAgentId(target.id);
    } else if (target.kind === 'agent_group') {
      setSelectedAgentGroupId(target.id);
    } else {
      setSelectedWorkflowId(target.id);
    }
  }

  function closeEntitySwitchModal() {
    setPendingEntitySwitch(null);
    setEntitySwitchBusy(false);
  }

  function requestEntitySwitch(kind: ConfigEntityKind, nextId: string) {
    const currentId =
      kind === 'workspace'
        ? selectedWorkspaceId
        : kind === 'agent'
          ? selectedAgentId
          : kind === 'agent_group'
            ? selectedAgentGroupId
            : selectedWorkflowId;
    if (!nextId || nextId === currentId) {
      return;
    }
    if (currentId && isEntityDirty(kind, currentId)) {
      setPendingEntitySwitch({ kind, id: nextId });
      return;
    }
    applyEntitySwitch({ kind, id: nextId });
  }

  async function saveAndSwitchEntity() {
    if (!pendingEntitySwitch) {
      return;
    }
    setEntitySwitchBusy(true);
    const ok = await saveConfigFromForm();
    if (ok) {
      applyEntitySwitch(pendingEntitySwitch);
      closeEntitySwitchModal();
      return;
    }
    setEntitySwitchBusy(false);
  }

  function discardAndSwitchEntity() {
    if (!pendingEntitySwitch) {
      return;
    }
    applyEntitySwitch(pendingEntitySwitch);
    closeEntitySwitchModal();
  }

  function stepEditorKey(workflowKey: string, stepType: WorkflowStepType): string {
    return `${workflowKey}:${stepType}`;
  }

  function getSimulationContext(editorKey: string): PrehookSimulationContext {
    return prehookSimulationInputs[editorKey] ?? { ...DEFAULT_PREHOOK_SIMULATION_CONTEXT };
  }

  function updateSimulationField(
    editorKey: string,
    field: StepPrehookVisualField,
    value: number | boolean
  ) {
    setPrehookSimulationInputs((prev) => ({
      ...prev,
      [editorKey]: {
        ...(prev[editorKey] ?? { ...DEFAULT_PREHOOK_SIMULATION_CONTEXT }),
        [field]: value
      }
    }));
  }

  function updateStepPrehook(
    workflowKey: string,
    stepType: WorkflowStepType,
    updater: (prehook: StepPrehookConfig) => StepPrehookConfig
  ) {
    updateConfig((draft) => {
      const target = draft.workflows[workflowKey].steps.find((entry) => entry.type === stepType);
      if (!target?.prehook) {
        return;
      }
      target.prehook = updater(target.prehook);
    });
  }

  async function simulatePrehook(workflowKey: string, stepType: WorkflowStepType) {
    const editorKey = stepEditorKey(workflowKey, stepType);
    const step = configDraft?.workflows[workflowKey]?.steps.find((entry) => entry.type === stepType);
    if (!step?.prehook) {
      return;
    }
    const mode = resolvePrehookMode(step.prehook);
    const expression =
      mode === 'visual'
        ? compileVisualExpressionToCel(step.prehook.ui?.expr)
        : step.prehook.when.trim();
    if (!expression) {
      setPrehookSimulationResults((prev) => ({
        ...prev,
        [editorKey]: { error: 'Expression is empty.' }
      }));
      return;
    }
    const context = getSimulationContext(editorKey);
    try {
      const output = await api.simulatePrehook({
        expression,
        step: stepType,
        context: {
          cycle: Number(context.cycle),
          active_ticket_count: Number(context.active_ticket_count),
          new_ticket_count: Number(context.new_ticket_count),
          qa_exit_code: Number(context.qa_exit_code),
          fix_exit_code: Number(context.fix_exit_code),
          retest_exit_code: Number(context.retest_exit_code),
          qa_failed: Boolean(context.qa_failed),
          fix_required: Boolean(context.fix_required)
        }
      });
      setPrehookSimulationResults((prev) => ({
        ...prev,
        [editorKey]: {
          result: output.result,
          explanation: formatSimulationExplanation(output.expression, context)
        }
      }));
    } catch (err) {
      setPrehookSimulationResults((prev) => ({
        ...prev,
        [editorKey]: { error: String(err) }
      }));
    }
  }

  function setPrehookPreset(
    workflowKey: string,
    stepType: WorkflowStepType,
    presetId: string
  ) {
    const preset = (STEP_PREHOOK_PRESETS[stepType] ?? []).find((entry) => entry.id === presetId);
    if (!preset) {
      return;
    }
    updateStepPrehook(workflowKey, stepType, (prehook) => {
      const expr = cloneVisualExpression(preset.expr);
      return {
        ...prehook,
        when: compileVisualExpressionToCel(expr),
        reason: preset.reason ?? prehook.reason,
        ui: {
          ...(prehook.ui ?? {}),
          mode: 'visual',
          preset_id: preset.id,
          expr
        }
      };
    });
  }

  function updateVisualExpression(
    workflowKey: string,
    stepType: WorkflowStepType,
    mutator: (expr: StepPrehookVisualExpression) => StepPrehookVisualExpression
  ) {
    updateStepPrehook(workflowKey, stepType, (prehook) => {
      const nextExpr = mutator(cloneVisualExpression(prehook.ui?.expr));
      return {
        ...prehook,
        when: compileVisualExpressionToCel(nextExpr),
        ui: {
          ...(prehook.ui ?? {}),
          mode: 'visual',
          expr: nextExpr
        }
      };
    });
  }

  function addWorkspace() {
    const id = `workspace-${Date.now()}`;
    updateConfig((draft) => {
      draft.workspaces[id] = {
        root_path: '../..',
        qa_targets: ['docs/qa'],
        ticket_dir: 'docs/ticket'
      };
    });
    setSelectedWorkspaceId(id);
  }

  function addAgent() {
    const id = `agent-${Date.now()}`;
    updateConfig((draft) => {
      draft.agents[id] = {
        templates: {}
      };
    });
    setSelectedAgentId(id);
  }

  function addAgentGroup() {
    const id = `group-${Date.now()}`;
    updateConfig((draft) => {
      draft.agent_groups[id] = { agents: [] };
    });
    setSelectedAgentGroupId(id);
  }

  function removeAgentGroup(groupId: string) {
    const index = agentGroupKeys.indexOf(groupId);
    const nextSelection = agentGroupKeys[index + 1] ?? agentGroupKeys[index - 1] ?? null;
    updateConfig((draft) => {
      delete draft.agent_groups[groupId];
    });
    setSelectedAgentGroupId(nextSelection);
  }

  function addWorkflow() {
    const id = `workflow-${Date.now()}`;
    updateConfig((draft) => {
      const firstGroup = Object.keys(draft.agent_groups)[0] ?? '';
      draft.workflows[id] = {
        steps: defaultWorkflowSteps(firstGroup),
        loop: {
          mode: 'once',
          guard: {
            enabled: true,
            stop_when_no_unresolved: true,
            agent_group_id: undefined
          }
        }
      };
    });
    setSelectedWorkflowId(id);
  }

  function removeAgent(agentId: string) {
    const index = agentKeys.indexOf(agentId);
    const nextSelection = agentKeys[index + 1] ?? agentKeys[index - 1] ?? null;
    updateConfig((draft) => {
      delete draft.agents[agentId];
    });
    setSelectedAgentId(nextSelection);
  }

  function removeWorkspace(workspaceId: string) {
    const index = workspaceKeys.indexOf(workspaceId);
    const nextSelection = workspaceKeys[index + 1] ?? workspaceKeys[index - 1] ?? null;
    updateConfig((draft) => {
      delete draft.workspaces[workspaceId];
    });
    setSelectedWorkspaceId(nextSelection);
  }

  function removeWorkflow(workflowKey: string) {
    const index = workflowKeys.indexOf(workflowKey);
    const nextSelection = workflowKeys[index + 1] ?? workflowKeys[index - 1] ?? null;
    updateConfig((draft) => {
      delete draft.workflows[workflowKey];
    });
    setSelectedWorkflowId(nextSelection);
  }

  async function saveConfigFromForm(): Promise<boolean> {
    if (!configDraft) {
      return false;
    }
    setConfigBusy(true);
    try {
      const overview = await api.saveConfigFromForm({ config: configDraft });
      const normalized = cloneConfig(overview.config);
      ensureWorkflowShape(normalized);
      setConfigOverview(overview);
      setConfigDraft(normalized);
      setConfigSavedSnapshot(cloneConfig(normalized));
      setYamlDraft(overview.yaml);
      setConfigMessage(`已保存配置版本 v${overview.version}（表单）`);
      await Promise.all([loadCreateTaskOptions(), loadConfigVersions()]);
      setError('');
      return true;
    } catch (err) {
      setError(String(err));
      return false;
    } finally {
      setConfigBusy(false);
    }
  }

  async function validateYamlDraft() {
    setConfigBusy(true);
    try {
      const result = await api.validateConfigYaml({ yaml: yamlDraft });
      setYamlDraft(result.normalized_yaml);
      setConfigMessage('YAML 校验通过');
      setError('');
    } catch (err) {
      setError(String(err));
    } finally {
      setConfigBusy(false);
    }
  }

  async function saveConfigFromYaml() {
    setConfigBusy(true);
    try {
      const overview = await api.saveConfigFromYaml({ yaml: yamlDraft });
      const normalized = cloneConfig(overview.config);
      ensureWorkflowShape(normalized);
      setConfigOverview(overview);
      setConfigDraft(normalized);
      setConfigSavedSnapshot(cloneConfig(normalized));
      setYamlDraft(overview.yaml);
      setConfigMessage(`已保存配置版本 v${overview.version}（YAML）`);
      await Promise.all([loadCreateTaskOptions(), loadConfigVersions()]);
      setError('');
    } catch (err) {
      setError(String(err));
    } finally {
      setConfigBusy(false);
    }
  }

  return (
    <div className="shell">
      <div className="background-grid" />

      <header className="topbar animate-fade-in-up">
        <div className="title-row">
          <div>
            <h1>Auth9 QA Orchestrator</h1>
            <p>Liquid Glass dashboard for QA -&gt; Fix -&gt; Retest operations</p>
          </div>
          <div className="topbar-actions">
            <div className="view-tabs" role="tablist" aria-label="Main Views">
              <button
                className={`ghost-button view-tab ${viewTab === 'tasks' ? 'active' : ''}`}
                role="tab"
                aria-pressed={viewTab === 'tasks'}
                onClick={() => setViewTab('tasks')}
              >
                Tasks
              </button>
              <button
                className={`ghost-button view-tab ${viewTab === 'config' ? 'active' : ''}`}
                role="tab"
                aria-pressed={viewTab === 'config'}
                onClick={() => setViewTab('config')}
              >
                Config
              </button>
            </div>
            <button className="ghost-button" onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}>
              Theme: {theme}
            </button>
          </div>
        </div>

        <div className="stats-grid">
          <article className="stat-card stat-card-blue">
            <div className="stat-label">Total Tasks</div>
            <div className="stat-value">{taskStats.total}</div>
          </article>
          <article className="stat-card stat-card-cyan">
            <div className="stat-label">Running</div>
            <div className="stat-value">{taskStats.running}</div>
          </article>
          <article className="stat-card stat-card-green">
            <div className="stat-label">Completed</div>
            <div className="stat-value">{taskStats.completed}</div>
          </article>
          <article className="stat-card stat-card-red">
            <div className="stat-label">Failed</div>
            <div className="stat-value">{taskStats.failed}</div>
          </article>
        </div>
      </header>

      {error && <div className="error-box">{error}</div>}

      {viewTab === 'tasks' && (
        <main className="tasks-layout">
          <section className="panel tasks-sidebar animate-fade-in-up delay-1">
            <div className="sidebar-head">
              <h2>Tasks</h2>
              <button className="ghost-button" onClick={() => setIsCreateModalOpen(true)}>
                New Sprint
              </button>
            </div>
            <div className="task-list">
              {tasks.length === 0 && (
                <div className="empty-state">
                  <p className="muted">No tasks yet.</p>
                  <button className="ghost-button" onClick={() => setIsCreateModalOpen(true)}>
                    Create QA Sprint
                  </button>
                </div>
              )}
              {tasks.map((task) => {
                const pct = task.total_items === 0 ? 0 : Math.round((task.finished_items / task.total_items) * 100);
                return (
                  <button
                    className={`task-card ${selectedTaskId === task.id ? 'active' : ''}`}
                    key={task.id}
                    onClick={() => setSelectedTaskId(task.id)}
                  >
                    <div className="task-card-top">
                      <strong>{task.name}</strong>
                      <span className={STATUS_CLASS[task.status] ?? 'badge gray'}>{task.status}</span>
                    </div>
                    <div className="task-card-meta">Workspace: {task.workspace_id}</div>
                    <div className="task-card-meta">Workflow: {task.workflow_id}</div>
                    <div className="task-card-meta">
                      {task.finished_items}/{task.total_items} finished ({pct}%)
                    </div>
                    <div className="progress-line">
                      <span style={{ width: `${pct}%` }} />
                    </div>
                  </button>
                );
              })}
            </div>
          </section>

          <section className="panel detail-panel task-detail-main animate-fade-in-up delay-2">
            <div className="detail-head">
              <div>
                <h2>{selectedTask?.name ?? 'Task Details'}</h2>
                {selectedTask && (
                  <p className="muted">
                    workspace: {selectedTask.workspace_id} | workflow: {selectedTask.workflow_id}
                  </p>
                )}
              </div>
              <button className="ghost-button" onClick={() => setIsInfoOverlayOpen((current) => !current)}>
                {isInfoOverlayOpen ? 'Hide Info' : 'Show Info'}
              </button>
            </div>

            <div className="logs-stage">
              <section className="log-section">
                <h3>Live Logs</h3>
                <p className="muted log-legend">Legend: `!` means stderr (error output stream).</p>
                <pre className="log-box">{logs || 'No logs yet.'}</pre>
              </section>

              {isInfoOverlayOpen && (
                <aside className="info-overlay">
                  <div className="overlay-head">
                    <strong>Task Info</strong>
                    <button className="ghost-button" onClick={() => setIsInfoOverlayOpen(false)}>
                      Close
                    </button>
                  </div>
                  <div className="sub-stats-grid compact">
                    <article className="stat-pill">
                      <span>Items</span>
                      <strong>{itemStats.total}</strong>
                    </article>
                    <article className="stat-pill">
                      <span>Active</span>
                      <strong>{itemStats.active}</strong>
                    </article>
                    <article className="stat-pill">
                      <span>Unresolved</span>
                      <strong>{itemStats.unresolved}</strong>
                    </article>
                    <article className="stat-pill">
                      <span>Verified</span>
                      <strong>{itemStats.completed}</strong>
                    </article>
                    <article className="stat-pill">
                      <span>QA Runs</span>
                      <strong>{runStats.qa}</strong>
                    </article>
                    <article className="stat-pill">
                      <span>Init Runs</span>
                      <strong>{runStats.init_once}</strong>
                    </article>
                    <article className="stat-pill">
                      <span>Fix Runs</span>
                      <strong>{runStats.fix}</strong>
                    </article>
                    <article className="stat-pill">
                      <span>Retest Runs</span>
                      <strong>{runStats.retest}</strong>
                    </article>
                    <article className="stat-pill">
                      <span>Guard Runs</span>
                      <strong>{runStats.loop_guard}</strong>
                    </article>
                  </div>
                </aside>
              )}
            </div>

            <section className="items-panel">
              <div className="items-panel-head">
                <h3>Items ({filteredItems.length})</h3>
                <button className="ghost-button" onClick={() => setIsItemsPanelOpen((current) => !current)}>
                  {isItemsPanelOpen ? 'Hide' : 'Show'}
                </button>
              </div>
              {isItemsPanelOpen && (
                <>
                  <div className="toolbar-row">
                    <select value={itemFilter} onChange={(event) => setItemFilter(event.target.value as ItemFilter)}>
                      <option value="all">All Items</option>
                      <option value="active">Active</option>
                      <option value="unresolved">Unresolved</option>
                      <option value="completed">Completed</option>
                    </select>
                    <input
                      value={itemQuery}
                      onChange={(event) => setItemQuery(event.target.value)}
                      placeholder="Filter by QA file path"
                    />
                  </div>

                  <div className="items-table-wrap">
                    <table className="items-table">
                      <thead>
                        <tr>
                          <th>#</th>
                          <th>QA File</th>
                          <th>Status</th>
                          <th>Tickets</th>
                          <th>Error</th>
                          <th>Action</th>
                        </tr>
                      </thead>
                      <tbody>
                        {filteredItems.map((item) => (
                          <tr key={item.id}>
                            <td>{item.order_no}</td>
                            <td className="file-col">{item.qa_file_path}</td>
                            <td>
                              <span className={STATUS_CLASS[item.status] ?? 'badge gray'}>{item.status}</span>
                            </td>
                            <td>{item.ticket_files.length}</td>
                            <td className="error-col">{item.last_error || '-'}</td>
                            <td>
                              <button onClick={() => withTaskAction(() => api.retryTaskItem(item.id))} disabled={busy}>
                                Retry
                              </button>
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </>
              )}
            </section>
          </section>

          <aside className="panel task-control-panel animate-fade-in-up delay-3">
            <h2>Control Panel</h2>
            <div className="control-actions">
              <button disabled={!selectedTaskId || busy} onClick={() => withTaskAction(() => api.startTask(selectedTaskId!))}>
                Start
              </button>
              <button disabled={!selectedTaskId || busy} onClick={() => withTaskAction(() => api.pauseTask(selectedTaskId!))}>
                Pause
              </button>
              <button disabled={!selectedTaskId || busy} onClick={() => withTaskAction(() => api.resumeTask(selectedTaskId!))}>
                Resume
              </button>
              <button className="danger-button" disabled={!selectedTaskId || busy} onClick={() => void deleteSelectedTask()}>
                Delete
              </button>
            </div>

            <section className="ticket-section">
              <h3>Tickets ({ticketList.length})</h3>
              <div className="ticket-list">
                {ticketList.length === 0 && <p className="muted">No tickets linked to this task.</p>}
                {ticketList.map((ticket) => (
                  <article className="ticket-card" key={ticket.path}>
                    <div className="ticket-card-head">
                      <span className={STATUS_CLASS[ticket.status] ?? 'badge gray'}>{ticket.status}</span>
                    </div>
                    <code>{ticket.path}</code>
                    <p className="muted">Source: {ticket.source}</p>
                  </article>
                ))}
              </div>
            </section>
          </aside>
        </main>
      )}

      {viewTab === 'tasks' && isCreateModalOpen && (
        <div className="modal-backdrop" onClick={closeCreateModal} role="presentation">
          <section className="modal-card" role="dialog" aria-modal="true" aria-label="Create QA Sprint" onClick={(event) => event.stopPropagation()}>
            <div className="modal-head">
              <h2>Create QA Sprint</h2>
              <button className="ghost-button" onClick={closeCreateModal}>
                Close
              </button>
            </div>
            <label>
              Task Name
              <input value={name} onChange={(event) => setName(event.target.value)} />
            </label>
            <label>
              Goal
              <input value={goal} onChange={(event) => setGoal(event.target.value)} />
            </label>
            <label>
              Workspace
              <select value={workspaceId} onChange={(event) => setWorkspaceId(event.target.value)}>
                {(createOptions?.workspaces ?? []).map((entry) => (
                  <option key={entry.id} value={entry.id}>
                    {entry.id}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Workflow
              <select value={workflowId} onChange={(event) => setWorkflowId(event.target.value)}>
                {(createOptions?.workflows ?? []).map((entry) => (
                  <option key={entry.id} value={entry.id}>
                    {entry.id}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Target Files (optional, one path per line)
              <textarea
                value={targets}
                onChange={(event) => setTargets(event.target.value)}
                placeholder="docs/qa/user/01-crud.md"
              />
            </label>
            <button onClick={createTask} disabled={busy || !createOptions}>
              Create Task
            </button>
          </section>
        </div>
      )}

      {pendingEntitySwitch && (
        <div className="modal-backdrop" onClick={closeEntitySwitchModal} role="presentation">
          <section
            className="modal-card switch-confirm-modal"
            role="dialog"
            aria-modal="true"
            aria-label="切换配置对象确认"
            onClick={(event) => event.stopPropagation()}
          >
            <div className="modal-head">
              <h2>切换前确认</h2>
              <button className="ghost-button" onClick={closeEntitySwitchModal}>
                取消
              </button>
            </div>
            <p className="muted">
              当前
              {pendingEntitySwitch.kind === 'workspace'
                ? 'Workspace'
                : pendingEntitySwitch.kind === 'agent'
                  ? 'Agent'
                  : 'Workflow'}
              存在未保存修改，是否保存后再切换？
            </p>
            <div className="actions">
              <button disabled={entitySwitchBusy || configBusy} onClick={() => void saveAndSwitchEntity()}>
                保存并切换
              </button>
              <button className="ghost-button" disabled={entitySwitchBusy || configBusy} onClick={discardAndSwitchEntity}>
                直接切换
              </button>
            </div>
          </section>
        </div>
      )}

      {viewTab === 'config' && (
        <main className={`config-layout ${isConfigVersionsOpen ? '' : 'versions-collapsed'}`}>
          <section className="panel config-panel animate-fade-in-up delay-1">
            <div className="config-shell">
              <div className="detail-head config-detail-head">
                <div>
                  <h2>Config Center</h2>
                  <p className="muted">
                    {configOverview
                      ? `当前版本 v${configOverview.version} | 更新时间 ${configOverview.updated_at}`
                      : '配置加载中...'}
                  </p>
                </div>
                <div className="actions">
                  <button
                    className="ghost-button"
                    disabled={configBusy}
                    onClick={() => loadConfigOverview().catch((err) => setError(String(err)))}
                  >
                    重新加载
                  </button>
                  {configTab === 'yaml' ? (
                    <>
                      <button className="ghost-button" disabled={configBusy} onClick={validateYamlDraft}>
                        校验 YAML
                      </button>
                      <button disabled={configBusy} onClick={saveConfigFromYaml}>
                        保存 YAML
                      </button>
                    </>
                  ) : (
                    <button disabled={configBusy || !configDraft} onClick={saveConfigFromForm}>
                      保存配置
                    </button>
                  )}
                </div>
              </div>

              <div className="config-form-tabs" role="tablist" aria-label="配置页签">
                {CONFIG_FORM_TABS.map((tab) => (
                  <button
                    key={tab.id}
                    type="button"
                    role="tab"
                    aria-selected={configTab === tab.id}
                    className={`ghost-button config-form-tab ${configTab === tab.id ? 'active' : ''}`}
                    onClick={() => setConfigTab(tab.id)}
                  >
                    <span>{tab.label}</span>
                    <small>{tab.hint}</small>
                  </button>
                ))}
              </div>

              {configMessage && <p className="muted config-message">{configMessage}</p>}

              {configDraft && (
                <div className="config-grid">
                  {configTab === 'overview' && (
                    <section className="config-block config-overview-block">
                      <div className="config-block-head">
                        <h3>总览与默认配置</h3>
                        <div className="actions">
                          <button className="ghost-button" type="button" onClick={() => setConfigTab('workspace')}>
                            前往 Workspace
                          </button>
                          <button className="ghost-button" type="button" onClick={() => setConfigTab('workflow')}>
                            前往 Workflow
                          </button>
                        </div>
                      </div>
                      <p className="muted">
                        先设置默认 Workspace/Workflow，再分别进入对应 Tab 完成详细配置，可减少任务创建时的误选。
                      </p>
                      <label>
                        默认 Workspace (Default Workspace)
                        <small className="config-field-hint">创建任务时优先自动选中</small>
                        <select
                          value={configDraft.defaults.workspace}
                          onChange={(event) =>
                            updateConfig((draft) => {
                              draft.defaults.workspace = event.target.value;
                            })
                          }
                        >
                          {workspaceKeys.map((key) => (
                            <option key={key} value={key}>
                              {key}
                            </option>
                          ))}
                        </select>
                      </label>
                      <label>
                        默认 Workflow (Default Workflow)
                        <small className="config-field-hint">用于未手动指定流程的任务</small>
                        <select
                          value={configDraft.defaults.workflow}
                          onChange={(event) =>
                            updateConfig((draft) => {
                              draft.defaults.workflow = event.target.value;
                            })
                          }
                        >
                          {workflowKeys.map((key) => (
                            <option key={key} value={key}>
                              {key}
                            </option>
                          ))}
                        </select>
                      </label>
                    </section>
                  )}

                  {configTab === 'workspace' && (
                    <section className="config-block">
                      <div className="config-block-head">
                        <div>
                          <h3>Workspace 设定</h3>
                          <p className="muted">管理目录、QA 目标与工单路径，避免执行路径误配置。</p>
                        </div>
                        <button className="ghost-button" onClick={addWorkspace}>新增 Workspace</button>
                      </div>
                      <div className="entity-split-layout">
                        <aside className="entity-nav-panel">
                          <strong>Workspace 列表</strong>
                          <div className="entity-nav-list">
                            {workspaceKeys.map((id) => (
                              <button
                                key={id}
                                type="button"
                                className={`ghost-button entity-nav-item ${selectedWorkspaceId === id ? 'active' : ''}`}
                                onClick={() => requestEntitySwitch('workspace', id)}
                              >
                                <span>{id}</span>
                                {selectedWorkspaceId === id && selectedWorkspaceDirty && (
                                  <span className="dirty-dot" aria-label="unsaved changes" />
                                )}
                              </button>
                            ))}
                          </div>
                        </aside>

                        <div className="entity-editor-panel">
                          {selectedWorkspaceId ? (
                            <>
                              <div className="entity-editor-head">
                                <div>
                                  <strong>当前 Workspace: {selectedWorkspaceId}</strong>
                                  <p className="muted">
                                    {selectedWorkspaceDirty ? '存在未保存变更' : '已与最近保存版本同步'}
                                  </p>
                                </div>
                                <div className="actions">
                                  <button
                                    type="button"
                                    className="ghost-button"
                                    disabled={selectedWorkspaceIndex <= 0}
                                    onClick={() =>
                                      requestEntitySwitch(
                                        'workspace',
                                        workspaceKeys[selectedWorkspaceIndex - 1] ?? selectedWorkspaceId
                                      )
                                    }
                                  >
                                    上一个
                                  </button>
                                  <button
                                    type="button"
                                    className="ghost-button"
                                    disabled={selectedWorkspaceIndex < 0 || selectedWorkspaceIndex >= workspaceKeys.length - 1}
                                    onClick={() =>
                                      requestEntitySwitch(
                                        'workspace',
                                        workspaceKeys[selectedWorkspaceIndex + 1] ?? selectedWorkspaceId
                                      )
                                    }
                                  >
                                    下一个
                                  </button>
                                </div>
                              </div>
                              <div className="config-list">
                                {workspaceKeys
                                  .filter((id) => id === selectedWorkspaceId)
                                  .map((id) => {
                                    const ws = configDraft.workspaces[id];
                                    return (
                                      <article key={id} className="config-card">
                                        <div className="config-card-head">
                                          <strong>{id}</strong>
                                          <button className="ghost-button danger-button" onClick={() => removeWorkspace(id)}>
                                            删除
                                          </button>
                                        </div>
                                        <label>
                                          根目录 (Root Path)
                                          <small className="config-field-hint">例如 `../..`，需指向可执行 QA 的工作区</small>
                                          <input
                                            value={ws.root_path}
                                            onChange={(event) =>
                                              updateConfig((draft) => {
                                                draft.workspaces[id].root_path = event.target.value;
                                              })
                                            }
                                          />
                                        </label>
                                        <label>
                                          QA 目标 (QA Targets，每行一个)
                                          <small className="config-field-hint">例如 `docs/qa`、`tests/e2e`</small>
                                          <textarea
                                            value={(ws.qa_targets ?? []).join('\n')}
                                            onChange={(event) =>
                                              updateConfig((draft) => {
                                                draft.workspaces[id].qa_targets = event.target.value
                                                  .split('\n')
                                                  .map((line) => line.trim())
                                                  .filter(Boolean);
                                              })
                                            }
                                          />
                                        </label>
                                        <label>
                                          工单目录 (Ticket Dir)
                                          <small className="config-field-hint">用于写入自动生成的 ticket 文件</small>
                                          <input
                                            value={ws.ticket_dir}
                                            onChange={(event) =>
                                              updateConfig((draft) => {
                                                draft.workspaces[id].ticket_dir = event.target.value;
                                              })
                                            }
                                          />
                                        </label>
                                      </article>
                                    );
                                  })}
                              </div>
                            </>
                          ) : (
                            <p className="muted">暂无 Workspace，请先新增。</p>
                          )}
                        </div>
                      </div>
                    </section>
                  )}

                  {configTab === 'agent' && (
                    <section className="config-block">
                      <div className="config-block-head">
                        <div>
                          <h3>Agent 设定</h3>
                          <p className="muted">配置各阶段模板，让执行行为更稳定且可解释。</p>
                        </div>
                        <button className="ghost-button" onClick={addAgent}>新增 Agent</button>
                      </div>
                      <div className="entity-split-layout">
                        <aside className="entity-nav-panel">
                          <strong>Agent 列表</strong>
                          <div className="entity-nav-list">
                            {agentKeys.map((id) => (
                              <button
                                key={id}
                                type="button"
                                className={`ghost-button entity-nav-item ${selectedAgentId === id ? 'active' : ''}`}
                                onClick={() => requestEntitySwitch('agent', id)}
                              >
                                <span>{id}</span>
                                {selectedAgentId === id && selectedAgentDirty && (
                                  <span className="dirty-dot" aria-label="unsaved changes" />
                                )}
                              </button>
                            ))}
                          </div>
                        </aside>

                        <div className="entity-editor-panel">
                          {selectedAgentId ? (
                            <>
                              <div className="entity-editor-head">
                                <div>
                                  <strong>当前 Agent: {selectedAgentId}</strong>
                                  <p className="muted">
                                    {selectedAgentDirty ? '存在未保存变更' : '已与最近保存版本同步'}
                                  </p>
                                </div>
                                <div className="actions">
                                  <button
                                    type="button"
                                    className="ghost-button"
                                    disabled={selectedAgentIndex <= 0}
                                    onClick={() =>
                                      requestEntitySwitch(
                                        'agent',
                                        agentKeys[selectedAgentIndex - 1] ?? selectedAgentId
                                      )
                                    }
                                  >
                                    上一个
                                  </button>
                                  <button
                                    type="button"
                                    className="ghost-button"
                                    disabled={selectedAgentIndex < 0 || selectedAgentIndex >= agentKeys.length - 1}
                                    onClick={() =>
                                      requestEntitySwitch(
                                        'agent',
                                        agentKeys[selectedAgentIndex + 1] ?? selectedAgentId
                                      )
                                    }
                                  >
                                    下一个
                                  </button>
                                </div>
                              </div>

                              <div className="config-list">
                                {agentKeys
                                  .filter((id) => id === selectedAgentId)
                                  .map((id) => {
                                    const agent = configDraft.agents[id];
                                    return (
                                      <article key={id} className="config-card">
                                        <div className="config-card-head">
                                          <strong>{id}</strong>
                                          <button className="ghost-button danger-button" onClick={() => removeAgent(id)}>
                                            删除
                                          </button>
                                        </div>
                                        <label>
                                          Init 模板 (init_once)
                                          <small className="config-field-hint">任务初始化阶段执行</small>
                                          <textarea
                                            value={agent.templates.init_once ?? ''}
                                            onChange={(event) =>
                                              updateConfig((draft) => {
                                                draft.agents[id].templates.init_once = event.target.value || undefined;
                                              })
                                            }
                                          />
                                        </label>
                                        <label>
                                          QA 模板 (qa)
                                          <small className="config-field-hint">质量检查阶段执行</small>
                                          <textarea
                                            value={agent.templates.qa ?? ''}
                                            onChange={(event) =>
                                              updateConfig((draft) => {
                                                draft.agents[id].templates.qa = event.target.value || undefined;
                                              })
                                            }
                                          />
                                        </label>
                                        <label>
                                          Fix 模板 (fix)
                                          <small className="config-field-hint">自动修复阶段执行</small>
                                          <textarea
                                            value={agent.templates.fix ?? ''}
                                            onChange={(event) =>
                                              updateConfig((draft) => {
                                                draft.agents[id].templates.fix = event.target.value || undefined;
                                              })
                                            }
                                          />
                                        </label>
                                        <label>
                                          Retest 模板 (retest)
                                          <small className="config-field-hint">修复后复测阶段执行</small>
                                          <textarea
                                            value={agent.templates.retest ?? ''}
                                            onChange={(event) =>
                                              updateConfig((draft) => {
                                                draft.agents[id].templates.retest = event.target.value || undefined;
                                              })
                                            }
                                          />
                                        </label>
                                        <label>
                                          Loop Guard 模板 (loop_guard)
                                          <small className="config-field-hint">循环守卫决策阶段执行（可选）</small>
                                          <textarea
                                            value={agent.templates.loop_guard ?? ''}
                                            onChange={(event) =>
                                              updateConfig((draft) => {
                                                draft.agents[id].templates.loop_guard = event.target.value || undefined;
                                              })
                                            }
                                          />
                                        </label>
                                      </article>
                                    );
                                  })}
                              </div>
                            </>
                          ) : (
                            <p className="muted">暂无 Agent，请先新增。</p>
                          )}
                        </div>
                      </div>
                    </section>
                  )}

                  {configTab === 'agent_group' && (
                    <section className="config-block">
                      <div className="config-block-head">
                        <div>
                          <h3>Agent Group 设定</h3>
                          <p className="muted">配置 Agent 分组，实现多 Agent 轮转与故障自动恢复。</p>
                        </div>
                        <button className="ghost-button" onClick={addAgentGroup}>新增 Agent Group</button>
                      </div>
                      <div className="entity-split-layout">
                        <aside className="entity-nav-panel">
                          <strong>Agent Group 列表</strong>
                          <div className="entity-nav-list">
                            {agentGroupKeys.map((gid) => (
                              <button
                                key={gid}
                                type="button"
                                className={`ghost-button entity-nav-item ${selectedAgentGroupId === gid ? 'active' : ''}`}
                                onClick={() => requestEntitySwitch('agent_group', gid)}
                              >
                                <span>{gid}</span>
                                {selectedAgentGroupId === gid && selectedAgentGroupDirty && (
                                  <span className="dirty-dot" aria-label="unsaved changes" />
                                )}
                              </button>
                            ))}
                          </div>
                        </aside>
                        <div className="entity-editor-panel">
                          {selectedAgentGroupId && configDraft?.agent_groups[selectedAgentGroupId] ? (
                            <>
                              <div className="entity-editor-head">
                                <div>
                                  <strong>当前 Agent Group: {selectedAgentGroupId}</strong>
                                  <p className="muted">
                                    {selectedAgentGroupDirty ? '存在未保存变更' : '已与最近保存版本同步'}
                                  </p>
                                </div>
                                <div className="actions">
                                  <button
                                    type="button"
                                    className="ghost-button"
                                    disabled={selectedAgentGroupIndex <= 0}
                                    onClick={() =>
                                      requestEntitySwitch('agent_group', agentGroupKeys[selectedAgentGroupIndex - 1] ?? selectedAgentGroupId)
                                    }
                                  >
                                    上一个
                                  </button>
                                  <button
                                    type="button"
                                    className="ghost-button"
                                    disabled={selectedAgentGroupIndex < 0 || selectedAgentGroupIndex >= agentGroupKeys.length - 1}
                                    onClick={() =>
                                      requestEntitySwitch('agent_group', agentGroupKeys[selectedAgentGroupIndex + 1] ?? selectedAgentGroupId)
                                    }
                                  >
                                    下一个
                                  </button>
                                  {agentGroupKeys
                                    .filter((gid) => gid !== selectedAgentGroupId)
                                    .length > 0 && (
                                    <button className="ghost-button danger-button" onClick={() => removeAgentGroup(selectedAgentGroupId)}>
                                      删除
                                    </button>
                                  )}
                                </div>
                              </div>
                              <label>Agent 成员</label>
                              <div className="agent-add-row" ref={agentGroupDropdownRef}>
                                <div className="agent-add-dropdown">
                                  <input
                                    type="text"
                                    placeholder="搜索 Agent..."
                                    value={agentGroupAddFilter}
                                    onChange={(e) => { setAgentGroupAddFilter(e.target.value); setAgentGroupDropdownOpen(true); }}
                                    onFocus={() => setAgentGroupDropdownOpen(true)}
                                  />
                                  {agentGroupDropdownOpen && (() => {
                                    const group = configDraft.agent_groups[selectedAgentGroupId!];
                                    const available = agentKeys
                                      .filter((id) => !(group?.agents ?? []).includes(id))
                                      .filter((id) => !agentGroupAddFilter || id.toLowerCase().includes(agentGroupAddFilter.toLowerCase()));
                                    return available.length > 0 ? (
                                      <div className="agent-add-dropdown-list">
                                        {available.map((id) => (
                                          <button
                                            key={id}
                                            type="button"
                                            className="agent-add-dropdown-item"
                                            onClick={() => {
                                              updateConfig((draft) => {
                                                const g = draft.agent_groups[selectedAgentGroupId!];
                                                if (g && !g.agents.includes(id)) g.agents.push(id);
                                              });
                                              setAgentGroupAddFilter('');
                                              setAgentGroupDropdownOpen(false);
                                            }}
                                          >
                                            {id}
                                          </button>
                                        ))}
                                      </div>
                                    ) : null;
                                  })()}
                                </div>
                              </div>

                              <h4 style={{ margin: '8px 0 4px' }}>已添加成员</h4>
                              <div className="agent-member-list">
                                {(configDraft.agent_groups[selectedAgentGroupId!]?.agents ?? []).map((agentId) => {
                                  const info = agentHealthList.find((h) => h.agent_id === agentId);
                                  return (
                                    <div key={agentId} className="agent-member-card">
                                      <div className="agent-member-info">
                                        <span className="agent-member-name">{agentId}</span>
                                        <span className={`badge ${info?.healthy !== false ? 'green' : 'red'}`}>
                                          {info?.healthy !== false ? 'healthy' : 'diseased'}
                                        </span>
                                        {info && !info.healthy && info.diseased_until && (
                                          <span className="badge gray">恢复于 {new Date(info.diseased_until).toLocaleString()}</span>
                                        )}
                                        {info && info.consecutive_errors > 0 && (
                                          <span className="badge amber">错误×{info.consecutive_errors}</span>
                                        )}
                                      </div>
                                      <button
                                        type="button"
                                        className="ghost-button agent-member-remove"
                                        onClick={() => setPendingRemoveAgentId(agentId)}
                                        title="移除"
                                      >
                                        ✕
                                      </button>
                                    </div>
                                  );
                                })}
                                {(configDraft.agent_groups[selectedAgentGroupId!]?.agents ?? []).length === 0 && (
                                  <p className="muted">暂无成员，请从上方下拉菜单追加。</p>
                                )}
                              </div>

                              {pendingRemoveAgentId && (
                                <div className="modal-backdrop" onClick={() => setPendingRemoveAgentId(null)} role="presentation">
                                  <section className="modal-card switch-confirm-modal" role="dialog" aria-modal="true"
                                    onClick={(e) => e.stopPropagation()}>
                                    <div className="modal-head">
                                      <h2>确认移除</h2>
                                      <button className="ghost-button" onClick={() => setPendingRemoveAgentId(null)}>取消</button>
                                    </div>
                                    <p className="muted">
                                      确定要将 <strong>{pendingRemoveAgentId}</strong> 从 Agent Group <strong>{selectedAgentGroupId}</strong> 中移除吗？
                                    </p>
                                    <div className="actions">
                                      <button onClick={() => {
                                        updateConfig((draft) => {
                                          const g = draft.agent_groups[selectedAgentGroupId!];
                                          if (g) g.agents = g.agents.filter((a) => a !== pendingRemoveAgentId);
                                        });
                                        setPendingRemoveAgentId(null);
                                      }}>
                                        确认移除
                                      </button>
                                      <button className="ghost-button" onClick={() => setPendingRemoveAgentId(null)}>取消</button>
                                    </div>
                                  </section>
                                </div>
                              )}
                            </>
                          ) : (
                            <p className="muted">请选择一个 Agent Group 或新增一个。</p>
                          )}
                        </div>
                      </div>
                    </section>
                  )}

                  {configTab === 'workflow' && (
                    <section className="config-block">
                      <div className="config-block-head">
                        <div>
                          <h3>Workflow 设定</h3>
                          <p className="muted">定义步骤启用、Agent 绑定、Prehook 规则和循环策略。</p>
                        </div>
                        <button className="ghost-button" onClick={addWorkflow}>新增 Workflow</button>
                      </div>
                      <div className="entity-split-layout">
                        <aside className="entity-nav-panel">
                          <strong>Workflow 列表</strong>
                          <div className="entity-nav-list">
                            {workflowKeys.map((id) => (
                              <button
                                key={id}
                                type="button"
                                className={`ghost-button entity-nav-item ${selectedWorkflowId === id ? 'active' : ''}`}
                                onClick={() => requestEntitySwitch('workflow', id)}
                              >
                                <span>{id}</span>
                                {selectedWorkflowId === id && selectedWorkflowDirty && (
                                  <span className="dirty-dot" aria-label="unsaved changes" />
                                )}
                              </button>
                            ))}
                          </div>
                        </aside>

                        <div className="entity-editor-panel">
                          {selectedWorkflowId ? (
                            <>
                              <div className="entity-editor-head">
                                <div>
                                  <strong>当前 Workflow: {selectedWorkflowId}</strong>
                                  <p className="muted">
                                    {selectedWorkflowDirty ? '存在未保存变更' : '已与最近保存版本同步'}
                                  </p>
                                </div>
                                <div className="actions">
                                  <button
                                    type="button"
                                    className="ghost-button"
                                    disabled={selectedWorkflowIndex <= 0}
                                    onClick={() =>
                                      requestEntitySwitch(
                                        'workflow',
                                        workflowKeys[selectedWorkflowIndex - 1] ?? selectedWorkflowId
                                      )
                                    }
                                  >
                                    上一个
                                  </button>
                                  <button
                                    type="button"
                                    className="ghost-button"
                                    disabled={
                                      selectedWorkflowIndex < 0 || selectedWorkflowIndex >= workflowKeys.length - 1
                                    }
                                    onClick={() =>
                                      requestEntitySwitch(
                                        'workflow',
                                        workflowKeys[selectedWorkflowIndex + 1] ?? selectedWorkflowId
                                      )
                                    }
                                  >
                                    下一个
                                  </button>
                                </div>
                              </div>
                              <div className="config-list">
                                {workflowKeys.filter((id) => id === selectedWorkflowId).map((id) => {
                      const wf = configDraft.workflows[id];
                      const stepsByType = new Map(wf.steps.map((step) => [step.type, step]));
                      return (
                        <article key={id} className="config-card">
                          <div className="config-card-head">
                            <strong>{id}</strong>
                            <button className="ghost-button danger-button" onClick={() => removeWorkflow(id)}>
                              删除
                            </button>
                          </div>
                          <h4>步骤配置 (Steps)</h4>
                          {WORKFLOW_STEP_ORDER.map((stepType) => {
                            const step = stepsByType.get(stepType);
                            if (!step) {
                              return null;
                            }
                            const isBuiltinTicketScan = stepType === 'ticket_scan';
                            return (
                              <div key={stepType} className="step-row">
                                <label>
                                  <input
                                    type="checkbox"
                                    checked={step.enabled}
                                    onChange={(event) =>
                                      updateConfig((draft) => {
                                        const target = draft.workflows[id].steps.find((entry) => entry.type === stepType);
                                        if (target) {
                                          target.enabled = event.target.checked;
                                          if (!event.target.checked) {
                                            target.agent_group_id = undefined;
                                          } else if (isBuiltinTicketScan) {
                                            target.agent_group_id = undefined;
                                          } else if (!target.agent_group_id && agentGroupKeys[0]) {
                                            target.agent_group_id = agentGroupKeys[0];
                                          }
                                        }
                                      })
                                    }
                                  />
                                  {stepType}
                                </label>
                                <select
                                  value={isBuiltinTicketScan ? '' : (step.agent_group_id ?? '')}
                                  disabled={!step.enabled || isBuiltinTicketScan}
                                  onChange={(event) =>
                                    updateConfig((draft) => {
                                      const target = draft.workflows[id].steps.find((entry) => entry.type === stepType);
                                      if (target) {
                                        if (isBuiltinTicketScan) {
                                          target.agent_group_id = undefined;
                                          return;
                                        }
                                        target.agent_group_id = event.target.value || undefined;
                                      }
                                    })
                                  }
                                >
                                  {isBuiltinTicketScan && <option value="">(builtin)</option>}
                                  <option value="">(none)</option>
                                  {agentGroupKeys.map((groupId) => (
                                    <option key={groupId} value={groupId}>
                                      {groupId}
                                    </option>
                                  ))}
                                </select>
                                <div className="step-prehook">
                                  <label>
                                    <input
                                      type="checkbox"
                                      checked={Boolean(step.prehook)}
                                      onChange={(event) =>
                                        updateConfig((draft) => {
                                          const target = draft.workflows[id].steps.find((entry) => entry.type === stepType);
                                          if (target) {
                                            target.prehook = event.target.checked
                                              ? buildPrehookFromPreset(stepType)
                                              : undefined;
                                          }
                                        })
                                      }
                                    />
                                    Enable Prehook
                                  </label>
                                  {step.prehook && (
                                    <>
                                      <div className="prehook-mode-row">
                                        <button
                                          type="button"
                                          className={`ghost-button prehook-mode-btn ${resolvePrehookMode(step.prehook) === 'visual' ? 'active' : ''}`}
                                          onClick={() =>
                                            updateStepPrehook(id, stepType, (prehook) => {
                                              const fallback = STEP_PREHOOK_PRESETS[stepType]?.[0];
                                              const expr = cloneVisualExpression(prehook.ui?.expr ?? fallback?.expr);
                                              return {
                                                ...prehook,
                                                when: compileVisualExpressionToCel(expr),
                                                ui: {
                                                  ...(prehook.ui ?? {}),
                                                  mode: 'visual',
                                                  preset_id: prehook.ui?.preset_id ?? fallback?.id,
                                                  expr
                                                }
                                              };
                                            })
                                          }
                                        >
                                          Visual Rules
                                        </button>
                                        <button
                                          type="button"
                                          className={`ghost-button prehook-mode-btn ${resolvePrehookMode(step.prehook) === 'cel' ? 'active' : ''}`}
                                          onClick={() =>
                                            updateStepPrehook(id, stepType, (prehook) => ({
                                              ...prehook,
                                              ui: {
                                                ...(prehook.ui ?? {}),
                                                mode: 'cel'
                                              }
                                            }))
                                          }
                                        >
                                          Advanced CEL
                                        </button>
                                      </div>

                                      {resolvePrehookMode(step.prehook) === 'visual' ? (
                                        <>
                                          <label>
                                            Preset
                                            <select
                                              value={step.prehook.ui?.preset_id ?? STEP_PREHOOK_PRESETS[stepType]?.[0]?.id ?? ''}
                                              onChange={(event) =>
                                                setPrehookPreset(id, stepType, event.target.value)
                                              }
                                            >
                                              {(STEP_PREHOOK_PRESETS[stepType] ?? []).map((preset) => (
                                                <option key={preset.id} value={preset.id}>
                                                  {preset.label}
                                                </option>
                                              ))}
                                            </select>
                                          </label>
                                          <p className="muted">
                                            {(STEP_PREHOOK_PRESETS[stepType] ?? []).find(
                                              (preset) =>
                                                preset.id ===
                                                (step.prehook?.ui?.preset_id ??
                                                  STEP_PREHOOK_PRESETS[stepType]?.[0]?.id)
                                            )?.description ?? 'No preset description.'}
                                          </p>
                                          <label>
                                            Match Strategy
                                            <select
                                              value={step.prehook.ui?.expr?.op ?? 'all'}
                                              onChange={(event) =>
                                                updateVisualExpression(id, stepType, (expr) => ({
                                                  ...expr,
                                                  op: event.target.value === 'any' ? 'any' : 'all'
                                                }))
                                              }
                                            >
                                              <option value="all">All conditions (AND)</option>
                                              <option value="any">Any condition (OR)</option>
                                            </select>
                                          </label>
                                          <div className="prehook-rules">
                                            {(step.prehook.ui?.expr?.rules ?? []).map((rule, ruleIndex) => {
                                              const fieldMeta = getFieldMeta(rule.field);
                                              const comparators =
                                                fieldMeta.valueType === 'boolean'
                                                  ? BOOLEAN_COMPARATORS
                                                  : NUMBER_COMPARATORS;
                                              return (
                                                <div className="prehook-rule-row" key={`${rule.field}-${ruleIndex}`}>
                                                  <select
                                                    value={rule.field}
                                                    onChange={(event) =>
                                                      updateVisualExpression(id, stepType, (expr) => {
                                                        const nextRules = expr.rules.map((entry, index) => {
                                                          if (index !== ruleIndex) {
                                                            return entry;
                                                          }
                                                          const nextField = event.target
                                                            .value as StepPrehookVisualField;
                                                          const nextMeta = getFieldMeta(nextField);
                                                          return {
                                                            field: nextField,
                                                            cmp:
                                                              nextMeta.valueType === 'boolean'
                                                                ? '=='
                                                                : '>',
                                                            value:
                                                              nextMeta.valueType === 'boolean'
                                                                ? false
                                                                : 0
                                                          } as StepPrehookVisualRule;
                                                        });
                                                        return { ...expr, rules: nextRules };
                                                      })
                                                    }
                                                  >
                                                    {PREHOOK_FIELDS.map((field) => (
                                                      <option key={field.id} value={field.id}>
                                                        {field.label}
                                                      </option>
                                                    ))}
                                                  </select>
                                                  <select
                                                    value={rule.cmp}
                                                    onChange={(event) =>
                                                      updateVisualExpression(id, stepType, (expr) => ({
                                                        ...expr,
                                                        rules: expr.rules.map((entry, index) =>
                                                          index === ruleIndex
                                                            ? {
                                                                ...entry,
                                                                cmp:
                                                                  event.target
                                                                    .value as StepPrehookVisualComparator
                                                              }
                                                            : entry
                                                        )
                                                      }))
                                                    }
                                                  >
                                                    {comparators.map((cmp) => (
                                                      <option key={cmp} value={cmp}>
                                                        {cmp}
                                                      </option>
                                                    ))}
                                                  </select>
                                                  {fieldMeta.valueType === 'boolean' ? (
                                                    <select
                                                      value={rule.value ? 'true' : 'false'}
                                                      onChange={(event) =>
                                                        updateVisualExpression(id, stepType, (expr) => ({
                                                          ...expr,
                                                          rules: expr.rules.map((entry, index) =>
                                                            index === ruleIndex
                                                              ? {
                                                                  ...entry,
                                                                  value:
                                                                    event.target.value === 'true'
                                                                }
                                                              : entry
                                                          )
                                                        }))
                                                      }
                                                    >
                                                      <option value="true">true</option>
                                                      <option value="false">false</option>
                                                    </select>
                                                  ) : (
                                                    <input
                                                      type="number"
                                                      value={Number(rule.value)}
                                                      onChange={(event) =>
                                                        updateVisualExpression(id, stepType, (expr) => ({
                                                          ...expr,
                                                          rules: expr.rules.map((entry, index) =>
                                                            index === ruleIndex
                                                              ? {
                                                                  ...entry,
                                                                  value: Number(event.target.value || 0)
                                                                }
                                                              : entry
                                                          )
                                                        }))
                                                      }
                                                    />
                                                  )}
                                                  <button
                                                    type="button"
                                                    className="ghost-button prehook-rule-remove"
                                                    onClick={() =>
                                                      updateVisualExpression(id, stepType, (expr) => ({
                                                        ...expr,
                                                        rules: expr.rules.filter(
                                                          (_, index) => index !== ruleIndex
                                                        )
                                                      }))
                                                    }
                                                  >
                                                    Remove
                                                  </button>
                                                </div>
                                              );
                                            })}
                                          </div>
                                          <button
                                            type="button"
                                            className="ghost-button prehook-add-rule"
                                            onClick={() =>
                                              updateVisualExpression(id, stepType, (expr) => ({
                                                ...expr,
                                                rules: [
                                                  ...(expr.rules ?? []),
                                                  {
                                                    field: 'active_ticket_count',
                                                    cmp: '>',
                                                    value: 0
                                                  }
                                                ]
                                              }))
                                            }
                                          >
                                            Add Condition
                                          </button>
                                          <label>
                                            Generated CEL
                                            <input value={step.prehook.when} readOnly />
                                          </label>
                                          <div className="prehook-simulator">
                                            <strong>Simulate</strong>
                                            <div className="prehook-sim-grid">
                                              {(
                                                (
                                                  step.prehook.ui?.expr?.rules.map(
                                                    (entry) => entry.field
                                                  ) ?? [
                                                    'active_ticket_count',
                                                    'fix_exit_code',
                                                    'qa_failed',
                                                    'fix_required'
                                                  ]
                                                ).filter(
                                                  (field, index, arr) =>
                                                    arr.indexOf(field) === index
                                                ) as StepPrehookVisualField[]
                                              ).map((field) => {
                                                const meta = getFieldMeta(field);
                                                const key = stepEditorKey(id, stepType);
                                                const context = getSimulationContext(key);
                                                const currentValue = context[field];
                                                return (
                                                  <label key={field}>
                                                    {meta.label}
                                                    {meta.valueType === 'boolean' ? (
                                                      <select
                                                        value={currentValue ? 'true' : 'false'}
                                                        onChange={(event) =>
                                                          updateSimulationField(
                                                            key,
                                                            field,
                                                            event.target.value === 'true'
                                                          )
                                                        }
                                                      >
                                                        <option value="true">true</option>
                                                        <option value="false">false</option>
                                                      </select>
                                                    ) : (
                                                      <input
                                                        type="number"
                                                        value={Number(currentValue)}
                                                        onChange={(event) =>
                                                          updateSimulationField(
                                                            key,
                                                            field,
                                                            Number(event.target.value || 0)
                                                          )
                                                        }
                                                      />
                                                    )}
                                                  </label>
                                                );
                                              })}
                                            </div>
                                            <button
                                              type="button"
                                              className="ghost-button"
                                              onClick={() => {
                                                void simulatePrehook(id, stepType);
                                              }}
                                            >
                                              Simulate
                                            </button>
                                            {(() => {
                                              const simulationResult =
                                                prehookSimulationResults[stepEditorKey(id, stepType)];
                                              if (!simulationResult) {
                                                return null;
                                              }
                                              return (
                                                <p className="muted">
                                                  {'error' in simulationResult
                                                    ? simulationResult.error
                                                    : `Result: ${
                                                        simulationResult.result ? 'run' : 'skip'
                                                      } | ${simulationResult.explanation}`}
                                                </p>
                                              );
                                            })()}
                                          </div>
                                        </>
                                      ) : (
                                        <>
                                          <label>
                                            CEL Expression
                                            <textarea
                                              value={step.prehook.when}
                                              onChange={(event) =>
                                                updateStepPrehook(id, stepType, (prehook) => ({
                                                  ...prehook,
                                                  when: event.target.value,
                                                  ui: {
                                                    ...(prehook.ui ?? {}),
                                                    mode: 'cel'
                                                  }
                                                }))
                                              }
                                              placeholder="active_ticket_count > 0 && fix_exit_code == 0"
                                            />
                                          </label>
                                          <div className="prehook-simulator">
                                            <strong>Simulate</strong>
                                            <div className="prehook-sim-grid">
                                              {PREHOOK_FIELDS.map((field) => {
                                                const key = stepEditorKey(id, stepType);
                                                const context = getSimulationContext(key);
                                                const currentValue = context[field.id];
                                                return (
                                                  <label key={field.id}>
                                                    {field.label}
                                                    {field.valueType === 'boolean' ? (
                                                      <select
                                                        value={currentValue ? 'true' : 'false'}
                                                        onChange={(event) =>
                                                          updateSimulationField(
                                                            key,
                                                            field.id,
                                                            event.target.value === 'true'
                                                          )
                                                        }
                                                      >
                                                        <option value="true">true</option>
                                                        <option value="false">false</option>
                                                      </select>
                                                    ) : (
                                                      <input
                                                        type="number"
                                                        value={Number(currentValue)}
                                                        onChange={(event) =>
                                                          updateSimulationField(
                                                            key,
                                                            field.id,
                                                            Number(event.target.value || 0)
                                                          )
                                                        }
                                                      />
                                                    )}
                                                  </label>
                                                );
                                              })}
                                            </div>
                                            <button
                                              type="button"
                                              className="ghost-button"
                                              onClick={() => {
                                                void simulatePrehook(id, stepType);
                                              }}
                                            >
                                              Simulate
                                            </button>
                                            {(() => {
                                              const simulationResult =
                                                prehookSimulationResults[stepEditorKey(id, stepType)];
                                              if (!simulationResult) {
                                                return null;
                                              }
                                              return (
                                                <p className="muted">
                                                  {'error' in simulationResult
                                                    ? simulationResult.error
                                                    : `Result: ${
                                                        simulationResult.result ? 'run' : 'skip'
                                                      } | ${simulationResult.explanation}`}
                                                </p>
                                              );
                                            })()}
                                          </div>
                                        </>
                                      )}
                                      <label>
                                        Reason (optional)
                                        <input
                                          value={step.prehook.reason ?? ''}
                                          onChange={(event) =>
                                            updateStepPrehook(id, stepType, (prehook) => ({
                                              ...prehook,
                                              reason: event.target.value || undefined
                                            }))
                                          }
                                          placeholder="skip fix when no tickets"
                                        />
                                      </label>
                                    </>
                                  )}
                                </div>
                              </div>
                            );
                          })}
                          <h4>循环策略 (Loop)</h4>
                          <label>
                            循环模式 (Loop Mode)
                            <select
                              value={wf.loop.mode}
                              onChange={(event) =>
                                updateConfig((draft) => {
                                  draft.workflows[id].loop.mode = event.target.value as 'once' | 'infinite';
                                })
                              }
                            >
                              <option value="once">once</option>
                              <option value="infinite">infinite</option>
                            </select>
                          </label>
                          <label>
                            <input
                              type="checkbox"
                              checked={wf.loop.guard.enabled}
                              onChange={(event) =>
                                updateConfig((draft) => {
                                  draft.workflows[id].loop.guard.enabled = event.target.checked;
                                })
                              }
                            />
                            启用 Guard
                          </label>
                          <label>
                            <input
                              type="checkbox"
                              checked={wf.loop.guard.stop_when_no_unresolved}
                              onChange={(event) =>
                                updateConfig((draft) => {
                                  draft.workflows[id].loop.guard.stop_when_no_unresolved = event.target.checked;
                                })
                              }
                            />
                            无未解决项时停止
                          </label>
                          <label>
                            Guard Agent（可选）
                            <select
                              value={wf.loop.guard.agent_group_id ?? ''}
                              onChange={(event) =>
                                updateConfig((draft) => {
                                  draft.workflows[id].loop.guard.agent_group_id = event.target.value || undefined;
                                })
                              }
                            >
                              <option value="">(none)</option>
                              {agentGroupKeys.map((groupId) => (
                                <option key={groupId} value={groupId}>
                                  {groupId}
                                </option>
                              ))}
                            </select>
                          </label>
                          <label>
                            最大循环次数（可选）
                            <input
                              type="number"
                              min={1}
                              value={wf.loop.guard.max_cycles ?? ''}
                              onChange={(event) =>
                                updateConfig((draft) => {
                                  const raw = event.target.value.trim();
                                  draft.workflows[id].loop.guard.max_cycles = raw
                                    ? Math.max(1, Number(raw))
                                    : undefined;
                                })
                              }
                            />
                          </label>
                                    </article>
                                  );
                                })}
                              </div>
                            </>
                          ) : (
                            <p className="muted">暂无 Workflow，请先新增。</p>
                          )}
                        </div>
                      </div>
                    </section>
                  )}

                  {configTab === 'yaml' && (
                    <section className="config-block config-yaml-block">
                      <div className="config-block-head">
                        <h3>YAML 高级编辑</h3>
                        <span className="badge amber">高级模式</span>
                      </div>
                      <p className="muted">
                        直接编辑 YAML 可能影响全部配置。建议先在 Workspace/Agent/Workflow Tab 中修改，再在此进行高级调整。
                      </p>
                      <label>
                        Config YAML
                        <textarea
                          className="yaml-editor"
                          value={yamlDraft}
                          onChange={(event) => setYamlDraft(event.target.value)}
                        />
                      </label>
                    </section>
                  )}
                </div>
              )}
              {!configDraft && <p className="muted">配置加载中，请稍候...</p>}
            </div>
          </section>

          <section
            className={`panel config-history-panel animate-fade-in-up delay-2 ${
              isConfigVersionsOpen ? '' : 'collapsed'
            }`}
          >
            <div
              className="config-history-head"
              role="button"
              tabIndex={0}
              aria-expanded={isConfigVersionsOpen}
              aria-controls="config-versions-list"
              onClick={() => setIsConfigVersionsOpen((current) => !current)}
              onKeyDown={(event) => {
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault();
                  setIsConfigVersionsOpen((current) => !current);
                }
              }}
            >
              <div className="config-history-title">
                <h3>{isConfigVersionsOpen ? 'Config Versions' : 'Versions'}</h3>
                {isConfigVersionsOpen && (
                  <p className="muted">已保存 {configVersions.length} 个版本，点击版本可加载到 YAML Tab。</p>
                )}
              </div>
              <button
                type="button"
                className="ghost-button config-history-toggle"
                aria-label={isConfigVersionsOpen ? '收起 Config Versions' : '展开 Config Versions'}
                onClick={(event) => {
                  event.stopPropagation();
                  setIsConfigVersionsOpen((current) => !current);
                }}
              >
                {isConfigVersionsOpen ? '«' : '»'}
              </button>
            </div>

            {isConfigVersionsOpen && (
              <div className="config-history-list" id="config-versions-list">
                {configVersions.map((entry) => (
                  <button
                    key={entry.version}
                    className="task-card"
                    onClick={() =>
                      api
                        .getConfigVersion(entry.version)
                        .then((detail) => {
                          setYamlDraft(detail.yaml);
                          setConfigTab('yaml');
                          setConfigMessage(`已加载历史版本 v${entry.version}，保存后才会生效。`);
                        })
                        .catch((err) => setError(String(err)))
                    }
                  >
                    <div className="task-card-top">
                      <strong>v{entry.version}</strong>
                      <span className="badge gray">{entry.author}</span>
                    </div>
                    <div className="task-card-meta">{entry.created_at}</div>
                  </button>
                ))}
              </div>
            )}
          </section>
        </main>
      )}
    </div>
  );
}
