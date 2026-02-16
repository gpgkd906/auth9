#!/usr/bin/env bash
# Run QA tests for each document in docs/qa/ and docs/security/ using opencode
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
ORCHESTRATOR_LAUNCHER="$PROJECT_ROOT/tools/qa-orchestrator/scripts/open-ui.sh"
ORCHESTRATOR_CLI="$PROJECT_ROOT/tools/qa-orchestrator/scripts/run-cli.sh"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Timeout: 30 minutes per agent run
AGENT_TIMEOUT=1800
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
    'include_uiux': sys.argv[2] == 'true',
    'results': results
}
with open(sys.argv[3], 'w') as f:
    json.dump(data, f, indent=2, ensure_ascii=False)
" "$AGENT_MODE" "$INCLUDE_UIUX" "$PROGRESS_FILE" < <(
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
print(str(data.get('include_uiux', False)).lower())
for file, status in data.get('results', {}).items():
    print('RESULT:' + file + '=' + status)
" "$PROGRESS_FILE" 2>/dev/null) || return 1

    local saved_mode saved_uiux
    saved_mode=$(sed -n '1p' <<< "$progress")
    saved_uiux=$(sed -n '2p' <<< "$progress")

    if [[ "$saved_mode" != "$AGENT_MODE" ]]; then
        echo -e "${YELLOW}Note: switching agent from '${saved_mode}' to '${AGENT_MODE}' for remaining files.${NC}"
    fi

    # Inherit include_uiux from progress file unless explicitly overridden by CLI
    if [[ "$INCLUDE_UIUX_EXPLICIT" != true && "$saved_uiux" == "true" ]]; then
        INCLUDE_UIUX=true
        echo -e "${YELLOW}Note: restored include_uiux=true from progress file.${NC}"
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

if [[ "${1:-}" == "--orchestrator" ]]; then
    if [[ ! -x "$ORCHESTRATOR_LAUNCHER" ]]; then
        echo "Orchestrator launcher not found: $ORCHESTRATOR_LAUNCHER"
        exit 1
    fi
    exec "$ORCHESTRATOR_LAUNCHER"
fi

if [[ "${1:-}" == "--orchestrator-cli" ]]; then
    if [[ ! -x "$ORCHESTRATOR_CLI" ]]; then
        echo "Orchestrator CLI not found: $ORCHESTRATOR_CLI"
        exit 1
    fi
    shift
    exec "$ORCHESTRATOR_CLI" "$@"
fi

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    echo "Usage: $0 [--orchestrator] [--orchestrator-cli] [--agent opencode|gemini] [--uiux] [--resume] [cli-options]"
    echo ""
    echo "Default: run QA docs via opencode in shell mode."
    echo "  --orchestrator       Launch tools/qa-orchestrator (Tauri UI workflow)"
    echo "  --orchestrator-cli   Run tools/qa-orchestrator in CLI automation mode"
    echo "  --agent <mode>       Agent mode: 'opencode' (default), 'gemini', 'kimi', 'minimax',"
    echo "                       'big-pickle', 'glm-air', 'glm-5', 'gpt-oss', 'kilo-minimax', 'qwen3-coder'"
    echo "  --uiux               Include UI/UX test documents (docs/uiux/)"
    echo "  --resume             Resume from last interrupted run"
    exit 0
fi

# Parse flags
AGENT_MODE="opencode"
INCLUDE_UIUX=false
INCLUDE_UIUX_EXPLICIT=false
RESUME=false
while [[ $# -gt 0 ]]; do
    case "$1" in
        --agent)
            AGENT_MODE="${2:-opencode}"
            shift 2
            ;;
        --uiux)
            INCLUDE_UIUX=true
            INCLUDE_UIUX_EXPLICIT=true
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
if [[ "$INCLUDE_UIUX" == true ]]; then
    echo -e "${YELLOW}UI/UX tests: included${NC}"
fi
if [[ "$RESUME" == true ]]; then
    echo -e "${YELLOW}Resume: enabled${NC}"
fi
echo ""

echo -e "${CYAN}========================================${NC}"
echo -e "${CYAN}  Auth9 QA Test Runner${NC}"
echo -e "${CYAN}========================================${NC}"
echo ""

# Collect all .md files under docs/qa/ and docs/security/, excluding README.md
# Optionally include docs/uiux/ when --uiux is specified
mapfile -t qa_files < <(
    {
        find "$QA_DIR" -name '*.md' ! -name 'README.md' 2>/dev/null || true
        find "$SECURITY_DIR" -name '*.md' ! -name 'README.md' 2>/dev/null || true
        if [[ "$INCLUDE_UIUX" == true ]]; then
            find "$UIUX_DIR" -name '*.md' ! -name 'README.md' 2>/dev/null || true
        fi
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
        kimi)
            run_cmd "${t[@]}" opencode run "读取文档：${rel_path}，执行QA测试" -m "opencode/kimi-k2.5-free"
            ;;
        minimax)
            run_cmd "${t[@]}" opencode run "读取文档：${rel_path}，执行QA测试" -m "minimax-coding-plan/MiniMax-M2.5"
            ;;
        free-minimax)
            run_cmd "${t[@]}" opencode run "读取文档：${rel_path}，执行QA测试" -m "opencode/minimax-m2.5-free"
            ;;
        big-pickle)
            run_cmd "${t[@]}" opencode run "读取文档：${rel_path}，执行QA测试" -m "opencode/big-pickle"
            ;;
        glm-air)
            run_cmd "${t[@]}" kilocode run "读取文档：${rel_path}，执行QA测试" -m "kilo/z-ai/glm-4.5-air:free"
            ;;
        glm-5)
            run_cmd "${t[@]}" kilocode run "读取文档：${rel_path}，执行QA测试" -m "kilo/z-ai/glm-5:free"
            ;;
        gpt-oss)
            run_cmd "${t[@]}" kilocode run "读取文档：${rel_path}，执行QA测试" -m "kilo/openai/gpt-oss-120b:free"
            ;;
        kilo-minimax)
            run_cmd "${t[@]}" kilocode run "读取文档：${rel_path}，执行QA测试" -m "kilo/minimax/minimax-m2.5:free"
            ;;
        qwen3-coder)
            run_cmd "${t[@]}" kilocode run "读取文档：${rel_path}，执行QA测试" -m "kilo/qwen/qwen3-coder:free"
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

    run_agent "$rel_path" && exit_code=0 || exit_code=$?
    if [[ $exit_code -eq 0 ]]; then
        passed=$((passed + 1))
        completed_results["$rel_path"]="passed"
        echo -e "${GREEN}✓ PASSED: ${rel_path}${NC}"
    elif [[ $exit_code -eq 124 ]]; then
        failed=$((failed + 1))
        failed_files+=("$rel_path (TIMEOUT)")
        completed_results["$rel_path"]="timeout"
        echo -e "${RED}✗ TIMEOUT: ${rel_path} (exceeded 30 minutes)${NC}"
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
echo -e "Total:  $total"
echo -e "${GREEN}Passed: $passed${NC}"
echo -e "${RED}Failed: $failed${NC}"

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
