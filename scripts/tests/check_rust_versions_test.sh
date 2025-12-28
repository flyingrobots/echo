#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

set -euo pipefail

script_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_root}/../.." && pwd)"
checker_src="${repo_root}/scripts/check_rust_versions.sh"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

with_tmp_repo() (
  set -euo pipefail
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT

  mkdir -p "$tmp/scripts" "$tmp/crates/foo" "$tmp/specs/bar"
  cp "$checker_src" "$tmp/scripts/check_rust_versions.sh"
  chmod +x "$tmp/scripts/check_rust_versions.sh"

  cat > "$tmp/rust-toolchain.toml" <<'EOF'
[toolchain]
channel = "1.90.0"
EOF

  cd "$tmp"
  "$@"
)

test_passes_with_matching_versions() {
  with_tmp_repo bash -c '
    set -euo pipefail
    cat > crates/foo/Cargo.toml <<EOF
[package]
name = "foo"
version = "0.1.0"
edition = "2021"
rust-version = "1.90.0"
EOF
    cat > specs/bar/Cargo.toml <<EOF
[package]
name = "bar"
version = "0.1.0"
edition = "2021"
rust-version = "1.90.0"
EOF
    ./scripts/check_rust_versions.sh >/dev/null
  '
}

test_parses_inline_comment_with_quotes() {
  with_tmp_repo bash -c '
    set -euo pipefail
    cat > crates/foo/Cargo.toml <<EOF
[package]
name = "foo"
version = "0.1.0"
edition = "2021"
rust-version = "1.90.0" # comment with "quotes"
EOF
    ./scripts/check_rust_versions.sh >/dev/null
  '
}

test_fails_when_rust_version_missing() {
  with_tmp_repo bash -c '
    set -euo pipefail
    cat > crates/foo/Cargo.toml <<EOF
[package]
name = "foo"
version = "0.1.0"
edition = "2021"
EOF
    out="$({ ./scripts/check_rust_versions.sh 2>&1; } || true)"
    echo "$out" | grep -q "rust-version missing"
  '
}

test_fails_on_mismatch() {
  with_tmp_repo bash -c '
    set -euo pipefail
    cat > crates/foo/Cargo.toml <<EOF
[package]
name = "foo"
version = "0.1.0"
edition = "2021"
rust-version = "1.89.0"
EOF
    out="$({ ./scripts/check_rust_versions.sh 2>&1; } || true)"
    echo "$out" | grep -q "rust-version mismatch"
  '
}

test_detects_nested_manifests() {
  with_tmp_repo bash -c '
    set -euo pipefail
    mkdir -p crates/foo/nested
    cat > crates/foo/Cargo.toml <<EOF
[package]
name = "foo"
version = "0.1.0"
edition = "2021"
rust-version = "1.90.0"
EOF
    cat > crates/foo/nested/Cargo.toml <<EOF
[package]
name = "foo_nested"
version = "0.1.0"
edition = "2021"
rust-version = "1.89.0"
EOF
    out="$({ ./scripts/check_rust_versions.sh 2>&1; } || true)"
    echo "$out" | grep -q "crates/foo/nested/Cargo.toml"
  '
}

main() {
  [[ -f "$checker_src" ]] || fail "checker script missing: $checker_src"

  test_passes_with_matching_versions
  test_parses_inline_comment_with_quotes
  test_fails_when_rust_version_missing
  test_fails_on_mismatch
  test_detects_nested_manifests
}

main "$@"
