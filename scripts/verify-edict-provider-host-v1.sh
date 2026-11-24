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

# The shared edict.core-cost/v2 corpus (flyingrobots/echo#1001) must be exactly
# what the pinned Amblicit compiler produces at each program's minimum budgets.
readonly CORPUS_MANIFEST="$ROOT/tests/edict-core-cost-v2-corpus/Cargo.toml"
readonly CORPUS_TARGET_DIR="$ROOT/target/edict-core-cost-v2-corpus"
cargo +1.96.0 fmt --manifest-path "$CORPUS_MANIFEST" --all -- --check
CARGO_TARGET_DIR="$CORPUS_TARGET_DIR" \
  cargo +1.96.0 clippy \
    --manifest-path "$CORPUS_MANIFEST" \
    --locked \
    --all-targets \
    -- \
    -D warnings
CARGO_TARGET_DIR="$CORPUS_TARGET_DIR" \
  cargo +1.96.0 run \
    --release \
    --manifest-path "$CORPUS_MANIFEST" \
    --locked \
    -- "$ROOT/crates/warp-core/tests/fixtures/edict-core-cost-v2" --check

# Public compiler witnesses for typed outgoing-edge and guided closure reads,
# for a flat authored effect program, and for an authored effect program with
# an effect step under the echo.dpo@2 effect target profile.
# The unmodified Amblicit compiler, built from a pinned public revision,
# compiles authored source through the checked provider package and its
# separate verifier; the predecessor package must refuse the same source.
# Building the compiler is heavy, so a local run must opt in explicitly.
if [[ "${CI:-}" == "true" || "${ECHO_GRAPH_READ_PUBLIC_WITNESS:-}" == "1" ]]; then
  readonly AMBLICIT_REVISION=7dbcbc23d72705c5e224fc994a6c767881958912
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
    --control "$ROOT/crates/echo-edict-provider-lowerer/tests/fixtures/node-atom-read/source" \
    --work-root "$witness_dir/run" \
    --check

  # Public compiler witness for a flat authored effect program: the checked
  # lowerer emits an edict-effect-program/v1 package and the separate verifier
  # accepts it; the predecessor provider must refuse the same source.
  readonly OLD_EFFECT_PROVIDER_REVISION=a9792ddc4d757a6c01e2e190c41ec9b521706472
  if ! git -C "$ROOT" cat-file -e "$OLD_EFFECT_PROVIDER_REVISION^{commit}" 2>/dev/null; then
    git -C "$ROOT" fetch --no-tags --depth=1 origin "$OLD_EFFECT_PROVIDER_REVISION"
  fi
  mkdir "$witness_dir/old-effect-provider"
  git -C "$ROOT" archive "$OLD_EFFECT_PROVIDER_REVISION" schemas/edict-provider/package/v1 |
    tar -x -C "$witness_dir/old-effect-provider"

  cargo +1.96.0 run --locked -p xtask --example effect_program_publication -- \
    --compiler "$AMBLICIT_ROOT/bin/amblicit" \
    --provider "$ROOT/schemas/edict-provider/package/v1" \
    --old-provider "$witness_dir/old-effect-provider/schemas/edict-provider/package/v1" \
    --fixtures "$ROOT/crates/warp-core/tests/fixtures/edict-effect-program" \
    --work-root "$witness_dir/effect-run" \
    --check

  # Public compiler witness for a source helper with a precondition (#989):
  # the call is proven by the intent's `where` clause, the checked lowerer
  # emits the package and the verifier accepts it; the predecessor provider's
  # contract pack predates preconditions and must refuse the Core.
  cargo +1.96.0 run --locked -p xtask --example effect_program_publication -- \
    --name precondition \
    --compiler "$AMBLICIT_ROOT/bin/amblicit" \
    --provider "$ROOT/schemas/edict-provider/package/v1" \
    --old-provider "$witness_dir/old-effect-provider/schemas/edict-provider/package/v1" \
    --fixtures "$ROOT/crates/warp-core/tests/fixtures/edict-function-precondition" \
    --work-root "$witness_dir/precondition-run" \
    --check

  # Public compiler witness for source helpers that build, test and consume an
  # Option (#995): the checked lowerer emits the package and the verifier
  # accepts it; the provider promoted before Option support must refuse it at
  # Echo's lowerer.
  readonly OLD_OPTION_PROVIDER_REVISION=d527f47176d506deae30d3ee4bbf36b924e7bb30
  if ! git -C "$ROOT" cat-file -e "$OLD_OPTION_PROVIDER_REVISION^{commit}" 2>/dev/null; then
    git -C "$ROOT" fetch --no-tags --depth=1 origin "$OLD_OPTION_PROVIDER_REVISION"
  fi
  mkdir "$witness_dir/old-option-provider"
  git -C "$ROOT" archive "$OLD_OPTION_PROVIDER_REVISION" schemas/edict-provider/package/v1 |
    tar -x -C "$witness_dir/old-option-provider"
  cargo +1.96.0 run --locked -p xtask --example effect_program_publication -- \
    --name option \
    --compiler "$AMBLICIT_ROOT/bin/amblicit" \
    --provider "$ROOT/schemas/edict-provider/package/v1" \
    --old-provider "$witness_dir/old-option-provider/schemas/edict-provider/package/v1" \
    --fixtures "$ROOT/crates/warp-core/tests/fixtures/edict-option-value" \
    --work-root "$witness_dir/option-run" \
    --check

  # Public compiler witness for every checked integer form and min/max (#998):
  # the checked lowerer emits the package and the verifier accepts it; the
  # provider promoted before checked-integer support must refuse it at Echo's
  # lowerer.
  readonly OLD_CHECKED_PROVIDER_REVISION=9141e79519d3fb8253a928c60d6b42cff76b31af
  if ! git -C "$ROOT" cat-file -e "$OLD_CHECKED_PROVIDER_REVISION^{commit}" 2>/dev/null; then
    git -C "$ROOT" fetch --no-tags --depth=1 origin "$OLD_CHECKED_PROVIDER_REVISION"
  fi
  mkdir "$witness_dir/old-checked-provider"
  git -C "$ROOT" archive "$OLD_CHECKED_PROVIDER_REVISION" schemas/edict-provider/package/v1 |
    tar -x -C "$witness_dir/old-checked-provider"
  cargo +1.96.0 run --locked -p xtask --example effect_program_publication -- \
    --name checked \
    --compiler "$AMBLICIT_ROOT/bin/amblicit" \
    --provider "$ROOT/schemas/edict-provider/package/v1" \
    --old-provider "$witness_dir/old-checked-provider/schemas/edict-provider/package/v1" \
    --fixtures "$ROOT/crates/warp-core/tests/fixtures/edict-checked-integer" \
    --work-root "$witness_dir/checked-run" \
    --check

  # Public compiler witness for an authored effect program with an effect step
  # under the echo.dpo@2 effect target profile (#985): the checked lowerer
  # emits its package and the separate verifier accepts it; the same program
  # under echo.dpo@1, and under a substituted echo.dpo@2 profile, must be
  # refused at Echo's lowerer, and so must the same read called without a
  # failure map although its intrinsic declares failures.
  cargo +1.96.0 run --locked -p xtask --example effect_step_publication -- \
    --name read \
    --compiler "$AMBLICIT_ROOT/bin/amblicit" \
    --provider "$ROOT/schemas/edict-provider/package/v1" \
    --fixtures "$ROOT/crates/warp-core/tests/fixtures/edict-effect-step" \
    --work-root "$witness_dir/effect-step-run" \
    --check

  # The same witness for a program calling all five echo.dpo@2 mutation
  # intrinsics without a failure map (amblicit#311), with the same controls.
  cargo +1.96.0 run --locked -p xtask --example effect_step_publication -- \
    --name write \
    --compiler "$AMBLICIT_ROOT/bin/amblicit" \
    --provider "$ROOT/schemas/edict-provider/package/v1" \
    --fixtures "$ROOT/crates/warp-core/tests/fixtures/edict-effect-mutations" \
    --work-root "$witness_dir/effect-mutations-run" \
    --check

  # Public source witness for a bounded loop and both statement-branch arms,
  # compiled to structured echo.span-ir/v3 under the current echo.dpo@2 profile.
  cargo +1.96.0 run --locked -p xtask --example effect_step_publication -- \
    --name structured \
    --compiler "$AMBLICIT_ROOT/bin/amblicit" \
    --provider "$ROOT/schemas/edict-provider/package/v1" \
    --fixtures "$ROOT/crates/warp-core/tests/fixtures/edict-effect-structured" \
    --work-root "$witness_dir/effect-structured-run" \
    --check

  # The same witness for a program whose helper comes from a digest-pinned
  # source module (echo#1009): the compiler sends an edict.source-closure/v1
  # source and the lowerer checks its `use module` pins.
  cargo +1.96.0 run --locked -p xtask --example effect_step_publication -- \
    --name source-module \
    --compiler "$AMBLICIT_ROOT/bin/amblicit" \
    --provider "$ROOT/schemas/edict-provider/package/v1" \
    --fixtures "$ROOT/crates/warp-core/tests/fixtures/edict-effect-source-module" \
    --work-root "$witness_dir/effect-source-module-run" \
    --check

  # The same witness for a program that reads a node's outgoing edges through
  # echo.dpo@2.outgoing-edge-read and writes an edge to what it read
  # (echo#982), with the same echo.dpo@1 and substituted-profile controls.
  cargo +1.96.0 run --locked -p xtask --example effect_step_publication -- \
    --name link \
    --compiler "$AMBLICIT_ROOT/bin/amblicit" \
    --provider "$ROOT/schemas/edict-provider/package/v1" \
    --fixtures "$ROOT/crates/warp-core/tests/fixtures/edict-effect-graph-read" \
    --work-root "$witness_dir/effect-graph-read-run" \
    --check

  # The same witness for a program that reads a guided closure through
  # echo.dpo@2.closure-read with a declared selector (echo#1016).
  cargo +1.96.0 run --locked -p xtask --example effect_step_publication -- \
    --name closure \
    --compiler "$AMBLICIT_ROOT/bin/amblicit" \
    --provider "$ROOT/schemas/edict-provider/package/v1" \
    --fixtures "$ROOT/crates/warp-core/tests/fixtures/edict-effect-closure-read" \
    --work-root "$witness_dir/effect-closure-read-run" \
    --check

  # Public source witness for a helper with a declared typed failure that
  # the intent maps to a terminal obstruction (core.failure/v1, #996).
  cargo +1.96.0 run --locked -p xtask --example effect_step_publication -- \
    --name typed \
    --compiler "$AMBLICIT_ROOT/bin/amblicit" \
    --provider "$ROOT/schemas/edict-provider/package/v1" \
    --fixtures "$ROOT/crates/warp-core/tests/fixtures/edict-effect-typed-failure" \
    --work-root "$witness_dir/effect-typed-failure-run" \
    --check

  # Public source witness for a fallible record whose authored field order
  # decides the escaping failure (orderedRecord, #997): { z: fail First, a: fail Second }.
  cargo +1.96.0 run --locked -p xtask --example effect_step_publication -- \
    --name ordered-z \
    --compiler "$AMBLICIT_ROOT/bin/amblicit" \
    --provider "$ROOT/schemas/edict-provider/package/v1" \
    --fixtures "$ROOT/crates/warp-core/tests/fixtures/edict-effect-ordered-record-z" \
    --work-root "$witness_dir/effect-ordered-record-z-run" \
    --check

  # Public source witness for a fallible record whose authored field order
  # decides the escaping failure (orderedRecord, #997): { a: fail Second, z: fail First }.
  cargo +1.96.0 run --locked -p xtask --example effect_step_publication -- \
    --name ordered-a \
    --compiler "$AMBLICIT_ROOT/bin/amblicit" \
    --provider "$ROOT/schemas/edict-provider/package/v1" \
    --fixtures "$ROOT/crates/warp-core/tests/fixtures/edict-effect-ordered-record-a" \
    --work-root "$witness_dir/effect-ordered-record-a-run" \
    --check

  # Public source witness for the three echo.dpo@2 ContentAddressV1 identity
  # constructors, bound to the catalog v2 contracts (echo#972).
  cargo +1.96.0 run --locked -p xtask --example effect_step_publication -- \
    --name identity \
    --compiler "$AMBLICIT_ROOT/bin/amblicit" \
    --provider "$ROOT/schemas/edict-provider/package/v1" \
    --fixtures "$ROOT/crates/warp-core/tests/fixtures/edict-content-address" \
    --work-root "$witness_dir/effect-identity-run" \
    --check

  # Public source witness for a Buffer bump whose ReplaceAtom names the
  # attachment value digest of the atom it read (echo#1011), and its control
  # that digests bytes it never read.
  cargo +1.96.0 run --locked -p xtask --example effect_step_publication -- \
    --name bump \
    --compiler "$AMBLICIT_ROOT/bin/amblicit" \
    --provider "$ROOT/schemas/edict-provider/package/v1" \
    --fixtures "$ROOT/crates/warp-core/tests/fixtures/edict-attachment-digest" \
    --work-root "$witness_dir/effect-bump-run" \
    --check

  cargo +1.96.0 run --locked -p xtask --example effect_step_publication -- \
    --name unread \
    --compiler "$AMBLICIT_ROOT/bin/amblicit" \
    --provider "$ROOT/schemas/edict-provider/package/v1" \
    --fixtures "$ROOT/crates/warp-core/tests/fixtures/edict-attachment-digest/controls/unread-bytes" \
    --work-root "$witness_dir/effect-unread-run" \
    --check

  # The same identity constructors called from a source helper and from a
  # fold step (echo#1005), with the same echo.dpo@1 and substituted-profile
  # controls.
  cargo +1.96.0 run --locked -p xtask --example effect_step_publication -- \
    --name helper \
    --compiler "$AMBLICIT_ROOT/bin/amblicit" \
    --provider "$ROOT/schemas/edict-provider/package/v1" \
    --fixtures "$ROOT/crates/warp-core/tests/fixtures/edict-helper-identity" \
    --work-root "$witness_dir/effect-helper-identity-run" \
    --check
fi
