#!/usr/bin/env bash
# Run QA tests for each document in docs/qa/ and docs/security/ using opencode
set -euo pipefail

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

# Counters
total=0
passed=0
failed=0
failed_files=()

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
    echo "Usage: $0 [--orchestrator] [--orchestrator-cli] [--agent opencode|gemini] [--uiux] [cli-options]"
    echo ""
    echo "Default: run QA docs via opencode in shell mode."
    echo "  --orchestrator       Launch tools/qa-orchestrator (Tauri UI workflow)"
    echo "  --orchestrator-cli   Run tools/qa-orchestrator in CLI automation mode"
    echo "  --agent <mode>       Agent mode: 'opencode' (default) or 'gemini'"
    echo "  --uiux               Include UI/UX test documents (docs/uiux/)"
    exit 0
fi

# Parse flags
AGENT_MODE="opencode"
INCLUDE_UIUX=false
while [[ $# -gt 0 ]]; do
    case "$1" in
        --agent)
            AGENT_MODE="${2:-opencode}"
            shift 2
            ;;
        --uiux)
            INCLUDE_UIUX=true
            shift
            ;;
        *)
            break
            ;;
    esac
done

echo -e "${YELLOW}Agent mode: ${AGENT_MODE}${NC}"
if [[ "$INCLUDE_UIUX" == true ]]; then
    echo -e "${YELLOW}UI/UX tests: included${NC}"
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
echo ""

# Run agent command based on AGENT_MODE
run_agent() {
    local rel_path="$1"
    case "$AGENT_MODE" in
        opencode)
            opencode run "读取文档：${rel_path}，执行QA测试" -m "deepseek/deepseek-chat"
            ;;
        gemini)
            gemini -m gemini-3-flash-preview -p "读取文档：${rel_path}，执行QA测试" --yolo
            ;;
        *)
            echo -e "${RED}Unknown agent mode: ${AGENT_MODE}${NC}"
            return 1
            ;;
    esac
}

for file in "${qa_files[@]}"; do
    total=$((total + 1))
    # Convert to relative path from project root
    rel_path="${file#"$PROJECT_ROOT"/}"

    echo -e "${CYAN}----------------------------------------${NC}"
    echo -e "${CYAN}[$total/${#qa_files[@]}] Processing: ${rel_path}${NC}"
    echo -e "${CYAN}----------------------------------------${NC}"

    if run_agent "$rel_path"; then
        passed=$((passed + 1))
        echo -e "${GREEN}✓ PASSED: ${rel_path}${NC}"
    else
        failed=$((failed + 1))
        failed_files+=("$rel_path")
        echo -e "${RED}✗ FAILED: ${rel_path}${NC}"
    fi

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

echo ""
echo -e "${GREEN}All QA tests passed!${NC}"
