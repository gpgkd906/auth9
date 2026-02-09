#!/usr/bin/env bash
# Generic upgrade script for Kubernetes
#
# Usage:
#   ./deploy/upgrade.sh [--namespace NS] [--component core|portal|all] [--dry-run]

set -e

NAMESPACE="${NAMESPACE:-{{namespace}}}"
COMPONENT="all"
DRY_RUN=""

while [[ $# -gt 0 ]]; do
  case $1 in
    --namespace)
      NAMESPACE="$2"; shift 2;;
    --component)
      COMPONENT="$2"; shift 2;;
    --dry-run)
      DRY_RUN="true"; shift;;
    -h|--help)
      echo "Usage: $0 [--namespace NS] [--component core|portal|all] [--dry-run]"; exit 0;;
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

upgrade_component() {
  local deployment="$1"
  run_cmd kubectl rollout restart deployment/"$deployment" -n "$NAMESPACE"
}

case "$COMPONENT" in
  core)
    upgrade_component "{{project_name}}-core";;
  portal)
    upgrade_component "{{project_name}}-portal";;
  all)
    upgrade_component "{{project_name}}-core"
    upgrade_component "{{project_name}}-portal";;
  *)
    echo "Unknown component: $COMPONENT"; exit 1;;
 esac

if [ -z "$DRY_RUN" ]; then
  kubectl rollout status deployment/{{project_name}}-core -n "$NAMESPACE" --timeout=300s || true
  kubectl rollout status deployment/{{project_name}}-portal -n "$NAMESPACE" --timeout=300s || true
fi

if [ "${ENABLE_AUTH9_EXTRAS:-}" = "true" ]; then
  echo "[Auth9 extras] Add keycloak rollout here if needed."
fi
