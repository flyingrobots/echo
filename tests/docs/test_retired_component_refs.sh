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
  "GUIDE must not advertise the frozen advanced guide as doctrine" \
  "GUIDE.md" \
  'ADVANCED_GUIDE\.md'

check_absent \
  "GUIDE must not describe Echo as a graph-orchestrating simulation engine" \
  "GUIDE.md" \
  'simulation engine orchestrates the causal graph|application or game'

check_absent \
  "GUIDE must not advertise the deleted App Core surface" \
  "GUIDE.md" \
  'App Core'

check_absent \
  "GUIDE must not teach handwritten rule authorship" \
  "GUIDE.md" \
  'writing a new rule|Declare your `Footprint`'

check_absent \
  "GUIDE must not advertise the nonexistent DIND seed option" \
  "GUIDE.md" \
  'dind run --seed'

for required_guide_claim in \
  'witnessed causal history' \
  'generated contracts' \
  'cargo xtask dind run --emit-repro'; do
  if ! rg -q -F -- "${required_guide_claim}" GUIDE.md; then
    echo "retired-component-ref: GUIDE missing current claim: ${required_guide_claim}" >&2
    failures=$((failures + 1))
  fi
done

check_absent \
  "WASM ABI spec must not cite the retired session-protocol EINT implementation" \
  "docs/spec/SPEC-0009-wasm-abi-v3.md" \
  'crates/echo-session-proto/src/eint_v2\.rs'

check_absent \
  "historical ABI v3 spec must not claim active or current authority" \
  "docs/spec/SPEC-0009-wasm-abi-v3.md" \
  'Status:\*\* Active|specifies the current WASM|current WASM export'

for required_v3_claim in \
  '**Status:** Superseded' \
  '[canonical WASM ABI specification](SPEC-0009-wasm-abi.md)'; do
  if ! rg -q -F -- "${required_v3_claim}" docs/spec/SPEC-0009-wasm-abi-v3.md; then
    echo "retired-component-ref: historical ABI v3 spec missing: ${required_v3_claim}" >&2
    failures=$((failures + 1))
  fi
done

check_missing \
  "the unimplemented WARP view protocol spec must stay retired" \
  "docs/spec/warp-view-protocol.md"

check_missing \
  "the retired WARP view dashboard assets must stay retired" \
  "docs/assets/wvp"

check_missing \
  "the empty-value unordered ABI exemption file must stay retired" \
  ".ban-unordered-abi-allowlist"

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

check_absent \
  "the changelog must not resurrect the retired session-protocol layer" \
  "CHANGELOG.md" \
  'session-protocol EINT v2'

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
