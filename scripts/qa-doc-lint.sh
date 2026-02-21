#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root_dir"

fail=0

all_docs=$(find docs/qa -name '*.md' | sort)
test_docs=$(printf "%s\n" "$all_docs" | grep -v '/_' || true)
all_without_readme=$(printf "%s\n" "$test_docs" | sed 's#^docs/qa/##' | grep -v '^README.md$')
indexed=$(rg -o "\(\./[^)]+\.md\)" docs/qa/README.md | sed -E 's#^\(\./##; s#\)$##' | grep -v '^_' | sort -u)

not_indexed=$(comm -23 <(printf "%s\n" "$all_without_readme" | sort) <(printf "%s\n" "$indexed" | sort) || true)
indexed_missing=$(comm -13 <(printf "%s\n" "$all_without_readme" | sort) <(printf "%s\n" "$indexed" | sort) || true)

if [[ -n "$not_indexed" ]]; then
  echo "[FAIL] README 未索引以下文档:"
  printf "%s\n" "$not_indexed"
  fail=1
fi

if [[ -n "$indexed_missing" ]]; then
  echo "[FAIL] README 索引存在但文件缺失:"
  printf "%s\n" "$indexed_missing"
  fail=1
fi

over_scenario=()
while IFS= read -r f; do
  n=$( (rg -n '^## 场景' "$f" || true) | wc -l | tr -d ' ' )
  if (( n > 5 )); then
    over_scenario+=("${f#docs/qa/}:$n")
  fi
done < <(printf "%s\n" "$test_docs")

if (( ${#over_scenario[@]} > 0 )); then
  echo "[WARN] 以下文档场景数超过 5（需后续拆分）:"
  printf '%s\n' "${over_scenario[@]}"
fi

missing_checklist=()
while IFS= read -r f; do
  [[ "$f" == "docs/qa/README.md" ]] && continue
  if ! rg -q "## 检查清单|## 回归测试检查清单" "$f"; then
    missing_checklist+=("${f#docs/qa/}")
  fi
done < <(printf "%s\n" "$test_docs")

if (( ${#missing_checklist[@]} > 0 )); then
  echo "[FAIL] 以下文档缺少检查清单:"
  printf '%s\n' "${missing_checklist[@]}"
  fail=1
fi

ui_docs=$(rg -l "Portal UI|侧边栏|Tab|Quick Links|导航" docs/qa --glob '*.md' || true)
missing_visibility=()
while IFS= read -r f; do
  [[ -z "$f" ]] && continue
  if ! rg -q "入口可见性" "$f"; then
    missing_visibility+=("${f#docs/qa/}")
  fi
done < <(printf "%s\n" "$ui_docs")

if (( ${#missing_visibility[@]} > 0 )); then
  echo "[WARN] 以下 UI 文档未标注入口可见性场景:"
  printf '%s\n' "${missing_visibility[@]}"
fi

auth_bad=()
auth_docs=$(rg -l -F "关闭浏览器" docs/qa --glob '*.md' || true)
while IFS= read -r f; do
  [[ -z "$f" ]] && continue
  if ! rg -q "无痕|隐私|auth9_session|Sign out|Session 为持久化" "$f"; then
    auth_bad+=("${f#docs/qa/}")
  fi
done < <(printf "%s\n" "$auth_docs")

if (( ${#auth_bad[@]} > 0 )); then
  echo "[FAIL] 以下文档存在关闭浏览器后认证校验但缺少可执行说明:"
  printf '%s\n' "${auth_bad[@]}"
  fail=1
fi

if (( fail > 0 )); then
  echo "\nqa-doc-lint: FAILED"
  exit 1
fi

echo "qa-doc-lint: PASSED"
