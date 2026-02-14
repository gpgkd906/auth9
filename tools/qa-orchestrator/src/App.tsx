import { useEffect, useMemo, useState } from 'react';
import { api } from './api';
import type {
  ConfigOverview,
  ConfigVersionSummary,
  CreateTaskOptions,
  OrchestratorConfigModel,
  TaskDetail,
  TaskMode,
  TaskSummary
} from './types';

const MODES: { value: TaskMode; label: string; desc: string }[] = [
  {
    value: 'qa_fix_retest',
    label: 'QA -> Fix -> Retest',
    desc: 'Recommended full workflow'
  },
  { value: 'qa_fix', label: 'QA -> Fix', desc: 'Skip retest phase' },
  { value: 'qa_only', label: 'QA only', desc: 'Collect failures only' }
];

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
type ConfigEditor = 'form' | 'yaml';

function cloneConfig(config: OrchestratorConfigModel): OrchestratorConfigModel {
  return JSON.parse(JSON.stringify(config)) as OrchestratorConfigModel;
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
  const [mode, setMode] = useState<TaskMode>('qa_fix_retest');
  const [workspaceId, setWorkspaceId] = useState('');
  const [workflowId, setWorkflowId] = useState('');
  const [targets, setTargets] = useState('');
  const [createOptions, setCreateOptions] = useState<CreateTaskOptions | null>(null);

  const [itemFilter, setItemFilter] = useState<ItemFilter>('all');
  const [itemQuery, setItemQuery] = useState('');
  const [liveRefresh, setLiveRefresh] = useState(true);
  const [theme, setTheme] = useState<Theme>('light');

  const [configEditor, setConfigEditor] = useState<ConfigEditor>('form');
  const [configOverview, setConfigOverview] = useState<ConfigOverview | null>(null);
  const [configDraft, setConfigDraft] = useState<OrchestratorConfigModel | null>(null);
  const [yamlDraft, setYamlDraft] = useState('');
  const [configVersions, setConfigVersions] = useState<ConfigVersionSummary[]>([]);
  const [configBusy, setConfigBusy] = useState(false);
  const [configMessage, setConfigMessage] = useState('');

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
    const counts = { qa: 0, fix: 0, retest: 0, custom: 0 };
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

  async function loadTasks() {
    const data = await api.listTasks();
    setTasks(data);
    if (!selectedTaskId && data.length > 0) {
      setSelectedTaskId(data[0].id);
    }
  }

  async function loadTaskDetails(taskId: string) {
    const data = await api.getTaskDetails(taskId);
    setDetail(data);
    const chunks = await api.streamTaskLogs(taskId, 350);
    setLogs(chunks.map((chunk) => chunk.content).join('\n\n'));
  }

  async function refreshSnapshot(forceTaskId?: string) {
    const latest = await api.listTasks();
    setTasks(latest);

    const focusId = forceTaskId ?? selectedTaskId ?? latest[0]?.id ?? null;
    if (focusId) {
      if (!selectedTaskId) {
        setSelectedTaskId(focusId);
      }
      await loadTaskDetails(focusId);
    }
  }

  async function loadCreateTaskOptions() {
    const options = await api.getCreateTaskOptions();
    setCreateOptions(options);
    setWorkspaceId((current) => current || options.defaults.workspace_id);
    setWorkflowId((current) => current || options.defaults.workflow_id);
  }

  async function loadConfigOverview() {
    const overview = await api.getConfigOverview();
    setConfigOverview(overview);
    setConfigDraft(cloneConfig(overview.config));
    setYamlDraft(overview.yaml);
  }

  async function loadConfigVersions() {
    const versions = await api.listConfigVersions();
    setConfigVersions(versions);
  }

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
    if (!liveRefresh || viewTab !== 'tasks') {
      return;
    }
    const timer = setInterval(() => {
      refreshSnapshot().catch((err) => setError(String(err)));
    }, 2000);
    return () => clearInterval(timer);
  }, [selectedTaskId, liveRefresh, viewTab]);

  useEffect(() => {
    if (!selectedTaskId || viewTab !== 'tasks') {
      return;
    }
    loadTaskDetails(selectedTaskId).catch((err) => setError(String(err)));
  }, [selectedTaskId, viewTab]);

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
        mode,
        workspace_id: workspaceId,
        workflow_id: workflowId,
        target_files: targetFiles.length > 0 ? targetFiles : undefined
      });
      await refreshSnapshot(created.id);
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

  function addWorkspace() {
    updateConfig((draft) => {
      const id = `workspace-${Date.now()}`;
      draft.workspaces[id] = {
        root_path: '../..',
        qa_targets: ['docs/qa'],
        ticket_dir: 'docs/ticket'
      };
    });
  }

  function addAgent() {
    updateConfig((draft) => {
      const id = `agent-${Date.now()}`;
      draft.agents[id] = {
        templates: {}
      };
    });
  }

  function addWorkflow() {
    updateConfig((draft) => {
      const id = `workflow-${Date.now()}`;
      const firstAgent = Object.keys(draft.agents)[0] ?? '';
      draft.workflows[id] = {
        qa: firstAgent,
        fix: undefined,
        retest: undefined
      };
    });
  }

  async function saveConfigFromForm() {
    if (!configDraft) {
      return;
    }
    setConfigBusy(true);
    try {
      const overview = await api.saveConfigFromForm({ config: configDraft });
      setConfigOverview(overview);
      setConfigDraft(cloneConfig(overview.config));
      setYamlDraft(overview.yaml);
      setConfigMessage(`Saved config version ${overview.version}`);
      await Promise.all([loadCreateTaskOptions(), loadConfigVersions()]);
      setError('');
    } catch (err) {
      setError(String(err));
    } finally {
      setConfigBusy(false);
    }
  }

  async function validateYamlDraft() {
    setConfigBusy(true);
    try {
      const result = await api.validateConfigYaml({ yaml: yamlDraft });
      setYamlDraft(result.normalized_yaml);
      setConfigMessage('YAML is valid');
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
      setConfigOverview(overview);
      setConfigDraft(cloneConfig(overview.config));
      setYamlDraft(overview.yaml);
      setConfigMessage(`Saved config version ${overview.version}`);
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
            <button className="ghost-button" onClick={() => setLiveRefresh((value) => !value)}>
              Live: {liveRefresh ? 'On' : 'Off'}
            </button>
            <button className="ghost-button" onClick={() => refreshSnapshot().catch((err) => setError(String(err)))}>
              Refresh
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
        <main className="layout">
          <section className="panel create-panel animate-fade-in-up delay-1">
            <h2>Create QA Sprint</h2>
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
              Workflow Mode
              <select
                value={mode}
                onChange={(event) => {
                  const nextMode = event.target.value as TaskMode;
                  setMode(nextMode);
                  if (createOptions?.workflows.some((entry) => entry.id === nextMode)) {
                    setWorkflowId(nextMode);
                  }
                }}
              >
                {MODES.map((entry) => (
                  <option key={entry.value} value={entry.value}>
                    {entry.label} - {entry.desc}
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

          <section className="panel tasks-panel animate-fade-in-up delay-2">
            <h2>Tasks</h2>
            <div className="task-list">
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
                    <div className="task-card-meta">Mode: {task.mode}</div>
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

          <section className="panel detail-panel animate-fade-in-up delay-3">
            <div className="detail-head">
              <div>
                <h2>{selectedTask?.name ?? 'Task Details'}</h2>
                {selectedTask && (
                  <p className="muted">
                    workspace: {selectedTask.workspace_id} | workflow: {selectedTask.workflow_id}
                  </p>
                )}
              </div>
              <div className="actions">
                <button disabled={!selectedTaskId || busy} onClick={() => withTaskAction(() => api.startTask(selectedTaskId!))}>
                  Start
                </button>
                <button disabled={!selectedTaskId || busy} onClick={() => withTaskAction(() => api.pauseTask(selectedTaskId!))}>
                  Pause
                </button>
                <button disabled={!selectedTaskId || busy} onClick={() => withTaskAction(() => api.resumeTask(selectedTaskId!))}>
                  Resume
                </button>
              </div>
            </div>

            <div className="sub-stats-grid">
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
                <span>Fix Runs</span>
                <strong>{runStats.fix}</strong>
              </article>
              <article className="stat-pill">
                <span>Retest Runs</span>
                <strong>{runStats.retest}</strong>
              </article>
            </div>

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

            <div className="detail-bottom">
              <section className="log-section">
                <h3>Live Logs</h3>
                <pre className="log-box">{logs || 'No logs yet.'}</pre>
              </section>

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
            </div>
          </section>
        </main>
      )}

      {viewTab === 'config' && (
        <main className="config-layout">
          <section className="panel config-panel animate-fade-in-up delay-1">
            <div className="detail-head">
              <div>
                <h2>Config Center</h2>
                <p className="muted">
                  {configOverview
                    ? `version ${configOverview.version} | updated ${configOverview.updated_at}`
                    : 'loading...'}
                </p>
              </div>
              <div className="actions">
                <button className="ghost-button" onClick={() => setConfigEditor(configEditor === 'form' ? 'yaml' : 'form')}>
                  Editor: {configEditor}
                </button>
                <button className="ghost-button" disabled={configBusy} onClick={() => loadConfigOverview().catch((err) => setError(String(err)))}>
                  Reload
                </button>
                {configEditor === 'form' ? (
                  <button disabled={configBusy || !configDraft} onClick={saveConfigFromForm}>
                    Save Form
                  </button>
                ) : (
                  <>
                    <button className="ghost-button" disabled={configBusy} onClick={validateYamlDraft}>
                      Validate
                    </button>
                    <button disabled={configBusy} onClick={saveConfigFromYaml}>
                      Save YAML
                    </button>
                  </>
                )}
              </div>
            </div>

            {configMessage && <p className="muted">{configMessage}</p>}

            {configEditor === 'yaml' && (
              <label>
                Config YAML
                <textarea
                  className="yaml-editor"
                  value={yamlDraft}
                  onChange={(event) => setYamlDraft(event.target.value)}
                />
              </label>
            )}

            {configEditor === 'form' && configDraft && (
              <div className="config-grid">
                <section className="config-block">
                  <h3>Defaults</h3>
                  <label>
                    Default Workspace
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
                    Default Workflow
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

                <section className="config-block">
                  <div className="config-block-head">
                    <h3>Workspaces</h3>
                    <button className="ghost-button" onClick={addWorkspace}>Add</button>
                  </div>
                  <div className="config-list">
                    {workspaceKeys.map((id) => {
                      const ws = configDraft.workspaces[id];
                      return (
                        <article key={id} className="config-card">
                          <div className="config-card-head">
                            <strong>{id}</strong>
                            <button
                              className="ghost-button"
                              onClick={() =>
                                updateConfig((draft) => {
                                  delete draft.workspaces[id];
                                })
                              }
                            >
                              Delete
                            </button>
                          </div>
                          <label>
                            Root Path
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
                            QA Targets (one per line)
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
                            Ticket Dir
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
                </section>

                <section className="config-block">
                  <div className="config-block-head">
                    <h3>Agents</h3>
                    <button className="ghost-button" onClick={addAgent}>Add</button>
                  </div>
                  <div className="config-list">
                    {agentKeys.map((id) => {
                      const agent = configDraft.agents[id];
                      return (
                        <article key={id} className="config-card">
                          <div className="config-card-head">
                            <strong>{id}</strong>
                            <button
                              className="ghost-button"
                              onClick={() =>
                                updateConfig((draft) => {
                                  delete draft.agents[id];
                                })
                              }
                            >
                              Delete
                            </button>
                          </div>
                          <label>
                            QA Template
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
                            Fix Template
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
                            Retest Template
                            <textarea
                              value={agent.templates.retest ?? ''}
                              onChange={(event) =>
                                updateConfig((draft) => {
                                  draft.agents[id].templates.retest = event.target.value || undefined;
                                })
                              }
                            />
                          </label>
                        </article>
                      );
                    })}
                  </div>
                </section>

                <section className="config-block">
                  <div className="config-block-head">
                    <h3>Workflows</h3>
                    <button className="ghost-button" onClick={addWorkflow}>Add</button>
                  </div>
                  <div className="config-list">
                    {workflowKeys.map((id) => {
                      const wf = configDraft.workflows[id];
                      return (
                        <article key={id} className="config-card">
                          <div className="config-card-head">
                            <strong>{id}</strong>
                            <button
                              className="ghost-button"
                              onClick={() =>
                                updateConfig((draft) => {
                                  delete draft.workflows[id];
                                })
                              }
                            >
                              Delete
                            </button>
                          </div>
                          <label>
                            QA Agent
                            <select
                              value={wf.qa}
                              onChange={(event) =>
                                updateConfig((draft) => {
                                  draft.workflows[id].qa = event.target.value;
                                })
                              }
                            >
                              {agentKeys.map((agentId) => (
                                <option key={agentId} value={agentId}>
                                  {agentId}
                                </option>
                              ))}
                            </select>
                          </label>
                          <label>
                            Fix Agent (optional)
                            <select
                              value={wf.fix ?? ''}
                              onChange={(event) =>
                                updateConfig((draft) => {
                                  draft.workflows[id].fix = event.target.value || undefined;
                                })
                              }
                            >
                              <option value="">(none)</option>
                              {agentKeys.map((agentId) => (
                                <option key={agentId} value={agentId}>
                                  {agentId}
                                </option>
                              ))}
                            </select>
                          </label>
                          <label>
                            Retest Agent (optional)
                            <select
                              value={wf.retest ?? ''}
                              onChange={(event) =>
                                updateConfig((draft) => {
                                  draft.workflows[id].retest = event.target.value || undefined;
                                })
                              }
                            >
                              <option value="">(none)</option>
                              {agentKeys.map((agentId) => (
                                <option key={agentId} value={agentId}>
                                  {agentId}
                                </option>
                              ))}
                            </select>
                          </label>
                        </article>
                      );
                    })}
                  </div>
                </section>
              </div>
            )}
          </section>

          <section className="panel config-history-panel animate-fade-in-up delay-2">
            <h3>Config Versions</h3>
            <div className="config-history-list">
              {configVersions.map((entry) => (
                <button
                  key={entry.version}
                  className="task-card"
                  onClick={() =>
                    api
                      .getConfigVersion(entry.version)
                      .then((detail) => {
                        setYamlDraft(detail.yaml);
                        setConfigEditor('yaml');
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
          </section>
        </main>
      )}
    </div>
  );
}
