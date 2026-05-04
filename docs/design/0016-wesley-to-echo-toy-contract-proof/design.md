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
- [Wesley To Echo Toy Contract Proof](../../method/backlog/up-next/PLATFORM_wesley-to-echo-toy-contract-proof.md)

## Status

GREEN 1.

## Hill

A tiny generated contract can use existing Echo substrate:

```text
GraphQL / Wesley IR
  -> echo-wesley-gen generated Rust
  -> generated op id and REGISTRY
  -> generated app-level EINT helper
  -> dispatch_intent(...)
  -> generated read helper over observe(...) / ReadingEnvelope
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

Implementation:

- `echo-wesley-gen` now imports `pack_intent_v1(...)` when operations are
  present.
- Mutation operations emit raw-vars helpers such as
  `pack_increment_intent(vars)`.
- Query operations emit frontier query-view helpers such as
  `counter_value_observation_request(worldline_id, vars)`.
- Query helpers use the existing `ObservationRequest`,
  `ObservationFrame::QueryView`, and `ObservationProjection::Query` shape.

Focused witness:

```sh
cargo test -p echo-wesley-gen test_toy_contract_generates_eint_and_observation_helpers
```

Result: passed.

Broader generator witness:

```sh
cargo test -p echo-wesley-gen
cargo clippy -p echo-wesley-gen --all-targets -- -D warnings -D missing_docs
```

Result: passed.

## GREEN direction

GREEN 1 stayed inside `echo-wesley-gen`.

Implemented shape:

- generate `pack_<mutation>_intent(...)` helpers that call
  `pack_intent_v1(OP_..., &vars)`;
- generate read-helper shapes for query ops that map to `ObservationRequest`;
- keep Echo core app-agnostic;
- keep host-side generated payload validation deferred.

Still deferred:

- typed vars encoders for operation argument structs;
- compiled generated output smoke test in a consumer crate;
- actual `dispatch_intent(...)` integration proof;
- actual `observe(...)` integration proof;
- registry metadata handshake proof against an installed kernel.

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

The RED deliberately includes a generated query/read helper. If the current
`ObservationRequest` shape cannot honestly express the toy query, the next
GREEN should stop at the precise missing observation bridge instead of
inventing a broad `query_contract(...)` ABI.
