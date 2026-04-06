#!/usr/bin/env bash
# Run QA tests for each document in docs/qa/, docs/security/, docs/uiux/.
# Supports two execution paths:
#   FAST PATH: docs with a matching test script in scripts/qa/auto/ run directly
#   AGENT PATH: docs without scripts are tested via an AI agent
set -euo pipefail

# Ensure child processes are killed when script is interrupted
cleanup() {
    echo ""
    echo -e "\033[0;31mInterrupted. Killing child processes...\033[0m"
    # Kill entire process group
    kill -- -$$ 2>/dev/null || true
}
trap cleanup SIGINT SIGTERM SIGHUP

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
QA_DIR="$PROJECT_ROOT/docs/qa"
SECURITY_DIR="$PROJECT_ROOT/docs/security"
UIUX_DIR="$PROJECT_ROOT/docs/uiux"
OIDC_DIR="$PROJECT_ROOT/docs/oidc"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Timeouts
AGENT_TIMEOUT=1800   # 30 minutes per agent run
SCRIPT_TIMEOUT=300   # 5 minutes per script run
if command -v gtimeout &>/dev/null; then
    TIMEOUT_CMD="gtimeout"
elif command -v timeout &>/dev/null; then
    TIMEOUT_CMD="timeout"
else
    TIMEOUT_CMD=""
fi

# Progress file for resume support
PROGRESS_FILE="$SCRIPT_DIR/qa/.run-qa-test.json"

# Counters
total=0
passed=0
failed=0
failed_files=()
declare -A completed_results

# Save progress to JSON after each file
save_progress() {
    mkdir -p "$(dirname "$PROGRESS_FILE")"
    python3 -c "
import json, sys
results = {}
for line in sys.stdin:
    line = line.strip()
    if line and '=' in line:
        k, v = line.split('=', 1)
        results[k] = v
data = {
    'agent_mode': sys.argv[1],
    'only_mode': sys.argv[2],
    'results': results
}
with open(sys.argv[3], 'w') as f:
    json.dump(data, f, indent=2, ensure_ascii=False)
" "$AGENT_MODE" "$ONLY_MODE" "$PROGRESS_FILE" < <(
        for key in "${!completed_results[@]}"; do
            echo "${key}=${completed_results[$key]}"
        done
    )
}

# Load progress from JSON, returns 0 on success
load_progress() {
    [[ -f "$PROGRESS_FILE" ]] || return 1

    local progress
    progress=$(python3 -c "
import json, sys
with open(sys.argv[1]) as f:
    data = json.load(f)
print(data.get('agent_mode', ''))
print(data.get('only_mode', ''))
for file, status in data.get('results', {}).items():
    print('RESULT:' + file + '=' + status)
" "$PROGRESS_FILE" 2>/dev/null) || return 1

    local saved_mode saved_only_mode
    saved_mode=$(sed -n '1p' <<< "$progress")
    saved_only_mode=$(sed -n '2p' <<< "$progress")

    if [[ "$saved_mode" != "$AGENT_MODE" ]]; then
        echo -e "${YELLOW}Note: switching agent from '${saved_mode}' to '${AGENT_MODE}' for remaining files.${NC}"
    fi

    # Inherit only_mode from progress file unless explicitly overridden by CLI
    if [[ -z "$ONLY_MODE" && -n "$saved_only_mode" ]]; then
        ONLY_MODE="$saved_only_mode"
        echo -e "${YELLOW}Note: restored only_mode='${ONLY_MODE}' from progress file.${NC}"
    fi

    while IFS= read -r line; do
        if [[ "$line" == RESULT:* ]]; then
            local entry="${line#RESULT:}"
            local file="${entry%%=*}"
            local status="${entry#*=}"
            completed_results["$file"]="$status"
            total=$((total + 1))
            case "$status" in
                passed) passed=$((passed + 1)) ;;
                timeout)
                    failed=$((failed + 1))
                    failed_files+=("$file (TIMEOUT)")
                    ;;
                *)
                    failed=$((failed + 1))
                    failed_files+=("$file")
                    ;;
            esac
        fi
    done <<< "$progress"

    echo -e "${GREEN}Resumed from progress file: ${total} completed (${passed} passed, ${failed} failed)${NC}"
    return 0
}

# ---------------------------------------------------------------------------
# FAST PATH: resolve and run pre-built test scripts
# ---------------------------------------------------------------------------
script_passed=0
script_failed=0

