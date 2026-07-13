#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

failures=0

check_absent() {
  local label="$1"
  local file="$2"
  local pattern="$3"
  local matches

  if matches="$(rg -n -- "${pattern}" "${file}")"; then
    echo "retired-component-ref: ${label}" >&2
    echo "${matches}" >&2
    failures=$((failures + 1))
  fi
}

check_absent \
  "GUIDE must not link to the retired echo-ttd crate" \
  "GUIDE.md" \
  '\]\(\./crates/echo-ttd\)'

check_absent \
  "WASM ABI spec must not cite the retired session-protocol EINT implementation" \
  "docs/spec/SPEC-0009-wasm-abi-v3.md" \
  'crates/echo-session-proto/src/eint_v2\.rs'

if ((failures > 0)); then
  echo "retired-component-ref: ${failures} violation(s)" >&2
  exit 1
fi

echo "retired-component-ref: all checks passed"
