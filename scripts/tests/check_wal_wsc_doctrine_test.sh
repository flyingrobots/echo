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
  mkdir -p "${tmp}/docs/architecture" "${tmp}/docs/releases" "${tmp}/docs/topics"
  cp "${repo_root}/docs/topics/WAL.md" "${tmp}/docs/topics/WAL.md"
  cp "${repo_root}/docs/topics/RuntimeAuthority.md" "${tmp}/docs/topics/RuntimeAuthority.md"
  cp "${repo_root}/docs/architecture/continuum-transport.md" "${tmp}/docs/architecture/continuum-transport.md"
  cp "${repo_root}/docs/releases/echo-1.0-contract.md" "${tmp}/docs/releases/echo-1.0-contract.md"
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
    { gsub(/projected WAL root or storage manifest/, "projected graph state"); print }
  ' "${tmp}/docs/topics/WAL.md" >"${tmp}/docs/topics/WAL.md.tmp"
  mv "${tmp}/docs/topics/WAL.md.tmp" "${tmp}/docs/topics/WAL.md"

  out="$({ ECHO_REPO_ROOT="$tmp" "$checker"; } 2>&1 || true)"
  echo "$out" | grep -q "WAL topic names recovery bootstrap source" || {
    echo "$out" >&2
    fail "checker did not report the missing recovery bootstrap phrase"
  }
}

test_missing_runtime_authority_phrase_fails() {
  local tmp out
  make_fixture tmp

  awk '
    {
      gsub(/An admission ticket witnesses lawful eligibility. It is not execution./, "An admission ticket executes work.")
      print
    }
  ' "${tmp}/docs/topics/RuntimeAuthority.md" >"${tmp}/docs/topics/RuntimeAuthority.md.tmp"
  mv "${tmp}/docs/topics/RuntimeAuthority.md.tmp" "${tmp}/docs/topics/RuntimeAuthority.md"

  out="$({ ECHO_REPO_ROOT="$tmp" "$checker"; } 2>&1 || true)"
  echo "$out" | grep -q "runtime authority distinguishes admission from execution" || {
    echo "$out" >&2
    fail "checker did not report the missing admission boundary"
  }
}

test_missing_release_project_link_fails() {
  local tmp out
  make_fixture tmp

  awk '
    { gsub(/https:\/\/github.com\/users\/flyingrobots\/projects\/15/, "https://example.invalid/project"); print }
  ' "${tmp}/docs/releases/echo-1.0-contract.md" >"${tmp}/docs/releases/echo-1.0-contract.md.tmp"
  mv "${tmp}/docs/releases/echo-1.0-contract.md.tmp" "${tmp}/docs/releases/echo-1.0-contract.md"

  out="$({ ECHO_REPO_ROOT="$tmp" "$checker"; } 2>&1 || true)"
  echo "$out" | grep -q "release contract links Continuum Stack Convergence Project" || {
    echo "$out" >&2
    fail "checker did not report the missing release contract Project link"
  }
}

test_release_contract_decouples_edict() {
  local release_contract
  release_contract="${repo_root}/docs/releases/echo-1.0-contract.md"

  if grep -Fq -- "https://github.com/flyingrobots/echo/issues/589" "$release_contract"; then
    fail "Echo 1.0 release contract still gates release on Edict issue #589"
  fi

  grep -Fq -- 'Edict and `jedit` compatibility work does not gate Echo 1.0.' "$release_contract" || {
    fail "Echo 1.0 release contract does not state the Edict decoupling decision"
  }
}

test_checker_rejects_edict_release_gate() {
  local tmp out
  make_fixture tmp

  printf '\n%s\n' "https://github.com/flyingrobots/echo/issues/589" >>"${tmp}/docs/releases/echo-1.0-contract.md"

  out="$({ ECHO_REPO_ROOT="$tmp" "$checker"; } 2>&1 || true)"
  echo "$out" | grep -q "release contract rejects Edict release gate" || {
    echo "$out" >&2
    fail "checker did not reject the retired Edict release gate"
  }
}

test_missing_duplicate_replay_law_fails() {
  local tmp out
  make_fixture tmp

  awk '
    { gsub(/Duplicate replay is idempotent./, "Duplicate replay is best effort."); print }
  ' "${tmp}/docs/topics/WAL.md" >"${tmp}/docs/topics/WAL.md.tmp"
  mv "${tmp}/docs/topics/WAL.md.tmp" "${tmp}/docs/topics/WAL.md"

  out="$({ ECHO_REPO_ROOT="$tmp" "$checker"; } 2>&1 || true)"
  echo "$out" | grep -q "WAL topic requires idempotent duplicate replay" || {
    echo "$out" >&2
    fail "checker did not report the missing duplicate replay law"
  }
}

test_stale_durability_claims_fail() {
  local tmp out
  make_fixture tmp

  cat >>"${tmp}/docs/topics/WAL.md" <<'EOF'

filesystem runtime WAL witness is missing
WSC import recovery is authoritative without WAL-backed validation
retained payload recovery can rely on posture-only refs
EOF

  out="$({ ECHO_REPO_ROOT="$tmp" "$checker"; } 2>&1 || true)"
  echo "$out" | grep -q "durability docs reject missing filesystem runtime WAL witness claim" || {
    echo "$out" >&2
    fail "checker did not report stale filesystem runtime WAL witness claim"
  }
  echo "$out" | grep -q "durability docs reject premature WSC import authority" || {
    echo "$out" >&2
    fail "checker did not report stale WSC import authority claim"
  }
  echo "$out" | grep -q "durability docs reject posture-only retained payload recovery" || {
    echo "$out" >&2
    fail "checker did not report stale retained payload recovery claim"
  }
}

main() {
  [[ -x "$checker" ]] || fail "checker script missing or not executable: $checker"

  test_current_repo_passes
  test_isolated_fixture_passes
  test_missing_bootstrap_phrase_fails
  test_missing_runtime_authority_phrase_fails
  test_missing_release_project_link_fails
  test_release_contract_decouples_edict
  test_checker_rejects_edict_release_gate
  test_missing_duplicate_replay_law_fails
  test_stale_durability_claims_fail
}

main "$@"
