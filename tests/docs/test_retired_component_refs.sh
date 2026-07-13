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

check_missing() {
  local label="$1"
  local path="$2"

  if [[ -e "${path}" ]]; then
    echo "retired-component-path: ${label}: ${path}" >&2
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

check_missing \
  "the unimplemented WARP view protocol spec must stay retired" \
  "docs/spec/warp-view-protocol.md"

check_missing \
  "the unused TTD WASM ABI module must stay retired" \
  "crates/echo-wasm-abi/src/ttd.rs"

check_absent \
  "the WASM ABI root must not export the retired TTD module" \
  "crates/echo-wasm-abi/src/lib.rs" \
  '^pub (mod ttd|use ttd::\*);$'

check_absent \
  "the fixed-timestep invariant must not classify deleted TTD protocol fields" \
  "docs/invariants/FIXED-TIMESTEP.md" \
  'TtdrHeader|TTD protocol|Generated TTD protocol|Legacy `OpEnvelope\.ts`'

check_absent \
  "the canonical codec must not cite a deleted crate as its authority" \
  "crates/echo-wasm-abi/src/canonical.rs" \
  'echo-session-proto'

check_absent \
  "the workspace lockfile must not retain the retired TTD protocol importer" \
  "pnpm-lock.yaml" \
  '^[[:space:]]+packages/ttd-protocol-ts:'

check_absent \
  "the changelog must describe only tracked PR deletions" \
  "CHANGELOG.md" \
  'untracked `ttd-browser` prebuilt artifact'

for map_file in \
  docs/README.md \
  crates/echo-graph/README.md \
  crates/warp-wasm/README.md; do
  check_absent \
    "live docs must not advertise the retired WARP view protocol" \
    "${map_file}" \
    'warp-view-protocol'
done

check_absent \
  "echo-dry-tests docs must not advertise the deleted config fake" \
  "crates/echo-dry-tests/README.md" \
  'InMemoryConfigStore|[Cc]onfiguration store'

check_absent \
  "echo-dry-tests must not retain config-fake serialization dependencies" \
  "crates/echo-dry-tests/Cargo.toml" \
  '^serde(_json)?[[:space:]]*='

for architecture_file in \
  ARCHITECTURE.md \
  crates/echo-graph/README.md \
  crates/echo-wasm-abi/README.md; do
  check_absent \
    "live architecture docs must not name deleted crates as active peers" \
    "${architecture_file}" \
    'echo-app-core|echo-session-proto'
done

if ((failures > 0)); then
  echo "retired-component-ref: ${failures} violation(s)" >&2
  exit 1
fi

echo "retired-component-ref: all checks passed"
