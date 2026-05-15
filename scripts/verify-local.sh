#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#
# Local verification entrypoint used by git hooks and explicit make targets.
# The goal is to keep the edit loop fast while still escalating to the full
# workspace gates for determinism-critical, CI, and build-system changes.
set -euo pipefail

MODE="${1:-auto}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
VERIFY_REPORT_TIMING="${VERIFY_REPORT_TIMING:-1}"

cd "$REPO_ROOT"

PINNED_FROM_FILE=$(awk -F '"' '/^channel/ {print $2}' rust-toolchain.toml 2>/dev/null || echo "")
PINNED="${PINNED:-${PINNED_FROM_FILE:-1.90.0}}"
VERIFY_FORCE="${VERIFY_FORCE:-0}"
STAMP_DIR="${VERIFY_STAMP_DIR:-.git/verify-local}"
VERIFY_USE_NEXTEST="${VERIFY_USE_NEXTEST:-0}"
VERIFY_LANE_MODE="${VERIFY_LANE_MODE:-parallel}"
VERIFY_LANE_ROOT="${VERIFY_LANE_ROOT:-target/verify-lanes}"
VERIFY_TIMING_FILE="${VERIFY_TIMING_FILE:-$STAMP_DIR/timing.jsonl}"
VERIFY_RUN_CACHE_STATE="${VERIFY_RUN_CACHE_STATE:-fresh}"
VERIFY_CLASSIFICATION="${VERIFY_CLASSIFICATION:-unknown}"
SECONDS=0

format_elapsed() {
  local total_seconds="$1"
  local hours=$((total_seconds / 3600))
  local minutes=$(((total_seconds % 3600) / 60))
  local seconds=$((total_seconds % 60))

  if [[ $hours -gt 0 ]]; then
    printf '%dh%02dm%02ds' "$hours" "$minutes" "$seconds"
    return
  fi

  if [[ $minutes -gt 0 ]]; then
    printf '%dm%02ds' "$minutes" "$seconds"
    return
  fi

  printf '%ds' "$seconds"
}

utc_timestamp() {
  date -u +%Y-%m-%dT%H:%M:%SZ
}

now_seconds() {
  date +%s
}

json_escape() {
  local value="$1"
  value="${value//\\/\\\\}"
  value="${value//\"/\\\"}"
  value="${value//$'\n'/\\n}"
  value="${value//$'\r'/\\r}"
  value="${value//$'\t'/\\t}"
  printf '%s' "$value"
}

worktree_tree() (
  set -euo pipefail
  local tmp_index
  tmp_index="$(mktemp "${TMPDIR:-/tmp}/verify-local-index.XXXXXX")"
  trap 'rm -f "$tmp_index"' EXIT
  rm -f "$tmp_index"
  GIT_INDEX_FILE="$tmp_index" git read-tree HEAD
  GIT_INDEX_FILE="$tmp_index" git add -A -- .
  GIT_INDEX_FILE="$tmp_index" git write-tree
)

timing_lock_metadata_file() {
  printf '%s\n' "$1/owner"
}

timing_write_lock_metadata() {
  local lock_dir="$1"
  local meta_file
  meta_file="$(timing_lock_metadata_file "$lock_dir")"
  printf 'pid=%s\nstarted_at=%s\n' "$$" "$(now_seconds)" >"$meta_file" 2>/dev/null || true
}

timing_lock_is_stale() {
  local lock_dir="$1"
  local meta_file pid="" started_at="" key value now stale_after
  meta_file="$(timing_lock_metadata_file "$lock_dir")"
  stale_after="${VERIFY_TIMING_STALE_LOCK_SECS:-30}"

  if [[ ! -f "$meta_file" ]]; then
    return 1
  fi

  while IFS='=' read -r key value; do
    case "$key" in
      pid) pid="$value" ;;
      started_at) started_at="$value" ;;
    esac
  done <"$meta_file"

  if [[ -n "$pid" ]] && ! kill -0 "$pid" 2>/dev/null; then
    return 0
  fi

  if [[ "$started_at" =~ ^[0-9]+$ ]]; then
    now="$(now_seconds)"
    if (( now >= started_at && now - started_at >= stale_after )); then
      return 0
    fi
  fi

  return 1
}

timing_reap_stale_lock() {
  local lock_dir="$1"
  if ! timing_lock_is_stale "$lock_dir"; then
    return 1
  fi
  rm -rf "$lock_dir" 2>/dev/null || true
  return 0
}

timing_release_lock() {
  local lock_dir="$1"
  rm -f "$(timing_lock_metadata_file "$lock_dir")" 2>/dev/null || true
  rmdir "$lock_dir" 2>/dev/null || true
}

append_timing_record() {
  local record_type="$1"
  local name="$2"
  local elapsed_seconds="$3"
  local exit_status="$4"
  local timing_dir lock_dir lock_acquired=0 attempts=0

  timing_dir="$(dirname "$VERIFY_TIMING_FILE")"
  mkdir -p "$timing_dir" 2>/dev/null || return 0
  lock_dir="${VERIFY_TIMING_FILE}.lock"
  while (( attempts < 100 )); do
    if mkdir "$lock_dir" 2>/dev/null; then
      lock_acquired=1
      timing_write_lock_metadata "$lock_dir"
      break
    fi
    timing_reap_stale_lock "$lock_dir" || true
    attempts=$(( attempts + 1 ))
    sleep 0.01
  done
  if [[ "$lock_acquired" != "1" ]]; then
    return 0
  fi

  printf \
    '{"ts":"%s","record_type":"%s","mode":"%s","context":"%s","classification":"%s","name":"%s","elapsed_seconds":%s,"exit_status":%s,"cache":"%s","subject":"%s"}\n' \
    "$(json_escape "$(utc_timestamp)")" \
    "$(json_escape "$record_type")" \
    "$(json_escape "$MODE")" \
    "$(json_escape "${VERIFY_MODE_CONTEXT:-unknown}")" \
    "$(json_escape "${VERIFY_CLASSIFICATION:-unknown}")" \
    "$(json_escape "$name")" \
    "$elapsed_seconds" \
    "$exit_status" \
    "$(json_escape "${VERIFY_RUN_CACHE_STATE:-fresh}")" \
    "$(json_escape "${VERIFY_STAMP_SUBJECT:-unknown}")" >>"$VERIFY_TIMING_FILE" || true

  timing_release_lock "$lock_dir"
}

report_lane_timing() {
  local lane="$1"
  local elapsed_seconds="$2"
  local exit_status="$3"
  local lane_status="pass"

  if [[ "$exit_status" -ne 0 ]]; then
    lane_status="fail"
  fi

  echo "[verify-local][timing] lane=${lane} status=${lane_status} elapsed=$(format_elapsed "$elapsed_seconds")"
}

report_timing() {
  local status="$1"
  if [[ "$VERIFY_REPORT_TIMING" != "1" ]]; then
    return
  fi

  local elapsed
  elapsed="$(format_elapsed "$SECONDS")"
  append_timing_record "run" "$MODE" "$SECONDS" "$status"
  if [[ "$status" -eq 0 ]]; then
    echo "[verify-local] completed in ${elapsed} (${VERIFY_RUN_CACHE_STATE})"
  else
    echo "[verify-local] failed after ${elapsed} (${VERIFY_RUN_CACHE_STATE})" >&2
  fi
}

on_exit() {
  local status="$?"
  trap - EXIT
  report_timing "$status"
  exit "$status"
}

