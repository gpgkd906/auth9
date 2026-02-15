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

## Test & Coverage

Install test dependencies once:

```bash
cd tools/qa-orchestrator
npm install -D vitest @vitest/coverage-v8
```

```bash
cd tools/qa-orchestrator
npm run test
npm run test:coverage
npm run test:tauri
npm run test:tauri:coverage
```

Coverage requirement (unit scope): `>= 90%` for lines/functions/branches/statements.

Current coverage gates:

- Frontend: `vitest.config.ts` (>=90%)
- Tauri: `src-tauri/Makefile` using `cargo llvm-cov --fail-under-lines 90`

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
./tools/qa-orchestrator/scripts/run-cli.sh --workspace auth9 --workflow qa_fix_retest
./tools/qa-orchestrator/scripts/run-cli.sh --target-file docs/qa/user/01-crud.md
./tools/qa-orchestrator/scripts/run-cli.sh --no-auto-resume --workflow qa_only
```

## Workflow Model

- Workflow is a configurable step pipeline: `init_once`, `qa`, `ticket_scan`, `fix`, `retest`
- Each step can be enabled/disabled and mapped to an agent
- `ticket_scan` is a built-in step (no agent required) that scans `ticket_dir` and maps active tickets to task items
- Each step can define optional `prehook` rules to decide run/skip per item
- Workflow supports `finalize.rules[]` to decide final item status (`skipped/qa_passed/fixed/verified/unresolved`) via CEL
- Loop policy is defined per workflow: `once` or `infinite`
- Loop guard supports rule-based stop conditions and optional guard agent decision (`loop.guard.agent_id`)

## Prehook (Low-friction mode)

- Default editor is **Visual Rules** (no CEL required)
- Built-in presets:
  - `ticket_scan`: always run (or guard by active ticket context)
  - `fix`: run only when `active_ticket_count > 0`
  - `retest`: run only when `active_ticket_count > 0 && fix_exit_code == 0`
- `Advanced CEL` mode is optional for power users
- Runtime still evaluates `prehook.when` (CEL); visual editor writes CEL automatically
- `Simulate` in both modes runs backend CEL evaluator (`simulate_prehook`) for parity with runtime
- UI-only metadata is stored under `prehook.ui` for round-trip editing
- Final state decisions can be configured with `workflow.finalize.rules[]` (first-match wins)

Available visual fields:

- `active_ticket_count`, `new_ticket_count`, `cycle`
- `qa_exit_code`, `fix_exit_code`, `retest_exit_code`
- `qa_failed`, `fix_required`

## Config Model

`config/default.yaml` defines:

- `workspaces`: isolated roots and path scopes (`root_path`, `qa_targets`, `ticket_dir`)
- `agents`: step templates (`init_once`, `qa`, `fix`, `retest`, `loop_guard`)
- `workflows`: step array + loop policy
- `defaults`: default `workspace` and `workflow`

Runtime source of truth:

- active config is stored in SQLite (`orchestrator_config` tables)
- `config/default.yaml` is updated on every save as mirror/export
- config changes hot-reload for new task creation; running tasks keep their own snapshots

Template placeholders:

- `{rel_path}`: current QA/security markdown file path
- `{ticket_paths}`: space-separated ticket file paths for current item
- loop guard template placeholders: `{task_id}`, `{cycle}`, `{unresolved_items}`

Path safety rules:

- all task paths are resolved relative to the selected workspace root
- path escape (`..`) is rejected
- existing paths are canonicalized and must remain inside workspace root

## Existing Scripts Compatibility

Existing scripts remain usable:

- `scripts/run-qa-tests.sh`
- `scripts/fix-tickets.sh`

Use `--orchestrator` on either script to launch this UI workflow.
