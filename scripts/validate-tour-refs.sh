#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
set -euo pipefail

# Validate that symbol references in tour-de-code documents exist in the codebase.
# This prevents documentation rot by catching renamed/removed functions early.
#
# Usage:
#   ./scripts/validate-tour-refs.sh
#
# The script extracts Rust symbol names from \texttt{...} blocks in the tour
# documents and verifies each one exists in the crates/ directory.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TOUR_FILES=(
  "docs/study/echo-tour-de-code.tex"
  "docs/study/echo-tour-de-code-with-commentary.tex"
)

# Symbols to skip (LaTeX formatting, common words, etc.)
SKIP_PATTERNS=(
  "^[0-9]"           # Line numbers
  "^crates/"         # File paths (not symbols)
  "^src/"            # File paths
  "\.rs$"            # File extensions
  "\.tex$"
  "^[A-Z_]+$"        # ALL_CAPS constants (often env vars)
  "^\\\\texttt"      # LaTeX artifacts
  "^\\.\\."          # Range syntax
  "^&"               # Reference syntax
  "^<"               # Generic syntax
  "^>"
  "^\\["             # Array syntax
  "^\\]"
)

if ! command -v rg >/dev/null 2>&1; then
  echo "ERROR: ripgrep (rg) is required." >&2
  exit 2
fi

echo "validate-tour-refs: checking symbol references in tour documents"
echo

missing=0
checked=0

for tour_file in "${TOUR_FILES[@]}"; do
  if [[ ! -f "$tour_file" ]]; then
    echo "SKIP: $tour_file (not found)"
    continue
  fi

  echo "Scanning: $tour_file"

  # Extract potential Rust symbols from \texttt{...} blocks
  # Look for snake_case function names and PascalCase type names
  symbols=$(rg -o '\\texttt\{[^}]+\}' "$tour_file" 2>/dev/null | \
    sed 's/\\texttt{//g; s/}//g' | \
    rg -o '[a-z_][a-z0-9_]*(\(\))?|[A-Z][a-zA-Z0-9]*' | \
    sed 's/()$//' | \
    sort -u || true)

  for sym in $symbols; do
    # Skip if matches any skip pattern
    skip=0
    for pat in "${SKIP_PATTERNS[@]}"; do
      if echo "$sym" | grep -qE "$pat"; then
        skip=1
        break
      fi
    done
    [[ $skip -eq 1 ]] && continue

    # Skip very short symbols (likely false positives)
    [[ ${#sym} -lt 3 ]] && continue

    checked=$((checked + 1))

    # Search for the symbol in Rust files
    # For functions: fn symbol_name
    # For structs/enums: struct/enum SymbolName
    # For traits: trait TraitName
    if ! rg -q "(fn |struct |enum |trait |type |impl |mod )$sym\\b" crates/ 2>/dev/null; then
      # Try as a method or field reference
      if ! rg -q "\\b$sym\\b" crates/ 2>/dev/null; then
        echo "  MISSING: $sym"
        missing=$((missing + 1))
      fi
    fi
  done
done

echo
echo "validate-tour-refs: checked $checked symbols, $missing missing"

if [[ $missing -gt 0 ]]; then
  echo "FAILED: Some referenced symbols not found in codebase."
  echo "Update the tour documents or add the missing symbols."
  exit 1
fi

echo "PASSED: All referenced symbols exist."
