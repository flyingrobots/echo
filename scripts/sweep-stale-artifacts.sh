#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
# Sweeps build artifacts older than 14 days from target directories
# Deletes entire stale subdirectories for speed (not individual files)
set -euo pipefail

DAYS="${1:-14}"

echo "🧹 Sweeping stale build artifacts (>${DAYS} days old)..."

SWEPT=0
for base in target target-fmt target-clippy target-test target-doc; do
  if [[ -d "$base" ]]; then
    # Find top-level subdirs that are entirely stale (dir mtime > DAYS)
    while IFS= read -r -d '' subdir; do
      SIZE=$(du -sh "$subdir" 2>/dev/null | cut -f1)
      echo "   rm -rf $subdir ($SIZE)"
      rm -rf "$subdir"
      SWEPT=$((SWEPT + 1))
    done < <(find "$base" -mindepth 1 -maxdepth 2 -type d -mtime +"$DAYS" -print0 2>/dev/null)
  fi
done

if (( SWEPT > 0 )); then
  echo "🧹 Swept $SWEPT stale directories ✨"
else
  echo "🧹 Nothing to sweep ✨"
fi