trap on_exit EXIT

sha256_file() {
  local file="$1"
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file" | awk '{print $1}'
  elif command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{print $1}'
  elif command -v python3 >/dev/null 2>&1; then
    python3 - "$file" <<'PY'
import hashlib
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
print(hashlib.sha256(path.read_bytes()).hexdigest())
PY
  else
    echo "verify-local: missing sha256 tool (need shasum, sha256sum, or python3)" >&2
    exit 1
  fi
}

SCRIPT_HASH="$(sha256_file "$0")"

readonly FULL_CRITICAL_PREFIXES=(
  "crates/warp-core/"
  "crates/warp-geom/"
  "crates/warp-wasm/"
  "crates/echo-wasm-abi/"
  "crates/echo-scene-port/"
  "crates/echo-scene-codec/"
  "crates/echo-graph/"
  "crates/echo-ttd/"
  "crates/echo-dind-harness/"
  "crates/echo-dind-tests/"
  "crates/ttd-browser/"
  "crates/ttd-protocol-rs/"
  ".github/workflows/"
  ".githooks/"
  "scripts/"
  "xtask/"
)

readonly FULL_CRITICAL_EXACT=(
  "Cargo.toml"
  "Cargo.lock"
  "rust-toolchain.toml"
  "package.json"
  "pnpm-lock.yaml"
  "pnpm-workspace.yaml"
  "deny.toml"
  "audit.toml"
  "det-policy.yaml"
  "Makefile"
)

readonly FULL_TOOLING_PREFIXES=(
  ".github/workflows/"
  ".githooks/"
  "scripts/"
)

readonly FULL_TOOLING_EXACT=(
  "Makefile"
  "package.json"
  "pnpm-lock.yaml"
  "pnpm-workspace.yaml"
  "deny.toml"
  "audit.toml"
  "det-policy.yaml"
)

readonly FULL_BROAD_RUST_EXACT=(
  "Cargo.toml"
  "Cargo.lock"
  "rust-toolchain.toml"
)

readonly FULL_LOCAL_PACKAGES=(
  "warp-core"
  "warp-geom"
  "warp-wasm"
  "echo-wasm-abi"
  "echo-scene-port"
  "echo-scene-codec"
  "echo-graph"
  "echo-ttd"
  "echo-dind-harness"
  "echo-dind-tests"
  "ttd-browser"
  "ttd-protocol-rs"
  "xtask"
)

readonly FULL_LOCAL_TEST_PACKAGES=(
  "warp-geom"
  "echo-graph"
  "echo-scene-port"
  "echo-scene-codec"
  "echo-ttd"
  "echo-dind-harness"
  "echo-dind-tests"
  "ttd-browser"
)

readonly FULL_LOCAL_CLIPPY_CORE_PACKAGES=(
  "warp-core"
  "warp-geom"
  "warp-wasm"
  "echo-wasm-abi"
)

readonly FULL_LOCAL_CLIPPY_SUPPORT_PACKAGES=(
  "echo-scene-port"
  "echo-scene-codec"
  "echo-graph"
  "echo-ttd"
  "echo-dind-harness"
  "echo-dind-tests"
  "ttd-browser"
  "ttd-protocol-rs"
)

readonly FULL_LOCAL_CLIPPY_BIN_ONLY_PACKAGES=(
  "xtask"
)

readonly FULL_LOCAL_RUSTDOC_PACKAGES=(
  "warp-core"
  "warp-geom"
  "warp-wasm"
)

readonly FAST_CLIPPY_LIB_ONLY_PACKAGES=(
  "warp-core"
  "warp-wasm"
  "ttd-browser"
  "echo-dind-harness"
  "echo-dind-tests"
)

FULL_SCOPE_MODE=""
FULL_SCOPE_HAS_TOOLING=0
FULL_SCOPE_SELECTED_CRATES=()
FULL_SCOPE_CLIPPY_CORE_PACKAGES=()
FULL_SCOPE_CLIPPY_SUPPORT_PACKAGES=()
FULL_SCOPE_CLIPPY_BIN_ONLY_PACKAGES=()
FULL_SCOPE_TEST_SUPPORT_PACKAGES=()
FULL_SCOPE_RUSTDOC_PACKAGES=()
FULL_SCOPE_RUN_WARP_CORE_SMOKE=0
FULL_SCOPE_WARP_WASM_TEST_MODE="none"
FULL_SCOPE_ECHO_WASM_ABI_RUN_LIB=0
FULL_SCOPE_ECHO_WASM_ABI_EXTRA_TESTS=()
FULL_SCOPE_WARP_CORE_EXTRA_TESTS=()
FULL_SCOPE_WARP_CORE_RUN_PRNG=0

ensure_command() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "[verify-local] missing dependency: $cmd" >&2
    exit 1
  fi
}

ensure_toolchain() {
  ensure_command cargo
  ensure_command rustup
  if ! rustup toolchain list | grep -qE "^${PINNED}(-|$)"; then
    echo "[verify-local] missing toolchain: $PINNED" >&2
    echo "[verify-local] Run: rustup toolchain install $PINNED" >&2
    exit 1
  fi
}

use_nextest() {
  [[ "$VERIFY_USE_NEXTEST" == "1" ]] && command -v cargo-nextest >/dev/null 2>&1
}

list_changed_branch_files() {
  if git rev-parse --verify '@{upstream}' >/dev/null 2>&1; then
    git diff --name-only --diff-filter=ACMRTUXBD '@{upstream}...HEAD'
    return
  fi

  local candidate
  local merge_base
  for candidate in origin/main main origin/master master; do
    if git rev-parse --verify "$candidate" >/dev/null 2>&1; then
      merge_base="$(git merge-base HEAD "$candidate")"
      git diff --name-only --diff-filter=ACMRTUXBD "${merge_base}...HEAD"
      return
    fi
  done

  git diff-tree --root --no-commit-id --name-only -r --diff-filter=ACMRTUXBD HEAD
}

list_changed_index_files() {
  git diff --cached --name-only --diff-filter=ACMRTUXBD
}

list_changed_worktree_files() {
  {
    git diff --name-only --diff-filter=ACMRTUXBD HEAD
    git ls-files --others --exclude-standard
  } | awk 'NF' | sort -u
}

list_changed_full_files() {
  {
    list_changed_branch_files
    list_changed_worktree_files
  } | awk 'NF' | sort -u
}

mode_context() {
  case "$1" in
    pre-commit|detect-pre-commit)
      printf 'pre-commit\n'
      ;;
    fast|ultra-fast)
      printf 'working-tree\n'
      ;;
    *)
      printf '%s\n' "$1"
      ;;
  esac
}

list_changed_files() {
  local context="$1"

  if [[ -n "${VERIFY_CHANGED_FILES_FILE:-}" ]]; then
    cat "$VERIFY_CHANGED_FILES_FILE"
    return
  fi

  if [[ "$context" == "pre-commit" ]]; then
    list_changed_index_files
    return
  fi

  if [[ "$context" == "working-tree" ]]; then
    list_changed_worktree_files
    return
  fi

  if [[ "$context" == "full" ]]; then
    list_changed_full_files
    return
  fi

  list_changed_branch_files
}

is_full_path() {
  local file="$1"
  local prefix
  for prefix in "${FULL_CRITICAL_PREFIXES[@]}"; do
    if [[ "$file" == "$prefix"* ]]; then
      return 0
    fi
  done
  local exact
  for exact in "${FULL_CRITICAL_EXACT[@]}"; do
    if [[ "$file" == "$exact" ]]; then
      return 0
    fi
  done
  return 1
}

