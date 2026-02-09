---
name: deploy-gh-k8s
description: End-to-end deployment workflow for this repo that gates on the latest GitHub Actions run, deploys with deploy/upgrade.sh to Kubernetes, monitors kubectl resources, troubleshoots failures (logs, code, IaC), and performs health checks. Use when the user asks to deploy or says “进行部署”, “发布”, “上线”, or requests a Kubernetes deployment/check.
---

# Deploy GH K8s

## Overview

Execute a safe, gated deployment: verify the latest GitHub Actions workflow run is successful, then deploy to Kubernetes with `deploy/upgrade.sh`, monitor cluster resources, troubleshoot failures, and finish with health checks.

## Workflow

1. Identify the latest GitHub Actions workflow run.
- Use `gh run list -L 1` in the repo.
- If a specific branch/PR is mentioned, filter to that branch or PR.

2. If the latest run failed, inspect and fix.
- Use `gh run view <run_id> --json ...` and `gh run view <run_id> --log`.
- Summarize the failure, implement fixes (code or IaC), and report back.
- Ask the user to review and re-submit (do not deploy yet).

3. If the latest run succeeded, deploy to Kubernetes.
- Run `deploy/upgrade.sh` from repo root.
- Monitor resources during rollout:
  - `kubectl get pods -A`
  - `kubectl get deploy -A`
  - `kubectl get sts -A`
  - `kubectl get events -A --sort-by=.lastTimestamp`

4. If deployment errors appear, troubleshoot.
- Identify failing resources and pull logs:
  - `kubectl describe <resource> <name> -n <ns>`
  - `kubectl logs <pod> -n <ns> --tail=200`
- Fix issues in code or IaC as needed, then report back.
- Ask the user to review and re-submit (do not claim success).

5. If deployment succeeds, run health checks.
- Use repo-appropriate health checks (service endpoints, readiness probes, or scripted checks if present).
- Report completion and any follow-up observations.

## Notes

- Prefer actionable, minimal output in user updates: status, key failure snippet, and next required action.
- Do not attempt external CI providers; only report their URLs if encountered.
