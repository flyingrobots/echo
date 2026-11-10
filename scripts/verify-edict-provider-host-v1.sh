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

cargo +1.96.0 xtask provider-lowerer-component build \
  --target-dir "$COMPONENT_TARGET_DIR"
cargo +1.96.0 xtask provider-lowerer-component audit \
  --input "$component"
cargo +1.96.0 xtask provider-verifier-component build \
  --target-dir "$VERIFIER_COMPONENT_TARGET_DIR"
cargo +1.96.0 xtask provider-verifier-component audit \
  --input "$verifier_component"

cargo +1.96.0 fmt --manifest-path "$MANIFEST" --all -- --check

CARGO_TARGET_DIR="$HOST_TARGET_DIR" \
  cargo +1.96.0 test \
    --manifest-path "$MANIFEST" \
    --locked \
    --test conformance_contract

ECHO_PROVIDER_LOWERER_COMPONENT="$component" \
  ECHO_PROVIDER_VERIFIER_COMPONENT="$verifier_component" \
  CARGO_TARGET_DIR="$HOST_TARGET_DIR" \
  cargo +1.96.0 test \
    --manifest-path "$MANIFEST" \
    --locked \
    --test host_contract

CARGO_TARGET_DIR="$HOST_TARGET_DIR" \
  cargo +1.96.0 test \
    --manifest-path "$MANIFEST" \
    --locked \
    --test generated_helper_contract

CARGO_TARGET_DIR="$HOST_TARGET_DIR" \
  cargo +1.96.0 test \
    --manifest-path "$MANIFEST" \
    --locked \
    --test package_contract

CARGO_TARGET_DIR="$HOST_TARGET_DIR" \
  cargo +1.96.0 test \
    --manifest-path "$MANIFEST" \
    --locked \
    --test verifier_resource_sync

CARGO_TARGET_DIR="$HOST_TARGET_DIR" \
  cargo +1.96.0 clippy \
    --manifest-path "$MANIFEST" \
    --locked \
    --all-targets \
    -- \
    -D warnings

# Public compiler witness for typed outgoing-edge and guided closure reads.
# The unmodified Amblicit compiler, built from a pinned public revision,
# compiles authored source through the checked provider package and its
# separate verifier; the predecessor package must refuse the same source.
# Building the compiler is heavy, so a local run must opt in explicitly.
if [[ "${CI:-}" == "true" || "${ECHO_GRAPH_READ_PUBLIC_WITNESS:-}" == "1" ]]; then
  readonly AMBLICIT_REVISION=66bf6bac369f67e32ddaee099c3198bd30f885a1
  readonly OLD_PROVIDER_REVISION=089c849f766d5331835e7d10148a378cdb1237d5
  readonly AMBLICIT_ROOT="$ROOT/target/amblicit-$AMBLICIT_REVISION"
  readonly GRAPH_READ_FIXTURES="$ROOT/crates/warp-core/tests/fixtures/edict-graph-read"

  if [[ ! -x "$AMBLICIT_ROOT/bin/amblicit" ]]; then
    cargo +1.96.0 install --locked \
      --git https://github.com/flyingrobots/amblicit \
      --rev "$AMBLICIT_REVISION" \
      --root "$AMBLICIT_ROOT" \
      --bin amblicit \
      amblicit-cli
  fi

  if ! git -C "$ROOT" cat-file -e "$OLD_PROVIDER_REVISION^{commit}" 2>/dev/null; then
    git -C "$ROOT" fetch --no-tags --depth=1 origin "$OLD_PROVIDER_REVISION"
  fi
  mkdir -p "$ROOT/target"
  witness_dir="$(mktemp -d "$ROOT/target/graph-read-witness.XXXXXX")"
  readonly witness_dir
  mkdir "$witness_dir/old-provider"
  git -C "$ROOT" archive "$OLD_PROVIDER_REVISION" schemas/edict-provider/package/v1 |
    tar -x -C "$witness_dir/old-provider"

  cargo +1.96.0 run --locked -p xtask --example graph_read_publication -- \
    --compiler "$AMBLICIT_ROOT/bin/amblicit" \
    --provider "$ROOT/schemas/edict-provider/package/v1" \
    --old-provider "$witness_dir/old-provider/schemas/edict-provider/package/v1" \
    --fixtures "$GRAPH_READ_FIXTURES" \
    --work-root "$witness_dir/run" \
    --check
fi
