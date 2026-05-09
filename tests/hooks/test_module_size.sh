#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.." || exit 1

max_lines=3000
file="crates/warp-core/src/optic.rs"
line_count="$(wc -l <"$file" | tr -d ' ')"

if (( line_count > max_lines )); then
  echo "$file has $line_count lines; keep module roots under $max_lines lines" >&2
  exit 1
fi
