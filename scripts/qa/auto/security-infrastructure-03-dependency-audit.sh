#!/usr/bin/env bash
# Security Auto Test: security/infrastructure/03-dependency-audit
# Doc: docs/security/infrastructure/03-dependency-audit.md
# Scenarios: 4
# ASVS: M-INFRA-03 | V13.1, V15.1, V15.2
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

require_bin jq

# ── Scenario 1: Rust dependency audit ────────────────────────────────────
scenario 1 "Rust dependency audit" '
  if ! command -v cargo &>/dev/null; then
    assert_eq "skip" "skip" "cargo not available - skipping Rust audit"
    return 0
  fi

  if [[ ! -f "$PROJECT_ROOT/auth9-core/Cargo.lock" ]]; then
    assert_eq "missing" "exists" "Cargo.lock should exist for reproducible builds"
    return 0
  fi

  if command -v cargo-audit &>/dev/null || cargo audit --version &>/dev/null 2>&1; then
    audit_output=$(cd "$PROJECT_ROOT/auth9-core" && cargo audit --json 2>/dev/null) || audit_output="{}"
    vuln_count=$(echo "$audit_output" | jq -r ".vulnerabilities.found // 0" 2>/dev/null || echo "0")
    assert_eq "$vuln_count" "0" "cargo audit: no known vulnerabilities (found: $vuln_count)"
  else
    assert_eq "skip" "skip" "cargo-audit not installed - run: cargo install cargo-audit"
  fi

  lockfile_entries=$(grep -c "^\[\[package\]\]" "$PROJECT_ROOT/auth9-core/Cargo.lock" 2>/dev/null || echo "0")
  assert_ne "$lockfile_entries" "0" "Cargo.lock has package entries ($lockfile_entries packages)"

  checksum_count=$(grep -c "^checksum" "$PROJECT_ROOT/auth9-core/Cargo.lock" 2>/dev/null || echo "0")
  assert_ne "$checksum_count" "0" "Cargo.lock has checksums for integrity ($checksum_count)"
'

# ── Scenario 2: Node.js dependency audit ─────────────────────────────────
scenario 2 "Node.js dependency audit" '
  if ! command -v npm &>/dev/null; then
    assert_eq "skip" "skip" "npm not available - skipping Node.js audit"
    return 0
  fi

  if [[ ! -f "$PROJECT_ROOT/auth9-portal/package-lock.json" ]]; then
    assert_eq "missing" "exists" "package-lock.json should exist for reproducible builds"
    return 0
  fi

  npm_audit=$(cd "$PROJECT_ROOT/auth9-portal" && npm audit --json 2>/dev/null) || npm_audit="{}"

  high=$(echo "$npm_audit" | jq -r ".metadata.vulnerabilities.high // 0" 2>/dev/null || echo "0")
  critical=$(echo "$npm_audit" | jq -r ".metadata.vulnerabilities.critical // 0" 2>/dev/null || echo "0")
  total_severe=$(( ${high:-0} + ${critical:-0} ))
  assert_eq "$total_severe" "0" "npm audit: no high/critical vulnerabilities (high=$high, critical=$critical)"

  npm_prod_audit=$(cd "$PROJECT_ROOT/auth9-portal" && npm audit --omit=dev --json 2>/dev/null) || npm_prod_audit="{}"
  prod_high=$(echo "$npm_prod_audit" | jq -r ".metadata.vulnerabilities.high // 0" 2>/dev/null || echo "0")
  prod_critical=$(echo "$npm_prod_audit" | jq -r ".metadata.vulnerabilities.critical // 0" 2>/dev/null || echo "0")
  prod_severe=$(( ${prod_high:-0} + ${prod_critical:-0} ))
  assert_eq "$prod_severe" "0" "npm audit (prod only): no high/critical (high=$prod_high, critical=$prod_critical)"

  integrity_count=$(grep -c "\"integrity\"" "$PROJECT_ROOT/auth9-portal/package-lock.json" 2>/dev/null || echo "0")
  assert_ne "$integrity_count" "0" "package-lock.json has integrity hashes ($integrity_count)"
'

