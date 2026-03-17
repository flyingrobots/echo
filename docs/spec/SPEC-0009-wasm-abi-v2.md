<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# SPEC-0009: WASM ABI Contract v2

> **Status:** Active | **ABI Version:** 2 | **Crate:** `warp-wasm`

## Overview

This document specifies the Phase 6 Slice A WASM export surface, wire
encoding, and error protocol for the Echo deterministic simulation boundary.
The ABI remains app-agnostic, but the public read surface is now explicitly
observation-first:

- `observe(request)` is the only public read export.
- `dispatch_intent(...)` and `step(...)` remain the write / advance boundary.
- legacy read adapters from ABI v1 are removed from the public boundary.

## Architecture

```text
┌─────────────────────────────────────────────────┐
│                 JS / Host Adapter                │
│  (decodes CBOR envelopes, drives tick loop)      │
└───────────────────┬─────────────────────────────┘
                    │  wasm-bindgen exports
┌───────────────────▼─────────────────────────────┐
│              warp-wasm (boundary)                │
│  thread_local RefCell<Option<Box<dyn KernelPort>>>│
│  Encodes Result<T, AbiError> → CBOR envelope    │
└───────────────────┬─────────────────────────────┘
                    │  KernelPort trait
┌───────────────────▼─────────────────────────────┐
│           WarpKernel (engine feature)            │
│  Wraps warp-core::Engine and ObservationService │
└─────────────────────────────────────────────────┘
```

## Exports

All exports are `#[wasm_bindgen]` functions. Return types are CBOR-encoded
`Uint8Array` unless noted otherwise.

| Export                    | Signature              | Returns                        |
| ------------------------- | ---------------------- | ------------------------------ |
| `init()`                  | `() → Uint8Array`      | `HeadInfo` envelope            |
| `dispatch_intent(bytes)`  | `(&[u8]) → Uint8Array` | `DispatchResponse` envelope    |
| `step(budget)`            | `(u32) → Uint8Array`   | `StepResponse` envelope        |
| `observe(request)`        | `(&[u8]) → Uint8Array` | `ObservationArtifact` envelope |
| `get_registry_info()`     | `() → Uint8Array`      | `RegistryInfo` envelope        |
| `get_codec_id()`          | `() → JsValue`         | `string \| null`               |
| `get_registry_version()`  | `() → JsValue`         | `string \| null`               |
| `get_schema_sha256_hex()` | `() → JsValue`         | `string \| null`               |

Removed in ABI v2:

- `drain_view_ops()`
- `get_head()`
- `execute_query(id, vars)`
- `snapshot_at(tick)`
- `render_snapshot(bytes)`

## Wire Envelope

All `Uint8Array` returns use a CBOR envelope with an `ok` discriminator:

### Success

```cbor
{ "ok": true, ...response_fields }
```

### Error

```cbor
{ "ok": false, "code": <u32>, "message": <string> }
```

JS callers check `ok` before decoding the rest. The CBOR encoding follows the
canonical rules in `docs/js-cbor-mapping.md`.

## Response Types

### ObservationRequest

The request payload for `observe(request)` is canonical-CBOR bytes that decode
to:

- `coordinate.worldline_id: bytes(32)`
- `coordinate.at: frontier | tick`
- `frame: commit_boundary | recorded_truth | query_view`
- `projection: head | snapshot | truth_channels | query`

### ObservationArtifact

| Field           | Type                            | Description                                    |
| --------------- | ------------------------------- | ---------------------------------------------- |
| `resolved`      | `ResolvedObservationCoordinate` | Explicit resolved coordinate metadata          |
| `frame`         | enum                            | Declared semantic frame                        |
| `projection`    | enum                            | Declared projection                            |
| `artifact_hash` | bytes(32)                       | Canonical observation artifact hash            |
| `payload`       | tagged union                    | Head, snapshot, recorded truth, or query bytes |

`artifact_hash` is computed as
`blake3("echo:observation-artifact:v1\0" || canonical_cbor(hash_input))`.

### ResolvedObservationCoordinate

| Field                 | Type      | Description                                 |
| --------------------- | --------- | ------------------------------------------- |
| `observation_version` | u32       | Observation contract version                |
| `worldline_id`        | bytes(32) | Worldline actually observed                 |
| `requested_at`        | enum      | Original coordinate selector                |
| `resolved_tick`       | u64       | Concrete resolved tick                      |
| `state_root`          | bytes(32) | Canonical graph-only state hash             |
| `commit_hash`         | bytes(32) | Canonical commit hash at the resolved point |

