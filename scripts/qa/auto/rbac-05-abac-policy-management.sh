#!/usr/bin/env bash
# QA Auto Test: rbac/05-abac-policy-management
# Doc: docs/qa/rbac/05-abac-policy-management.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

scenario 1 "创建 ABAC 草稿版本" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  TENANT_ID=$(db_query "SELECT id FROM tenants LIMIT 1;" | tr -d "[:space:]")
  if [[ -z "$TENANT_ID" ]]; then
    echo "No tenant found in DB" >&2
    return 1
  fi

  resp=$(api_post "/api/v1/tenants/${TENANT_ID}/abac/policies" "{\"change_note\":\"QA draft test\",\"policy\":{\"rules\":[{\"id\":\"qa-rule-1\",\"effect\":\"allow\",\"description\":\"allow all for QA\",\"conditions\":{}}]}}")
  assert_http_status "$(resp_status "$resp")" 200 "create ABAC draft returns 200"

  body=$(resp_body "$resp")
  assert_json_exists "$body" ".data.id" "response contains version id"
  VERSION_ID=$(echo "$body" | jq -r ".data.id")

  assert_db_not_empty "SELECT id FROM abac_policy_sets WHERE tenant_id = '\''${TENANT_ID}'\'';" "abac_policy_sets record exists"

  assert_db_not_empty "SELECT id FROM abac_policy_set_versions WHERE id = '\''${VERSION_ID}'\'' AND status = '\''draft'\'';" "version is draft"

  list_resp=$(api_get "/api/v1/tenants/${TENANT_ID}/abac/policies")
  assert_http_status "$(resp_status "$list_resp")" 200 "list policies returns 200"
  list_body=$(resp_body "$list_resp")
  assert_contains "$list_body" "$VERSION_ID" "list contains new draft version"

  qa_set_token ""
'

scenario 2 "发布策略并切换到 shadow 模式" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  TENANT_ID=$(db_query "SELECT id FROM tenants LIMIT 1;" | tr -d "[:space:]")

  draft_resp=$(api_post "/api/v1/tenants/${TENANT_ID}/abac/policies" "{\"change_note\":\"QA publish test\",\"policy\":{\"rules\":[{\"id\":\"qa-pub-rule\",\"effect\":\"allow\",\"description\":\"test publish\",\"conditions\":{}}]}}")
  VERSION_ID=$(resp_body "$draft_resp" | jq -r ".data.id")

  pub_resp=$(api_post "/api/v1/tenants/${TENANT_ID}/abac/policies/${VERSION_ID}/publish" "{\"mode\":\"shadow\"}")
  assert_http_status "$(resp_status "$pub_resp")" 200 "publish policy returns 200"

  PS_MODE=$(db_query "SELECT mode FROM abac_policy_sets WHERE tenant_id = '\''${TENANT_ID}'\'';" | tr -d "[:space:]")
  assert_eq "$PS_MODE" "shadow" "policy set mode is shadow"

  PUB_VER=$(db_query "SELECT published_version_id FROM abac_policy_sets WHERE tenant_id = '\''${TENANT_ID}'\'';" | tr -d "[:space:]")
  assert_eq "$PUB_VER" "$VERSION_ID" "published_version_id points to new version"

  VER_STATUS=$(db_query "SELECT status FROM abac_policy_set_versions WHERE id = '\''${VERSION_ID}'\'';" | tr -d "[:space:]")
  assert_eq "$VER_STATUS" "published" "version status is published"

  qa_set_token ""
'

