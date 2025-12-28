#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
deny_file="${repo_root}/deny.toml"

# Single source of truth for cargo-audit policy ignores: derive from deny.toml.
# Rationale for each ignored advisory lives alongside the deny config comments.

ignore_ids=()
if [[ -f "$deny_file" ]]; then
  while IFS= read -r id; do
    [[ -n "$id" ]] || continue
    ignore_ids+=("$id")
  done < <(
    awk '
      /^\[advisories\]/ { in_adv = 1; next }
      in_adv && /^\[/ { in_adv = 0 }

      in_adv && /^[[:space:]]*ignore[[:space:]]*=[[:space:]]*\[/ { in_ignore = 1 }

      in_adv && in_ignore {
        # Strip comments and extract all quoted strings.
        sub(/#.*/, "", $0)
        while (match($0, /"[^"]+"/)) {
          s = substr($0, RSTART + 1, RLENGTH - 2)
          print s
          $0 = substr($0, RSTART + RLENGTH)
        }
        if ($0 ~ /]/) { in_ignore = 0; exit }
      }
    ' "$deny_file"
  )
else
  echo "Warning: deny.toml not found; running cargo audit without ignores" >&2
fi

ignore_flags=()
for id in "${ignore_ids[@]}"; do
  ignore_flags+=(--ignore "$id")
done

if [[ "${#ignore_ids[@]}" -ne 0 ]]; then
  echo "cargo-audit: ignoring advisories from deny.toml:" >&2
  printf '  - %s\n' "${ignore_ids[@]}" >&2
fi

cargo audit --deny warnings "${ignore_flags[@]}"
