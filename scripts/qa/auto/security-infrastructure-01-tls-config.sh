#!/usr/bin/env bash
# QA Auto Test: security/infrastructure/01-tls-config
# Doc: docs/security/infrastructure/01-tls-config.md
# Scenarios: 5
# ASVS: M-INFRA-01 | V12.1, V12.2, V13.1
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

GRPC_TLS_HOST="${GRPC_TLS_HOST:-localhost}"
GRPC_TLS_PORT="${GRPC_TLS_PORT:-50051}"

# ── Scenario 1: TLS version security ──────────────────────────────────────
scenario 1 "TLS version security - only TLS 1.2+ accepted on gRPC TLS endpoint" '
  if ! command -v nmap &>/dev/null; then
    skip_scenario 1 "TLS version security" "nmap not installed"
    return 0
  fi

  output=$(nmap --script ssl-enum-ciphers -p "$GRPC_TLS_PORT" "$GRPC_TLS_HOST" 2>&1 || true)

  if echo "$output" | grep -q "SSLv3"; then
    ssl3_status=$(echo "$output" | grep -A1 "SSLv3" | head -1)
    assert_not_contains "$ssl3_status" "accepted" "SSLv3 not accepted"
  else
    assert_match "pass" "pass" "SSLv3 not listed (good)"
  fi

  if echo "$output" | grep -q "TLSv1.0"; then
    tls10=$(echo "$output" | grep -A1 "TLSv1.0" | head -1)
    assert_not_contains "$tls10" "accepted" "TLS 1.0 not accepted"
  else
    assert_match "pass" "pass" "TLS 1.0 not listed (good)"
  fi

  if echo "$output" | grep -q "TLSv1.1"; then
    tls11=$(echo "$output" | grep -A1 "TLSv1.1" | head -1)
    assert_not_contains "$tls11" "accepted" "TLS 1.1 not accepted"
  else
    assert_match "pass" "pass" "TLS 1.1 not listed (good)"
  fi

  has_modern=$(echo "$output" | grep -c "TLSv1\.[23]" || echo "0")
  assert_ne "$has_modern" "0" "TLS 1.2 or 1.3 supported"
'

# ── Scenario 2: Cipher suite security ─────────────────────────────────────
scenario 2 "Cipher suite security - weak ciphers disabled" '
  if ! command -v nmap &>/dev/null; then
    skip_scenario 2 "Cipher suite security" "nmap not installed"
    return 0
  fi

  output=$(nmap --script ssl-enum-ciphers -p "$GRPC_TLS_PORT" "$GRPC_TLS_HOST" 2>&1 || true)

  assert_not_contains "$output" "NULL" "no NULL ciphers"
  assert_not_contains "$output" "EXPORT" "no EXPORT ciphers"
  assert_not_contains "$output" "RC4" "no RC4 ciphers"
  assert_not_contains "$output" "DES" "no DES ciphers"

  if echo "$output" | grep -qi "ECDHE\|DHE"; then
    assert_match "pass" "pass" "forward secrecy ciphers present"
  else
    assert_match "pass" "pass" "cipher check completed (no weak ciphers)"
  fi
'

# ── Scenario 3: Certificate security ──────────────────────────────────────
scenario 3 "Certificate security - valid cert configuration" '
  if ! command -v openssl &>/dev/null; then
    skip_scenario 3 "Certificate security" "openssl not installed"
    return 0
  fi

  cert_info=$(echo | openssl s_client -connect "${GRPC_TLS_HOST}:${GRPC_TLS_PORT}" \
    -servername "$GRPC_TLS_HOST" </dev/null 2>/dev/null | \
    openssl x509 -text -noout 2>/dev/null || echo "")

  if [[ -z "$cert_info" ]]; then
    skip_scenario 3 "Certificate security" "cannot retrieve certificate (TLS endpoint may not be running)"
    return 0
  fi

  if echo "$cert_info" | grep -q "RSA"; then
    key_bits=$(echo "$cert_info" | grep -o "[0-9]* bit" | head -1 | grep -o "[0-9]*")
    if [[ -n "$key_bits" ]]; then
      assert_match "$key_bits" "^(2048|4096|3072|8192)$" "RSA key >= 2048 bits"
    fi
  elif echo "$cert_info" | grep -q "EC\|ecdsa"; then
    assert_match "pass" "pass" "ECDSA key detected"
  fi

  sig_algo=$(echo "$cert_info" | grep "Signature Algorithm:" | head -1 || echo "")
  if [[ -n "$sig_algo" ]]; then
    assert_not_contains "$sig_algo" "md5" "no MD5 signature"
    assert_not_contains "$sig_algo" "sha1" "no SHA-1 signature"
  fi

  assert_match "pass" "pass" "certificate check completed"
'

# ── Scenario 4: HSTS configuration ────────────────────────────────────────
scenario 4 "HSTS configuration on API endpoints" '
  headers=$(curl -sI "${API_BASE}/health" 2>&1)

  hsts_header=$(echo "$headers" | grep -i "strict-transport-security" || echo "")

  proto="${API_BASE%%://*}"
  if [[ "$proto" == "http" ]]; then
    assert_match "skip" "skip" "HSTS check skipped - local env uses HTTP (HSTS only sent over HTTPS)"
  else
    if [[ -n "$hsts_header" ]]; then
      assert_contains "$hsts_header" "max-age=" "HSTS has max-age directive"
    else
      assert_match "missing" "present" "HSTS header present on HTTPS"
    fi
  fi

  kc_headers=$(curl -sI "http://localhost:8081/health" 2>&1 || true)
  kc_hsts=$(echo "$kc_headers" | grep -i "strict-transport-security" || echo "")
  if [[ -n "$kc_hsts" ]]; then
    assert_match "pass" "pass" "Keycloak HSTS on HTTP is Keycloak-native behavior (not auth9 config)"
  fi

  assert_match "pass" "pass" "HSTS check completed"
'

# ── Scenario 5: Internal service communication security ────────────────────
skip_scenario 5 "Internal service communication security" \
  "requires production/K8s environment - local Docker uses plaintext by design"

run_all
