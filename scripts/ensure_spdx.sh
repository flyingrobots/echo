#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

set -euo pipefail

# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

# Script to ensure correct SPDX headers are present.
# Usage: ./scripts/ensure_spdx.sh [--check] [--all] [files...]
# Defaults to operating on staged files if no files/flags provided.

CHECK_MODE=0
ALL_MODE=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --check) CHECK_MODE=1; shift ;;
    --all) ALL_MODE=1; shift ;;
    *) break ;;
  esac
done

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"

# License Templates
CODE_LICENSE="SPDX-License-Identifier: Apache-2.0"
DUAL_LICENSE="SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0"
COPYRIGHT="© James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>"

# Stats
MODIFIED_COUNT=0
FAILED_COUNT=0

# -----------------------------------------------------------------------------
# Helper Functions
# -----------------------------------------------------------------------------

should_skip() {
  local f="$1"
  [[ ! -f "$f" ]] && return 0
  
  case "$f" in
    target/*|node_modules/*|vendor/*|docs/benchmarks/vendor/*|dist/*|.cache/*) return 0 ;;
    .git/*|.idea/*|.vscode/*|.DS_Store) return 0 ;;
    *.png|*.jpg|*.jpeg|*.gif|*.webp|*.svg|*.ico|*.pdf|*.woff|*.woff2|*.ttf|*.eot|*.map) return 0 ;;
    *.lock|package-lock.json|yarn.lock|Cargo.lock) return 0 ;;
    LICENSE*|NOTICE|COPYING|*.license) return 0 ;;
    docs/echo-total.md) return 0 ;;
    *.json) return 0 ;;
  esac
  return 1
}

is_source_code() {
  local f="$1"
  case "$f" in
    *.rs|*.js|*.ts|*.py|*.sh|*.bash|*.c|*.h|*.cpp|*.hpp|*.go|*.java|*.kt|*.scala|*.swift|*.dart) return 0 ;;
    *) return 1 ;;
  esac
}

get_comment_style() {
  local f="$1"
  case "$f" in
    *.rs|*.js|*.ts|*.c|*.h|*.cpp|*.hpp|*.go|*.java|*.kt|*.scala|*.swift|*.dart) echo "slash" ;;
    *.sh|*.bash|*.py|*.rb|*.pl|*.yaml|*.yml|*.toml|*.dockerfile|Dockerfile|Makefile|*.gitignore|*.editorconfig|*.conf|*.ini) echo "hash" ;;
    *.tex|*.sty|*.cls) echo "percent" ;;
    *.md|*.html|*.xml) echo "xml" ;;
    *) echo "unknown" ;;
  esac
}

get_header_content() {
  local f="$1"
  local style="$2"
  local license_line
  
  if is_source_code "$f"; then
    license_line="$CODE_LICENSE"
  else
    license_line="$DUAL_LICENSE"
  fi

  case "$style" in
    slash)
      printf "// %s\n// %s" "$license_line" "$COPYRIGHT"
      ;;
    hash)
      printf "# %s\n# %s" "$license_line" "$COPYRIGHT"
      ;;
    percent)
      printf "%% %s\n%% %s" "$license_line" "$COPYRIGHT"
      ;;
    xml)
      printf "<!-- %s -->\n<!-- %s -->" "$license_line" "$COPYRIGHT"
      ;;
  esac
}

check_valid_header() {
  local f="$1"
  local expected="$2"
  
  # Read first few lines
  local head_content
  head_content=$(head -n 10 "$f")

  # Check if the *exact expected block* is present in the header.
  # We treat the multi-line string $expected as a fixed string search.
  # Because grepping a multi-line string is tricky, we check line by line or use grep -F.
  # Simplification: Check if the first line of expected is in the file, 
  # AND it matches the comment style.
  
  # Actually, we can just check if the exact expected string appears in the head.
  # Using awk or python would be safer for multi-line, but let's use grep -F.
  if echo "$head_content" | grep -Fq "$expected"; then
      return 0
  fi
  return 1
}

has_malformed_header() {
  local f="$1"
  # Check if the "raw" license ID exists but maybe not commented correctly
  # or incorrect license type.
  local head_content
  head_content=$(head -n 10 "$f")
  
  if echo "$head_content" | grep -Fq "SPDX-License-Identifier"; then
      return 0
  fi
  if echo "$head_content" | grep -Fq "James Ross"; then
      return 0
  fi
  return 1
}

strip_existing_headers() {
  local f="$1"
  local temp_file
  temp_file=$(mktemp)
  
  # Use AWK to filter out lines in the first 15 lines that match SPDX/Copyright patterns.
  # This effectively removes "bad" headers or "wrong license" headers.
  # We preserve shebangs because they typically don't match the pattern.
  
  awk ' 
    BEGIN { in_header = 1; count = 0 }
    { 
      count++;
      if (count <= 15) {
        if ($0 ~ /SPDX-License-Identifier/ || $0 ~ /James Ross .* FLYING/) {
          next
        }
      }
      print
    }
  ' "$f" > "$temp_file"
  
  cat "$temp_file" > "$f"
  rm "$temp_file"
}

insert_header() {
  local f="$1"
  local header="$2"
  local temp_file
  temp_file=$(mktemp)
  
  local first_line
  first_line=$(head -n 1 "$f" || true)
  
  if [[ "$first_line" =~ ^#! ]]; then
    echo "$first_line" > "$temp_file"
    echo "$header" >> "$temp_file"
    tail -n +2 "$f" >> "$temp_file"
  elif [[ "$first_line" =~ ^\<?xml ]]; then
    echo "$first_line" > "$temp_file"
    echo "$header" >> "$temp_file"
    tail -n +2 "$f" >> "$temp_file"
  else
    echo "$header" > "$temp_file"
    cat "$f" >> "$temp_file"
  fi
  
  cat "$temp_file" > "$f"
  rm "$temp_file"
}

process_file() {
  local f="$1"
  if should_skip "$f"; then return; fi
  
  local style
  style=$(get_comment_style "$f")
  if [[ "$style" == "unknown" ]]; then return; fi
  
  local expected_header
  expected_header=$(get_header_content "$f" "$style")
  
  if check_valid_header "$f" "$expected_header"; then
    return 0
  fi

  if [[ "$CHECK_MODE" -eq 1 ]]; then
    echo "[FAIL] Incorrect or missing SPDX header: $f"
    FAILED_COUNT=$((FAILED_COUNT + 1))
  else
    # Repair mode:
    # 1. Strip any existing (malformed/wrong) header lines from top
    strip_existing_headers "$f"
    # 2. Insert correct header
    insert_header "$f" "$expected_header"
    echo "[FIXED] Repaired header: $f"
    MODIFIED_COUNT=$((MODIFIED_COUNT + 1))
  fi
}

# -----------------------------------------------------------------------------
# Main Execution
# -----------------------------------------------------------------------------

FILES_TO_CHECK=()

if [[ $# -gt 0 ]]; then
  FILES_TO_CHECK=("$@")
elif [[ "$ALL_MODE" -eq 1 ]]; then
  while IFS= read -r -d '' file; do
    FILES_TO_CHECK+=("$file")
  done < <(git ls-files -z)
else
  # Default to staged
  while IFS= read -r -d '' file; do
    FILES_TO_CHECK+=("$file")
  done < <(git diff --cached --name-only -z)
fi

if [[ ${#FILES_TO_CHECK[@]} -eq 0 ]]; then
  exit 0
fi

for f in "${FILES_TO_CHECK[@]}"; do
  process_file "$f"
done

if [[ "$CHECK_MODE" -eq 1 ]]; then
  if [[ "$FAILED_COUNT" -gt 0 ]]; then
    echo "-------------------------------------------------------"
    echo "SPDX Check Failed: $FAILED_COUNT files have missing or incorrect headers."
    echo "Run './scripts/ensure_spdx.sh' (without --check) to auto-repair."
    exit 1
  fi
else
  if [[ "$MODIFIED_COUNT" -gt 0 ]]; then
    echo "-------------------------------------------------------"
    echo "Repaired SPDX headers in $MODIFIED_COUNT files."
    echo "Please review and stage these changes."
    exit 1
  fi
fi

exit 0