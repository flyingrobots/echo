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
existing_files=()

for file in "${FILES[@]}"; do
  if [[ ! -f "$file" ]]; then
    echo "Warning: task list file not found: $file" >&2
    continue
  fi
  existing_files+=("$file")
done

if [[ "${#existing_files[@]}" -eq 0 ]]; then
  echo "Error: no task list files found to validate" >&2
  exit 1
fi

awk '
  /^[[:space:]]*-[[:space:]]*\[[[:space:]xX]\]/ {
    status = "unchecked"
    if ($0 ~ /^[[:space:]]*-[[:space:]]*\[[xX]\]/) {
      status = "checked"
    }

    text = $0
    sub(/^[[:space:]]*-[[:space:]]*\[[[:space:]xX]\][[:space:]]*/, "", text)
    gsub(/[[:space:]]+$/, "", text)
    gsub(/[[:space:]]+/, " ", text)
    if (text == "") next

    norm = tolower(text)

    if (seen_status[norm] != "" && seen_status[norm] != status) {
      print "Task list conflict:" > "/dev/stderr"
      print "  - \"" seen_text[norm] "\" appears as both " seen_status[norm] " (" seen_file[norm] ") and " status " (" FILENAME ")." > "/dev/stderr"
      fail = 1
    }

    if (seen_text[norm] == "") {
      seen_text[norm] = text
    }
    if (seen_file[norm] == "") {
      seen_file[norm] = FILENAME
    }
    seen_status[norm] = status
  }
  END { exit (fail ? 1 : 0) }
' "${existing_files[@]}" || fail=1

if [[ "$fail" -ne 0 ]]; then
  exit 1
fi

exit 0
