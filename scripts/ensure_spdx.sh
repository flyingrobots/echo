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

has_header() {
  local f="$1"
  local style="$2"
  local head_content
  head_content=$(head -n 10 "$f")
  
  if [[ "$head_content" == *"$CODE_LICENSE"* ]] || [[ "$head_content" == *"$DUAL_LICENSE"* ]]; then
    return 0
  fi
  return 1
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
  
  local header_text
  header_text=$(get_header_content "$f" "$style")
  
  if ! has_header "$f" "$style"; then
    if [[ "$CHECK_MODE" -eq 1 ]]; then
      echo "[FAIL] Missing SPDX header: $f"
      FAILED_COUNT=$((FAILED_COUNT + 1))
    else
      insert_header "$f" "$header_text"
      echo "[FIXED] Inserted header: $f"
      MODIFIED_COUNT=$((MODIFIED_COUNT + 1))
    fi
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
    echo "SPDX Check Failed: $FAILED_COUNT files missing headers."
    exit 1
  fi
else
  if [[ "$MODIFIED_COUNT" -gt 0 ]]; then
    echo "-------------------------------------------------------"
    echo "Inserted SPDX headers in $MODIFIED_COUNT files."
    echo "Please review and stage these changes."
    exit 1
  fi
fi

exit 0
