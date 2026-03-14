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

report_timing() {
  local status="$1"
  if [[ "$VERIFY_REPORT_TIMING" != "1" ]]; then
    return
  fi

  local elapsed
  elapsed="$(format_elapsed "$SECONDS")"
  if [[ "$status" -eq 0 ]]; then
    echo "[verify-local] completed in ${elapsed}"
  else
    echo "[verify-local] failed after ${elapsed}" >&2
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
  "crates/ttd-manifest/"
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
  "ttd-manifest"
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
    git diff --name-only --diff-filter=ACMRTUXB '@{upstream}...HEAD'
    return
  fi

  local candidate
  local merge_base
  for candidate in origin/main main origin/master master; do
    if git rev-parse --verify "$candidate" >/dev/null 2>&1; then
      merge_base="$(git merge-base HEAD "$candidate")"
      git diff --name-only --diff-filter=ACMRTUXB "${merge_base}...HEAD"
      return
    fi
  done

  git diff-tree --root --no-commit-id --name-only -r --diff-filter=ACMRTUXB HEAD
}

list_changed_index_files() {
  git diff --cached --name-only --diff-filter=ACMRTUXB
}

mode_context() {
  case "$1" in
    pre-commit|detect-pre-commit)
      printf 'pre-commit\n'
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

