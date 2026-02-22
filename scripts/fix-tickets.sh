#!/usr/bin/env bash
# Automatically fix QA tickets in docs/ticket/ using Claude Code agent.
# Processes tickets in batches of 6 (configurable via BATCH_SIZE).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TICKET_DIR="$PROJECT_ROOT/docs/ticket"
BATCH_SIZE="${BATCH_SIZE:-6}"
MAX_BATCHES="${MAX_BATCHES:-0}" # 0 = unlimited
AGENT="${AGENT:-claude}" # claude or codex

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Log file
LOG_FILE="$PROJECT_ROOT/fix-tickets-$(date +%y%m%d_%H%M%S).log"

log() {
    local msg="[$(date '+%Y-%m-%d %H:%M:%S')] $*"
    echo "$msg" >> "$LOG_FILE"
    echo -e "$msg"
}

usage() {
    echo "Usage: $0 [options]"
    echo ""
    echo "Scan docs/ticket/ and fix tickets in batches using Claude Code agent."
    echo ""
    echo "Options:"
    echo "  -a, --agent NAME      Agent to use: claude (default), codex"
    echo "  -b, --batch-size N    Number of tickets per batch (default: 6)"
    echo "  -m, --max-batches N   Maximum number of batches to process (default: 0 = unlimited)"
    echo "  -d, --dry-run         Show what would be processed without running"
    echo "  -h, --help            Show this help message"
    echo ""
    echo "Environment Variables:"
    echo "  AGENT                 Same as --agent"
    echo "  BATCH_SIZE            Same as --batch-size"
    echo "  MAX_BATCHES           Same as --max-batches"
    exit 0
}

# Parse arguments
DRY_RUN=false
while [[ $# -gt 0 ]]; do
    case $1 in
        -a|--agent)
            AGENT="$2"
            if [[ "$AGENT" != "claude" && "$AGENT" != "codex" ]]; then
                echo -e "${RED}Unknown agent: $AGENT (supported: claude, codex)${NC}"
                exit 1
            fi
            shift 2
            ;;
        -b|--batch-size)
            BATCH_SIZE="$2"
            shift 2
            ;;
        -m|--max-batches)
            MAX_BATCHES="$2"
            shift 2
            ;;
        -d|--dry-run)
            DRY_RUN=true
            shift
            ;;
        -h|--help)
            usage
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            usage
            ;;
    esac
done

echo -e "${CYAN}${BOLD}========================================${NC}"
echo -e "${CYAN}${BOLD}  Auth9 Ticket Fix Runner${NC}"
echo -e "${CYAN}${BOLD}========================================${NC}"
echo -e "  Agent:        ${BOLD}${AGENT}${NC}"
echo -e "  Batch size:   ${BOLD}${BATCH_SIZE}${NC}"
echo -e "  Max batches:  ${BOLD}${MAX_BATCHES:-unlimited}${NC}"
echo -e "  Log file:     ${LOG_FILE}"
echo ""

# Collect ticket files (exclude README.md)
mapfile -t ticket_files < <(find "$TICKET_DIR" -maxdepth 1 -name '*.md' ! -name 'README.md' -print | sort)

