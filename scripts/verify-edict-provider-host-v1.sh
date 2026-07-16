#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly MANIFEST="$ROOT/tests/edict-provider-host-v1/Cargo.toml"
readonly HOST_TARGET_DIR="$ROOT/target/edict-provider-host-v1"
readonly COMPONENT_TARGET_DIR="$ROOT/target/provider-lowerer-local"
readonly VERIFIER_COMPONENT_TARGET_DIR="$ROOT/target/provider-verifier-local"

component="${ECHO_PROVIDER_LOWERER_COMPONENT:-schemas/edict-provider/components/v1/lowerer.echo-dpo.component.wasm}"
if [[ "$component" != /* ]]; then
  component="$ROOT/$component"
fi
readonly component

verifier_component="${ECHO_PROVIDER_VERIFIER_COMPONENT:-schemas/edict-provider/components/v1/verifier.echo-dpo.component.wasm}"
if [[ "$verifier_component" != /* ]]; then
  verifier_component="$ROOT/$verifier_component"
fi
readonly verifier_component

cd "$ROOT"

cargo +1.90.0 xtask provider-lowerer-component build \
  --target-dir "$COMPONENT_TARGET_DIR"
cargo +1.90.0 xtask provider-lowerer-component audit \
  --input "$component"
cargo +1.90.0 xtask provider-verifier-component build \
  --target-dir "$VERIFIER_COMPONENT_TARGET_DIR"
cargo +1.90.0 xtask provider-verifier-component audit \
  --input "$verifier_component"

cargo +1.94.0 fmt --manifest-path "$MANIFEST" --all -- --check

ECHO_PROVIDER_LOWERER_COMPONENT="$component" \
  ECHO_PROVIDER_VERIFIER_COMPONENT="$verifier_component" \
  CARGO_TARGET_DIR="$HOST_TARGET_DIR" \
  cargo +1.94.0 test \
    --manifest-path "$MANIFEST" \
    --locked \
    --test host_contract

CARGO_TARGET_DIR="$HOST_TARGET_DIR" \
  cargo +1.94.0 test \
    --manifest-path "$MANIFEST" \
    --locked \
    --test package_contract

CARGO_TARGET_DIR="$HOST_TARGET_DIR" \
  cargo +1.94.0 test \
    --manifest-path "$MANIFEST" \
    --locked \
    --test verifier_resource_sync

CARGO_TARGET_DIR="$HOST_TARGET_DIR" \
  cargo +1.94.0 clippy \
    --manifest-path "$MANIFEST" \
    --locked \
    --all-targets \
    -- \
    -D warnings
