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

  cat > "$tmp/Cargo.toml" <<'EOF'
[workspace]
members = ["crates/foo", "specs/bar"]
resolver = "2"

[workspace.package]
rust-version = "1.90.0"
EOF

  # Every package/version choice is explicit, including the general toolchain.
  cat > "$tmp/scripts/rust-msrv-policy.tsv" <<'EOF'
toolchain 1.90.0
workspace 1.90.0
crates/foo/Cargo.toml 1.90.0
specs/bar/Cargo.toml 1.90.0
EOF
  for spec in "crates/foo foo" "specs/bar bar"; do
    read -r directory name <<< "$spec"
    cat > "$tmp/$directory/Cargo.toml" <<EOF
[package]
name = "$name"
version = "0.1.0"
edition = "2021"
rust-version = "1.90.0"
EOF
  done

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

test_passes_with_workspace_inherited_version() {
  with_tmp_repo bash -c '
    set -euo pipefail
    cat > crates/foo/Cargo.toml <<EOF
[package]
name = "foo"
version = "0.1.0"
edition = "2021"
rust-version.workspace = true
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


# These fixtures test an intentional per-package split, not a permissive
# declaration <= toolchain comparison. Provider leaves stay at the old MSRV.
test_passes_with_explicit_split() {
  with_tmp_repo bash -c '
    set -euo pipefail
    sed "s/1.90.0/1.96.0/" rust-toolchain.toml > replacement
    mv replacement rust-toolchain.toml
    sed "s/1.90.0/1.96.0/" crates/foo/Cargo.toml > replacement
    mv replacement crates/foo/Cargo.toml
    cat > scripts/rust-msrv-policy.tsv <<EOF
toolchain 1.96.0
workspace 1.90.0
crates/foo/Cargo.toml 1.96.0
specs/bar/Cargo.toml 1.90.0
EOF
    ./scripts/check_rust_versions.sh >/dev/null
  '
}

test_fails_for_unregistered_manifest() {
  with_tmp_repo bash -c '
    set -euo pipefail
    mkdir -p crates/unregistered
    cp crates/foo/Cargo.toml crates/unregistered/Cargo.toml
    if ./scripts/check_rust_versions.sh > refusal.log 2>&1; then exit 1; fi
    grep -q "unregistered.*crates/unregistered/Cargo.toml" refusal.log
  '
}

test_fails_for_stale_exception() {
  with_tmp_repo bash -c '
    set -euo pipefail
    printf "%s\n" "crates/unknown/Cargo.toml 1.90.0" >> scripts/rust-msrv-policy.tsv
    if ./scripts/check_rust_versions.sh > refusal.log 2>&1; then exit 1; fi
    grep -q "missing.*crates/unknown/Cargo.toml" refusal.log
  '
}

test_fails_for_duplicate_policy_entry() {
  with_tmp_repo bash -c '
    set -euo pipefail
    printf "%s\n" "crates/foo/Cargo.toml 1.90.0" >> scripts/rust-msrv-policy.tsv
    if ./scripts/check_rust_versions.sh > refusal.log 2>&1; then exit 1; fi
    grep -q "duplicate.*crates/foo/Cargo.toml" refusal.log
  '
}

test_fails_when_consumer_claims_old_msrv() {
  with_tmp_repo bash -c '
    set -euo pipefail
    sed "s/1.90.0/1.96.0/" rust-toolchain.toml > replacement
    mv replacement rust-toolchain.toml
    cat > scripts/rust-msrv-policy.tsv <<EOF
toolchain 1.96.0
workspace 1.90.0
crates/foo/Cargo.toml 1.96.0
specs/bar/Cargo.toml 1.90.0
EOF
    if ./scripts/check_rust_versions.sh > refusal.log 2>&1; then exit 1; fi
    grep -q "rust-version mismatch" refusal.log
    grep -q "crates/foo/Cargo.toml" refusal.log
  '
}

test_fails_when_toolchain_drifts_from_policy() {
  with_tmp_repo bash -c '
    set -euo pipefail
    sed "s/1.90.0/1.97.0/" rust-toolchain.toml > replacement
    mv replacement rust-toolchain.toml
    if ./scripts/check_rust_versions.sh > refusal.log 2>&1; then exit 1; fi
    grep -q "toolchain.*policy" refusal.log
  '
}

test_fails_when_workspace_default_drifts() {
  with_tmp_repo bash -c '
    set -euo pipefail
    sed "s/1.90.0/1.89.0/" Cargo.toml > replacement
    mv replacement Cargo.toml
    if ./scripts/check_rust_versions.sh > refusal.log 2>&1; then exit 1; fi
    grep -q "workspace.*policy" refusal.log
  '
}

test_fails_without_policy() {
  with_tmp_repo bash -c '
    set -euo pipefail
    mv scripts/rust-msrv-policy.tsv scripts/withheld-policy.tsv
    if ./scripts/check_rust_versions.sh > refusal.log 2>&1; then exit 1; fi
    grep -q "policy.*missing" refusal.log
  '
}

test_system_bash_handles_initially_empty_policy_arrays() {
  with_tmp_repo bash -c '
    set -euo pipefail
    /bin/bash ./scripts/check_rust_versions.sh >/dev/null
  '
}

main() {
  [[ -f "$checker_src" ]] || fail "checker script missing: $checker_src"

  test_system_bash_handles_initially_empty_policy_arrays
  test_passes_with_matching_versions
  test_passes_with_workspace_inherited_version
  test_parses_inline_comment_with_quotes
  test_fails_when_rust_version_missing
  test_fails_on_mismatch
  test_detects_nested_manifests
  test_passes_with_explicit_split
  test_fails_for_unregistered_manifest
  test_fails_for_stale_exception
  test_fails_for_duplicate_policy_entry
  test_fails_when_consumer_claims_old_msrv
  test_fails_when_toolchain_drifts_from_policy
  test_fails_when_workspace_default_drifts
  test_fails_without_policy
}

main "$@"
