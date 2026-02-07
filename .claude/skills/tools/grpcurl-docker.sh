#!/usr/bin/env bash
# Run grpcurl from inside the Docker Compose network (ephemeral container).
#
# Why:
# - Host -> container gRPC ports may be blocked.
# - auth9-grpc-tls provides a TLS endpoint; use -insecure for local testing.
#
# Usage:
#   .claude/skills/tools/grpcurl-docker.sh [grpcurl args...]
#
# Examples:
#   .claude/skills/tools/grpcurl-docker.sh -insecure auth9-grpc-tls:50051 list
#   .claude/skills/tools/grpcurl-docker.sh -insecure -H "x-api-key: dev-grpc-api-key" \
#     -import-path /proto -proto auth9.proto \
#     -d '{"identity_token":"dummy","tenant_id":"dummy","service_id":"dummy"}' \
#     auth9-grpc-tls:50051 auth9.TokenExchange/ExchangeToken

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

GRPC_TARGET_DEFAULT="auth9-grpc-tls:50051"
GRPC_IMPORT_PATH_HOST_DEFAULT="auth9-core/proto"
GRPC_PROTO_DEFAULT="auth9.proto"

GRPC_TARGET="${GRPC_TARGET:-$GRPC_TARGET_DEFAULT}"
GRPC_IMPORT_PATH_HOST="${GRPC_IMPORT_PATH_HOST:-$GRPC_IMPORT_PATH_HOST_DEFAULT}"
GRPC_PROTO="${GRPC_PROTO:-$GRPC_PROTO_DEFAULT}"

if [ "${1:-}" = "-h" ] || [ "${1:-}" = "--help" ] || [ $# -eq 0 ]; then
  cat <<EOF
Usage:
  grpcurl-docker.sh [grpcurl args...]

Environment:
  GRPC_NETWORK            Docker network name (auto-detect if unset)
  GRPC_TARGET             Default target (default: $GRPC_TARGET_DEFAULT)
  GRPC_IMPORT_PATH_HOST   Host proto dir (default: $GRPC_IMPORT_PATH_HOST_DEFAULT)
  GRPC_PROTO              Proto filename (default: $GRPC_PROTO_DEFAULT)

Notes:
  - Mounts "\$PROJECT_ROOT/\$GRPC_IMPORT_PATH_HOST" to /proto:ro
  - You usually want: -insecure -import-path /proto -proto \$GRPC_PROTO
EOF
  exit 0
fi

detect_network() {
  if [ -n "${GRPC_NETWORK:-}" ]; then
    echo "$GRPC_NETWORK"
    return 0
  fi

  # Prefer "*_auth9-network" (docker compose project prefix) then "auth9-network".
  local n
  n="$(docker network ls --format '{{.Name}}' | grep -E '(^|_)auth9-network$' | head -n 1 || true)"
  if [ -n "$n" ]; then
    echo "$n"
    return 0
  fi

  echo "auth9_auth9-network"
}

NETWORK="$(detect_network)"

# shellcheck disable=SC2086
exec docker run --rm \
  --network "$NETWORK" \
  -v "$PROJECT_ROOT/$GRPC_IMPORT_PATH_HOST:/proto:ro" \
  fullstorydev/grpcurl \
  "$@"

