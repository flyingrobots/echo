<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# warp-wasm still speaks CBOR everywhere except the EINT path

Status: bad code.

## Where

`crates/warp-wasm/src/lib.rs` and `crates/warp-wasm/src/warp_kernel.rs`.

## Smell

Per the 2026-05-28 audit (during 0024 universal LE binary codec
Phase 3):

- Public ABI exports named `*_cbor`: `dispatch_intent_cbor`,
  `observe_cbor`, `get_registry_info_cbor`,
  `dispatch_control_intent_trusted_cbor`, plus optic/settlement variants.
- `observe_cbor` decodes `ObservationRequest` as CBOR
- `dispatch_optic_intent` decodes `DispatchOpticIntentRequest` as CBOR
- `observe_settlement*` / `read_settlement*` decode requests as CBOR
- ALL response envelopes (`OkEnvelope<T>`, `ErrEnvelope`) are
  CBOR-encoded
- `warp_kernel.rs` reports `codec_id = "cbor-canonical-v1"` while the
  EINT payload path is fully LE binary — inconsistent self-description

## Why it matters

The mutation path goes EINT-in / CBOR-out today. That works because
jedit can CBOR-decode the response envelope, but it means:

- The "engine native protocol" claim is half-true; queries still need
  CBOR-encoded ObservationRequest from jedit
- Function names mislead consumers about the actual wire format
- `codec_id` advertises the wrong protocol

## Suggested cycle

1. Add `Encode`/`Decode` impls for `ObservationRequest`,
   `DispatchOpticIntentRequest`, `SettlementRequest`, `OkEnvelope<T>`,
   `ErrEnvelope`
2. Rename `*_cbor` exports to `*_wire` (or similar). Hold the rename
   under a feature flag so jedit can adopt the new names in lockstep.
3. Update `codec_id` to `le-binary-v1`
4. Update jedit `echo-wasm-kernel.ts` consumer to call the new names

This is multi-turn. Don't attempt until the mutation cutover
(jedit/spec/rope-codec.spec.mjs, jedit/spec/eint.spec.mjs) has been
proven end-to-end.

## Surface when

After the jedit optic-client EINT cutover lands and the user wants to
move queries onto LE binary.
