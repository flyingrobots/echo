<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0016 - Wesley To Echo Toy Contract Proof

_Prove one boring Wesley-generated contract path from generated op metadata to
EINT dispatch and an observation/read bridge._

Legend: [PLATFORM](../../method/legends/PLATFORM.md)

Depends on:

- [0013 - Wesley Compiled Contract Hosting Doctrine](../0013-wesley-compiled-contract-hosting-doctrine/design.md)
- [0014 - EINT, Registry, And Observation Boundary Inventory](../0014-eint-registry-observation-boundary-inventory/design.md)
- [0015 - Registry Provider Host Boundary Decision](../0015-registry-provider-host-boundary-decision/design.md)
- [Retro: 0016 - Wesley To Echo Toy Contract Proof](../../method/retro/0016-wesley-to-echo-toy-contract-proof/retro.md)

## Status

Accepted.

## Hill

A tiny generated contract can use existing Echo substrate:

```text
GraphQL / Wesley IR
  -> echo-wesley-gen generated Rust
  -> generated op id and REGISTRY
  -> generated app-level EINT helper
  -> dispatch_intent(...)
  -> generated ObservationRequest helper
  -> consumer-owned KernelPort::observe(...) / ReadingEnvelope handling
```

## Opening constraint

Do not create a new intent envelope, registry trait, ABI, host-side app payload
validator, dynamic loading path, or jedit-specific API.

The first proof must reuse:

- EINT v1;
- `pack_intent_v1(...)`;
- `dispatch_intent(...)`;
- `RegistryProvider`;
- generated `REGISTRY`;
- `observe(...)` / `ReadingEnvelope` if the current read boundary can carry the
  toy query.

## RED witness

Focused test:

```sh
cargo test -p echo-wesley-gen test_toy_contract_generates_eint_and_observation_helpers
```

The test uses a tiny counter contract:

- mutation: `increment(input: IncrementInput): CounterValue`
- query: `counterValue: CounterValue`

The canonical toy counter IR fixture is:

```text
crates/echo-wesley-gen/tests/fixtures/toy-counter/echo-ir-v1.json
```

It first proves current generated output already includes:

- `OP_INCREMENT`
- `OP_COUNTER_VALUE`
- `REGISTRY`

It then requires the missing first-consumer bridge:

- import or use of `pack_intent_v1(...)`;
- generated mutation helper such as `pack_increment_intent`;
- EINT packing with `OP_INCREMENT`;
- generated query/read helper such as `counter_value_observation_request`.

## Expected failure

Current `echo-wesley-gen` emits DTOs, operation constants, operation catalogs,
and `GeneratedRegistry`. It does not yet emit app-level EINT dispatch helpers
or generated observation helpers.

That is the correct RED. The missing piece is not Echo's intent ingress or
registry metadata. The missing piece is generated consumer glue that binds the
existing substrate into a contract-shaped app helper.

Observed RED:

```text
test test_toy_contract_generates_eint_and_observation_helpers ... FAILED
generated toy contract output is missing first-consumer bridge:
use echo_wasm_abi::pack_intent_v1;
```

## GREEN 1 witness

Historical implementation, superseded by GREEN 3:

- `echo-wesley-gen` now imports `pack_intent_v1(...)` when operations are
  present.
- Mutation operations originally emitted raw-vars helpers such as
  `pack_increment_intent(vars)`.
- Query operations originally emitted frontier query-view helpers such as
  `counter_value_observation_request(worldline_id, vars)`.
- Query helpers use the existing `ObservationRequest`,
  `ObservationFrame::QueryView`, and `ObservationProjection::Query` shape.

Focused witness:

```sh
cargo test -p echo-wesley-gen test_toy_contract_generates_eint_and_observation_helpers
```

Result: passed.

## GREEN 2 witness

Implementation:

- `echo-wesley-gen` now has a consumer smoke test that writes generated Rust
  into a temporary standalone crate under `target/`.
- The smoke crate depends on local `echo-wasm-abi` and `echo-registry-api`.
- The smoke crate compiles the generated output as consumer code.
- The smoke crate exercises:
    - generated `REGISTRY` metadata;
    - generated `pack_increment_intent(...)`;
    - EINT unpacking through `unpack_intent_v1(...)`;
    - a toy `KernelPort::dispatch_intent(...)` implementation;
    - generated `counter_value_observation_request(...)`;
    - a toy `KernelPort::observe(...)` implementation returning query bytes.

Focused witness:

```sh
cargo test -p echo-wesley-gen \
  test_toy_contract_generated_output_compiles_in_consumer_crate
```

Result: passed.

## GREEN 3 witness

Implementation:

