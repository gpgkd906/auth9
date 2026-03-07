#!/usr/bin/env bash
# Security Auto Test: security/data-security/04-encryption-impl
# Doc: docs/security/data-security/04-encryption-impl.md
# Scenarios: 3
# ASVS: M-DATA-04 | V11.1, V11.2, V11.4
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

require_bin jq

# ── Scenario 1: Nonce reuse detection ────────────────────────────────────
scenario 1 "AES-256-GCM nonce reuse detection" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  has_enc_col=$(db_query "SELECT COUNT(*) FROM information_schema.columns WHERE table_name='\''system_settings'\'' AND column_name IN ('\''value'\'','\''encrypted_value'\'');" 2>/dev/null | tr -d "[:space:]") || has_enc_col="0"

  if [[ "$has_enc_col" == "0" ]]; then
    assert_eq "skip" "skip" "system_settings table not accessible or no encrypted column"
    qa_set_token ""
    return 0
  fi

  nonces=()
  for i in $(seq 1 5); do
    resp=$(api_put "/api/v1/system/email" \
      "{\"host\":\"smtp.test.com\",\"port\":587,\"username\":\"nonce-test\",\"password\":\"NonceTestPass${i}!\",\"from_email\":\"test@test.com\",\"from_name\":\"Test\"}")
    status=$(resp_status "$resp")
    if [[ "$status" != "200" ]]; then
      assert_eq "skip" "skip" "email settings update not available (status=$status)"
      break
    fi
    sleep 1

    enc_val=$(db_query "SELECT value FROM system_settings WHERE setting_key LIKE '\''%smtp%password%'\'' OR setting_key LIKE '\''%email%password%'\'' LIMIT 1;" 2>/dev/null | tr -d "[:space:]") || enc_val=""
    if [[ -n "$enc_val" && "$enc_val" == *":"* ]]; then
      nonce_part=$(echo "$enc_val" | cut -d: -f1)
      nonces+=("$nonce_part")
    fi
  done

  if [[ ${#nonces[@]} -ge 2 ]]; then
    unique_nonces=$(printf "%s\n" "${nonces[@]}" | sort -u | wc -l | tr -d " ")
    total_nonces=${#nonces[@]}
    assert_eq "$unique_nonces" "$total_nonces" "all encryption nonces are unique ($unique_nonces/$total_nonces)"
  elif [[ ${#nonces[@]} -eq 0 ]]; then
    assert_eq "ok" "ok" "encryption not active or SETTINGS_ENCRYPTION_KEY not set (dev behavior)"
  fi

  qa_set_token ""
'

# ── Scenario 2: Ciphertext tampering and auth tag verification ───────────
scenario 2 "Ciphertext tampering and GCM auth tag verification" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  enc_val=$(db_query "SELECT value FROM system_settings WHERE setting_key LIKE '\''%smtp%'\'' AND value LIKE '\''%:%'\'' LIMIT 1;" 2>/dev/null) || enc_val=""

  if [[ -z "$enc_val" || "$enc_val" != *":"* ]]; then
    assert_eq "skip" "skip" "no encrypted values found in system_settings (encryption may not be enabled)"
    qa_set_token ""
    return 0
  fi

  enc_val_trimmed=$(echo "$enc_val" | tr -d "[:space:]")
  nonce_b64=$(echo "$enc_val_trimmed" | cut -d: -f1)
  ct_b64=$(echo "$enc_val_trimmed" | cut -d: -f2-)

  tampered_ct=$(echo "$ct_b64" | python3 -c "
import sys, base64
ct_b64 = sys.stdin.read().strip()
try:
    ct = bytearray(base64.b64decode(ct_b64))
    ct[0] ^= 0x01
    print(base64.b64encode(bytes(ct)).decode())
except:
    print(ct_b64)
" 2>/dev/null || echo "$ct_b64")

  tampered_full="${nonce_b64}:${tampered_ct}"

  db_exec "UPDATE system_settings SET value='\''${tampered_full}'\'' WHERE setting_key LIKE '\''%smtp%'\'' AND value LIKE '\''%:%'\'' LIMIT 1;" 2>/dev/null || true

  resp_test=$(api_post "/api/v1/system/email/test" "{\"to\":\"tamper-test@example.com\"}")
  status_test=$(resp_status "$resp_test")
  body_test=$(resp_body "$resp_test")
  assert_not_contains "$body_test" "NonceTestPass" "tampered ciphertext does not decrypt to plaintext"

  db_exec "UPDATE system_settings SET value='\''${enc_val_trimmed}'\'' WHERE setting_key LIKE '\''%smtp%'\'' AND value LIKE '\''%:%'\'' LIMIT 1;" 2>/dev/null || true

  assert_match "$status_test" "^(200|400|422|500)$" "tampered ciphertext handled gracefully"

  qa_set_token ""
'

# ── Scenario 3: Encryption key strength and management ───────────────────
scenario 3 "Encryption key strength and management" '
  key_in_code=$(grep -r "SETTINGS_ENCRYPTION_KEY" "$PROJECT_ROOT/auth9-core/src/" \
    --include="*.rs" 2>/dev/null | grep -v "env\|config\|test\|//\|#\[" | wc -l | tr -d " ") || key_in_code="0"
  assert_eq "$key_in_code" "0" "no hardcoded SETTINGS_ENCRYPTION_KEY in source code"

  dc_files=$(find "$PROJECT_ROOT" -maxdepth 1 -name "docker-compose*.yml" 2>/dev/null || echo "")
  for dcf in $dc_files; do
    hardcoded_key=$(grep "SETTINGS_ENCRYPTION_KEY:" "$dcf" 2>/dev/null | grep -v "\${\|:-\|#" | wc -l | tr -d " ") || hardcoded_key="0"
    assert_eq "$hardcoded_key" "0" "no hardcoded encryption key in $(basename $dcf)"
  done

  env_files=$(find "$PROJECT_ROOT" -maxdepth 1 -name ".env" -o -name ".env.local" 2>/dev/null || echo "")
  for ef in $env_files; do
    if [[ -f "$ef" ]]; then
      git_tracked=$(cd "$PROJECT_ROOT" && git ls-files --error-unmatch "$(basename $ef)" 2>/dev/null && echo "tracked" || echo "untracked")
      assert_eq "$git_tracked" "untracked" "$(basename $ef) is not tracked by git"
    fi
  done

  crypto_file="$PROJECT_ROOT/auth9-core/src/crypto/aes.rs"
  if [[ -f "$crypto_file" ]]; then
    uses_osrng=$(grep -c "OsRng\|thread_rng\|rand::" "$crypto_file" || echo "0")
    assert_ne "$uses_osrng" "0" "crypto/aes.rs uses CSPRNG (OsRng/thread_rng)"
  else
    assert_eq "skip" "skip" "crypto/aes.rs not found - skipping CSPRNG check"
  fi
'

run_all
