import { useEffect, useMemo, useState } from 'react';
import { api } from './api';
import type { TaskDetail, TaskMode, TaskSummary } from './types';

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

export function App() {
  const [tasks, setTasks] = useState<TaskSummary[]>([]);
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null);
  const [detail, setDetail] = useState<TaskDetail | null>(null);
  const [logs, setLogs] = useState<string>('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>('');

  const [name, setName] = useState('');
  const [goal, setGoal] = useState('Automate QA sprint with auto-fix and restart');
  const [mode, setMode] = useState<TaskMode>('qa_fix_retest');
  const [targets, setTargets] = useState('');

  const [itemFilter, setItemFilter] = useState<ItemFilter>('all');
  const [itemQuery, setItemQuery] = useState('');
  const [liveRefresh, setLiveRefresh] = useState(true);
  const [theme, setTheme] = useState<Theme>('light');

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

  useEffect(() => {
    const saved = (localStorage.getItem('auth9-theme') as Theme | null) ?? 'light';
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
        await refreshSnapshot();
        setError('');
      })
      .catch((err) => setError(String(err)));
  }, []);

  useEffect(() => {
    if (!liveRefresh) {
      return;
    }
    const timer = setInterval(() => {
      refreshSnapshot().catch((err) => setError(String(err)));
    }, 2000);
    return () => clearInterval(timer);
  }, [selectedTaskId, liveRefresh]);

  useEffect(() => {
    if (!selectedTaskId) {
      setDetail(null);
      setLogs('');
      return;
    }
    loadTaskDetails(selectedTaskId).catch((err) => setError(String(err)));
  }, [selectedTaskId]);

  async function createTask() {
    setBusy(true);
    try {
      const targetFiles = targets
        .split('\n')
        .map((value) => value.trim())
        .filter(Boolean);
      const created = await api.createTask({
        name: name.trim() || undefined,
        goal: goal.trim() || undefined,
        mode,
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
            Workflow Mode
            <select value={mode} onChange={(event) => setMode(event.target.value as TaskMode)}>
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
          <button onClick={createTask} disabled={busy}>
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
            <h2>{selectedTask?.name ?? 'Task Details'}</h2>
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
    </div>
  );
}
