<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# SPEC-0009 - WASM ABI Contract

_Define the current deterministic browser boundary for intent ingress, scheduler metadata, and observer-relative reads._

Legend: PLATFORM

Current ABI version: 9

Depends on:

- [JS to Canonical CBOR Mapping](js-cbor-mapping.md)
- [ABI Golden Vectors](abi-golden-vectors.md)
- [SPEC-0004 - Worldlines, Playback, and Observation](SPEC-0004-worldlines-playback-truthbus.md)

## Why this packet exists

The WASM boundary is where browser and host code meet the Echo runtime. It must be small, deterministic, and explicit about what kind of operation is crossing: intent admission, scheduler inspection, or observation.

ABI version 9 keeps the current export shape and makes observation requests
name their observer plan, optional hosted observer instance, read budget, and
rights posture explicitly. Observation artifacts continue to carry
reading-envelope metadata for emitted readings.

## Human users / jobs / hills

Human users need browser tools that do not silently depend on wall-clock timing or unsupported exports.

The hill: a UI can submit an intent, ask for scheduler status, or request an observation artifact; it cannot reach around the runtime to step, query, or render private state.

## Agent users / jobs / hills

Agent users need a stable automation boundary.

The hill: an agent can generate canonical CBOR, call one export, inspect an `ok` envelope, and correlate the returned reading with logical ticks.

## Decision 1: The ABI implements only the current epoch

`ABI_VERSION` detects host/runtime mismatch. It does not promise compatibility with historical export shapes. Current exports are `init`, `dispatch_intent`, `observe`, `scheduler_status`, `get_registry_info`, `get_codec_id`, `get_registry_version`, and `get_schema_sha256_hex`.

Removed exports stay removed: `step`, `snapshot_at`, `render_snapshot`, `execute_query`, `get_head`, and `drain_view_ops`.

## Decision 2: All writes enter through EINT

`dispatch_intent(bytes)` accepts Echo intent envelopes: `"EINT" || op_id:u32le || vars_len:u32le || vars`.

## Decision 3: Observation is the only public world-state read

`observe(request)` returns an observation artifact with resolved coordinate, reading envelope, declared frame, declared projection, artifact hash, and payload.

The observation request names the observer plan, optional hosted observer
instance, read budget, and rights posture. The reading envelope names the
observer plan, hosted observer instance when present, native observer basis,
witness refs, parent/basis posture, budget posture, rights posture, and
residual posture. Built-in observations currently emit `complete` residual
posture for clean derived readings. The ABI also names `residual`,
`plurality_preserved`, and `obstructed` so external consumers can recognize
bounded non-clean readings without treating the payload as a generic state read.

## Decision 4: The ABI uses logical clocks only

The ABI names worldline ticks for append position, global ticks for runtime-cycle correlation, and run ids for control-plane generation. No ABI field depends on wall-clock time for semantics.

## Decision 5: Results use canonical CBOR envelopes

Byte-returning exports use CBOR result envelopes: `{ "ok": true, ... }` or `{ "ok": false, "code": u32, "message": string }`.

Implementation evidence: `crates/warp-wasm/src/lib.rs`, `crates/warp-wasm/src/kernel.rs`, `crates/echo-wasm-abi/src/*`, and `crates/echo-wasm-abi/tests/*`.
