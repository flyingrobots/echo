#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

hook_timing_now_ns() {
  if command -v python3 >/dev/null 2>&1; then
    python3 - <<'PY'
import time

print(time.monotonic_ns())
PY
  else
    printf '%s000000000\n' "$(date +%s)"
  fi
}

hook_timing_prepare() {
  DX_HOOK_REPO_ROOT="$1"
  DX_HOOK_NAME="$2"
  DX_HOOK_START_NS="$(hook_timing_now_ns)"
  DX_HOOK_TIMING_RECORDED=0
}

hook_timing_append() {
  local exit_code="${1:-$?}"
  if [[ "${DX_HOOK_TIMING_RECORDED:-0}" == "1" ]]; then
    return 0
  fi
  DX_HOOK_TIMING_RECORDED=1

  local repo_root="${DX_HOOK_REPO_ROOT:-}"
  local hook_name="${DX_HOOK_NAME:-}"
  local start_ns="${DX_HOOK_START_NS:-}"
  if [[ -z "$repo_root" || -z "$hook_name" || -z "$start_ns" ]]; then
    return 0
  fi

  local end_ns elapsed_ns elapsed_ms csv_dir csv_file timestamp_utc
  end_ns="$(hook_timing_now_ns)"
  elapsed_ns=$(( end_ns - start_ns ))
  if (( elapsed_ns < 0 )); then
    elapsed_ns=0
  fi
  elapsed_ms=$(( elapsed_ns / 1000000 ))
  csv_dir="${repo_root}/.dx-debug"
  csv_file="${csv_dir}/${hook_name}-times.csv"
  timestamp_utc="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

  mkdir -p "$csv_dir" 2>/dev/null || return 0
  if [[ ! -f "$csv_file" ]]; then
    printf 'timestamp_utc,elapsed_ms,exit_code,pid\n' >>"$csv_file" 2>/dev/null || return 0
  fi
  printf '%s,%s,%s,%s\n' \
    "$timestamp_utc" \
    "$elapsed_ms" \
    "$exit_code" \
    "$$" >>"$csv_file" 2>/dev/null || true
}
