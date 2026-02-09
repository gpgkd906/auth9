#!/usr/bin/env bash
# Generic cleanup script for Kubernetes
#
# Usage:
#   ./deploy/cleanup.sh [--namespace NS] [--dry-run]

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

if ! kubectl get namespace "$NAMESPACE" &>/dev/null; then
  echo "Namespace '$NAMESPACE' not found."; exit 0
fi

run_cmd kubectl delete deployments --all -n "$NAMESPACE" --ignore-not-found=true
run_cmd kubectl delete statefulsets --all -n "$NAMESPACE" --ignore-not-found=true
run_cmd kubectl delete services --all -n "$NAMESPACE" --ignore-not-found=true
run_cmd kubectl delete configmaps --all -n "$NAMESPACE" --ignore-not-found=true
run_cmd kubectl delete secrets --all -n "$NAMESPACE" --ignore-not-found=true
run_cmd kubectl delete hpa --all -n "$NAMESPACE" --ignore-not-found=true
run_cmd kubectl delete jobs --all -n "$NAMESPACE" --ignore-not-found=true
run_cmd kubectl delete pvc --all -n "$NAMESPACE" --ignore-not-found=true

if [ "${ENABLE_AUTH9_EXTRAS:-}" = "true" ]; then
  echo "[Auth9 extras] Add database reset job here if needed."
fi

