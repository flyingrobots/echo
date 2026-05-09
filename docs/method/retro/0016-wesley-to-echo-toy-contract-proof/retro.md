<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Retro: 0016-wesley-to-echo-toy-contract-proof

Cycle: `0016-wesley-to-echo-toy-contract-proof`
Design: [`docs/design/0016-wesley-to-echo-toy-contract-proof/`](../../../design/0016-wesley-to-echo-toy-contract-proof/)
Witness: generated toy counter smoke crate under `target/` during
`echo-wesley-gen` tests.

## Outcome

- Status: Accepted.
- Summary: Closed the `M044` backlog item by proving a Wesley-generated toy
  contract can use generated op metadata, typed vars helpers, EINT packing,
  installed `warp-wasm` kernel dispatch, installed-kernel observation, and
  installed registry metadata without adding application-specific Echo APIs.

## Evidence

- `crates/echo-wesley-gen/tests/fixtures/toy-counter/echo-ir-v1.json` is the
  shared toy counter IR fixture.
- `crates/echo-wesley-gen/tests/generation.rs` generates Rust from that fixture
  and compiles a standalone consumer smoke crate.
- The smoke crate installs an application-owned `ToyKernel` through
  `warp_wasm::install_kernel(...)`.
- `crates/warp-wasm/src/lib.rs` exposes native CBOR-envelope helpers for the
  installed-kernel boundary:
    - `dispatch_intent_cbor(...)`;
    - `observe_cbor(...)`;
    - `get_registry_info_cbor()`.
- Those helpers do not change the `wasm_bindgen` exports; they let native tests
  exercise the same success/error envelope contract without JavaScript
  `Uint8Array` bindings.

## Verification

- `cargo test -p echo-wesley-gen test_toy_contract_generated_output_compiles_in_consumer_crate`

## Drift Check

- Echo core still does not learn text, editor, or `jedit` nouns.
- No new intent envelope, registry model, dynamic loader, or app payload
  validator was introduced.
- The toy contract is still a consumer fixture. It proves the host shape; it is
  not a privileged domain in Echo.
- Query reads are carried through the generic `QueryView` /
  `ObservationProjection::Query` and `OpticApertureShape::QueryBytes` shapes.

## Follow-Up

- Pull `M012` for contract-aware receipt and reading identity.
- Pull `M023` for retained contract artifacts and cached bounded readings in
  `echo-cas`.
- Use generated `jedit` Wesley output as a fixture only after identity and
  retention rules are honest enough for a serious consumer.
