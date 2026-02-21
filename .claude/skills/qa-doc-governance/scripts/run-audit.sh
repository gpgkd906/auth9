#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
cd "$ROOT_DIR"

echo "[qa-doc-governance] root=$ROOT_DIR"

docs=$(find docs/qa -name '*.md' | sort)
qa_docs=$(printf "%s\n" "$docs" | grep -v 'docs/qa/README.md' | grep -v 'docs/qa/_' || true)

total_docs=$(printf "%s\n" "$qa_docs" | sed '/^$/d' | wc -l | tr -d ' ')
total_scenarios=$(while IFS= read -r f; do
  [[ -z "$f" ]] && continue
  (rg -n '^## 场景' "$f" || true) | wc -l | tr -d ' '
done < <(printf "%s\n" "$qa_docs") | awk '{s+=$1} END{print s+0}')

echo "[summary] docs=$total_docs scenarios=$total_scenarios"

echo "[check] files with >5 scenarios"
while IFS= read -r f; do
  [[ -z "$f" ]] && continue
  n=$(( $( (rg -n '^## 场景' "$f" || true) | wc -l ) ))
  if (( n > 5 )); then
    echo "  - ${f#docs/qa/}:$n"
  fi
done < <(printf "%s\n" "$qa_docs")

echo "[check] files without checklist"
while IFS= read -r f; do
  [[ -z "$f" ]] && continue
  if ! rg -q "## 检查清单|## 回归测试检查清单" "$f"; then
    echo "  - ${f#docs/qa/}"
  fi
done < <(printf "%s\n" "$qa_docs")

echo "[check] ui docs without entry visibility"
ui_docs=$(rg -l "Portal UI|侧边栏|导航|Tab|Quick Links|按钮" docs/qa --glob '*.md' || true)
while IFS= read -r f; do
  [[ -z "$f" ]] && continue
  if ! rg -q "入口可见性" "$f"; then
    echo "  - ${f#docs/qa/}"
  fi
done < <(printf "%s\n" "$ui_docs")

echo "[check] README index drift"
all_without_readme=$(printf "%s\n" "$qa_docs" | sed 's#^docs/qa/##' | sort)
indexed=$(rg -o "\(\./[^)]+\.md\)" docs/qa/README.md | sed -E 's#^\(\./##; s#\)$##' | grep -v '^_' | sort -u)

not_indexed=$(comm -23 <(printf "%s\n" "$all_without_readme") <(printf "%s\n" "$indexed") || true)
indexed_missing=$(comm -13 <(printf "%s\n" "$all_without_readme") <(printf "%s\n" "$indexed") || true)

if [[ -n "$not_indexed" ]]; then
  echo "  - missing in README:"; printf '%s\n' "$not_indexed" | sed 's/^/    * /'
fi
if [[ -n "$indexed_missing" ]]; then
  echo "  - missing in filesystem:"; printf '%s\n' "$indexed_missing" | sed 's/^/    * /'
fi

if [[ -z "$not_indexed" && -z "$indexed_missing" ]]; then
  echo "  - none"
fi

echo "[hint] run scripts/qa-doc-lint.sh for strict pass/fail gate"