scenario 3 "回滚到历史版本" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  TENANT_ID=$(db_query "SELECT id FROM tenants LIMIT 1;" | tr -d "[:space:]")

  v1_resp=$(api_post "/api/v1/tenants/${TENANT_ID}/abac/policies" "{\"change_note\":\"QA rollback v1\",\"policy\":{\"rules\":[{\"id\":\"qa-rb-v1\",\"effect\":\"allow\",\"description\":\"v1 rule\",\"conditions\":{}}]}}")
  V1_ID=$(resp_body "$v1_resp" | jq -r ".data.id")
  api_post "/api/v1/tenants/${TENANT_ID}/abac/policies/${V1_ID}/publish" "{\"mode\":\"shadow\"}" >/dev/null

  v2_resp=$(api_post "/api/v1/tenants/${TENANT_ID}/abac/policies" "{\"change_note\":\"QA rollback v2\",\"policy\":{\"rules\":[{\"id\":\"qa-rb-v2\",\"effect\":\"deny\",\"description\":\"v2 rule\",\"conditions\":{}}]}}")
  V2_ID=$(resp_body "$v2_resp" | jq -r ".data.id")
  api_post "/api/v1/tenants/${TENANT_ID}/abac/policies/${V2_ID}/publish" "{\"mode\":\"shadow\"}" >/dev/null

  CUR_PUB=$(db_query "SELECT published_version_id FROM abac_policy_sets WHERE tenant_id = '\''${TENANT_ID}'\'';" | tr -d "[:space:]")
  assert_eq "$CUR_PUB" "$V2_ID" "current published is v2 before rollback"

  rb_resp=$(api_post "/api/v1/tenants/${TENANT_ID}/abac/policies/${V1_ID}/rollback" "{\"mode\":\"shadow\"}")
  assert_http_status "$(resp_status "$rb_resp")" 200 "rollback to v1 returns 200"

  AFTER_PUB=$(db_query "SELECT published_version_id FROM abac_policy_sets WHERE tenant_id = '\''${TENANT_ID}'\'';" | tr -d "[:space:]")
  assert_eq "$AFTER_PUB" "$V1_ID" "published_version_id is v1 after rollback"

  V2_STATUS=$(db_query "SELECT status FROM abac_policy_set_versions WHERE id = '\''${V2_ID}'\'';" | tr -d "[:space:]")
  assert_eq "$V2_STATUS" "archived" "v2 status is archived after rollback"

  V1_STATUS=$(db_query "SELECT status FROM abac_policy_set_versions WHERE id = '\''${V1_ID}'\'';" | tr -d "[:space:]")
  assert_eq "$V1_STATUS" "published" "v1 status is published after rollback"

  qa_set_token ""
'

scenario 4 "策略模拟（allow/deny 命中规则）" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  TENANT_ID=$(db_query "SELECT id FROM tenants LIMIT 1;" | tr -d "[:space:]")

  draft_resp=$(api_post "/api/v1/tenants/${TENANT_ID}/abac/policies" "{\"change_note\":\"QA simulate test\",\"policy\":{\"rules\":[{\"id\":\"qa-sim-allow\",\"effect\":\"allow\",\"description\":\"allow admin\",\"conditions\":{\"subject.roles\":{\"contains\":\"admin\"}}}]}}")
  SIM_VER=$(resp_body "$draft_resp" | jq -r ".data.id")
  api_post "/api/v1/tenants/${TENANT_ID}/abac/policies/${SIM_VER}/publish" "{\"mode\":\"shadow\"}" >/dev/null

  sim_resp=$(api_post "/api/v1/tenants/${TENANT_ID}/abac/simulate" "{\"simulation\":{\"action\":\"user_manage\",\"resource_type\":\"tenant\",\"subject\":{\"roles\":[\"admin\"]},\"resource\":{\"tenant_id\":\"${TENANT_ID}\"},\"request\":{\"ip\":\"127.0.0.1\"},\"env\":{\"hour\":10}}}")
  assert_http_status "$(resp_status "$sim_resp")" 200 "simulate returns 200"
  sim_body=$(resp_body "$sim_resp")
  assert_json_exists "$sim_body" ".data.decision" "simulation result has decision"

  sim_inline=$(api_post "/api/v1/tenants/${TENANT_ID}/abac/simulate" "{\"policy\":{\"rules\":[{\"id\":\"inline-deny\",\"effect\":\"deny\",\"description\":\"deny all\",\"conditions\":{}}]},\"simulation\":{\"action\":\"user_manage\",\"resource_type\":\"tenant\",\"subject\":{\"roles\":[\"viewer\"]},\"resource\":{},\"request\":{},\"env\":{}}}")
  assert_http_status "$(resp_status "$sim_inline")" 200 "simulate with inline policy returns 200"
  inline_body=$(resp_body "$sim_inline")
  assert_json_exists "$inline_body" ".data.decision" "inline simulation has decision"

  PUB_UNCHANGED=$(db_query "SELECT published_version_id FROM abac_policy_sets WHERE tenant_id = '\''${TENANT_ID}'\'';" | tr -d "[:space:]")
  assert_eq "$PUB_UNCHANGED" "$SIM_VER" "simulation did not change published version"

  qa_set_token ""