- `echo-wesley-gen` now emits per-operation vars structs in the generated
  helper namespace, such as `__echo_wesley_generated::IncrementVars` and
  `__echo_wesley_generated::CounterValueVars`.
- Each generated operation receives a canonical vars encoder such as
  `encode_increment_vars(...)`.
- Ergonomic mutation helpers now accept generated vars structs, encode them
  with Echo canonical CBOR, and then pack EINT v1.
- Ergonomic query helpers now accept generated vars structs, encode them with
  Echo canonical CBOR, and then build `ObservationRequest`.
- Raw-vars helpers remain available only under explicit names such as
  `pack_increment_intent_raw_vars(...)` and
  `counter_value_observation_request_raw_vars(...)`.

Focused witness:

```sh
cargo test -p echo-wesley-gen \
  test_toy_contract_generated_output_compiles_in_consumer_crate
```

The smoke kernel now decodes generated EINT vars and query vars through
`decode_cbor(...)` before asserting the app-level contract values. This closes
the accidental nondeterminism seam where the app-facing helper accepted
arbitrary raw vars bytes.

Result: passed.

## GREEN 4 witness

Implementation:

- The toy counter IR is now a named test fixture at
  `crates/echo-wesley-gen/tests/fixtures/toy-counter/echo-ir-v1.json`.
- The fixture has a README that defines the proof boundary and forbids copying
  the toy IR into new tests.
- Both toy generator tests now use the same `include_str!(...)` source.

This prevents future installed-host smoke work from creating a third,
silently-divergent copy of "the toy contract."

Focused witness:

```sh
cargo test -p echo-wesley-gen
```

Result: passed.

Broader generator witness:

```sh
cargo test -p echo-wesley-gen
cargo clippy -p echo-wesley-gen --all-targets -- -D warnings -D missing_docs
```

Result: passed.

## GREEN 5 witness

Implementation:

- `warp-wasm` now exposes native Rust CBOR-envelope helpers matching the
  installed-kernel WASM boundary:
    - `dispatch_intent_cbor(...)`;
    - `observe_cbor(...)`;
    - `get_registry_info_cbor()`.
- These helpers do not change the `wasm_bindgen` exports. They make the
  installed-kernel envelope path testable without `js_sys::Uint8Array`.
- The generated toy consumer smoke crate now depends on local `warp-wasm`.
- The smoke crate installs its application-owned `ToyKernel` with
  `warp_wasm::install_kernel(...)`.
- The smoke crate then verifies:
    - installed registry metadata matches generated `CODEC_ID`,
      `REGISTRY_VERSION`, and `SCHEMA_SHA256`;
    - generated `pack_increment_intent(...)` bytes dispatch through
      `warp_wasm::dispatch_intent_cbor(...)`;
    - generated `counter_value_observation_request(...)` bytes observe through
      `warp_wasm::observe_cbor(...)`;
    - the returned read is a `QueryBytes` `ObservationArtifact`.

Focused witness:

```sh
cargo test -p echo-wesley-gen \
  test_toy_contract_generated_output_compiles_in_consumer_crate
```

Result: passed.

## GREEN direction

GREEN stayed inside `echo-wesley-gen`.

Implemented shape:

- generate typed operation vars structs and canonical vars encoders;
- generate `pack_<mutation>_intent(...)` helpers that canonicalize typed vars
  before calling `pack_intent_v1(OP_..., &vars)`;
- keep explicit raw-vars helpers for plumbing callers that already hold
  canonical vars bytes;
- generate read-helper shapes for query ops that map to `ObservationRequest`;
- keep Echo core app-agnostic;
- keep host-side generated payload validation deferred;
- prove installed-kernel dispatch, observation, and registry metadata through
  `warp-wasm` native CBOR envelope helpers.

Still deferred to follow-on cards:

- contract-aware receipt and reading identity;
- contract artifact retention in `echo-cas`;
- real `jedit` generated fixture hosting;
- dynamic contract loading.

The phrase "actual integration proof" now means an Echo-installed or
application-owned kernel path. This cycle closes that proof for the toy counter
contract without adding app-specific Echo APIs.

## Non-goals

- Do not add a second envelope.
- Do not add a second registry model.
- Do not change `KernelPort`.
- Do not change `warp-wasm` exports.
- Do not implement host-side app payload validation.
- Do not implement dynamic contract loading.
- Do not add text-editing or jedit nouns.
- Do not add Continuum transport.

## Remaining design question

Resolved for the toy proof. `ObservationRequest` can honestly carry the toy
query as `ObservationFrame::QueryView` plus `ObservationProjection::Query`, and
the generated optic read helper can carry it as
`OpticApertureShape::QueryBytes`.

Follow-on cards still need to harden the identity and retention semantics of
those readings before `jedit` uses the path as a serious consumer.
