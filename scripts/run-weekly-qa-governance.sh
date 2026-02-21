#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

RUN_LINT=true
for arg in "$@"; do
  case "$arg" in
    --no-lint) RUN_LINT=false ;;
    *)
      echo "Unknown option: $arg"
      echo "Usage: $0 [--no-lint]"
      exit 2
      ;;
  esac
done

TS="$(date '+%Y%m%d_%H%M%S')"
LOG_DIR=".build-cache/qa-governance"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/weekly-${TS}.log"

{
  echo "[weekly-qa-governance] started: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  echo "[weekly-qa-governance] root: $ROOT_DIR"
  echo
  echo "[step] expanded audit"
  bash .claude/skills/qa-doc-governance/scripts/run-audit.sh
  echo
  if [[ "$RUN_LINT" == "true" ]]; then
    echo "[step] strict lint gate"
    ./scripts/qa-doc-lint.sh
  else
    echo "[step] strict lint gate skipped (--no-lint)"
  fi
  echo
  echo "[weekly-qa-governance] completed: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
} | tee "$LOG_FILE"

echo
echo "Report saved: $LOG_FILE"
