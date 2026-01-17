#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
# Sweeps incremental compilation cache older than N days
# Leaves deps/ alone (external crates don't change often)
set -euo pipefail

DAYS="${1:-14}"

echo "🧹 Sweeping stale incremental cache (>${DAYS} days old)..."

SWEPT=0
for base in target target-fmt target-clippy target-test target-doc; do
  # Only sweep incremental/ subdirs - these grow unboundedly
  # deps/ contains external crates that rarely change, leave them alone
  for profile in debug release; do
    incr="$base/$profile/incremental"
    if [[ -d "$incr" ]]; then
      while IFS= read -r -d '' subdir; do
        SIZE=$(du -sh "$subdir" 2>/dev/null | cut -f1)
        echo "   rm -rf $subdir ($SIZE)"
        rm -rf "$subdir"
        SWEPT=$((SWEPT + 1))
      done < <(find "$incr" -mindepth 1 -maxdepth 1 -type d -mtime +"$DAYS" -print0 2>/dev/null)
    fi
  done
done

if (( SWEPT > 0 )); then
  echo "🧹 Swept $SWEPT stale directories ✨"
else
  echo "🧹 Nothing to sweep ✨"
fi