# Resolve a test script path for a given doc.
# 1) For docs/qa/* or docs/oidc/*: look up test_script in _manifest.yaml
# 2) Fallback for any doc: derive path scripts/qa/auto/{dir-parts}.sh
# Returns the script path on stdout (empty if none found).
resolve_test_script() {
    local rel_path="$1"
    local script_path=""

    # Strategy 1: manifest lookup (docs/qa/ and docs/oidc/)
    local manifest_file="" doc_id=""
    if [[ "$rel_path" == docs/qa/* ]]; then
        manifest_file="$PROJECT_ROOT/docs/qa/_manifest.yaml"
        doc_id="${rel_path#docs/qa/}"
        doc_id="${doc_id%.md}"
    elif [[ "$rel_path" == docs/oidc/* ]]; then
        manifest_file="$PROJECT_ROOT/docs/oidc/_manifest.yaml"
        doc_id="${rel_path#docs/oidc/}"
        doc_id="${doc_id%.md}"
    fi
    if [[ -n "$manifest_file" && -f "$manifest_file" ]]; then
        script_path=$(python3 -c "
import sys
try:
    import yaml
except ImportError:
    sys.exit(0)
try:
    with open(sys.argv[2]) as f:
        data = yaml.safe_load(f)
    for doc in data.get('documents', []):
        if doc.get('id') == sys.argv[1]:
            print(doc.get('test_script', ''))
            break
except Exception:
    pass
" "$doc_id" "$manifest_file" 2>/dev/null || true)
    fi

    # Strategy 2: convention-based derivation
    if [[ -z "$script_path" ]]; then
        local base="${rel_path#docs/qa/}"
        if [[ "$base" == "$rel_path" ]]; then
            base="${rel_path#docs/}"
        fi
        base="${base%.md}"
        base="${base//\//-}"
        local candidate="scripts/qa/auto/${base}.sh"
        if [[ -f "$PROJECT_ROOT/$candidate" ]]; then
            script_path="$candidate"
        fi
    fi

    echo "$script_path"
}

# Execute a test script and parse JSONL summary output.
# Returns 0 on all-pass, 1 on any failure, 124 on timeout.
run_test_script() {
    local script_path="$1"
    local abs_script="$PROJECT_ROOT/$script_path"
    local output_file
    output_file=$(mktemp)

    local t=()
    if [[ -n "${TIMEOUT_CMD:-}" ]]; then
        t=("$TIMEOUT_CMD" "$SCRIPT_TIMEOUT")
    fi

    echo -e "${YELLOW}> [FAST] bash ${script_path}${NC}"
    local exit_code=0
    "${t[@]}" bash "$abs_script" > "$output_file" 2>&1 || exit_code=$?

    # Parse JSONL summary (last line with type=summary)
    local summary_line
    summary_line=$(grep '"type":"summary"' "$output_file" | tail -1 || true)

    if [[ -n "$summary_line" ]]; then
        local s_passed s_failed s_total
        s_passed=$(echo "$summary_line" | python3 -c "import json,sys; print(json.load(sys.stdin)['passed'])" 2>/dev/null || echo 0)
        s_failed=$(echo "$summary_line" | python3 -c "import json,sys; print(json.load(sys.stdin)['failed'])" 2>/dev/null || echo 0)
        s_total=$(echo "$summary_line" | python3 -c "import json,sys; print(json.load(sys.stdin)['total'])" 2>/dev/null || echo 0)
        script_passed=$((script_passed + s_passed))
        script_failed=$((script_failed + s_failed))
        echo -e "  Scenarios: ${s_total} total, ${GREEN}${s_passed} passed${NC}, ${RED}${s_failed} failed${NC}"
    fi

    # On failure, save full output for debugging
    if [[ $exit_code -ne 0 ]]; then
        local date_dir="$PROJECT_ROOT/artifacts/qa/$(date +%F)"
        mkdir -p "$date_dir"
        local log_name="${script_path##*/}"
        log_name="${log_name%.sh}.jsonl"
        cp "$output_file" "$date_dir/$log_name"
        echo -e "  ${YELLOW}Full output saved: artifacts/qa/$(date +%F)/$log_name${NC}"
    fi

    rm -f "$output_file"
    return $exit_code
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    echo "Usage: $0 [--agent <mode>] [--only-security|--only-uiux|--only-oidc] [--no-scripts] [--resume] [cli-options]"
    echo ""
    echo "Default: run all QA docs (docs/qa/, docs/security/, docs/uiux/, docs/oidc/)."
    echo "Docs with pre-built scripts in scripts/qa/auto/ run directly (FAST PATH)."
    echo "Docs without scripts are tested via an AI agent (AGENT PATH)."
    echo ""
    echo "  --agent <mode>       Agent mode: 'minicode' (default), 'minimax', 'opencode', 'gemini', 'glm-5', 'gpt-mini'"
    echo "  --only-security      Run only security test documents (docs/security/)"
    echo "  --only-uiux          Run only UI/UX test documents (docs/uiux/)"
    echo "  --only-oidc          Run only OIDC conformance test documents (docs/oidc/)"
    echo "  --no-scripts         Disable FAST PATH (always use agent)"
    echo "  --resume             Resume from last interrupted run"
    exit 0
fi

# Parse flags
AGENT_MODE="minicode"
ONLY_MODE=""  # empty = all, 'security' = only security, 'uiux' = only uiux, 'oidc' = only oidc
RESUME=false
USE_SCRIPTS=true
while [[ $# -gt 0 ]]; do
    case "$1" in
        --agent)
            AGENT_MODE="${2:-minicode}"
            shift 2
            ;;
        --only-security)
            ONLY_MODE="security"
            shift
            ;;
        --only-uiux)
            ONLY_MODE="uiux"
            shift
            ;;
        --only-oidc)
            ONLY_MODE="oidc"
            shift
            ;;
        --no-scripts)
            USE_SCRIPTS=false
            shift
            ;;
        --resume)
            RESUME=true
            shift
            ;;
        *)
            break
            ;;
    esac