### ObservationPayload

- `head` → `HeadObservation`
- `snapshot` → `SnapshotObservation`
- `truth_channels` → `ChannelData[]`
- `query_bytes` → raw bytes

### HeadInfo

Returned by `init()` and nested inside `StepResponse`.

| Field        | Type      | Description                  |
| ------------ | --------- | ---------------------------- |
| `tick`       | u64       | Number of committed ticks    |
| `state_root` | bytes(32) | Graph-only BLAKE3 state hash |
| `commit_id`  | bytes(32) | Canonical commit hash        |

### DispatchResponse

| Field       | Type      | Description                                    |
| ----------- | --------- | ---------------------------------------------- |
| `accepted`  | bool      | `true` if newly accepted, `false` if duplicate |
| `intent_id` | bytes(32) | Content-addressed intent hash                  |

### StepResponse

| Field            | Type     | Description                        |
| ---------------- | -------- | ---------------------------------- |
| `ticks_executed` | u32      | Ticks actually executed (≤ budget) |
| `head`           | HeadInfo | Post-step head state               |

### ChannelData

| Field        | Type      | Description                        |
| ------------ | --------- | ---------------------------------- |
| `channel_id` | bytes(32) | Materialization channel identifier |
| `data`       | bytes     | Raw finalized channel output       |

### RegistryInfo

| Field               | Type    | Description                                   |
| ------------------- | ------- | --------------------------------------------- |
| `codec_id`          | string? | Codec identifier (e.g. `"cbor-canonical-v1"`) |
| `registry_version`  | string? | Registry version                              |
| `schema_sha256_hex` | string? | Schema hash (hex)                             |
| `abi_version`       | u32     | ABI contract version (currently `2`)          |

## Error Codes

| Code | Name                           | Meaning                                                    |
| ---- | ------------------------------ | ---------------------------------------------------------- |
| 1    | `NOT_INITIALIZED`              | `init()` not called                                        |
| 2    | `INVALID_INTENT`               | Malformed EINT intent envelope                             |
| 3    | `ENGINE_ERROR`                 | Internal engine failure                                    |
| 4    | `LEGACY_INVALID_TICK`          | Reserved for the removed v1 snapshot adapter               |
| 5    | `NOT_SUPPORTED`                | Operation not implemented                                  |
| 6    | `CODEC_ERROR`                  | CBOR encode/decode failure                                 |
| 7    | `INVALID_PAYLOAD`              | Corrupted input bytes                                      |
| 8    | `INVALID_WORLDLINE`            | Requested worldline missing                                |
| 9    | `INVALID_TICK`                 | Requested observation tick missing                         |
| 10   | `UNSUPPORTED_FRAME_PROJECTION` | Invalid frame/projection pair                              |
| 11   | `UNSUPPORTED_QUERY`            | Query observation not yet implemented                      |
| 12   | `OBSERVATION_UNAVAILABLE`      | Valid request but no observation exists at that coordinate |

## Rust Boundary

`KernelPort` is the Rust-side ABI contract for `warp-wasm`.

- `dispatch_intent(...)`
- `step(...)`
- `observe(...)`
- `registry_info()`

The trait no longer exposes the removed v1 read adapters. Implementors that
need head or snapshot data must derive them from their own observation-backed
internals rather than adding parallel public read methods.

## Migration Notes for Host Adapters

### From ABI v1 to ABI v2

1. Replace any direct use of `get_head()`, `snapshot_at()`, `drain_view_ops()`,
   `execute_query(...)`, and `render_snapshot(...)` with `observe(request)`.
2. Treat `observe(request)` as the only canonical public read boundary.
3. Continue decoding `init()` and `step()` exactly as before; both still return
   head metadata envelopes.
4. Read `RegistryInfo.abi_version` and reject hosts that still expect the
   removed v1 exports.
5. Expect query-shaped observations to continue returning
   `UNSUPPORTED_QUERY` until a real observation-backed query implementation
   lands.

## Compatibility Note

ABI v2 is intentionally breaking. The removed v1 exports are absent, not
deprecated, and hosts must migrate to explicit observation requests.
