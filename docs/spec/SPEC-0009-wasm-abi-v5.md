<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# SPEC-0009: WASM ABI Contract v5

> **Status:** Active | **ABI Version:** 5 | **Crate:** `warp-wasm`

## Overview

This document specifies the current WASM export surface, wire encoding, and
error protocol for Echo's deterministic simulation boundary.

ABI v5 keeps the ABI v3 public export shape and extends observation artifacts
with read-side basis posture.

ABI v3 made three boundaries explicit:

- `observe(request)` is the only public world-state read export.
- `dispatch_intent(...)` is the only public write and control ingress surface.
- `scheduler_status()` is the read-only scheduler metadata export.

Echo internals do not consume wall-clock time. All clocks in this ABI are
logical monotone integers:

- `WorldlineTick` is per-worldline append identity.
- `GlobalTick` is runtime cycle correlation metadata.
- `RunId` is a control-plane generation token.

On the wire, `WorldlineTick`, `GlobalTick`, and `RunId` are canonical-CBOR
unsigned integers using the smallest legal width for their value. `null`
represents `Option<...>::None`.

`WorldlineTick(0)` is intentionally overloaded by coordinate type:

- In historical selectors such as `ObservationAt::Tick { worldline_tick: 0 }`,
  it names the first committed append.
- In frontier/head metadata such as `HeadInfo`, `HeadObservation`, and
  `ResolvedObservationCoordinate`, `worldline_tick = 0` with
  `commit_global_tick = null` means the worldline is still at `U0` and has not
  committed anything yet.

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

The EINT v1 byte layout is:

```text
"EINT" (4 bytes)
+ op_id (u32 little-endian)
+ vars_len (u32 little-endian)
+ vars (exactly vars_len bytes)
```

For privileged control intents, `op_id` is always `0xffffffff` and `vars` are
canonical-CBOR bytes that decode as `ControlIntentV1`.

Canonical payload shapes:

- `Start { mode: UntilIdle { cycle_limit: Option<u32> } }`

    ```cbor
    {
      "kind": "start",
      "mode": {
        "kind": "until_idle",
        "cycle_limit": <u32 or null>
      }
    }
    ```

- `Stop`

    ```cbor
    { "kind": "stop" }
    ```

- `SetHeadEligibility { head, eligibility }`

    ```cbor
    {
      "kind": "set_head_eligibility",
      "head": {
        "worldline_id": WorldlineId,
        "head_id": HeadId
      },
      "eligibility": "dormant" | "admitted"
    }
    ```

Notes:

- `cycle_limit`, when present, must be non-zero.
- The current engine-backed implementation supports `UntilIdle` only.
- No wall-clock scheduler mode exists in ABI v3.

Concrete `Start { mode: UntilIdle { cycle_limit: Some(1) } }` example:

```text
ControlIntentV1 payload (canonical CBOR hex):
a2646b696e64657374617274646d6f6465a2646b696e646a756e74696c5f69646c656b6379636c655f6c696d697401

Packed EINT envelope (hex):
45494e54ffffffff2f000000a2646b696e64657374617274646d6f6465a2646b696e646a756e74696c5f69646c656b6379636c655f6c696d697401
```

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

### Typed Field Encoding

The scheduler-facing enums use serde's declared shapes directly:

- `SchedulerState`, `WorkState`, `RunCompletion`, `HeadEligibility`, and
  `HeadDisposition` serialize as snake_case text strings.
- `SchedulerMode::UntilIdle { cycle_limit }` serializes as
  `{ "kind": "until_idle", "cycle_limit": <u32 or null> }`.
- `WorldlineId` and `HeadId` are typed opaque wrappers that serialize as
  `bytes(32)`. Array-of-32-integers encodings are invalid for these fields.

Concrete `scheduler_status()` example:

```cbor
{
  "state": "inactive",
  "active_mode": null,
  "work_state": "quiescent",
  "run_id": 7,
  "latest_cycle_global_tick": 9,
  "latest_commit_global_tick": 8,
  "last_quiescent_global_tick": 9,
  "last_run_completion": "quiesced"
}
```

Canonical CBOR hex for that payload:

```text
a865737461746568696e6163746976656672756e5f6964076a776f726b5f737461746569717569657363656e746b6163746976655f6d6f6465f6736c6173745f72756e5f636f6d706c6574696f6e68717569657363656478186c61746573745f6379636c655f676c6f62616c5f7469636b0978196c61746573745f636f6d6d69745f676c6f62616c5f7469636b08781a6c6173745f717569657363656e745f676c6f62616c5f7469636b09
```

## Response Types

### ObservationRequest

The request payload for `observe(request)` is canonical-CBOR bytes that decode
to:

- `coordinate.worldline_id: WorldlineId` encoded as `bytes(32)`
- `coordinate.at: frontier | tick { worldline_tick }`
- `frame: commit_boundary | recorded_truth | query_view`
- `projection: head | snapshot | truth_channels | query`

### ObservationArtifact

| Field           | Type                            | Description                                                  |
| --------------- | ------------------------------- | ------------------------------------------------------------ |
| `resolved`      | `ResolvedObservationCoordinate` | Explicit resolved coordinate metadata                        |
| `basis_posture` | `ObservationBasisPosture`       | Read-side parent/strand basis posture included in hash input |
| `frame`         | enum                            | Declared semantic frame                                      |
| `projection`    | enum                            | Declared projection                                          |
| `artifact_hash` | bytes(32)                       | Canonical observation artifact hash                          |
| `payload`       | tagged union                    | Head, snapshot, recorded truth, or query bytes               |

`artifact_hash` is computed as
`blake3("echo:observation-artifact:v2\0" || canonical_cbor(hash_input))`.

### ObservationBasisPosture

`basis_posture` is the read-side posture of the observed coordinate:

- `worldline`
- `strand_historical { strand_id }`
- `strand_at_anchor { strand_id }`
- `strand_parent_advanced_disjoint { strand_id, parent_from, parent_to }`
- `strand_revalidation_required { strand_id, parent_from, parent_to,
overlapping_slot_count, overlapping_slots_digest }`

The kernel retains the full overlapping slot list internally. The ABI carries a
count plus deterministic digest until a stable public slot representation is
introduced.

### ResolvedObservationCoordinate

| Field                        | Type            | Description                                                                                                                                                       |
| ---------------------------- | --------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `observation_version`        | u32             | Observation contract version                                                                                                                                      |
| `worldline_id`               | `WorldlineId`   | Worldline actually observed; serialized as `bytes(32)`                                                                                                            |
| `requested_at`               | enum            | Original coordinate selector                                                                                                                                      |
| `resolved_worldline_tick`    | `WorldlineTick` | Resolved coordinate; historical reads use zero-based committed append indices, while `0` plus `commit_global_tick = null` represents empty `U0` frontier metadata |
| `commit_global_tick`         | `GlobalTick?`   | Commit cycle stamp for the resolved commit; `null` means the resolved coordinate is empty `U0` rather than a committed append                                     |
| `observed_after_global_tick` | `GlobalTick?`   | Observation freshness watermark                                                                                                                                   |
| `state_root`                 | bytes(32)       | Canonical full materialization root hash; empty `U0` observations still carry the deterministic `U0` materialization root                                         |
| `commit_hash`                | bytes(32)       | Canonical frontier/commit hash at the resolved point; empty `U0` observations still carry the deterministic `U0` frontier snapshot hash                           |

### ObservationProjection

| Variant          | Shape                                                       | Description                                              |
| ---------------- | ----------------------------------------------------------- | -------------------------------------------------------- |
| `head`           | `{ "kind": "head" }`                                        | Head metadata projection                                 |
| `snapshot`       | `{ "kind": "snapshot" }`                                    | Snapshot metadata projection                             |
| `truth_channels` | `{ "kind": "truth_channels", "channels": bytes(32)[]? }`    | Recorded truth channel filter; `null` means all channels |
| `query`          | `{ "kind": "query", "query_id": u32, "vars_bytes": bytes }` | Query projection placeholder                             |

### ObservationPayload