done

# Load progress BEFORE file collection so INCLUDE_UIUX can be inherited
if [[ "$RESUME" == true ]]; then
    if load_progress; then
        RESUME_LOADED=true
    else
        RESUME_LOADED=false
        echo -e "${YELLOW}No valid progress file found. Starting from scratch.${NC}"
    fi
else
    RESUME_LOADED=false
fi

echo -e "${YELLOW}Agent mode: ${AGENT_MODE}${NC}"
if [[ -n "${TIMEOUT_CMD:-}" ]]; then
    echo -e "${YELLOW}Timeout: $((AGENT_TIMEOUT / 60)) minutes per agent run (${TIMEOUT_CMD})${NC}"
else
    echo -e "${YELLOW}Timeout: disabled (install coreutils for timeout support)${NC}"
fi
case "$ONLY_MODE" in
    security) echo -e "${YELLOW}Scope: security tests only (docs/security/)${NC}" ;;
    uiux)     echo -e "${YELLOW}Scope: UI/UX tests only (docs/uiux/)${NC}" ;;
    oidc)     echo -e "${YELLOW}Scope: OIDC conformance tests only (docs/oidc/)${NC}" ;;
    *)        echo -e "${YELLOW}Scope: all tests (docs/qa/, docs/security/, docs/uiux/, docs/oidc/)${NC}" ;;
esac
if [[ "$RESUME" == true ]]; then
    echo -e "${YELLOW}Resume: enabled${NC}"
fi
echo ""

echo -e "${CYAN}========================================${NC}"
echo -e "${CYAN}  Auth9 QA Test Runner${NC}"
echo -e "${CYAN}========================================${NC}"
echo ""

# Collect .md files based on ONLY_MODE, excluding README.md
# Default (empty): docs/qa/ + docs/security/ + docs/uiux/ + docs/oidc/
# --only-security: docs/security/ only
# --only-uiux: docs/uiux/ only
# --only-oidc: docs/oidc/ only
mapfile -t qa_files < <(
    {
        case "$ONLY_MODE" in
            security)
                find "$SECURITY_DIR" -name '*.md' ! -name 'README.md' 2>/dev/null || true
                ;;
            uiux)
                find "$UIUX_DIR" -name '*.md' ! -name 'README.md' 2>/dev/null || true
                ;;
            oidc)
                find "$OIDC_DIR" -name '*.md' ! -name 'README.md' ! -name '_standards.md' 2>/dev/null || true
                ;;
            *)
                find "$QA_DIR" -name '*.md' ! -name 'README.md' ! -name '_standards.md' 2>/dev/null || true
                find "$SECURITY_DIR" -name '*.md' ! -name 'README.md' 2>/dev/null || true
                find "$UIUX_DIR" -name '*.md' ! -name 'README.md' 2>/dev/null || true
                find "$OIDC_DIR" -name '*.md' ! -name 'README.md' ! -name '_standards.md' 2>/dev/null || true
                ;;
        esac
    } | sort
)

