# Auth9 QA Orchestrator

Tauri + React based QA workflow orchestrator for Auth9.

## Features

- SQLite-backed `task -> task_item` lifecycle tracking
- Workspace isolation (`workspace`) for root path and document scope
- Agent-driven command templates (`agent`) bound by `workflow` phase mapping
- Full shell command passthrough (`/bin/zsh -lc` by default)
- Auto-resume latest unfinished task on startup
- Real-time dashboard for task list, item progress, and command logs
- Config Center with `Form`/`YAML` switch for workspace/workflow/agent editing
- Config persistence in SQLite with hot reload for new tasks

## Directory

- `config/default.yaml`: command templates and runtime config
- `data/qa_orchestrator.db`: SQLite database (runtime)
- `data/logs/`: command stdout/stderr logs
- `src-tauri/`: orchestrator backend and scheduler
- `src/`: React dashboard

## Run

```bash
cd tools/qa-orchestrator
npm install
npm run tauri:dev
```

## UI vs CLI behavior

- UI startup (`npm run tauri:dev` or `scripts/open-ui.sh`):
  - does **not** auto-resume and does **not** auto-start QA
  - shows existing tasks and waits for user action (`Start`/`Resume`)
- CLI startup (`scripts/run-cli.sh`):
  - auto-resumes latest unfinished task (`running/interrupted/paused/pending`)
  - if no unfinished task exists, auto-creates a new task and starts execution

CLI examples:

```bash
./tools/qa-orchestrator/scripts/run-cli.sh
./tools/qa-orchestrator/scripts/run-cli.sh --mode qa_fix_retest
./tools/qa-orchestrator/scripts/run-cli.sh --workspace auth9 --workflow qa_fix_retest
./tools/qa-orchestrator/scripts/run-cli.sh --target-file docs/qa/user/01-crud.md
./tools/qa-orchestrator/scripts/run-cli.sh --no-auto-resume --mode qa_only
```

## Workflow Modes

- `qa_only`: run QA and collect unresolved failures
- `qa_fix`: run QA and auto-fix tickets
- `qa_fix_retest`: run QA, fix tickets, then retest (default)

## Config Model

`config/default.yaml` defines:

- `workspaces`: isolated roots and path scopes (`root_path`, `qa_targets`, `ticket_dir`)
- `agents`: phase templates (`qa`, `fix`, `retest`)
- `workflows`: `phase -> agent` mapping
- `defaults`: default `workspace` and `workflow`

Runtime source of truth:

- active config is stored in SQLite (`orchestrator_config` tables)
- `config/default.yaml` is updated on every save as mirror/export
- config changes hot-reload for new task creation; running tasks keep their own snapshots

Template placeholders:

- `{rel_path}`: current QA/security markdown file path
- `{ticket_paths}`: space-separated ticket file paths for current item

Path safety rules:

- all task paths are resolved relative to the selected workspace root
- path escape (`..`) is rejected
- existing paths are canonicalized and must remain inside workspace root

## Existing Scripts Compatibility

Existing scripts remain usable:

- `scripts/run-qa-tests.sh`
- `scripts/fix-tickets.sh`

Use `--orchestrator` on either script to launch this UI workflow.
