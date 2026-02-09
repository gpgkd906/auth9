#!/usr/bin/env bash
# Generic interactive deploy script for Kubernetes
#
# Usage:
#   ./deploy/deploy.sh [--namespace NS] [--dry-run]

set -e

NAMESPACE="${NAMESPACE:-{{namespace}}}"
DRY_RUN=""

while [[ $# -gt 0 ]]; do
  case $1 in
    --namespace)
      NAMESPACE="$2"; shift 2;;
    --dry-run)
      DRY_RUN="true"; shift;;
    -h|--help)
      echo "Usage: $0 [--namespace NS] [--dry-run]"; exit 0;;
    *)
      echo "Unknown option: $1"; exit 1;;
  esac
done

run_cmd() {
  if [ -n "$DRY_RUN" ]; then
    echo "[dry-run] $*"
  else
    "$@"
  fi
}

run_cmd kubectl create namespace "$NAMESPACE" 2>/dev/null || true

run_cmd kubectl apply -n "$NAMESPACE" -f k8s/base

# Basic health checks (optional)
if [ -z "$DRY_RUN" ]; then
  kubectl get pods -n "$NAMESPACE"
fi

if [ "${ENABLE_AUTH9_EXTRAS:-}" = "true" ]; then
  echo "[Auth9 extras] Apply additional manifests (Keycloak/DB) if present."
fi