'

scenario 5 "租户隔离与权限校验" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  TENANT_A=$(db_query "SELECT id FROM tenants LIMIT 1;" | tr -d "[:space:]")
  TENANT_B=$(db_query "SELECT id FROM tenants LIMIT 1 OFFSET 1;" | tr -d "[:space:]")
  if [[ -z "$TENANT_B" ]] || [[ "$TENANT_A" == "$TENANT_B" ]]; then
    echo "Need at least 2 tenants for isolation test, skipping cross-tenant check" >&2
    resp_a=$(api_get "/api/v1/tenants/${TENANT_A}/abac/policies")
    assert_http_status "$(resp_status "$resp_a")" 200 "admin can access tenant A ABAC"
    qa_set_token ""
    return 0
  fi

  resp_a=$(api_get "/api/v1/tenants/${TENANT_A}/abac/policies")
  assert_http_status "$(resp_status "$resp_a")" 200 "admin can access tenant A ABAC"

  USER_A=$(db_query "SELECT user_id FROM tenant_users WHERE tenant_id = '\''${TENANT_A}'\'' LIMIT 1;" | tr -d "[:space:]")
  if [[ -n "$USER_A" ]]; then
    TENANT_TOKEN_A=$(gen_tenant_token "$USER_A" "$TENANT_A")
    qa_set_token "$TENANT_TOKEN_A"

    resp_own=$(api_get "/api/v1/tenants/${TENANT_A}/abac/policies")
    own_status=$(resp_status "$resp_own")
    assert_eq "$own_status" "200" "tenant A token can access own ABAC"

    resp_cross=$(api_get "/api/v1/tenants/${TENANT_B}/abac/policies")
    cross_status=$(resp_status "$resp_cross")
    assert_eq "$cross_status" "403" "tenant A token cannot access tenant B ABAC"

    INITIAL_COUNT_B=$(db_query "SELECT COUNT(*) FROM abac_policy_set_versions psv JOIN abac_policy_sets ps ON ps.id = psv.policy_set_id WHERE ps.tenant_id = '\''${TENANT_B}'\'';" | tr -d "[:space:]")

    cross_create=$(api_post "/api/v1/tenants/${TENANT_B}/abac/policies" "{\"change_note\":\"cross-tenant attack\",\"policy\":{\"rules\":[]}}")
    cross_create_status=$(resp_status "$cross_create")
    assert_eq "$cross_create_status" "403" "tenant A token cannot create policy in tenant B"

    AFTER_COUNT_B=$(db_query "SELECT COUNT(*) FROM abac_policy_set_versions psv JOIN abac_policy_sets ps ON ps.id = psv.policy_set_id WHERE ps.tenant_id = '\''${TENANT_B}'\'';" | tr -d "[:space:]")
    assert_eq "$AFTER_COUNT_B" "$INITIAL_COUNT_B" "tenant B version count unchanged after cross-tenant attempt"
  fi

  qa_set_token ""
'

run_all
