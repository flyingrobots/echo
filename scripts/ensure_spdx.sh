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
        LICENSE*|NOTICE|COPYING|*.license|*/LICENSE*|*/NOTICE|*/COPYING|*/*.license) return 0 ;;
        *.json) return 0 ;; 
      esac
      return 1
    }
    
is_source_code() {
  local f="$1"
  case "$f" in
    *.rs|*.js|*.ts|*.py|*.sh|*.bash|*.c|*.h|*.cpp|*.hpp|*.go|*.java|*.kt|*.scala|*.swift|*.dart|*.wgsl|*.hlsl|*.glsl) return 0 ;;
    *) return 1 ;;
  esac
}

is_dual_licensed_material() {
  local f="$1"

  case "$f" in
    *.md|*.html|*.xml|*.tex|*.sty|*.cls) return 0 ;;
    *) return 1 ;;
  esac
}
    
    get_comment_style() {
      local f="$1"
      case "$f" in
        *.rs|*.js|*.ts|*.c|*.h|*.cpp|*.hpp|*.go|*.java|*.kt|*.scala|*.swift|*.dart) echo "slash" ;; 
        *.sh|*.bash|*.py|*.rb|*.pl|*.yaml|*.yml|*.toml|*.dockerfile|Dockerfile|*/Dockerfile|Makefile|*/Makefile|*.gitignore|*.editorconfig|*.conf|*.ini) echo "hash" ;; 
        *.tex|*.sty|*.cls) echo "percent" ;; 
        *.md|*.html|*.xml) echo "xml" ;; 
        *) echo "unknown" ;; 
      esac
    }
get_header_content() {
  local f="$1"
  local style="$2"
  local license_line
  
  if is_dual_licensed_material "$f"; then
    license_line="$DUAL_LICENSE"
  else
    license_line="$CODE_LICENSE"
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
  local expected_header_block="$2" # This is a multi-line string like "# SPDX...\n# ©..."
  
  local file_lines=()
  # Read file line by line, handling newlines properly
  while IFS= read -r line; do
    file_lines+=("$line")
  done < "$f"

  local i=0
  local first_line_in_file="${file_lines[0]:-}"

  # Skip shebang/xml if present
  if [[ "$first_line_in_file" =~ ^#! || "$first_line_in_file" =~ ^\<\?xml ]]; then
      i=1
  fi

  # Extract the lines from expected_header_block
  local expected_lines=()
  # Use process substitution with IFS=$'\n' to split multi-line string into array elements
  IFS=$'\n' read -r -d '' -a expected_lines <<< "$expected_header_block"

  # Check if we have enough expected header lines (should be 2)
  if [[ ${#expected_lines[@]} -ne 2 ]]; then
    return 1 # Should not happen if get_header_content is correct.
  fi

  # Check if file has enough lines to compare the header
  if [[ ${#file_lines[@]} -lt $((i + 1)) ]]; then return 1; fi # Not enough lines for header

  # Compare line by line
  if [[ "${file_lines[i]:-}" == "${expected_lines[0]}" && "${file_lines[i+1]:-}" == "${expected_lines[1]}" ]]; then
      return 0 # Exact header found
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
  if echo "$head_content" | grep -Fq "James Ross"; then # Looking for the copyright holder
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
    BEGIN { header_block_active = 1; line_num = 0 }
    {
      line_num++;
      if (header_block_active) {
        # Once we pass line 15, we are out of the header block.
        if (line_num > 15) { header_block_active = 0; }

        # If it is not a SPDX/Copyright line, and it is not a shebang/xml declaration,
        # then we are likely past the header block.
        # This condition is crucial for `in_header_block` to become 0.
        if (line_num > 1 && $0 !~ /^#!/ && $0 !~ /^\<\?xml/ && $0 !~ /SPDX-License-Identifier/ && $0 !~ /James Ross .* FLYING/) {
          header_block_active = 0;
        }

        # If still in header block, and it IS a shebang/xml declaration, always print it and continue.
        if (line_num == 1 && ($0 ~ /^#!/ || $0 ~ /^\<\?xml/)) {
            print;
            next;
        }

        # If it is a SPDX/Copyright line AND we are in the header block, skip it.
        if (($0 ~ /SPDX-License-Identifier/ || $0 ~ /James Ross .* FLYING/) && header_block_active) {
          next;
        }
      }
      print; # Print all other lines not skipped.
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
  
  # Logic to insert header AFTER shebang/xml declaration if present
  if [[ "$first_line" =~ ^#! ]]; then
    echo "$first_line" > "$temp_file"
    echo "$header" >> "$temp_file"
    tail -n +2 "$f" >> "$temp_file"
  elif [[ "$first_line" =~ ^\<\?xml ]]; then
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
  
  local expected_header_block
  expected_header_block=$(get_header_content "$f" "$style")
  
  if check_valid_header "$f" "$expected_header_block"; then
    return 0 # Header is already perfect, nothing to do.
  fi

  # If we reach here, header is either missing or malformed/incorrect.
  if [[ "$CHECK_MODE" -eq 1 ]]; then
    echo "[FAIL] Incorrect or missing SPDX header: $f"
    FAILED_COUNT=$((FAILED_COUNT + 1))
  else
    # Repair mode:
    # 1. Strip any existing (malformed/wrong) header lines from top
    strip_existing_headers "$f"
    # 2. Insert correct header
    insert_header "$f" "$expected_header_block"
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
