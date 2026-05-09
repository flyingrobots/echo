<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Retro: 0023-import-transport-intent-admission-path

Cycle: `0023-import-transport-intent-admission-path`
Design: [`docs/design/0023-import-transport-intent-admission-path/`](../../../design/0023-import-transport-intent-admission-path/)
Witness: [`witness/`](./witness/)

## Outcome

- Status: Accepted.
- Summary: Added Echo's first executable import-transport Intent path. A
  `CausalSuffixBundle` import proposal is now wrapped as an Echo-owned EINT v1
  payload, validated at the WASM/kernel boundary, admitted through
  `dispatch_intent`, and handled during the scheduled tick as a typed staged
  `ImportSuffixResult` graph artifact.

## Evidence

- `crates/echo-wasm-abi/src/lib.rs` defines
  `IMPORT_SUFFIX_INTENT_V1_OP_ID`,
  `pack_import_suffix_intent_v1(...)`, and
  `unpack_import_suffix_intent_v1(...)`.
- `crates/warp-wasm/src/warp_kernel.rs` rejects malformed import-suffix EINT
  payloads before ingress and registers the generic import command handler for
  engine-backed kernels.
- `crates/warp-core/src/cmd.rs` defines `cmd/import_suffix_intent`, preserving
  the ingress event and appending a deterministic result node with canonical
  CBOR `ImportSuffixResult`.
- Verification:
    - `cargo test -p echo-wasm-abi import_suffix_intent`
    - `cargo test -p warp-wasm --features engine import_suffix_intent`
    - `cargo test -p warp-core import_suffix`

## Drift Check

- Echo core still does not learn application nouns. The import command handles
  generic witnessed suffix bundles and typed admission posture only.
- Transport arrival remains outside history until wrapped as an EINT intent and
  selected by the scheduler.
- The first handler returns `Staged`, not `Admitted`, because full remote
  basis-aware admission, novelty indexing, and settlement/collapse are later
  slices.
- The original ingress event remains in the graph; the result is recorded as a
  separate causal artifact rather than overwriting the proposal bytes.

## Follow-Up

- Implement basis-aware import outcome evaluation with local target-basis
  evidence.
- Add retained shell-equivalence and novelty/loop-prevention indexes.
- Extend intent-driven settlement/braid/topology paths so staged imports can be
  realized without direct mutation APIs.