if [[ ${#qa_files[@]} -eq 0 ]]; then
    echo -e "${YELLOW}No test documents found.${NC}"
    exit 0
fi

echo -e "Found ${#qa_files[@]} test document(s) to process."

if [[ "$RESUME_LOADED" == true ]]; then
    remaining=$(( ${#qa_files[@]} - total ))
    echo -e "${YELLOW}Remaining: ${remaining} file(s) to process.${NC}"
fi
echo ""

# Run agent command based on AGENT_MODE
# Each invocation is wrapped with an optional timeout prefix
# Helper: print command then execute
run_cmd() {
    echo -e "${YELLOW}> $*${NC}"
    "$@"
}

run_agent() {
    local rel_path="$1"
    # When TIMEOUT_CMD is set, prefix each command with "timeout 1800"
    local t=()
    if [[ -n "${TIMEOUT_CMD:-}" ]]; then
        t=("$TIMEOUT_CMD" "$AGENT_TIMEOUT")
    fi
    case "$AGENT_MODE" in
        opencode)
            run_cmd "${t[@]}" opencode run "读取文档：${rel_path}，执行QA测试" -m "deepseek/deepseek-chat"
            ;;
        gemini)
            run_cmd "${t[@]}" gemini -m gemini-3-flash-preview -p "读取文档：${rel_path}，执行QA测试" --yolo
            ;;
        minicode)
            run_cmd "${t[@]}" minicode -p --dangerously-skip-permissions "读取文档：${rel_path}，执行QA测试"
            ;;
        minimax)
            run_cmd "${t[@]}" opencode run "读取文档：${rel_path}，执行QA测试" -m "minimax-coding-plan/MiniMax-M2.7-highspeed"
            ;;
        glm-5)
            run_cmd "${t[@]}" kilocode run "读取文档：${rel_path}，执行QA测试" -m "kilo/z-ai/glm-5:free"
            ;;
        gpt-mini)
            local codex_cmd="codex"
            local codex_model="gpt-5.4-mini"
            local prompt="读取文档：${rel_path}，执行QA测试"
            run_cmd "${t[@]}" "$codex_cmd" exec -m "$codex_model" --full-auto "$prompt"
            ;;
        *)
            echo -e "${RED}Unknown agent mode: ${AGENT_MODE}${NC}"
            return 1
            ;;
    esac
}

for ((i=0; i<${#qa_files[@]}; i++)); do
    file="${qa_files[$i]}"
    # Convert to relative path from project root
    rel_path="${file#"$PROJECT_ROOT"/}"

    # Skip already completed files (resume mode)
    if [[ -n "${completed_results[$rel_path]+x}" ]]; then
        echo -e "${CYAN}[$(( i + 1 ))/${#qa_files[@]}] Skipped (done): ${rel_path} [${completed_results[$rel_path]}]${NC}"
        continue
    fi

    total=$((total + 1))

    echo -e "${CYAN}----------------------------------------${NC}"
    echo -e "${CYAN}[$(( i + 1 ))/${#qa_files[@]}] Processing: ${rel_path}${NC}"
    echo -e "${CYAN}----------------------------------------${NC}"

    # Try FAST PATH: run pre-built test script directly
    test_script=""
    if [[ "$USE_SCRIPTS" == true ]]; then
        test_script=$(resolve_test_script "$rel_path")
    fi

    exit_code=0
    if [[ -n "$test_script" && -f "$PROJECT_ROOT/$test_script" ]]; then
        run_test_script "$test_script" && exit_code=0 || exit_code=$?
    else
        run_agent "$rel_path" && exit_code=0 || exit_code=$?
    fi

    if [[ $exit_code -eq 0 ]]; then
        passed=$((passed + 1))
        completed_results["$rel_path"]="passed"
        echo -e "${GREEN}✓ PASSED: ${rel_path}${NC}"
    elif [[ $exit_code -eq 124 ]]; then
        failed=$((failed + 1))
        failed_files+=("$rel_path (TIMEOUT)")
        completed_results["$rel_path"]="timeout"
        if [[ -n "$test_script" ]]; then
            echo -e "${RED}✗ TIMEOUT: ${rel_path} (script exceeded 5 minutes)${NC}"
        else
            echo -e "${RED}✗ TIMEOUT: ${rel_path} (agent exceeded 30 minutes)${NC}"
        fi
    else
        failed=$((failed + 1))
        failed_files+=("$rel_path")
        completed_results["$rel_path"]="failed"
        echo -e "${RED}✗ FAILED: ${rel_path}${NC}"
    fi

    # Save progress after each file
    save_progress

    echo ""
    sleep 2
done

# Summary
echo -e "${CYAN}========================================${NC}"
echo -e "${CYAN}  Summary${NC}"
echo -e "${CYAN}========================================${NC}"
echo -e "Total docs:  $total"
echo -e "${GREEN}Passed: $passed${NC}"
echo -e "${RED}Failed: $failed${NC}"
if [[ "$script_passed" -gt 0 || "$script_failed" -gt 0 ]]; then
    echo -e ""
    echo -e "Script scenarios: ${GREEN}${script_passed} passed${NC}, ${RED}${script_failed} failed${NC}"
fi

if [[ ${#failed_files[@]} -gt 0 ]]; then
    echo ""
    echo -e "${RED}Failed files:${NC}"
    for f in "${failed_files[@]}"; do
        echo -e "  ${RED}- $f${NC}"
    done
    exit 1
fi

# Clean up progress file on full completion
rm -f "$PROGRESS_FILE"

echo ""
echo -e "${GREEN}All QA tests passed!${NC}"
