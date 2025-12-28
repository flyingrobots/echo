#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

set -euo pipefail

script_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_root}/../.." && pwd)"
audit_script="${repo_root}/scripts/run_cargo_audit.sh"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

[[ -x "$audit_script" ]] || fail "cargo-audit runner missing or not executable: $audit_script"

# Prevent drift: ignore IDs must live in deny.toml, not hardcoded in the runner script.
if grep -q "RUSTSEC-" "$audit_script"; then
  fail "scripts/run_cargo_audit.sh must not hardcode advisory IDs; source them from deny.toml instead"
fi

if ! grep -q "deny\\.toml" "$audit_script"; then
  fail "scripts/run_cargo_audit.sh should reference deny.toml as its ignore source-of-truth"
fi

exit 0
