#!/usr/bin/env bash
# QA Assertion Library
# Provides standardized assertion functions with JSONL output.
# All assertions write to stdout; debug info goes to stderr.
#
# Usage: source this file from test scripts.
#   source "$(dirname "${BASH_SOURCE[0]}")/assert.sh"

_QA_PASS_COUNT=0
_QA_FAIL_COUNT=0

_qa_emit_assert() {
  local status="$1" label="$2" expected="$3" actual="$4"
  printf '{"type":"assert","status":"%s","label":"%s","expected":"%s","actual":"%s"}\n' \
    "$status" "$label" "$expected" "$actual"
}

_qa_pass() {
  _QA_PASS_COUNT=$((_QA_PASS_COUNT + 1))
  _qa_emit_assert "PASS" "$1" "$2" "$3"
}

_qa_fail() {
  _QA_FAIL_COUNT=$((_QA_FAIL_COUNT + 1))
  _qa_emit_assert "FAIL" "$1" "$2" "$3"
}

assert_eq() {
  local actual="$1" expected="$2" label="$3"
  if [[ "$actual" == "$expected" ]]; then
    _qa_pass "$label" "$expected" "$actual"
  else
    _qa_fail "$label" "$expected" "$actual"
  fi
}

assert_ne() {
  local actual="$1" not_expected="$2" label="$3"
  if [[ "$actual" != "$not_expected" ]]; then
    _qa_pass "$label" "!=$not_expected" "$actual"
  else
    _qa_fail "$label" "!=$not_expected" "$actual"
  fi
}

assert_contains() {
  local haystack="$1" needle="$2" label="$3"
  if [[ "$haystack" == *"$needle"* ]]; then
    _qa_pass "$label" "contains $needle" "(found)"
  else
    _qa_fail "$label" "contains $needle" "(not found)"
  fi
}

assert_not_contains() {
  local haystack="$1" needle="$2" label="$3"
  if [[ "$haystack" != *"$needle"* ]]; then
    _qa_pass "$label" "not contains $needle" "(absent)"
  else
    _qa_fail "$label" "not contains $needle" "(found)"
  fi
}

assert_match() {
  local string="$1" regex="$2" label="$3"
  if [[ "$string" =~ $regex ]]; then
    _qa_pass "$label" "~$regex" "$string"
  else
    _qa_fail "$label" "~$regex" "$string"
  fi
}

assert_http_status() {
  local actual="$1" expected="$2" label="$3"
  assert_eq "$actual" "$expected" "$label"
}

assert_json_field() {
  local json="$1" jq_path="$2" expected="$3" label="$4"
  local actual
  actual=$(echo "$json" | jq -r "$jq_path" 2>/dev/null) || actual="(jq error)"
  if [[ "$actual" == "$expected" ]]; then
    _qa_pass "$label" "$expected" "$actual"
  else
    _qa_fail "$label" "$expected" "$actual"
  fi
}

assert_json_exists() {
  local json="$1" jq_path="$2" label="$3"
  local val
  val=$(echo "$json" | jq -e "$jq_path" 2>/dev/null) && {
    _qa_pass "$label" "exists" "exists"
    return
  }
  _qa_fail "$label" "exists" "null/missing"
}

assert_json_not_exists() {
  local json="$1" jq_path="$2" label="$3"
  local val
  val=$(echo "$json" | jq -e "$jq_path" 2>/dev/null) && {
    _qa_fail "$label" "not exists" "exists"
    return
  }
  _qa_pass "$label" "not exists" "not exists"
}

assert_db() {
  local sql="$1" expected="$2" label="$3"
  local actual
  actual=$(db_query "$sql" 2>/dev/null | tr -d '[:space:]') || actual="(db error)"
  expected_trimmed=$(echo "$expected" | tr -d '[:space:]')
  if [[ "$actual" == "$expected_trimmed" ]]; then
    _qa_pass "$label" "$expected" "$actual"
  else
    _qa_fail "$label" "$expected" "$actual"
  fi
}

assert_db_not_empty() {
  local sql="$1" label="$2"
  local actual
  actual=$(db_query "$sql" 2>/dev/null) || actual=""
  if [[ -n "$actual" ]]; then
    _qa_pass "$label" "non-empty" "(has data)"
  else
    _qa_fail "$label" "non-empty" "(empty)"
  fi
}

qa_assert_counts() {
  echo "${_QA_PASS_COUNT}:${_QA_FAIL_COUNT}"
}

qa_reset_counts() {
  _QA_PASS_COUNT=0
  _QA_FAIL_COUNT=0
}