| Variant          | Shape                                                     | Description                     |
| ---------------- | --------------------------------------------------------- | ------------------------------- |
| `head`           | `{ "kind": "head", "head": HeadObservation }`             | Head metadata payload           |
| `snapshot`       | `{ "kind": "snapshot", "snapshot": SnapshotObservation }` | Snapshot metadata payload       |
| `truth_channels` | `{ "kind": "truth_channels", "channels": ChannelData[] }` | Recorded truth channel payloads |
| `query_bytes`    | `{ "kind": "query_bytes", "data": bytes }`                | Query result bytes              |

### HeadObservation

| Field                | Type            | Description                                                                                                               |
| -------------------- | --------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `worldline_tick`     | `WorldlineTick` | Frontier coordinate; `0` plus `commit_global_tick = null` means the observed frontier is empty `U0`                       |
| `commit_global_tick` | `GlobalTick?`   | Commit cycle stamp for the observed frontier; `null` means no committed append yet                                        |
| `state_root`         | bytes(32)       | Canonical full materialization root hash; empty `U0` observations still carry the deterministic `U0` materialization root |
| `commit_id`          | bytes(32)       | Canonical frontier hash; empty `U0` observations still carry the deterministic `U0` frontier snapshot hash                |

### SnapshotObservation

| Field                | Type            | Description                                                                                                                    |
| -------------------- | --------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `worldline_tick`     | `WorldlineTick` | Snapshot coordinate; historical reads use append indices, while frontier snapshot reads may return `0` + `null` for empty `U0` |
| `commit_global_tick` | `GlobalTick?`   | Commit cycle stamp; `null` is reserved for an empty-frontier `U0` snapshot resolved from `ObservationAt::Frontier`             |
| `state_root`         | bytes(32)       | Canonical full materialization root hash; empty `U0` snapshots still carry the deterministic `U0` materialization root         |
| `commit_id`          | bytes(32)       | Canonical snapshot hash; empty `U0` snapshots still carry the deterministic `U0` frontier snapshot hash                        |

### HeadInfo

Returned by `init()`.

| Field                | Type            | Description                                                                                                           |
| -------------------- | --------------- | --------------------------------------------------------------------------------------------------------------------- |
| `worldline_tick`     | `WorldlineTick` | Current committed frontier position; `0` plus `commit_global_tick = null` means empty `U0`                            |
| `commit_global_tick` | `GlobalTick?`   | Cycle stamp for the current commit; `null` means no commits yet                                                       |
| `state_root`         | bytes(32)       | Canonical full materialization BLAKE3 root hash; empty `U0` still carries the deterministic `U0` materialization root |
| `commit_id`          | bytes(32)       | Canonical frontier hash; empty `U0` still carries the deterministic `U0` frontier snapshot hash                       |

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
- `Stop` is a no-op when the scheduler is already inactive; it does not rewrite
  `last_run_completion` for a finished run.
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
| `abi_version`       | u32     | ABI contract version (currently `5`)          |

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
2. Continue treating `observe(request)` as the only canonical public
   world-state read boundary.
3. Route scheduler lifecycle and admission requests through
   `dispatch_intent(...)` using `ControlIntentV1` packed into an EINT envelope.
4. Read `RegistryInfo.abi_version` and reject hosts that still expect older
   step surface.
5. Rename host-side field access from bare `tick`-style fields to explicit
   `worldline_tick`, `commit_global_tick`, and
   `observed_after_global_tick` fields.
6. Treat all ABI clocks as logical coordinates only. They are not wall-clock
   durations, timer inputs, or global ordering cursors.
7. Expect query-shaped observations to continue returning
   `UNSUPPORTED_QUERY` until a real observation-backed query implementation
   lands.

### From ABI v4 to ABI v5

1. Decode `ObservationArtifact.basis_posture` on every observation response.
2. Include `basis_posture` in any host-side observation artifact hash
   verification.
3. Treat `strand_revalidation_required` as a visible read posture, not as a
   materialized-state failure.
4. Use `overlapping_slot_count` and `overlapping_slots_digest` as witness
   metadata only. The stable public slot representation is still deferred.

## Compatibility Note

ABI v5 is intentionally breaking relative to v4 for observation responses. ABI
v3 remains the major export-shape break that removed step/pump; v5 preserves
that export shape while adding read-side basis posture to observation artifacts.