is_docs_only_path() {
  local file="$1"
  [[ "$file" == docs/* || "$file" == *.md ]]
}

array_contains() {
  local needle="$1"
  shift
  local item
  for item in "$@"; do
    if [[ "$item" == "$needle" ]]; then
      return 0
    fi
  done
  return 1
}

append_unique() {
  local value="$1"
  local array_name="$2"
  local -n array_ref="$array_name"
  if ! array_contains "$value" ${array_ref[@]+"${array_ref[@]}"}; then
    array_ref+=("$value")
  fi
}

is_tooling_full_path() {
  local file="$1"
  local prefix
  for prefix in "${FULL_TOOLING_PREFIXES[@]}"; do
    if [[ "$file" == "$prefix"* ]]; then
      return 0
    fi
  done
  local exact
  for exact in "${FULL_TOOLING_EXACT[@]}"; do
    if [[ "$file" == "$exact" ]]; then
      return 0
    fi
  done
  return 1
}

is_broad_rust_full_path() {
  local file="$1"
  local exact
  for exact in "${FULL_BROAD_RUST_EXACT[@]}"; do
    if [[ "$file" == "$exact" ]]; then
      return 0
    fi
  done
  return 1
}

classify_change_set() {
  local had_files=0
  local classification="docs"
  local file
  while IFS= read -r file; do
    [[ -z "$file" ]] && continue
    had_files=1
    if is_full_path "$file"; then
      echo "full"
      return
    fi
    if is_docs_only_path "$file"; then
      continue
    fi
    classification="reduced"
  done <<< "${CHANGED_FILES}"

  if [[ $had_files -eq 0 ]]; then
    echo "docs"
  else
    echo "$classification"
  fi
}

list_changed_crates() {
  printf '%s\n' "$CHANGED_FILES" | sed -n 's#^crates/\([^/]*\)/.*#\1#p' | sort -u
}

list_changed_rust_crates() {
  local file crate
  while IFS= read -r file; do
    [[ -z "$file" ]] && continue
    case "$file" in
      crates/*/Cargo.toml|crates/*/build.rs|crates/*/src/*|crates/*/tests/*)
        crate="$(printf '%s\n' "$file" | sed -n 's#^crates/\([^/]*\)/.*#\1#p')"
        [[ -z "$crate" ]] && continue
        printf '%s\n' "$crate"
        ;;
    esac
  done <<< "${CHANGED_FILES}" | sort -u
}

list_changed_tooling_shell_files() {
  local file
  while IFS= read -r file; do
    [[ -z "$file" ]] && continue
    case "$file" in
      .githooks/*|scripts/*.sh|scripts/hooks/*|tests/hooks/*.sh)
        if is_shell_tooling_file "$file"; then
          printf '%s\n' "$file"
        fi
        ;;
    esac
  done <<< "${CHANGED_FILES}" | sort -u
}

is_shell_tooling_file() {
  local file="$1"
  [[ -f "$file" ]] || return 1
  case "$file" in
    *.sh) return 0 ;;
  esac
  local first_line=""
  IFS= read -r first_line < "$file" || true
  [[ "$first_line" =~ ^#!.*(^|[[:space:]/])(ba|z)?sh([[:space:]]|$) ]]
}

list_changed_critical_crates() {
  local file crate
  while IFS= read -r file; do
    [[ -z "$file" ]] && continue
    case "$file" in
      crates/*/Cargo.toml|crates/*/build.rs|crates/*/src/*|crates/*/tests/*)
        crate="$(printf '%s\n' "$file" | sed -n 's#^crates/\([^/]*\)/.*#\1#p')"
        [[ -z "$crate" ]] && continue
        if array_contains "$crate" ${FULL_LOCAL_PACKAGES[@]+"${FULL_LOCAL_PACKAGES[@]}"}; then
          printf '%s\n' "$crate"
        fi
        ;;
    esac
  done <<< "${CHANGED_FILES}" | sort -u
}

stamp_suite_for_classification() {
  local classification="$1"

  case "$classification" in
    docs|reduced|full)
      printf '%s\n' "$classification"
      ;;
    *)
      echo "verify-local: unknown stamp suite classification: $classification" >&2
      exit 1
      ;;
  esac
}

stamp_context_for_suite() {
  local suite="$1"

  if [[ "$VERIFY_MODE_CONTEXT" == "pre-commit" ]]; then
    printf 'pre-commit\n'
    return
  fi

  case "$suite" in
    full)
      printf 'full\n'
      ;;
    docs|reduced)
      printf '%s\n' "$VERIFY_MODE_CONTEXT"
      ;;
    *)
      echo "verify-local: unknown stamp context suite: $suite" >&2
      exit 1
      ;;
  esac
}

stamp_key() {
  local suite="$1"
  printf '%s-%s-%s-%s-%s' \
    "$suite" \
    "$PINNED" \
    "$(stamp_context_for_suite "$suite")" \
    "$VERIFY_STAMP_SUBJECT" \
    "$SCRIPT_HASH"
}

stamp_path() {
  local suite="$1"
  printf '%s/%s.ok' "$STAMP_DIR" "$(stamp_key "$suite")"
}

write_stamp() {
  local suite="$1"
  local path
  path="$(stamp_path "$suite")"
  mkdir -p "$STAMP_DIR"
  cat >"$path" <<EOF
suite=$suite
head=$(git rev-parse HEAD)
subject=$VERIFY_STAMP_SUBJECT
toolchain=$PINNED
script_hash=$SCRIPT_HASH
timestamp=$(utc_timestamp)
EOF
}

should_skip_via_stamp() {
  local suite="$1"
  if [[ "$VERIFY_FORCE" == "1" ]]; then
    return 1
  fi
  [[ -f "$(stamp_path "$suite")" ]]
}

run_docs_lint() {
  local discovered_md_files=()
  local md_files=()
  local md_file
  local should_validate_runtime_schema=0

  mapfile -t discovered_md_files < <(printf '%s\n' "$CHANGED_FILES" | awk '/\.md$/ {print}')
  for md_file in "${discovered_md_files[@]}"; do
    if [[ -f "$md_file" ]]; then
      md_files+=("$md_file")
    fi
  done

  while IFS= read -r changed_file; do
    [[ -z "$changed_file" ]] && continue
    case "$changed_file" in
      schemas/runtime/*.graphql|scripts/validate-runtime-schema-fragments.mjs|tests/hooks/test_runtime_schema_validation.sh)
        should_validate_runtime_schema=1
        ;;
    esac
  done <<< "${CHANGED_FILES}"

  if [[ ${#md_files[@]} -eq 0 && "$should_validate_runtime_schema" -eq 0 ]]; then
    return
  fi

  if [[ ${#md_files[@]} -ne 0 ]]; then
    if ! command -v npx >/dev/null 2>&1; then
      echo "[verify-local] npx not found; skipping markdown format check for ${#md_files[@]} changed markdown files" >&2
    else
      echo "[verify-local] prettier --check (${#md_files[@]} markdown files)"
      npx prettier --check "${md_files[@]}"
    fi
  fi

  if [[ "$should_validate_runtime_schema" -eq 1 ]]; then
    if ! command -v pnpm >/dev/null 2>&1; then
      echo "[verify-local] pnpm not found; cannot run runtime schema validation" >&2
      return 1
    fi
    echo "[verify-local] runtime schema validation"
    pnpm schema:runtime:check
  fi
}

run_targeted_checks() {
  local crates=("$@")
  local crate
  local rustdoc_crates=()

  if [[ ${#crates[@]} -eq 0 ]]; then
    echo "[verify-local] no changed crates detected; running docs-only checks"
    run_docs_lint
    return
  fi

  ensure_toolchain
  echo "[verify-local] cargo fmt --all -- --check"
  cargo +"$PINNED" fmt --all -- --check

  run_crate_lint_and_check targeted "${crates[@]}"

  for crate in "${FULL_LOCAL_RUSTDOC_PACKAGES[@]}"; do
    if printf '%s\n' "${crates[@]}" | grep -qx "$crate"; then
      rustdoc_crates+=("$crate")
    fi
  done

  for crate in "${rustdoc_crates[@]}"; do
    echo "[verify-local] rustdoc warnings gate (${crate})"
    RUSTDOCFLAGS="-D warnings" cargo +"$PINNED" doc -p "$crate" --no-deps
  done

  for crate in "${crates[@]}"; do
    if [[ ! -f "crates/${crate}/Cargo.toml" ]]; then
      continue
    fi
    local -a test_args=()
    mapfile -t test_args < <(targeted_test_args_for_crate "$crate")
    if use_nextest; then
      echo "[verify-local] cargo nextest run -p ${crate} ${test_args[*]}"
      cargo +"$PINNED" nextest run -p "$crate" "${test_args[@]}"
    else
      echo "[verify-local] cargo test -p ${crate} ${test_args[*]}"
      cargo +"$PINNED" test -p "$crate" "${test_args[@]}"
    fi
  done

  run_docs_lint
}

run_crate_lint_and_check() {
  local scope="$1"
  shift
  local crates=("$@")
  local crate

  for crate in "${crates[@]}"; do
    if [[ ! -f "crates/${crate}/Cargo.toml" ]]; then
      echo "[verify-local] skipping ${crate}: missing crates/${crate}/Cargo.toml" >&2
      continue
    fi
    local -a clippy_args=()
    mapfile -t clippy_args < <(clippy_target_args_for_scope "$crate" "$scope")
    echo "[verify-local] cargo clippy -p ${crate} ${clippy_args[*]}"
    cargo +"$PINNED" clippy -p "$crate" "${clippy_args[@]}" -- -D warnings -D missing_docs
    echo "[verify-local] cargo check -p ${crate}"
    cargo +"$PINNED" check -p "$crate" --quiet
  done
}

run_pre_commit_checks() {
  mapfile -t changed_crates < <(list_changed_rust_crates)
  if [[ ${#changed_crates[@]} -eq 0 ]]; then
    echo "[verify-local] pre-commit: no staged Rust crates detected"
    return
  fi

  ensure_toolchain
  echo "[verify-local] pre-commit verification for staged crates: ${changed_crates[*]}"
  run_crate_lint_and_check pre-commit "${changed_crates[@]}"
}

run_timed_step() {
  local lane="$1"
  local step_func="$2"
  shift 2

  local started_at
  started_at="$(now_seconds)"
  local rc=0

  set +e
  ( "$step_func" "$@" )
  rc=$?
  set -e

  local finished_at
  finished_at="$(now_seconds)"
  local elapsed_seconds=$((finished_at - started_at))

  report_lane_timing "$lane" "$elapsed_seconds" "$rc"
  append_timing_record "lane" "$lane" "$elapsed_seconds" "$rc"

  if [[ "$rc" -ne 0 ]]; then
    exit "$rc"
  fi
}

package_args() {
  local pkg
  for pkg in "$@"; do
    printf '%s\n' "-p" "$pkg"
  done
}

lane_target_dir() {
  local lane="$1"
  printf '%s/%s' "$VERIFY_LANE_ROOT" "$lane"
}

lane_cargo() {
  local lane="$1"
  shift
  mkdir -p "$VERIFY_LANE_ROOT"
  CARGO_TARGET_DIR="$(lane_target_dir "$lane")" cargo +"$PINNED" "$@"
}

should_run_parallel_lanes() {
  case "$VERIFY_LANE_MODE" in
    parallel)
      return 0
      ;;
    sequential|serial)
      return 1
      ;;
    *)
      echo "[verify-local] invalid VERIFY_LANE_MODE: $VERIFY_LANE_MODE" >&2
      exit 1
      ;;
  esac
}

run_parallel_lanes() {
  local suite="$1"
  shift

  local logdir
  logdir="$(mktemp -d "${TMPDIR:-/tmp}/verify-local-${suite}.XXXXXX")"
  local -a lane_names=()
  local -a lane_funcs=()
  local -a lane_pids=()
  local i

  cleanup_parallel_lanes() {
    local pid
    for pid in "${lane_pids[@]}"; do
      kill "$pid" 2>/dev/null || true
    done
    for pid in "${lane_pids[@]}"; do
      wait "$pid" 2>/dev/null || true
    done
    rm -rf "$logdir"
  }

  trap 'cleanup_parallel_lanes; trap - INT TERM; exit 130' INT TERM

  while [[ $# -gt 0 ]]; do
    lane_names+=("$1")
    lane_funcs+=("$2")
    shift 2
  done

  echo "[verify-local] ${suite}: launching ${#lane_names[@]} local lanes"
  echo "[verify-local] ${suite}: lanes=${lane_names[*]}"
  for i in "${!lane_names[@]}"; do
    (
      # Deliberately omit -e: we capture the lane exit explicitly in rc below.
      set -uo pipefail
      started_at="$(now_seconds)"
      rc=0
      ( "${lane_funcs[$i]}" ) || rc=$?
      finished_at="$(now_seconds)"
      printf '%s\n' "$((finished_at - started_at))" >"${logdir}/${lane_names[$i]}.elapsed"
      exit "$rc"
    ) >"${logdir}/${lane_names[$i]}.log" 2>&1 &
    lane_pids+=("$!")
  done

  local failed=0
  local rc
  local elapsed_seconds
  set +e
  for i in "${!lane_names[@]}"; do
    wait "${lane_pids[$i]}"
    rc=$?
    elapsed_seconds=0
    if [[ -f "${logdir}/${lane_names[$i]}.elapsed" ]]; then
      elapsed_seconds="$(<"${logdir}/${lane_names[$i]}.elapsed")"
    fi
    report_lane_timing "${lane_names[$i]}" "$elapsed_seconds" "$rc"
    append_timing_record "lane" "${lane_names[$i]}" "$elapsed_seconds" "$rc"
    if [[ $rc -ne 0 ]]; then
      failed=1
      echo "[verify-local] lane failed: ${lane_names[$i]}" >&2
    fi
  done
  set -e

  if [[ $failed -ne 0 ]]; then
    for i in "${!lane_names[@]}"; do
      local logfile="${logdir}/${lane_names[$i]}.log"
      if [[ ! -s "$logfile" ]]; then
        continue
      fi
      echo
      echo "--- ${lane_names[$i]} ---" >&2
      cat "$logfile" >&2
    done
    trap - INT TERM
    rm -rf "$logdir"
    exit 1
  fi

  trap - INT TERM
  rm -rf "$logdir"
}

crate_supports_lib_target() {
  local crate="$1"
  local crate_dir="crates/${crate}"
  local manifest="${crate_dir}/Cargo.toml"

  if [[ ! -f "$manifest" ]]; then
    return 1
  fi

  [[ -f "${crate_dir}/src/lib.rs" ]] && return 0
  grep -Eq '^\[lib\]' "$manifest"
}

crate_supports_bin_target() {
  local crate="$1"
  local crate_dir="crates/${crate}"
  local manifest="${crate_dir}/Cargo.toml"

  if [[ ! -f "$manifest" ]]; then
    return 1
  fi

  [[ -f "${crate_dir}/src/main.rs" ]] && return 0
  grep -Eq '^\[\[bin\]\]' "$manifest"
}

crate_is_fast_clippy_lib_only() {
  local crate="$1"
  local candidate
  for candidate in "${FAST_CLIPPY_LIB_ONLY_PACKAGES[@]}"; do
    if [[ "$crate" == "$candidate" ]]; then
      return 0
    fi
  done
  return 1
}

clippy_target_args_for_scope() {
  local crate="$1"
  local scope="$2"

  if [[ "$crate" == "xtask" ]]; then
    printf '%s\n' "--bins"
    return
  fi

  if [[ "$scope" == "full" ]]; then
    printf '%s\n' "--all-targets"
    return
  fi

  if crate_supports_lib_target "$crate"; then
    printf '%s\n' "--lib"
  elif crate_supports_bin_target "$crate"; then
    printf '%s\n' "--bins"
  else
    printf '%s\n' "--all-targets"
    return
  fi

  if crate_supports_lib_target "$crate" && ! crate_is_fast_clippy_lib_only "$crate"; then
    printf '%s\n' "--tests"
  fi
}

targeted_test_args_for_crate() {
  local crate="$1"

  if [[ "$crate" == "xtask" ]]; then
    printf '%s\n' "--bins"
    return
  fi

  if crate_supports_lib_target "$crate"; then
    printf '%s\n' "--lib"
  elif crate_supports_bin_target "$crate"; then
    printf '%s\n' "--bins"
  fi
  printf '%s\n' "--tests"
}

filter_package_set_by_selection() {
  local selection_name="$1"
  local candidate_name="$2"
  local pkg
  local -n selection_ref="$selection_name"
  local -n candidate_ref="$candidate_name"

  for pkg in "${candidate_ref[@]}"; do
    if array_contains "$pkg" ${selection_ref[@]+"${selection_ref[@]}"}; then
      printf '%s\n' "$pkg"
    fi
  done
}

prepare_warp_core_scope() {
  FULL_SCOPE_WARP_CORE_EXTRA_TESTS=()
  FULL_SCOPE_WARP_CORE_RUN_PRNG=0

  local file
  while IFS= read -r file; do
    [[ -z "$file" ]] && continue
    case "$file" in
      crates/warp-core/tests/*.rs)
        append_unique "$(basename "$file" .rs)" FULL_SCOPE_WARP_CORE_EXTRA_TESTS
        ;;
      crates/warp-core/src/optic_artifact.rs)
        append_unique "optic_artifact_registry_tests" FULL_SCOPE_WARP_CORE_EXTRA_TESTS
        append_unique "optic_invocation_admission_tests" FULL_SCOPE_WARP_CORE_EXTRA_TESTS
        append_unique "causal_fact_publication_tests" FULL_SCOPE_WARP_CORE_EXTRA_TESTS
        append_unique "capability_grant_intent_tests" FULL_SCOPE_WARP_CORE_EXTRA_TESTS
        ;;
      crates/warp-core/src/causal_facts.rs)
        append_unique "causal_fact_publication_tests" FULL_SCOPE_WARP_CORE_EXTRA_TESTS
        append_unique "optic_artifact_registry_tests" FULL_SCOPE_WARP_CORE_EXTRA_TESTS
        append_unique "optic_invocation_admission_tests" FULL_SCOPE_WARP_CORE_EXTRA_TESTS
        ;;
      crates/warp-core/src/coordinator.rs|\
      crates/warp-core/src/engine_impl.rs|\
      crates/warp-core/src/head.rs|\
      crates/warp-core/src/head_inbox.rs|\
      crates/warp-core/src/worldline_state.rs|\
      crates/warp-core/src/worldline_registry.rs|\
      crates/warp-core/src/runtime*.rs)
        append_unique "inbox" FULL_SCOPE_WARP_CORE_EXTRA_TESTS
        ;;
      crates/warp-core/src/playback.rs)
        append_unique "playback_cursor_tests" FULL_SCOPE_WARP_CORE_EXTRA_TESTS
        append_unique "outputs_playback_tests" FULL_SCOPE_WARP_CORE_EXTRA_TESTS
        ;;
      crates/warp-core/src/math/prng.rs)
        FULL_SCOPE_WARP_CORE_RUN_PRNG=1
        ;;
    esac
  done <<< "${CHANGED_FILES}"
}

prepare_warp_wasm_scope() {
  FULL_SCOPE_WARP_WASM_TEST_MODE="none"

  if [[ "$FULL_SCOPE_MODE" == "broad-rust" ]]; then
    FULL_SCOPE_WARP_WASM_TEST_MODE="engine-lib"
    return
  fi

  local file
  while IFS= read -r file; do
    [[ -z "$file" ]] && continue
    case "$file" in
      crates/warp-wasm/Cargo.toml|crates/warp-wasm/src/warp_kernel.rs)
        FULL_SCOPE_WARP_WASM_TEST_MODE="engine-lib"
        return
        ;;
      crates/warp-wasm/src/lib.rs)
        if [[ "$FULL_SCOPE_WARP_WASM_TEST_MODE" == "none" ]]; then
          FULL_SCOPE_WARP_WASM_TEST_MODE="plain-lib"
        fi
        ;;
    esac
  done <<< "${CHANGED_FILES}"
}

prepare_echo_wasm_abi_scope() {
  FULL_SCOPE_ECHO_WASM_ABI_RUN_LIB=0
  FULL_SCOPE_ECHO_WASM_ABI_EXTRA_TESTS=()

  if [[ "$FULL_SCOPE_MODE" == "broad-rust" ]]; then
    FULL_SCOPE_ECHO_WASM_ABI_RUN_LIB=1
    return
  fi

  local file test_name
  while IFS= read -r file; do
    [[ -z "$file" ]] && continue
    case "$file" in
      crates/echo-wasm-abi/Cargo.toml|\
      crates/echo-wasm-abi/src/lib.rs|\
      crates/echo-wasm-abi/src/kernel_port.rs|\
      crates/echo-wasm-abi/src/eintlog.rs|\
      crates/echo-wasm-abi/src/ttd.rs)
        FULL_SCOPE_ECHO_WASM_ABI_RUN_LIB=1
        ;;
      crates/echo-wasm-abi/src/canonical.rs)
        append_unique "canonical_vectors" FULL_SCOPE_ECHO_WASM_ABI_EXTRA_TESTS
        append_unique "non_canonical_floats" FULL_SCOPE_ECHO_WASM_ABI_EXTRA_TESTS
        ;;
      crates/echo-wasm-abi/src/codec.rs)
        append_unique "codec" FULL_SCOPE_ECHO_WASM_ABI_EXTRA_TESTS
        ;;
      crates/echo-wasm-abi/tests/*.rs)
        test_name="$(basename "$file" .rs)"
        append_unique "$test_name" FULL_SCOPE_ECHO_WASM_ABI_EXTRA_TESTS
        ;;
    esac
  done <<< "${CHANGED_FILES}"
}

prepare_full_scope() {
  local broad_rust_change=0
  local tooling_change=0
  local file

  while IFS= read -r file; do
    [[ -z "$file" ]] && continue
    if is_broad_rust_full_path "$file"; then
      broad_rust_change=1
    fi
    if is_tooling_full_path "$file"; then
      tooling_change=1
    fi
  done <<< "${CHANGED_FILES}"

  FULL_SCOPE_HAS_TOOLING=$tooling_change

  if [[ $broad_rust_change -eq 1 ]]; then
    FULL_SCOPE_MODE="broad-rust"
    FULL_SCOPE_SELECTED_CRATES=("${FULL_LOCAL_PACKAGES[@]}")
  else
    mapfile -t FULL_SCOPE_SELECTED_CRATES < <(list_changed_critical_crates)
    if [[ ${#FULL_SCOPE_SELECTED_CRATES[@]} -gt 0 ]]; then
      FULL_SCOPE_MODE="targeted-rust"
    else
      FULL_SCOPE_MODE="tooling-only"
    fi
  fi

  mapfile -t FULL_SCOPE_CLIPPY_CORE_PACKAGES < <(
    filter_package_set_by_selection FULL_SCOPE_SELECTED_CRATES FULL_LOCAL_CLIPPY_CORE_PACKAGES
  )
  mapfile -t FULL_SCOPE_CLIPPY_SUPPORT_PACKAGES < <(
    filter_package_set_by_selection FULL_SCOPE_SELECTED_CRATES FULL_LOCAL_CLIPPY_SUPPORT_PACKAGES
  )
  mapfile -t FULL_SCOPE_CLIPPY_BIN_ONLY_PACKAGES < <(
    filter_package_set_by_selection FULL_SCOPE_SELECTED_CRATES FULL_LOCAL_CLIPPY_BIN_ONLY_PACKAGES
  )
  mapfile -t FULL_SCOPE_TEST_SUPPORT_PACKAGES < <(
    filter_package_set_by_selection FULL_SCOPE_SELECTED_CRATES FULL_LOCAL_TEST_PACKAGES
  )
  mapfile -t FULL_SCOPE_RUSTDOC_PACKAGES < <(
    filter_package_set_by_selection FULL_SCOPE_SELECTED_CRATES FULL_LOCAL_RUSTDOC_PACKAGES
  )

  FULL_SCOPE_RUN_WARP_CORE_SMOKE=0
  FULL_SCOPE_WARP_WASM_TEST_MODE="none"
  FULL_SCOPE_ECHO_WASM_ABI_RUN_LIB=0
  FULL_SCOPE_ECHO_WASM_ABI_EXTRA_TESTS=()
  FULL_SCOPE_WARP_CORE_EXTRA_TESTS=()
  FULL_SCOPE_WARP_CORE_RUN_PRNG=0

  if array_contains "warp-core" ${FULL_SCOPE_SELECTED_CRATES[@]+"${FULL_SCOPE_SELECTED_CRATES[@]}"}; then
    FULL_SCOPE_RUN_WARP_CORE_SMOKE=1
    prepare_warp_core_scope
  fi
  if array_contains "warp-wasm" ${FULL_SCOPE_SELECTED_CRATES[@]+"${FULL_SCOPE_SELECTED_CRATES[@]}"}; then
    prepare_warp_wasm_scope
  fi
  if array_contains "echo-wasm-abi" ${FULL_SCOPE_SELECTED_CRATES[@]+"${FULL_SCOPE_SELECTED_CRATES[@]}"}; then
    prepare_echo_wasm_abi_scope
  fi
}

run_pattern_guards() {
  ensure_command rg

  echo "[verify-local] scanning banned patterns"
  local match_output
  if match_output=$(rg -n '#!\[allow\([^]]*missing_docs[^]]*\)\]' \
    crates \
    --glob 'crates/**/src/**/*.rs' \
    --glob '!**/telemetry.rs' \
    --glob '!**/tests/**' \
    --glob '!**/build.rs' \
    --glob '!**/*.generated.rs' 2>&1); then
    echo "pre-push: crate-level allow(missing_docs) is forbidden (except telemetry.rs and *.generated.rs)." >&2
    echo "$match_output" >&2
    exit 1
  fi
  if match_output=$(rg -n "\\#\\[\\s*no_mangle\\s*\\]" crates 2>&1); then
    echo "pre-push: #[no_mangle] is invalid; use #[unsafe(no_mangle)]." >&2
    echo "$match_output" >&2
    exit 1
  fi
}

run_spdx_check() {
  echo "[verify-local] checking SPDX headers"
  if [[ -x scripts/check_spdx.sh ]]; then
    scripts/check_spdx.sh || {
      echo "[verify-local] SPDX check failed. Run ./scripts/ensure_spdx.sh --all to fix." >&2
      exit 1
    }
  fi
}

run_determinism_guard() {
  if [[ -x scripts/ban-nondeterminism.sh ]]; then
    echo "[verify-local] determinism guard"
    scripts/ban-nondeterminism.sh
  fi
}

run_full_lane_fmt() {
  echo "[verify-local][fmt] cargo fmt --all -- --check"
  cargo +"$PINNED" fmt --all -- --check
}

run_full_lane_clippy_core() {
  if [[ ${#FULL_SCOPE_CLIPPY_CORE_PACKAGES[@]} -eq 0 ]]; then
    echo "[verify-local][clippy-core] no selected core packages"
    return
  fi
  local args=()
  mapfile -t args < <(package_args "${FULL_SCOPE_CLIPPY_CORE_PACKAGES[@]}")
  echo "[verify-local][clippy-core] curated clippy on selected core packages"
  lane_cargo "full-clippy-core" clippy "${args[@]}" --lib -- -D warnings -D missing_docs
}

run_full_lane_clippy_support() {
  if [[ ${#FULL_SCOPE_CLIPPY_SUPPORT_PACKAGES[@]} -eq 0 ]]; then
    echo "[verify-local][clippy-support] no selected support packages"
    return
  fi
  local args=()
  mapfile -t args < <(package_args "${FULL_SCOPE_CLIPPY_SUPPORT_PACKAGES[@]}")
  echo "[verify-local][clippy-support] curated clippy on selected support packages"
  lane_cargo "full-clippy-support" clippy "${args[@]}" --lib --tests -- -D warnings -D missing_docs
}

run_full_lane_clippy_bins() {
  if [[ ${#FULL_SCOPE_CLIPPY_BIN_ONLY_PACKAGES[@]} -eq 0 ]]; then
    echo "[verify-local][clippy-bins] no selected binary-only packages"
    return
  fi
  local args=()
  mapfile -t args < <(package_args "${FULL_SCOPE_CLIPPY_BIN_ONLY_PACKAGES[@]}")
  echo "[verify-local][clippy-bins] curated clippy on selected binary-only packages"
  lane_cargo "full-clippy-bins" clippy "${args[@]}" --bins -- -D warnings -D missing_docs
}

run_full_lane_tests_support() {
  if [[ ${#FULL_SCOPE_TEST_SUPPORT_PACKAGES[@]} -eq 0 ]]; then
    echo "[verify-local][tests-support] no selected support-package tests"
    return
  fi
  local args=()
  mapfile -t args < <(package_args "${FULL_SCOPE_TEST_SUPPORT_PACKAGES[@]}")
  echo "[verify-local][tests-support] selected support-package tests"
  lane_cargo "full-tests-support" test "${args[@]}" --lib --tests
}

run_full_lane_tests_runtime() {
  if [[ "$FULL_SCOPE_WARP_WASM_TEST_MODE" == "none" && "$FULL_SCOPE_ECHO_WASM_ABI_RUN_LIB" != "1" && ${#FULL_SCOPE_ECHO_WASM_ABI_EXTRA_TESTS[@]} -eq 0 ]]; then
    echo "[verify-local][tests-runtime] no selected runtime packages"
    return
  fi
  echo "[verify-local][tests-runtime] selected runtime checks"
  if [[ "$FULL_SCOPE_WARP_WASM_TEST_MODE" == "plain-lib" ]]; then
    lane_cargo "full-tests-runtime" test -p warp-wasm --lib
  fi
  if [[ "$FULL_SCOPE_WARP_WASM_TEST_MODE" == "engine-lib" ]]; then
    lane_cargo "full-tests-runtime" test -p warp-wasm --features engine --lib
  fi
  if [[ "$FULL_SCOPE_ECHO_WASM_ABI_RUN_LIB" == "1" ]]; then
    lane_cargo "full-tests-runtime" test -p echo-wasm-abi --lib
  fi
  local test_target
  for test_target in "${FULL_SCOPE_ECHO_WASM_ABI_EXTRA_TESTS[@]}"; do
    lane_cargo "full-tests-runtime" test -p echo-wasm-abi --test "$test_target"
  done
}

run_full_lane_tests_warp_core() {
  if [[ "$FULL_SCOPE_RUN_WARP_CORE_SMOKE" != "1" ]]; then
    echo "[verify-local][tests-warp-core] warp-core not selected"
    return
  fi
  echo "[verify-local][tests-warp-core] local warp-core smoke suite"
  lane_cargo "full-tests-warp-core" test -p warp-core --lib
  local test_target
  for test_target in "${FULL_SCOPE_WARP_CORE_EXTRA_TESTS[@]}"; do
    lane_cargo "full-tests-warp-core" test -p warp-core --test "$test_target"
  done
  if [[ "$FULL_SCOPE_WARP_CORE_RUN_PRNG" == "1" ]]; then
    lane_cargo "full-tests-warp-core" test -p warp-core --features golden_prng --test prng_golden_regression
  fi
}

run_full_lane_rustdoc() {
  if [[ ${#FULL_SCOPE_RUSTDOC_PACKAGES[@]} -eq 0 ]]; then
    echo "[verify-local][rustdoc] no selected public-doc crates"
    return
  fi
  local doc_pkg
  for doc_pkg in "${FULL_SCOPE_RUSTDOC_PACKAGES[@]}"; do
    echo "[verify-local][rustdoc] ${doc_pkg}"
    CARGO_TARGET_DIR="$(lane_target_dir "full-rustdoc")" \
      RUSTDOCFLAGS="-D warnings" \
      cargo +"$PINNED" doc -p "${doc_pkg}" --no-deps
  done
}

run_full_lane_hook_tests() {
  if [[ "$FULL_SCOPE_HAS_TOOLING" != "1" ]]; then
    echo "[verify-local][hook-tests] no tooling changes detected"
    return
  fi
  shopt -s nullglob
  local -a hook_tests=(tests/hooks/test_*.sh)
  shopt -u nullglob
  if [[ ${#hook_tests[@]} -eq 0 ]]; then
    echo "[verify-local][hook-tests] no hook regression scripts present"
    return
  fi
  echo "[verify-local][hook-tests] hook regression coverage"
  local hook_test
  for hook_test in "${hook_tests[@]}"; do
    bash "$hook_test"
  done
}

run_ultra_fast_tooling_smoke() {
  if [[ "$FULL_SCOPE_HAS_TOOLING" != "1" ]]; then
    return
  fi
  echo "[verify-local][ultra-fast] tooling smoke"
  mapfile -t shell_files < <(list_changed_tooling_shell_files)
  if [[ ${#shell_files[@]} -eq 0 ]]; then
    echo "[verify-local][ultra-fast] no changed shell tooling files"
    return
  fi
  local file
  for file in "${shell_files[@]}"; do
    echo "[verify-local][ultra-fast] bash -n ${file}"
    bash -n "$file"
  done
}

run_full_lane_guards() {
  run_pattern_guards
  run_spdx_check
  run_determinism_guard
  run_docs_lint
}

run_full_checks_sequential() {
  echo "[verify-local] critical local gate (${FULL_SCOPE_MODE})"
  run_timed_step "fmt" run_full_lane_fmt
  run_timed_step "hook-tests" run_full_lane_hook_tests
  run_timed_step "clippy-core" run_full_lane_clippy_core
  run_timed_step "clippy-support" run_full_lane_clippy_support
  run_timed_step "clippy-bins" run_full_lane_clippy_bins
  run_timed_step "tests-support" run_full_lane_tests_support
  run_timed_step "tests-runtime" run_full_lane_tests_runtime
  run_timed_step "tests-warp-core" run_full_lane_tests_warp_core
  run_timed_step "rustdoc" run_full_lane_rustdoc
  run_timed_step "guards" run_full_lane_guards
}

run_full_checks_parallel() {
  local -a lanes=("full" "fmt" run_full_lane_fmt "guards" run_full_lane_guards)

  echo "[verify-local] critical local gate (${FULL_SCOPE_MODE})"

  if [[ "$FULL_SCOPE_HAS_TOOLING" == "1" ]]; then
    lanes+=("hook-tests" run_full_lane_hook_tests)
  fi
  if [[ ${#FULL_SCOPE_CLIPPY_CORE_PACKAGES[@]} -gt 0 ]]; then
    lanes+=("clippy-core" run_full_lane_clippy_core)
  fi
  if [[ ${#FULL_SCOPE_CLIPPY_SUPPORT_PACKAGES[@]} -gt 0 ]]; then
    lanes+=("clippy-support" run_full_lane_clippy_support)
  fi
  if [[ ${#FULL_SCOPE_CLIPPY_BIN_ONLY_PACKAGES[@]} -gt 0 ]]; then
    lanes+=("clippy-bins" run_full_lane_clippy_bins)
  fi
  if [[ ${#FULL_SCOPE_TEST_SUPPORT_PACKAGES[@]} -gt 0 ]]; then
    lanes+=("tests-support" run_full_lane_tests_support)
  fi
  if [[ "$FULL_SCOPE_WARP_WASM_TEST_MODE" != "none" || "$FULL_SCOPE_ECHO_WASM_ABI_RUN_LIB" == "1" || ${#FULL_SCOPE_ECHO_WASM_ABI_EXTRA_TESTS[@]} -gt 0 ]]; then
    lanes+=("tests-runtime" run_full_lane_tests_runtime)
  fi
  if [[ "$FULL_SCOPE_RUN_WARP_CORE_SMOKE" == "1" ]]; then
    lanes+=("tests-warp-core" run_full_lane_tests_warp_core)
  fi
  if [[ ${#FULL_SCOPE_RUSTDOC_PACKAGES[@]} -gt 0 ]]; then
    lanes+=("rustdoc" run_full_lane_rustdoc)
  fi

  run_parallel_lanes "${lanes[@]}"
}

run_full_checks() {
  ensure_toolchain
  prepare_full_scope
  if should_run_parallel_lanes; then
    run_full_checks_parallel
    return
  fi
  run_full_checks_sequential
}

run_ultra_fast_smoke() {
  prepare_full_scope

  echo "[verify-local] ultra-fast critical smoke (${FULL_SCOPE_MODE})"

  if [[ "$FULL_SCOPE_HAS_TOOLING" == "1" ]]; then
    run_ultra_fast_tooling_smoke
  fi

  if [[ "$FULL_SCOPE_RUN_WARP_CORE_SMOKE" == "1" ]]; then
    echo "[verify-local][ultra-fast] warp-core smoke"
    cargo +"$PINNED" test -p warp-core --lib
    local warp_core_test_target
    for warp_core_test_target in "${FULL_SCOPE_WARP_CORE_EXTRA_TESTS[@]}"; do
      cargo +"$PINNED" test -p warp-core --test "$warp_core_test_target"
    done
    if [[ "$FULL_SCOPE_WARP_CORE_RUN_PRNG" == "1" ]]; then
      cargo +"$PINNED" test -p warp-core --features golden_prng --test prng_golden_regression
    fi
  fi

  if [[ "$FULL_SCOPE_WARP_WASM_TEST_MODE" == "plain-lib" ]]; then
    echo "[verify-local][ultra-fast] warp-wasm plain lib smoke"
    cargo +"$PINNED" test -p warp-wasm --lib
  fi
  if [[ "$FULL_SCOPE_WARP_WASM_TEST_MODE" == "engine-lib" ]]; then
    echo "[verify-local][ultra-fast] warp-wasm engine lib smoke"
    cargo +"$PINNED" test -p warp-wasm --features engine --lib
  fi
  if [[ "$FULL_SCOPE_ECHO_WASM_ABI_RUN_LIB" == "1" ]]; then
    echo "[verify-local][ultra-fast] echo-wasm-abi lib smoke"
    cargo +"$PINNED" test -p echo-wasm-abi --lib
  fi
  local abi_test_target
  for abi_test_target in "${FULL_SCOPE_ECHO_WASM_ABI_EXTRA_TESTS[@]}"; do
    echo "[verify-local][ultra-fast] echo-wasm-abi --test ${abi_test_target}"
    cargo +"$PINNED" test -p echo-wasm-abi --test "$abi_test_target"
  done
}

run_ultra_fast_checks() {
  local classification="$1"
  local -a changed_crates=()

  if [[ "$classification" == "docs" ]]; then
    echo "[verify-local] ultra-fast docs-only change set"
    run_docs_lint
    return
  fi

  mapfile -t changed_crates < <(list_changed_rust_crates)
  if [[ ${#changed_crates[@]} -eq 0 ]]; then
    if [[ "$classification" == "full" ]]; then
      run_ultra_fast_smoke
      return
    fi
    echo "[verify-local] ultra-fast: no changed Rust crates detected"
    run_docs_lint
    return
  fi

  ensure_toolchain
  echo "[verify-local] ultra-fast verification for changed Rust crates: ${changed_crates[*]}"
  echo "[verify-local] cargo fmt --all -- --check"
  cargo +"$PINNED" fmt --all -- --check

  local crate
  for crate in "${changed_crates[@]}"; do
    if [[ ! -f "crates/${crate}/Cargo.toml" ]]; then
      echo "[verify-local] skipping ${crate}: missing crates/${crate}/Cargo.toml" >&2
      continue
    fi
    echo "[verify-local] cargo check -p ${crate}"
    cargo +"$PINNED" check -p "$crate" --quiet
  done

  if [[ "$classification" == "full" ]]; then
    run_ultra_fast_smoke
  fi
}

run_auto_mode() {
  local classification="$1"
  local suite
  suite="$(stamp_suite_for_classification "$classification")"

  if should_skip_via_stamp "$suite"; then
    VERIFY_RUN_CACHE_STATE="cached"
    echo "[verify-local] reusing cached ${classification} verification for tree $(printf '%.12s' "$VERIFY_STAMP_SUBJECT")"
    return
  fi

  case "$classification" in
    docs)
      echo "[verify-local] docs-only change set"
      run_docs_lint
      ;;
    reduced)
      mapfile -t changed_crates < <(list_changed_crates)
      echo "[verify-local] reduced verification for changed crates: ${changed_crates[*]:-(none)}"
      run_targeted_checks "${changed_crates[@]}"
      ;;
    full)
      echo "[verify-local] full verification required by critical/tooling changes"
      run_full_checks
      ;;
    *)
      echo "[verify-local] unknown classification: $classification" >&2
      exit 1
      ;;
  esac

  write_stamp "$suite"
}

VERIFY_MODE_CONTEXT="$(mode_context "$MODE")"
if [[ -n "${VERIFY_STAMP_SUBJECT:-}" ]]; then
  :
elif [[ "$VERIFY_MODE_CONTEXT" == "pre-commit" || "$VERIFY_MODE_CONTEXT" == "working-tree" || "$MODE" == "full" ]]; then
  if [[ "$VERIFY_MODE_CONTEXT" == "pre-commit" ]]; then
    VERIFY_STAMP_SUBJECT="$(git write-tree)"
  else
    VERIFY_STAMP_SUBJECT="$(worktree_tree)"
  fi
else
  VERIFY_STAMP_SUBJECT="$(git rev-parse HEAD^{tree})"
fi
readonly VERIFY_MODE_CONTEXT VERIFY_STAMP_SUBJECT

CHANGED_FILES="$(list_changed_files "$VERIFY_MODE_CONTEXT")"
CLASSIFICATION="$(classify_change_set)"
VERIFY_CLASSIFICATION="$CLASSIFICATION"

case "$MODE" in
  detect|detect-pre-commit)
    VERIFY_REPORT_TIMING=0
    printf 'classification=%s\n' "$CLASSIFICATION"
    printf 'stamp_suite=%s\n' "$(stamp_suite_for_classification "$CLASSIFICATION")"
    printf 'stamp_context=%s\n' "$(stamp_context_for_suite "$(stamp_suite_for_classification "$CLASSIFICATION")")"
    printf 'changed_files=%s\n' "$(printf '%s' "$CHANGED_FILES" | awk 'NF {count++} END {print count+0}')"
    printf 'changed_crates=%s\n' "$(list_changed_crates | paste -sd, -)"
    ;;
  fast)
    mapfile -t changed_crates < <(list_changed_rust_crates)
    run_targeted_checks "${changed_crates[@]}"
    ;;
  ultra-fast)
    run_ultra_fast_checks "$CLASSIFICATION"
    ;;
  pre-commit)
    if should_skip_via_stamp "$(stamp_suite_for_classification "$CLASSIFICATION")"; then
      VERIFY_RUN_CACHE_STATE="cached"
      echo "[verify-local] reusing cached pre-commit verification for index $(printf '%.12s' "$VERIFY_STAMP_SUBJECT")"
      exit 0
    fi
    run_pre_commit_checks
    write_stamp "$(stamp_suite_for_classification "$CLASSIFICATION")"
    ;;
  pr|auto|pre-push)
    run_auto_mode "$CLASSIFICATION"
    ;;
  full)
    if should_skip_via_stamp "full"; then
      VERIFY_RUN_CACHE_STATE="cached"
      echo "[verify-local] reusing cached full verification for tree $(printf '%.12s' "$VERIFY_STAMP_SUBJECT")"
      exit 0
    fi
    run_full_checks
    write_stamp "full"
    ;;
  *)
    echo "usage: scripts/verify-local.sh [detect|detect-pre-commit|ultra-fast|fast|pre-commit|pr|full|auto|pre-push]" >&2
    exit 1
    ;;
esac