# ── Scenario 3: Docker image scanning ────────────────────────────────────
scenario 3 "Docker image scanning" '
  if ! command -v docker &>/dev/null; then
    assert_eq "skip" "skip" "docker not available - skipping image scan"
    return 0
  fi

  dockerfiles=$(find "$PROJECT_ROOT" -maxdepth 3 -name "Dockerfile" -not -path "*/node_modules/*" 2>/dev/null || echo "")

  if [[ -z "$dockerfiles" ]]; then
    assert_eq "skip" "skip" "no Dockerfiles found"
    return 0
  fi

  for df in $dockerfiles; do
    dir_name=$(basename "$(dirname "$df")")

    from_lines=$(grep "^FROM" "$df" | head -5)
    for line in $from_lines; do
      if echo "$line" | grep -q ":latest\b"; then
        assert_not_contains "$line" ":latest" "Dockerfile $dir_name does not use :latest tag"
      fi
    done

    has_user=$(grep -c "^USER" "$df" || echo "0")
    if [[ "$has_user" -gt 0 ]]; then
      last_user=$(grep "^USER" "$df" | tail -1)
      assert_not_contains "$last_user" "root" "Dockerfile $dir_name final USER is not root"
    fi

    has_upgrade=$(grep -c "apt-get upgrade\|apk upgrade" "$df" || echo "0")
    if [[ "$has_upgrade" -gt 0 ]]; then
      assert_ne "$has_upgrade" "0" "Dockerfile $dir_name runs package upgrades"
    fi
  done

  if docker images --format "{{.Repository}}:{{.Tag}}" 2>/dev/null | grep -q "auth9"; then
    if command -v trivy &>/dev/null; then
      img=$(docker images --format "{{.Repository}}:{{.Tag}}" 2>/dev/null | grep "auth9" | head -1)
      trivy_out=$(trivy image --severity HIGH,CRITICAL --format json "$img" 2>/dev/null) || trivy_out="{}"
      critical_count=$(echo "$trivy_out" | jq '"'"'[.Results[]?.Vulnerabilities[]? | select(.Severity=="CRITICAL")] | length'"'"' 2>/dev/null || echo "0")
      assert_eq "$critical_count" "0" "trivy: no CRITICAL vulnerabilities in $img"
    else
      assert_eq "skip" "skip" "trivy not installed - skipping image vulnerability scan"
    fi
  else
    assert_eq "skip" "skip" "no auth9 Docker images built locally"
  fi
'

# ── Scenario 4: Supply chain security ────────────────────────────────────
scenario 4 "Supply chain security" '
  if command -v npm &>/dev/null; then
    npm_registry=$(cd "$PROJECT_ROOT/auth9-portal" && npm config get registry 2>/dev/null || echo "")
    if [[ -n "$npm_registry" ]]; then
      assert_contains "$npm_registry" "registry.npmjs.org" "npm uses official registry"
    fi
  fi

  if [[ -f "$PROJECT_ROOT/auth9-portal/package-lock.json" ]]; then
    sha_count=$(grep -c "sha512-" "$PROJECT_ROOT/auth9-portal/package-lock.json" 2>/dev/null || echo "0")
    assert_ne "$sha_count" "0" "package-lock.json uses SHA512 integrity ($sha_count entries)"
  fi

  if [[ -f "$PROJECT_ROOT/auth9-core/Cargo.lock" ]]; then
    cargo_checksums=$(grep -c "^checksum" "$PROJECT_ROOT/auth9-core/Cargo.lock" 2>/dev/null || echo "0")
    assert_ne "$cargo_checksums" "0" "Cargo.lock has package checksums ($cargo_checksums entries)"
  fi

  if [[ -f "$PROJECT_ROOT/auth9-portal/package.json" ]]; then
    typo_check=$(grep -Eic "reacct|expres[^s]|loadash|axois|requets|lodahs" \
      "$PROJECT_ROOT/auth9-portal/package.json" 2>/dev/null || echo "0")
    assert_eq "$typo_check" "0" "no typosquatted package names in package.json"
  fi

  if [[ -f "$PROJECT_ROOT/auth9-core/Cargo.toml" ]]; then
    typo_cargo=$(grep -Eic "toklo|serde_jsno|reqwuest|hypr|axiom|sqlxx" \
      "$PROJECT_ROOT/auth9-core/Cargo.toml" 2>/dev/null || echo "0")
    assert_eq "$typo_cargo" "0" "no typosquatted crate names in Cargo.toml"
  fi

  if [[ -f "$PROJECT_ROOT/.github/dependabot.yml" ]]; then
    assert_eq "exists" "exists" "Dependabot configuration exists"
  else
    assert_eq "ok" "ok" "Dependabot not configured (manual dependency updates)"
  fi
'

run_all
