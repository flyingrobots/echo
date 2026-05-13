#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# Adjust these to your repo conventions
ABI_HINTS=(
  "abi"
  "codec"
  "message"
  "frame"
  "packet"
  "envelope"
  "dto"
  "wire"
)

ALLOWLIST="${UNORDERED_ABI_ALLOWLIST:-.ban-unordered-abi-allowlist}"

RG_ARGS=(
  --hidden
  --no-ignore
  --glob '!**/.git/**'
  --glob '!**/target/**'
  --glob '!**/node_modules/**'
)
GREP_ARGS=(-RInP --exclude-dir=.git --exclude-dir=target --exclude-dir=node_modules)

# You can allow file-level exceptions via allowlist (keep it tiny).
ALLOW_PATTERNS=()
if [[ -f "$ALLOWLIST" ]]; then
  while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    [[ "$line" =~ ^# ]] && continue
    pat="${line%%$'\t'*}"
    pat="${pat%% *}"
    [[ -z "$pat" ]] && continue
    ALLOW_PATTERNS+=("$pat")
  done < "$ALLOWLIST"
fi

# Find Rust files likely involved in ABI/wire formats.
# Build pattern and trim trailing '|' to avoid matching everything
pattern="$(printf '%s|' "${ABI_HINTS[@]}")"
pattern="${pattern%|}"
if command -v rg >/dev/null 2>&1; then
  mapfile -t files < <(rg "${RG_ARGS[@]}" -l -g'*.rs' "$pattern" crates/ || true)
else
  if ! printf 'x\n' | grep -P 'x' >/dev/null 2>&1; then
    echo "ERROR: ripgrep (rg) or grep -P is required." >&2
    exit 2
  fi
  mapfile -t files < <(
    find crates -type f -name '*.rs' \
      -not -path '*/.git/*' \
      -not -path '*/target/*' \
      -not -path '*/node_modules/*' \
      -print |
      grep -P "$pattern" || true
  )
fi
shopt -s globstar
filtered=()
for f in "${files[@]}"; do
  allowed=false
  for pat in "${ALLOW_PATTERNS[@]}"; do
    if [[ "$f" == $pat ]]; then
      allowed=true
      break
    fi
  done
  if [[ "$allowed" == false ]]; then
    filtered+=("$f")
  fi
done
files=("${filtered[@]}")

if [[ ${#files[@]} -eq 0 ]]; then
  echo "ban-unordered-abi: no ABI-ish files found (by heuristic). OK."
  exit 0
fi

echo "ban-unordered-abi: scanning ABI-ish Rust files..."
violations=0

# HashMap/HashSet are not allowed in ABI-ish types. Use Vec<(K,V)> sorted, BTreeMap, IndexMap with explicit canonicalization, etc.
if command -v rg >/dev/null 2>&1; then
  search_result=0
  rg "${RG_ARGS[@]}" -n -S '\b(HashMap|HashSet)\b' "${files[@]}" || search_result=$?
else
  search_result=0
  grep "${GREP_ARGS[@]}" '\b(HashMap|HashSet)\b' "${files[@]}" || search_result=$?
fi

if [[ $search_result -eq 0 ]]; then
  violations=$((violations+1))
elif [[ $search_result -gt 1 ]]; then
  exit "$search_result"
fi

if [[ $violations -ne 0 ]]; then
  echo "ban-unordered-abi: FAILED. Unordered containers found in ABI-ish code."
  echo "Fix by using Vec pairs + sorting, or BTreeMap + explicit canonical encode ordering."
  exit 1
fi

echo "ban-unordered-abi: PASSED."
