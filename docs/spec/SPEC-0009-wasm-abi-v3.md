<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# SPEC-0009: WASM ABI Contract v3

> **Status:** Active | **ABI Version:** 3 | **Crate:** `warp-wasm`

## Overview

This document specifies the current WASM export surface, wire encoding, and
error protocol for Echo's deterministic simulation boundary.

ABI v3 makes three boundaries explicit:

- `observe(request)` is the only public read export.
- `dispatch_intent(...)` is the only public write and control ingress surface.
- `scheduler_status()` is the read-only scheduler metadata export.

Echo internals do not consume wall-clock time. All clocks in this ABI are
logical monotone integers:

- `WorldlineTick` is per-worldline append identity.
- `GlobalTick` is runtime cycle correlation metadata.
- `RunId` is a control-plane generation token.

Scheduler lifecycle requests are carried as privileged control intents through
the same EINT intake path as domain intents. There is no public `step(...)`,
poll, or tick hook API in ABI v3.

## Architecture

```text
┌─────────────────────────────────────────────────┐
│                 JS / Host Adapter                │
│  (submits intents, decodes CBOR envelopes)       │
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
| `observe(request)`        | `(&[u8]) → Uint8Array` | `ObservationArtifact` envelope |
| `scheduler_status()`      | `() → Uint8Array`      | `SchedulerStatus` envelope     |
| `get_registry_info()`     | `() → Uint8Array`      | `RegistryInfo` envelope        |
| `get_codec_id()`          | `() → JsValue`         | `string \| null`               |
| `get_registry_version()`  | `() → JsValue`         | `string \| null`               |
| `get_schema_sha256_hex()` | `() → JsValue`         | `string \| null`               |

Removed before or by ABI v3:

- `drain_view_ops()`
- `get_head()`
- `execute_query(id, vars)`
- `snapshot_at(tick)`
- `render_snapshot(bytes)`
- `step(budget)`

## Intent Intake

All external writes enter Echo through EINT envelopes.

- Domain intents use their domain-specific `op_id`.
- Privileged scheduler/control intents use reserved op id `u32::MAX`
  (`CONTROL_INTENT_V1_OP_ID`).

Control intents decode as canonical-CBOR `ControlIntentV1`:

- `Start { mode: UntilIdle { cycle_limit: Option<u32> } }`
- `Stop`
- `SetHeadEligibility { head, eligibility }`

Notes:

- `cycle_limit`, when present, must be non-zero.
- The current engine-backed implementation supports `UntilIdle` only.
- No wall-clock scheduler mode exists in ABI v3.

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
- `coordinate.at: frontier | tick { worldline_tick }`
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

| Field                        | Type            | Description                                      |
| ---------------------------- | --------------- | ------------------------------------------------ |
| `observation_version`        | u32             | Observation contract version                     |
| `worldline_id`               | bytes(32)       | Worldline actually observed                      |
| `requested_at`               | enum            | Original coordinate selector                     |
| `resolved_worldline_tick`    | `WorldlineTick` | Concrete resolved committed worldline coordinate |
| `commit_global_tick`         | `GlobalTick?`   | Commit cycle stamp for the resolved commit       |
| `observed_after_global_tick` | `GlobalTick?`   | Observation freshness watermark                  |
| `state_root`                 | bytes(32)       | Canonical graph-only state hash                  |
| `commit_hash`                | bytes(32)       | Canonical commit hash at the resolved point      |

### ObservationPayload

- `head` → `HeadObservation`
- `snapshot` → `SnapshotObservation`
- `truth_channels` → `ChannelData[]`
- `query_bytes` → raw bytes

### HeadInfo

Returned by `init()`.

| Field                | Type            | Description                          |
| -------------------- | --------------- | ------------------------------------ |
| `worldline_tick`     | `WorldlineTick` | Current committed worldline position |
| `commit_global_tick` | `GlobalTick?`   | Cycle stamp for the current commit   |
| `state_root`         | bytes(32)       | Graph-only BLAKE3 state hash         |
| `commit_id`          | bytes(32)       | Canonical commit hash                |

### DispatchResponse

| Field              | Type              | Description                                    |
| ------------------ | ----------------- | ---------------------------------------------- |
| `accepted`         | bool              | `true` if newly accepted, `false` if duplicate |
| `intent_id`        | bytes(32)         | Content-addressed intent hash                  |
| `scheduler_status` | `SchedulerStatus` | Scheduler metadata after ingest/apply          |

### SchedulerStatus

| Field                        | Type             | Description                                          |
| ---------------------------- | ---------------- | ---------------------------------------------------- |
| `state`                      | `SchedulerState` | Scheduler lifecycle state                            |
| `active_mode`                | `SchedulerMode?` | Active mode while a run is configured                |
| `work_state`                 | `WorkState`      | Whether runnable work exists at the current boundary |
| `run_id`                     | `RunId?`         | Current or latest run generation token               |
| `latest_cycle_global_tick`   | `GlobalTick?`    | Latest completed runtime cycle                       |
| `latest_commit_global_tick`  | `GlobalTick?`    | Latest cycle that produced a commit                  |
| `last_quiescent_global_tick` | `GlobalTick?`    | Most recent transition into quiescence               |
| `last_run_completion`        | `RunCompletion?` | Why the most recent run ended                        |

Current engine-backed behavior:

- `init()` leaves the runtime inert.
- `Start { mode: UntilIdle { ... } }` runs synchronously inside the control
  intent handler and returns after the run completes.
- Hosts normally observe `state = inactive` plus `last_run_completion`, not a
  long-lived running scheduler loop.

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
| `abi_version`       | u32     | ABI contract version (currently `3`)          |

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
| 13   | `INVALID_CONTROL`              | Malformed or invalid control intent                        |

## Rust Boundary

`KernelPort` is the Rust-side ABI contract for `warp-wasm`.

- `dispatch_intent(...)`
- `observe(...)`
- `scheduler_status()`
- `registry_info()`

The trait does not expose the removed v1 read adapters or a public step/pump
surface. Implementors that need head or snapshot data must derive them from
their own observation-backed internals rather than adding parallel public read
methods.

## Migration Notes for Host Adapters

### From ABI v2 to ABI v3

1. Stop calling `step(...)`; the export is absent in ABI v3.
2. Continue treating `observe(request)` as the only canonical public read
   boundary.
3. Route scheduler lifecycle and admission requests through
   `dispatch_intent(...)` using `ControlIntentV1` packed into an EINT envelope.
4. Read `RegistryInfo.abi_version` and reject hosts that still expect the v2
   step surface.
5. Rename host-side field access from bare `tick`-style fields to explicit
   `worldline_tick`, `commit_global_tick`, and
   `observed_after_global_tick` fields.
6. Treat all ABI clocks as logical coordinates only. They are not wall-clock
   durations, timer inputs, or global ordering cursors.
7. Expect query-shaped observations to continue returning
   `UNSUPPORTED_QUERY` until a real observation-backed query implementation
   lands.

## Compatibility Note

ABI v3 is intentionally breaking. The removed step/pump surface is absent, not
deprecated, and hosts must migrate to explicit observation requests plus
intent-shaped scheduler control.
