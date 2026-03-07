#!/usr/bin/env bash
# QA Scenario Runner Framework
# Provides scenario registration, execution, timing, and JSONL output.
#
# Usage in test scripts:
#   #!/usr/bin/env bash
#   set -euo pipefail
#   SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
#   source "$SCRIPT_DIR/../lib/runner.sh"
#
#   scenario 1 "My test" '
#     resp=$(api_get /health)
#     assert_http_status "$(resp_status "$resp")" 200 "health ok"
#   '
#
#   run_all

_RUNNER_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$_RUNNER_LIB_DIR/setup.sh"
source "$_RUNNER_LIB_DIR/assert.sh"

# Scenario storage: parallel arrays
_SCENARIO_IDS=()
_SCENARIO_TITLES=()
_SCENARIO_BODIES=()

# Counters
_TOTAL_SCENARIOS=0
_PASSED_SCENARIOS=0
_FAILED_SCENARIOS=0
_SKIPPED_SCENARIOS=0
_RUNNER_START_MS=0

# ---------------------------------------------------------------------------
# Portable millisecond timestamp
# ---------------------------------------------------------------------------
_now_ms() {
  if command -v gdate &>/dev/null; then
    echo $(( $(gdate +%s%N) / 1000000 ))
  elif date +%s%N &>/dev/null 2>&1 && [[ "$(date +%s%N)" != *N ]]; then
    echo $(( $(date +%s%N) / 1000000 ))
  else
    echo $(( $(date +%s) * 1000 ))
  fi
}

# ---------------------------------------------------------------------------
# Register a scenario (deferred execution)
# ---------------------------------------------------------------------------
scenario() {
  local id="$1" title="$2" body="$3"
  _SCENARIO_IDS+=("$id")
  _SCENARIO_TITLES+=("$title")
  _SCENARIO_BODIES+=("$body")
}

# ---------------------------------------------------------------------------
# Skip a scenario with a reason
# ---------------------------------------------------------------------------
skip_scenario() {
  local id="$1" title="$2" reason="$3"
  _SKIPPED_SCENARIOS=$((_SKIPPED_SCENARIOS + 1))
  _TOTAL_SCENARIOS=$((_TOTAL_SCENARIOS + 1))
  printf '{"type":"skip","id":%s,"title":"%s","reason":"%s"}\n' \
    "$id" "$title" "$reason"
}

# ---------------------------------------------------------------------------
# Execute all registered scenarios, then print summary
# ---------------------------------------------------------------------------
run_all() {
  _RUNNER_START_MS=$(_now_ms)
  local count=${#_SCENARIO_IDS[@]}

  for ((i = 0; i < count; i++)); do
    _run_one_scenario "${_SCENARIO_IDS[$i]}" "${_SCENARIO_TITLES[$i]}" "${_SCENARIO_BODIES[$i]}"
  done

  local end_ms
  end_ms=$(_now_ms)
  local total_duration_ms=$(( end_ms - _RUNNER_START_MS ))

  printf '{"type":"summary","total":%d,"passed":%d,"failed":%d,"skipped":%d,"duration_ms":%d}\n' \
    "$_TOTAL_SCENARIOS" "$_PASSED_SCENARIOS" "$_FAILED_SCENARIOS" "$_SKIPPED_SCENARIOS" "$total_duration_ms"

  if [[ "$_FAILED_SCENARIOS" -gt 0 ]]; then
    exit 1
  fi
}

# ---------------------------------------------------------------------------
# Internal: execute a single scenario in a subshell
# ---------------------------------------------------------------------------
_run_one_scenario() {
  local id="$1" title="$2" body="$3"
  _TOTAL_SCENARIOS=$((_TOTAL_SCENARIOS + 1))

  local start_ms end_ms duration_ms
  start_ms=$(_now_ms)

  # Reset assertion counters for this scenario
  qa_reset_counts

  # Capture assertion JSONL output from the scenario body
  local scenario_output
  local scenario_exit=0
  scenario_output=$(eval "$body" 2>&1) || scenario_exit=$?

  end_ms=$(_now_ms)
  duration_ms=$(( end_ms - start_ms ))

  # Parse assertion counts from the output
  local assertions=0 failures=0
  while IFS= read -r line; do
    if [[ "$line" == '{"type":"assert"'* ]]; then
      assertions=$((assertions + 1))
      if echo "$line" | grep -q '"status":"FAIL"'; then
        failures=$((failures + 1))
      fi
    fi
  done <<< "$scenario_output"

  # Determine scenario status
  local status="PASS"
  if [[ "$scenario_exit" -ne 0 ]] || [[ "$failures" -gt 0 ]]; then
    status="FAIL"
    _FAILED_SCENARIOS=$((_FAILED_SCENARIOS + 1))
  else
    _PASSED_SCENARIOS=$((_PASSED_SCENARIOS + 1))
  fi

  # Emit individual assertion lines (for detailed reports)
  while IFS= read -r line; do
    if [[ "$line" == '{"type":"assert"'* ]]; then
      echo "$line"
    elif [[ -n "$line" && "$line" != '{"type":'* ]]; then
      echo "$line" >&2
    fi
  done <<< "$scenario_output"

  # Emit scenario summary
  local error_msg=""
  if [[ "$status" == "FAIL" && "$scenario_exit" -ne 0 && "$failures" -eq 0 ]]; then
    error_msg=$(echo "$scenario_output" | grep -v '^{"type":' | tail -1 | sed 's/"/\\"/g')
  fi

  if [[ -n "$error_msg" ]]; then
    printf '{"type":"scenario","id":%s,"title":"%s","status":"%s","assertions":%d,"failures":%d,"duration_ms":%d,"error":"%s"}\n' \
      "$id" "$title" "$status" "$assertions" "$failures" "$duration_ms" "$error_msg"
  else
    printf '{"type":"scenario","id":%s,"title":"%s","status":"%s","assertions":%d,"failures":%d,"duration_ms":%d}\n' \
      "$id" "$title" "$status" "$assertions" "$failures" "$duration_ms"
  fi
}
