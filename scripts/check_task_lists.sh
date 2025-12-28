#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

set -euo pipefail

# Guard against contradictory duplicated checklist items (e.g. the same task both checked and unchecked).
# Intended to keep living task docs unambiguous.

FILES=(
  "WASM-TASKS.md"
  "docs/tasks.md"
)

fail=0

for file in "${FILES[@]}"; do
  [[ -f "$file" ]] || continue

  awk -v file="$file" '
    /^[[:space:]]*-[[:space:]]*\\[[[:space:]xX]\\]/ {
      status = "unchecked"
      if ($0 ~ /^[[:space:]]*-[[:space:]]*\\[[xX]\\]/) {
        status = "checked"
      }

      text = $0
      sub(/^[[:space:]]*-[[:space:]]*\\[[[:space:]xX]\\][[:space:]]*/, "", text)
      gsub(/[[:space:]]+$/, "", text)
      gsub(/[[:space:]]+/, " ", text)
      if (text == "") next

      if (seen[text] != "" && seen[text] != status) {
        print "Task list conflict in " file ":" > "/dev/stderr"
        print "  - \"" text "\" appears as both " seen[text] " and " status "." > "/dev/stderr"
        fail = 1
      }

      seen[text] = status
    }
    END { exit (fail ? 1 : 0) }
  ' "$file" || fail=1
done

if [[ "$fail" -ne 0 ]]; then
  exit 1
fi

exit 0
