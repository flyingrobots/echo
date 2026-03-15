<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# SPEC-0009: WASM ABI Contract v1

> **Status:** Active | **ABI Version:** 1 | **Crate:** `warp-wasm`

## Overview

This document specifies the WASM export surface, wire encoding, and error
protocol for the Echo deterministic simulation boundary. The ABI is
**app-agnostic**: it operates on opaque intent bytes, tick budgets, and
materialized channel outputs without assuming any domain-specific schema.

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
│  Wraps warp-core::Engine, registers sys rules   │
└─────────────────────────────────────────────────┘
```

### App-Agnostic Injection

The boundary stores its kernel in a module-scoped `RefCell`. Any type
implementing `KernelPort` can be installed via `install_kernel()`. The
`engine` feature provides `WarpKernel` (wrapping `warp-core::Engine`),
but apps can implement `KernelPort` with any engine.

## Exports

All exports are `#[wasm_bindgen]` functions. Return types are CBOR-encoded
`Uint8Array` unless noted otherwise.

| Export                    | Signature                   | Returns                        |
| ------------------------- | --------------------------- | ------------------------------ |
| `init()`                  | `() → Uint8Array`           | `HeadInfo` envelope            |
| `dispatch_intent(bytes)`  | `(&[u8]) → Uint8Array`      | `DispatchResponse` envelope    |
| `step(budget)`            | `(u32) → Uint8Array`        | `StepResponse` envelope        |
| `observe(request)`        | `(&[u8]) → Uint8Array`      | `ObservationArtifact` envelope |
| `drain_view_ops()`        | `() → Uint8Array`           | `DrainResponse` envelope       |
| `get_head()`              | `() → Uint8Array`           | `HeadInfo` envelope            |
| `execute_query(id, vars)` | `(u32, &[u8]) → Uint8Array` | `RawBytesResponse` envelope    |
| `snapshot_at(tick)`       | `(u64) → Uint8Array`        | `RawBytesResponse` envelope    |
| `render_snapshot(bytes)`  | `(&[u8]) → Uint8Array`      | `RawBytesResponse` envelope    |
| `get_registry_info()`     | `() → Uint8Array`           | `RegistryInfo` envelope        |
| `get_codec_id()`          | `() → JsValue`              | `string \| null`               |
| `get_registry_version()`  | `() → JsValue`              | `string \| null`               |
| `get_schema_sha256_hex()` | `() → JsValue`              | `string \| null`               |

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

JS callers check `ok` before decoding the rest. The CBOR encoding follows
the canonical rules in `docs/js-cbor-mapping.md` (sorted keys, shortest
integers, no tags, definite lengths).

## Response Types

### ObservationRequest

The request payload for `observe(request)` is itself canonical-CBOR bytes that
decode to:

- `coordinate.worldline_id: bytes(32)`
- `coordinate.at: frontier | tick`
- `frame: commit_boundary | recorded_truth | query_view`
- `projection: head | snapshot | truth_channels | query`

This makes worldline, time, frame, and projection explicit on every read.

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

| Field        | Type      | Description                  |
| ------------ | --------- | ---------------------------- |
| `tick`       | u64       | Number of committed ticks    |
| `state_root` | bytes(32) | Graph-only BLAKE3 state hash |
| `commit_id`  | bytes(32) | Canonical commit hash (v2)   |

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

### DrainResponse

| Field      | Type  | Description           |
| ---------- | ----- | --------------------- |
| `channels` | array | List of `ChannelData` |

### ChannelData

| Field        | Type      | Description                        |
| ------------ | --------- | ---------------------------------- |
| `channel_id` | bytes(32) | Materialization channel identifier |
| `data`       | bytes     | Raw finalized channel output       |

### RawBytesResponse

Used by endpoints that return pre-encoded CBOR payloads (`execute_query`,
`snapshot_at`, `render_snapshot`). Wrapped in the standard `{ ok: true }`
envelope like all other responses.

| Field  | Type  | Description              |
| ------ | ----- | ------------------------ |
| `data` | bytes | Raw CBOR-encoded payload |

### RegistryInfo

| Field               | Type    | Description                                    |
| ------------------- | ------- | ---------------------------------------------- |
| `codec_id`          | string? | Codec identifier (e.g., `"cbor-canonical-v1"`) |
| `registry_version`  | string? | Registry version                               |
| `schema_sha256_hex` | string? | Schema hash (hex)                              |
| `abi_version`       | u32     | ABI contract version (currently `1`)           |

## Error Codes

| Code | Name                           | Meaning                                                    |
| ---- | ------------------------------ | ---------------------------------------------------------- |
| 1    | `NOT_INITIALIZED`              | `init()` not called                                        |
| 2    | `INVALID_INTENT`               | Malformed EINT intent envelope                             |
| 3    | `ENGINE_ERROR`                 | Internal engine failure                                    |
| 4    | `LEGACY_INVALID_TICK`          | Legacy snapshot/history tick out of bounds                 |
| 5    | `NOT_SUPPORTED`                | Operation not implemented                                  |
| 6    | `CODEC_ERROR`                  | CBOR encode/decode failure                                 |
| 7    | `INVALID_PAYLOAD`              | Corrupted input bytes                                      |
| 8    | `INVALID_WORLDLINE`            | Requested worldline missing                                |
| 9    | `INVALID_TICK`                 | Requested observation tick missing                         |
| 10   | `UNSUPPORTED_FRAME_PROJECTION` | Invalid frame/projection pair                              |
| 11   | `UNSUPPORTED_QUERY`            | Query observation not yet implemented                      |
| 12   | `OBSERVATION_UNAVAILABLE`      | Valid request but no observation exists at that coordinate |

## Versioning Strategy

- The ABI version is exposed via `RegistryInfo.abi_version` and the
  constant `echo_wasm_abi::kernel_port::ABI_VERSION`.
- **Additive changes** (new optional fields, new exports such as `observe`) do NOT bump the
  ABI version.
- **Breaking changes** (removed fields, changed semantics, new required
  fields, changed error codes) require an ABI version bump and a
  `BREAKING CHANGE` footer in the commit.
- The `KernelPort` trait is the Rust-side contract. Adding methods to it
  is a breaking change (use default methods for additive evolution).
- `execute_query` and `render_snapshot` have default implementations that
  return `NOT_SUPPORTED`. Implementors only need to override them when the
  engine supports these operations.

## Migration Notes for Host Adapters

### From placeholder exports (v0.1.0) to ABI v1

1. **All exports now return CBOR envelopes**, not empty bytes. Check `ok`
   field before processing.
2. **`init()` must be called** before any other export. Previous stubs
   silently returned empty bytes; now they return error code `1`.
3. **`dispatch_intent` returns data**. Previously a no-op void function;
   now returns `DispatchResponse` with the intent hash.
4. **`observe(request)`** is the canonical read boundary. Legacy read exports
   remain one-phase adapters above it.
5. **`execute_query`** currently lowers to `observe(..., query_view, query)`
   and returns error code `11` (`UNSUPPORTED_QUERY`) until real query support lands.
6. **`render_snapshot`** still returns error code `5`
   (`NOT_SUPPORTED`).
7. **JsValue exports unchanged**: `get_codec_id`, `get_registry_version`,
   `get_schema_sha256_hex` still return `JsValue` (`string | null`).

## Not Yet Implemented

These are honestly reported as `NOT_SUPPORTED` (error code 5):

- `execute_query`: Lowered through `observe(...)`, but real query evaluation is not yet built.
- `render_snapshot`: Snapshot-to-ViewOps projection not yet built.