stamp_key() {
  local suite="$1"
  printf '%s-%s-%s-%s-%s' \
    "$suite" \
    "$PINNED" \
    "$VERIFY_MODE_CONTEXT" \
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
toolchain=$PINNED
script_hash=$SCRIPT_HASH
timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)
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
  mapfile -t md_files < <(printf '%s\n' "$CHANGED_FILES" | awk '/\.md$/ {print}')
  if [[ ${#md_files[@]} -eq 0 ]]; then
    return
  fi
  if ! command -v npx >/dev/null 2>&1; then
    echo "[verify-local] npx not found; skipping markdown format check for ${#md_files[@]} changed markdown files" >&2
    return
  fi
  echo "[verify-local] prettier --check (${#md_files[@]} markdown files)"
  npx prettier --check "${md_files[@]}"
}

run_targeted_checks() {
  local crates=("$@")
  local crate

  if [[ ${#crates[@]} -eq 0 ]]; then
    echo "[verify-local] no changed crates detected; running docs-only checks"
    run_docs_lint
    return
  fi

  ensure_toolchain
  echo "[verify-local] cargo fmt --all -- --check"
  cargo +"$PINNED" fmt --all -- --check

  run_crate_lint_and_check "${crates[@]}"

  local public_doc_crates=("warp-core" "warp-geom" "warp-wasm")
  for crate in "${public_doc_crates[@]}"; do
    if printf '%s\n' "${crates[@]}" | grep -qx "$crate"; then
      echo "[verify-local] rustdoc warnings gate (${crate})"
      RUSTDOCFLAGS="-D warnings" cargo +"$PINNED" doc -p "$crate" --no-deps
    fi
  done

  for crate in "${crates[@]}"; do
    if [[ ! -f "crates/${crate}/Cargo.toml" ]]; then
      continue
    fi
    if use_nextest; then
      echo "[verify-local] cargo nextest run -p ${crate}"
      cargo +"$PINNED" nextest run -p "$crate"
    else
      echo "[verify-local] cargo test -p ${crate}"
      cargo +"$PINNED" test -p "$crate"
    fi
  done

  run_docs_lint
}

run_crate_lint_and_check() {
  local crates=("$@")
  local crate

  for crate in "${crates[@]}"; do
    if [[ ! -f "crates/${crate}/Cargo.toml" ]]; then
      echo "[verify-local] skipping ${crate}: missing crates/${crate}/Cargo.toml" >&2
      continue
    fi
    echo "[verify-local] cargo clippy -p ${crate} --all-targets"
    cargo +"$PINNED" clippy -p "$crate" --all-targets -- -D warnings -D missing_docs
    echo "[verify-local] cargo check -p ${crate}"
    cargo +"$PINNED" check -p "$crate" --quiet
  done
}

run_pre_commit_checks() {
  mapfile -t changed_crates < <(list_changed_crates)
  if [[ ${#changed_crates[@]} -eq 0 ]]; then
    echo "[verify-local] pre-commit: no staged crates detected"
    return
  fi

  ensure_toolchain
  echo "[verify-local] pre-commit verification for staged crates: ${changed_crates[*]}"
  run_crate_lint_and_check "${changed_crates[@]}"
}

package_args() {
  local pkg
  for pkg in "$@"; do
    printf '%s\n' "-p" "$pkg"
  done
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

run_full_checks() {
  ensure_toolchain
  echo "[verify-local] critical local gate"
  echo "[verify-local] cargo fmt --all -- --check"
  cargo +"$PINNED" fmt --all -- --check

  local full_args=()
  mapfile -t full_args < <(package_args "${FULL_LOCAL_PACKAGES[@]}")
  local full_test_args=()
  mapfile -t full_test_args < <(package_args "${FULL_LOCAL_TEST_PACKAGES[@]}")

  echo "[verify-local] cargo clippy on critical packages"
  cargo +"$PINNED" clippy "${full_args[@]}" --all-targets -- -D warnings -D missing_docs

  echo "[verify-local] tests on critical packages (lib + integration targets)"
  cargo +"$PINNED" test "${full_test_args[@]}" --lib --tests
  cargo +"$PINNED" test -p warp-wasm --features engine --lib
  cargo +"$PINNED" test -p echo-wasm-abi --lib
  cargo +"$PINNED" test -p warp-core --lib
  cargo +"$PINNED" test -p warp-core --test inbox
  cargo +"$PINNED" test -p warp-core --test invariant_property_tests
  cargo +"$PINNED" test -p warp-core --test golden_vectors_phase0
  cargo +"$PINNED" test -p warp-core --test materialization_determinism

  echo "[verify-local] PRNG golden regression (warp-core)"
  cargo +"$PINNED" test -p warp-core --features golden_prng --test prng_golden_regression

  local doc_pkg
  for doc_pkg in warp-core warp-geom warp-wasm; do
    echo "[verify-local] rustdoc warnings gate (${doc_pkg})"
    RUSTDOCFLAGS="-D warnings" cargo +"$PINNED" doc -p "${doc_pkg}" --no-deps
  done

  run_pattern_guards
  run_spdx_check
  run_determinism_guard
  run_docs_lint
}

run_auto_mode() {
  local classification="$1"
  local suite
  suite="$(stamp_suite_for_classification "$classification")"

  if should_skip_via_stamp "$suite"; then
    echo "[verify-local] reusing cached ${classification} verification for HEAD $(git rev-parse --short HEAD)"
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
elif [[ "$VERIFY_MODE_CONTEXT" == "pre-commit" ]]; then
  VERIFY_STAMP_SUBJECT="$(git write-tree)"
else
  VERIFY_STAMP_SUBJECT="$(git rev-parse HEAD)"
fi
readonly VERIFY_MODE_CONTEXT VERIFY_STAMP_SUBJECT

CHANGED_FILES="$(list_changed_files "$VERIFY_MODE_CONTEXT")"
CLASSIFICATION="$(classify_change_set)"

case "$MODE" in
  detect|detect-pre-commit)
    VERIFY_REPORT_TIMING=0
    printf 'classification=%s\n' "$CLASSIFICATION"
    printf 'stamp_suite=%s\n' "$(stamp_suite_for_classification "$CLASSIFICATION")"
    printf 'stamp_context=%s\n' "$VERIFY_MODE_CONTEXT"
    printf 'changed_files=%s\n' "$(printf '%s' "$CHANGED_FILES" | awk 'NF {count++} END {print count+0}')"
    printf 'changed_crates=%s\n' "$(list_changed_crates | paste -sd, -)"
    ;;
  fast)
    mapfile -t changed_crates < <(list_changed_crates)
    run_targeted_checks "${changed_crates[@]}"
    ;;
  pre-commit)
    if should_skip_via_stamp "$(stamp_suite_for_classification "$CLASSIFICATION")"; then
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
      echo "[verify-local] reusing cached full verification for HEAD $(git rev-parse --short HEAD)"
      exit 0
    fi
    run_full_checks
    write_stamp "full"
    ;;
  *)
    echo "usage: scripts/verify-local.sh [detect|detect-pre-commit|fast|pre-commit|pr|full|auto|pre-push]" >&2
    exit 1
    ;;
esac
