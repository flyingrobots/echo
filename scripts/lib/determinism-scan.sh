#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

# Shared line-oriented scanner for deterministic-boundary guard scripts.
# Uses ripgrep when available, with a perl/find fallback for minimal CI images.

DET_SCAN_EXTENSIONS=(
  '*.rs'
  '*.toml'
  '*.sh'
  '*.mjs'
  '*.js'
  '*.ts'
  '*.md'
  '*.graphql'
  '*.json'
  '*.yaml'
  '*.yml'
)

DET_SCAN_RG_ARGS=(
  --hidden
  --no-ignore
  --glob '!**/.git/**'
  --glob '!**/target/**'
  --glob '!**/node_modules/**'
  --glob '!**/.clippy.toml'
)

DET_WAIVER_RULES=()
DET_WAIVER_PATHS=()

det_load_waivers() {
  local allowlist="$1"
  DET_WAIVER_RULES=()
  DET_WAIVER_PATHS=()

  [[ -f "$allowlist" ]] || return 0

  local line rule path rest
  while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    [[ "$line" =~ ^[[:space:]]*# ]] && continue
    read -r rule path rest <<< "$line"
    [[ -n "${rule:-}" && -n "${path:-}" ]] || continue
    DET_WAIVER_RULES+=("$rule")
    DET_WAIVER_PATHS+=("$path")
  done < "$allowlist"
}

det_is_waived() {
  local rule="$1"
  local file="$2"
  local i

  for i in "${!DET_WAIVER_RULES[@]}"; do
    if [[ "${DET_WAIVER_RULES[$i]}" == "$rule" && "$file" == ${DET_WAIVER_PATHS[$i]} ]]; then
      return 0
    fi
  done

  return 1
}

det_has_rg() {
  [[ "${DETERMINISM_FORCE_NO_RG:-0}" != "1" ]] && command -v rg >/dev/null 2>&1
}

det_find_files() {
  local paths=("$@")
  local find_expr=()
  local ext

  for ext in "${DET_SCAN_EXTENSIONS[@]}"; do
    if [[ "${#find_expr[@]}" -gt 0 ]]; then
      find_expr+=(-o)
    fi
    find_expr+=(-name "$ext")
  done

  local existing_paths=()
  local path
  for path in "${paths[@]}"; do
    [[ -e "$path" ]] && existing_paths+=("$path")
  done
  [[ "${#existing_paths[@]}" -gt 0 ]] || return 0

  find "${existing_paths[@]}" -type f \
    \( "${find_expr[@]}" \) \
    -not -path '*/.git/*' \
    -not -path '*/target/*' \
    -not -path '*/node_modules/*' \
    -not -name '.clippy.toml' \
    -print0
}

det_filter_matches() {
  local rule="$1"
  local found=1
  local match file

  while IFS= read -r match; do
    [[ -n "$match" ]] || continue
    file="${match%%:*}"
    if det_is_waived "$rule" "$file"; then
      continue
    fi
    printf '%s\n' "$match"
    found=0
  done

  return "$found"
}

det_scan_line() {
  local rule="$1"
  local pattern="$2"
  shift 2
  local paths=("$@")

  if det_has_rg; then
    rg "${DET_SCAN_RG_ARGS[@]}" -n -S "$pattern" "${paths[@]}" 2>/dev/null \
      | det_filter_matches "$rule"
    local pipe_status=("${PIPESTATUS[@]}")
    if [[ "${pipe_status[0]}" -gt 1 ]]; then
      return "${pipe_status[0]}"
    fi
    return "${pipe_status[1]}"
  fi

  if ! command -v perl >/dev/null 2>&1; then
    echo "determinism-scan: missing dependency: rg or perl" >&2
    return 2
  fi

  local found=1
  local file status
  while IFS= read -r -d '' file; do
    if SEARCH_PATTERN="$pattern" perl -ne '
      BEGIN { $found = 0; $pattern = $ENV{"SEARCH_PATTERN"}; }
      if (/$pattern/) { print "$ARGV:$.:$_"; $found = 1; }
      END { exit($found ? 0 : 1); }
    ' "$file" >/dev/null; then
      if ! det_is_waived "$rule" "$file"; then
        found=0
      fi
    else
      status=$?
      if [[ "$status" -gt 1 ]]; then
        return "$status"
      fi
    fi
  done < <(det_find_files "${paths[@]}")

  if [[ "$found" -eq 0 ]]; then
    while IFS= read -r -d '' file; do
      SEARCH_PATTERN="$pattern" perl -ne '
        BEGIN { $pattern = $ENV{"SEARCH_PATTERN"}; }
        if (/$pattern/) { print "$ARGV:$.:$_"; }
      ' "$file"
    done < <(det_find_files "${paths[@]}") | det_filter_matches "$rule"
    return 0
  fi

  return 1
}