total_tickets=${#ticket_files[@]}
if [[ $total_tickets -eq 0 ]]; then
    echo -e "${GREEN}No tickets found in $TICKET_DIR. All clear!${NC}"
    exit 0
fi

echo -e "Found ${BOLD}${total_tickets}${NC} ticket(s) to process."
echo ""

# Split into batches
batches=()
batch_count=$(( (total_tickets + BATCH_SIZE - 1) / BATCH_SIZE ))

if [[ $MAX_BATCHES -gt 0 && $batch_count -gt $MAX_BATCHES ]]; then
    batch_count=$MAX_BATCHES
    echo -e "${YELLOW}Limiting to ${MAX_BATCHES} batch(es) (${MAX_BATCHES} × ${BATCH_SIZE} = $((MAX_BATCHES * BATCH_SIZE)) tickets max).${NC}"
    echo ""
fi

# Counters
batch_passed=0
batch_failed=0
batch_failed_ids=()
total_processed=0

for (( b=0; b < batch_count; b++ )); do
    start=$(( b * BATCH_SIZE ))
    end=$(( start + BATCH_SIZE ))
    if [[ $end -gt $total_tickets ]]; then
        end=$total_tickets
    fi

    # Collect relative paths for this batch
    batch_paths=()
    for (( i=start; i < end; i++ )); do
        rel_path="${ticket_files[$i]#"$PROJECT_ROOT"/}"
        batch_paths+=("$rel_path")
    done

    batch_num=$(( b + 1 ))
    batch_ticket_count=${#batch_paths[@]}
    total_processed=$(( total_processed + batch_ticket_count ))

    echo -e "${CYAN}${BOLD}----------------------------------------${NC}"
    echo -e "${CYAN}${BOLD}  Batch ${batch_num}/${batch_count} (${batch_ticket_count} tickets)${NC}"
    echo -e "${CYAN}${BOLD}----------------------------------------${NC}"
    for p in "${batch_paths[@]}"; do
        echo -e "  ${CYAN}→${NC} $p"
    done
    echo ""

    # Build the prompt: /ticket-fix with all ticket paths
    prompt="/ticket-fix"
    for p in "${batch_paths[@]}"; do
        prompt+=" $p"
    done

    if $DRY_RUN; then
        echo -e "${YELLOW}[DRY RUN] Would run:${NC}"
        if [[ "$AGENT" == "codex" ]]; then
            echo -e "  codex exec -m gpt-5.3-codex --full-auto \"${prompt}\""
        else
            echo -e "  claude -p --model opus \"${prompt}\""
        fi
        echo ""
        continue
    fi

    log "Batch ${batch_num}: Processing ${batch_paths[*]} (agent: ${AGENT})"
    echo -e "${YELLOW}Starting ${AGENT} agent...${NC}"
    echo ""

    batch_output_file="$PROJECT_ROOT/.fix-tickets-batch-${batch_num}.tmp"
    set +e
    if [[ "$AGENT" == "codex" ]]; then
        # Run Codex agent
        codex exec \
            -m gpt-5.3-codex \
            --full-auto \
            "$prompt" \
            2>&1 | while IFS= read -r line; do
                echo -e "  ${CYAN}│${NC} $line"
                echo "$line" >> "$LOG_FILE"
            done
    else
        # Run Claude Code agent with streaming JSON for real-time progress
        claude -p \
            --dangerously-skip-permissions \
            --verbose \
            --model opus \
            --output-format stream-json \
            "$prompt" \
            2>&1 | while IFS= read -r line; do
                # Extract meaningful content from stream-json lines
                # Each line is a JSON object; extract "content" text for display
                if echo "$line" | grep -q '"type":"assistant"'; then
                    # Extract text content from assistant messages
                    content=$(echo "$line" | sed -n 's/.*"content":"\([^"]*\)".*/\1/p' 2>/dev/null)
                    if [[ -n "$content" ]]; then
                        echo -e "  ${CYAN}│${NC} $content"
                    fi
                elif echo "$line" | grep -q '"type":"tool_use"'; then
                    tool=$(echo "$line" | sed -n 's/.*"name":"\([^"]*\)".*/\1/p' 2>/dev/null)
                    if [[ -n "$tool" ]]; then
                        echo -e "  ${YELLOW}│ 🔧 $tool${NC}"
                    fi
                elif echo "$line" | grep -q '"type":"result"'; then
                    echo -e "  ${GREEN}│ ✓ Result received${NC}"
                fi
                echo "$line" >> "$LOG_FILE"
            done
    fi
    exit_code=${PIPESTATUS[0]}
    set -e

    if [[ $exit_code -eq 0 ]]; then
        batch_passed=$(( batch_passed + 1 ))
        echo ""
        echo -e "${GREEN}${BOLD}✓ Batch ${batch_num} completed successfully${NC}"
        log "Batch ${batch_num}: PASSED"
    else
        batch_failed=$(( batch_failed + 1 ))
        batch_failed_ids+=("Batch ${batch_num}: ${batch_paths[*]}")
        echo ""
        echo -e "${RED}${BOLD}✗ Batch ${batch_num} failed (exit code: ${exit_code})${NC}"
        log "Batch ${batch_num}: FAILED (exit code: ${exit_code})"
    fi

    echo ""

    # Brief pause between batches to avoid rate limiting
    if [[ $b -lt $(( batch_count - 1 )) ]]; then
        echo -e "${YELLOW}Waiting 5s before next batch...${NC}"
        sleep 5
    fi
done

# Check remaining tickets after processing
remaining_tickets=0
if [[ -d "$TICKET_DIR" ]]; then
    remaining_tickets=$(find "$TICKET_DIR" -maxdepth 1 -name '*.md' ! -name 'README.md' | wc -l | tr -d ' ')
fi

# Summary
echo ""
echo -e "${CYAN}${BOLD}========================================${NC}"
echo -e "${CYAN}${BOLD}  Summary${NC}"
echo -e "${CYAN}${BOLD}========================================${NC}"
echo -e "  Tickets found:      ${total_tickets}"
echo -e "  Tickets processed:  ${total_processed}"
echo -e "  Batches total:      ${batch_count}"
echo -e "  ${GREEN}Batches passed:     ${batch_passed}${NC}"
echo -e "  ${RED}Batches failed:     ${batch_failed}${NC}"
echo -e "  Tickets remaining:  ${remaining_tickets}"
echo -e "  Log file:           ${LOG_FILE}"

if [[ ${#batch_failed_ids[@]} -gt 0 ]]; then
    echo ""
    echo -e "${RED}Failed batches:${NC}"
    for f in "${batch_failed_ids[@]}"; do
        echo -e "  ${RED}- $f${NC}"
    done
fi

if [[ $remaining_tickets -gt 0 ]]; then
    echo ""
    echo -e "${YELLOW}There are still ${remaining_tickets} ticket(s) remaining. Run again to continue.${NC}"
fi

if [[ $batch_failed -gt 0 ]]; then
    exit 1
fi

echo ""
echo -e "${GREEN}${BOLD}All batches completed successfully!${NC}"
