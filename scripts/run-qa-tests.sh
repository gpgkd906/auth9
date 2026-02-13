#!/usr/bin/env bash
# Run QA tests for each document in docs/qa/ and docs/security/ using opencode
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
QA_DIR="$PROJECT_ROOT/docs/qa"
SECURITY_DIR="$PROJECT_ROOT/docs/security"

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

echo -e "${CYAN}========================================${NC}"
echo -e "${CYAN}  Auth9 QA Test Runner${NC}"
echo -e "${CYAN}========================================${NC}"
echo ""

# Collect all .md files under docs/qa/ and docs/security/, excluding README.md
mapfile -t qa_files < <(
    {
        find "$QA_DIR" -name '*.md' ! -name 'README.md' 2>/dev/null || true
        find "$SECURITY_DIR" -name '*.md' ! -name 'README.md' 2>/dev/null || true
    } | sort
)

if [[ ${#qa_files[@]} -eq 0 ]]; then
    echo -e "${YELLOW}No QA/Security documents found in $QA_DIR or $SECURITY_DIR${NC}"
    exit 0
fi

echo -e "Found ${#qa_files[@]} QA/Security document(s) to process."
echo ""

for file in "${qa_files[@]}"; do
    total=$((total + 1))
    # Convert to relative path from project root
    rel_path="${file#"$PROJECT_ROOT"/}"

    echo -e "${CYAN}----------------------------------------${NC}"
    echo -e "${CYAN}[$total/${#qa_files[@]}] Processing: ${rel_path}${NC}"
    echo -e "${CYAN}----------------------------------------${NC}"

    if opencode run "读取文档：${rel_path}，执行QA测试" -m "deepseek/deepseek-chat"; then
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
