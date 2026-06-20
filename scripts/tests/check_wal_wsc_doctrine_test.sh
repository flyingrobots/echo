#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

set -euo pipefail

script_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_root}/../.." && pwd)"
checker="${repo_root}/scripts/check-wal-wsc-doctrine.sh"
tmp_dirs=()

cleanup_tmp_dirs() {
  local tmp
  for tmp in "${tmp_dirs[@]}"; do
    rm -rf "$tmp"
  done
}

trap cleanup_tmp_dirs EXIT

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

copy_fixture() {
  local tmp="$1"
  mkdir -p "${tmp}/docs/design" "${tmp}/docs/releases"
  cp "${repo_root}/docs/BEARING.md" "${tmp}/docs/BEARING.md"
  cp "${repo_root}/docs/WorkItems.md" "${tmp}/docs/WorkItems.md"
  cp \
    "${repo_root}/docs/design/work-item-sequencing-and-prioritization.md" \
    "${tmp}/docs/design/work-item-sequencing-and-prioritization.md"
  cp \
    "${repo_root}/docs/design/causal-wal-end-to-end.md" \
    "${tmp}/docs/design/causal-wal-end-to-end.md"
  cp \
    "${repo_root}/docs/design/wal-wsc-durability-roadmap.md" \
    "${tmp}/docs/design/wal-wsc-durability-roadmap.md"
  cp \
    "${repo_root}/docs/releases/echo-1.0-contract.md" \
    "${tmp}/docs/releases/echo-1.0-contract.md"
}

make_fixture() {
  local outvar="$1"
  local fixture_dir
  fixture_dir="$(mktemp -d)"
  tmp_dirs+=("$fixture_dir")
  copy_fixture "$fixture_dir"
  printf -v "$outvar" '%s' "$fixture_dir"
}

test_current_repo_passes() {
  "$checker" >/dev/null
}

test_isolated_fixture_passes() {
  local tmp
  make_fixture tmp
  ECHO_REPO_ROOT="$tmp" "$checker" >/dev/null
}

test_missing_bootstrap_phrase_fails() {
  local tmp out
  make_fixture tmp

  awk '
    { gsub(/configured WAL root or storage manifest/, "configured in-memory graph facts"); print }
  ' "${tmp}/docs/design/causal-wal-end-to-end.md" \
    >"${tmp}/docs/design/causal-wal-end-to-end.md.tmp"
  mv \
    "${tmp}/docs/design/causal-wal-end-to-end.md.tmp" \
    "${tmp}/docs/design/causal-wal-end-to-end.md"

  out="$({ ECHO_REPO_ROOT="$tmp" "$checker"; } 2>&1 || true)"
  echo "$out" | grep -q "WAL design names recovery bootstrap source" || {
    echo "$out" >&2
    fail "checker did not report the missing recovery bootstrap phrase"
  }
}

test_stale_workitems_backlog_link_fails() {
  local tmp out
  make_fixture tmp

  cat >>"${tmp}/docs/WorkItems.md" <<'EOF'

- [WAL/WSC Storage Relationship](method/backlog/v0.1.0/PLATFORM_wal-wsc-storage-relationship.md)
EOF

  out="$({ ECHO_REPO_ROOT="$tmp" "$checker"; } 2>&1 || true)"
  echo "$out" | grep -q "Work boundary removes stale WAL/WSC backlog link" || {
    echo "$out" >&2
    fail "checker did not report the stale WorkItems WAL/WSC backlog link"
  }
}

test_missing_release_project_link_fails() {
  local tmp out
  make_fixture tmp

  awk '
    { gsub(/https:\/\/github.com\/users\/flyingrobots\/projects\/14/, "https://example.invalid/project"); print }
  ' "${tmp}/docs/releases/echo-1.0-contract.md" \
    >"${tmp}/docs/releases/echo-1.0-contract.md.tmp"
  mv \
    "${tmp}/docs/releases/echo-1.0-contract.md.tmp" \
    "${tmp}/docs/releases/echo-1.0-contract.md"

  out="$({ ECHO_REPO_ROOT="$tmp" "$checker"; } 2>&1 || true)"
  echo "$out" | grep -q "release contract links Echo 1.0 Project" || {
    echo "$out" >&2
    fail "checker did not report the missing release contract Project link"
  }
}

test_live_roadmap_issue_map_fails() {
  local tmp out
  make_fixture tmp

  cat >>"${tmp}/docs/design/wal-wsc-durability-roadmap.md" <<'EOF'

## Roadmap Issue Map
EOF

  out="$({ ECHO_REPO_ROOT="$tmp" "$checker"; } 2>&1 || true)"
  echo "$out" | grep -q "WAL doctrine removes roadmap issue map" || {
    echo "$out" >&2
    fail "checker did not report the live roadmap issue map"
  }
}

test_live_workitems_audit_fails() {
  local tmp out
  make_fixture tmp

  cat >>"${tmp}/docs/WorkItems.md" <<'EOF'

Last audited: whenever.
EOF

  out="$({ ECHO_REPO_ROOT="$tmp" "$checker"; } 2>&1 || true)"
  echo "$out" | grep -q "Work boundary removes audit date" || {
    echo "$out" >&2
    fail "checker did not report the live WorkItems audit marker"
  }
}

main() {
  [[ -x "$checker" ]] || fail "checker script missing or not executable: $checker"

  test_current_repo_passes
  test_isolated_fixture_passes
  test_missing_bootstrap_phrase_fails
  test_stale_workitems_backlog_link_fails
  test_missing_release_project_link_fails
  test_live_roadmap_issue_map_fails
  test_live_workitems_audit_fails
}

main "$@"
