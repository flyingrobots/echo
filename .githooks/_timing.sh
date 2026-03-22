#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

hook_timing_detect_method() {
  if command -v python3 >/dev/null 2>&1; then
    printf '%s\n' "python3_monotonic_ns"
  else
    printf '%s\n' "date_epoch_seconds_as_ns"
  fi
}

hook_timing_fallback_now_ns() {
  printf '%s000000000\n' "$(date +%s)"
}

hook_timing_now_ns() {
  case "${DX_HOOK_TIMING_METHOD:-$(hook_timing_detect_method)}" in
    python3_monotonic_ns)
      if ! command -v python3 >/dev/null 2>&1; then
        if [[ -n "${DX_HOOK_START_NS:-}" ]]; then
          printf '%s\n' "${DX_HOOK_START_NS}"
        else
          hook_timing_fallback_now_ns
        fi
        return 0
      fi

      local monotonic_ns=""
      if monotonic_ns="$(
        python3 - <<'PY' 2>/dev/null
import time

print(time.monotonic_ns())
PY
      )"; then
        printf '%s\n' "$monotonic_ns"
      else
        if [[ -n "${DX_HOOK_START_NS:-}" ]]; then
          printf '%s\n' "${DX_HOOK_START_NS}"
        else
          hook_timing_fallback_now_ns
        fi
      fi
      ;;
    *)
      hook_timing_fallback_now_ns
      ;;
  esac
}

hook_timing_prepare() {
  DX_HOOK_REPO_ROOT="$1"
  DX_HOOK_NAME="$2"
  DX_HOOK_TIMING_METHOD="$(hook_timing_detect_method)"
  DX_HOOK_START_NS="$(hook_timing_now_ns)"
  DX_HOOK_TIMING_RECORDED=0
}

hook_timing_lock_metadata_file() {
  printf '%s\n' "$1/owner"
}

hook_timing_write_lock_metadata() {
  local lock_dir="$1"
  local meta_file
  meta_file="$(hook_timing_lock_metadata_file "$lock_dir")"
  printf 'pid=%s\nstarted_at=%s\n' "$$" "$(date +%s)" >"$meta_file" 2>/dev/null || true
}

hook_timing_lock_is_stale() {
  local lock_dir="$1"
  local meta_file pid="" started_at="" key value now stale_after
  meta_file="$(hook_timing_lock_metadata_file "$lock_dir")"
  stale_after="${DX_HOOK_TIMING_STALE_LOCK_SECS:-30}"

  if [[ ! -f "$meta_file" ]]; then
    return 1
  fi

  while IFS='=' read -r key value; do
    case "$key" in
      pid) pid="$value" ;;
      started_at) started_at="$value" ;;
    esac
  done <"$meta_file"

  if [[ -n "$pid" ]]; then
    if kill -0 "$pid" 2>/dev/null; then
      return 1
    fi
    return 0
  fi

  if [[ "$started_at" =~ ^[0-9]+$ ]]; then
    now="$(date +%s)"
    if (( now >= started_at && now - started_at >= stale_after )); then
      return 0
    fi
  fi

  return 1
}

hook_timing_reap_stale_lock() {
  local lock_dir="$1"
  if ! hook_timing_lock_is_stale "$lock_dir"; then
    return 1
  fi
  rm -rf "$lock_dir" 2>/dev/null || true
  return 0
}

hook_timing_release_lock() {
  local lock_dir="$1"
  rm -f "$(hook_timing_lock_metadata_file "$lock_dir")" 2>/dev/null || true
  rmdir "$lock_dir" 2>/dev/null || true
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
  local lock_dir lock_acquired=0 attempts=0
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
  lock_dir="${csv_file}.lock"
  while (( attempts < 100 )); do
    if mkdir "$lock_dir" 2>/dev/null; then
      lock_acquired=1
      hook_timing_write_lock_metadata "$lock_dir"
      break
    fi
    hook_timing_reap_stale_lock "$lock_dir" || true
    attempts=$(( attempts + 1 ))
    sleep 0.01
  done
  if [[ "$lock_acquired" != "1" ]]; then
    return 0
  fi

  if [[ ! -s "$csv_file" ]]; then
    printf 'timestamp_utc,elapsed_ms,exit_code,pid\n' >>"$csv_file" 2>/dev/null || {
      hook_timing_release_lock "$lock_dir"
      return 0
    }
  fi
  printf '%s,%s,%s,%s\n' \
    "$timestamp_utc" \
    "$elapsed_ms" \
    "$exit_code" \
    "$$" >>"$csv_file" 2>/dev/null || true
  hook_timing_release_lock "$lock_dir"
}
