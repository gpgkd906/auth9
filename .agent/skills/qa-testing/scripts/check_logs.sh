#!/bin/bash
# Check Docker service logs

set -e

SERVICE_NAME="${1:-auth9-core}"
LINES="${2:-50}"

echo "==========================================
Logs for $SERVICE_NAME (last $LINES lines)
=========================================="

docker logs "$SERVICE_NAME" --tail "$LINES"

echo ""
echo "=========================================="
echo "To see more logs, run:"
echo "docker logs $SERVICE_NAME --tail <lines>"
echo "=========================================="
