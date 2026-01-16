#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if ! command -v rg >/dev/null 2>&1; then
  echo "ERROR: ripgrep (rg) is required." >&2
  exit 2
fi

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

RG_ARGS=(--hidden --no-ignore --glob '!.git/*' --glob '!target/*' --glob '!**/node_modules/*')

# Find Rust files likely involved in ABI/wire formats.
# Build pattern and trim trailing '|' to avoid matching everything
pattern="$(printf '%s|' "${ABI_HINTS[@]}")"
pattern="${pattern%|}"
mapfile -t files < <(rg "${RG_ARGS[@]}" -l -g'*.rs' "$pattern" crates/ || true)

if [[ ${#files[@]} -eq 0 ]]; then
  echo "ban-unordered-abi: no ABI-ish files found (by heuristic). OK."
  exit 0
fi

echo "ban-unordered-abi: scanning ABI-ish Rust files..."
violations=0

# HashMap/HashSet are not allowed in ABI-ish types. Use Vec<(K,V)> sorted, BTreeMap, IndexMap with explicit canonicalization, etc.
if rg "${RG_ARGS[@]}" -n -S '\b(HashMap|HashSet)\b' "${files[@]}"; then
  violations=$((violations+1))
fi

if [[ $violations -ne 0 ]]; then
  echo "ban-unordered-abi: FAILED. Unordered containers found in ABI-ish code."
  echo "Fix by using Vec pairs + sorting, or BTreeMap + explicit canonical encode ordering."
  exit 1
fi

echo "ban-unordered-abi: PASSED."
