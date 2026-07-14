<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# SPEC-0009 - WASM ABI Contract

_Define the current deterministic browser boundary for intent ingress, scheduler metadata, and observer-relative reads._

Current ABI version: 12

Depends on:

- [JS to Canonical CBOR Mapping](js-cbor-mapping.md)
- [ABI Golden Vectors](abi-golden-vectors.md)
- [SPEC-0004 - Worldlines, Playback, and Observation](SPEC-0004-worldlines-playback-truthbus.md)

## Purpose

The WASM boundary is where browser and host code meet the Echo runtime. It must be small, deterministic, and explicit about what kind of operation is crossing: intent admission, scheduler inspection, or observation.

ABI version 12 retains the explicit trusted host-control export and carries
witnessed submission identity for accepted application ingress. Product read
requests bind optic identity, causal coordinate, focus, bounded aperture, law
versions, and capability. Emitted optic readings preserve reading-envelope and
witness/evidence posture; typed obstructions remain visible to the caller.

Browser tools must not silently depend on wall-clock timing or unsupported
exports. A UI can submit an intent, ask for scheduler status, or request a
bounded optic reading; it cannot reach around the runtime to step or render
private state.

The boundary also supports deterministic automation: a client can generate
canonical CBOR, call an export, inspect an `ok` envelope, and correlate the
returned reading with logical ticks.

## Decision 1: The ABI implements only the current epoch

`ABI_VERSION` detects host/runtime mismatch. It does not promise compatibility
with historical export shapes. Current callable exports are:

- `init`
- `dispatch_intent`
- `dispatch_control_intent_trusted`
- `dispatch_optic_intent`
- `observe_optic`
- `observe`
- `observe_neighborhood_site`
- `observe_neighborhood_core`
- `compare_settlement`
- `plan_settlement`
- `settle_strand`
- `get_registry_info`
- `scheduler_status`
- `get_codec_id`
- `get_registry_version`
- `get_schema_sha256_hex`

Removed exports stay removed: `step`, `snapshot_at`, `render_snapshot`, `execute_query`, `get_head`, and `drain_view_ops`.

## Decision 2: Application writes enter through EINT

`dispatch_intent(bytes)` accepts application Echo intent envelopes:
`"EINT" || op_id:u32le || vars_len:u32le || vars`.

The reserved scheduler/control op id is not an application intent. Public
application dispatch rejects it before the kernel can run scheduler control.
Trusted host/runtime control uses a separate authority path.

For accepted or duplicate application ingress, `DispatchResponse` carries both
the canonical `intent_id` and a witnessed `submission_id` plus
`submission_generation`. The submission fields are intake/audit correlation
metadata, not scheduler order, not worldline ticks, and not wall-clock time.
Trusted runtime control responses do not carry application submission identity.
The raw trusted host-control export is not an application API. High-level
browser or JavaScript application facades must not re-export it to untrusted
application code.

## Decision 3: Bounded optics define the product read shape

`observe_optic(request)` is the product-shaped ABI surface. Its request carries
an optic, explicit causal coordinate, focus, bounded aperture, projection and
reducer law versions, and capability basis. Its result is either an
`OpticReading` with evidence posture or a typed `OpticObstruction`; adapters
must not widen the request or erase an obstruction.

The current engine validates worldline focus/coordinate consistency, supported
head or snapshot apertures, attachment posture, and byte budget. It does not
verify `optic_id`, capability, or projection/reducer law claims against a
trusted opened-optic or installed-law authority. `QueryBytes` apertures,
including those emitted by Wesley query helpers, return
`UnsupportedProjectionLaw`. Field presence is not authorization evidence, and
ABI 12 does not yet provide a production-ready generated-query optic path.

`observe(request)`, `observe_neighborhood_site(request)`, and
`observe_neighborhood_core(request)` remain lower-level ABI surfaces for
explicit coordinate/projection materialization and diagnostic inspection. They
are read-only, but they do not by themselves confer the aperture, law,
capability, budget, residual, or obstruction semantics of an optic. The current
installed-contract query path uses raw `observe`; product adapters must not
treat that lower-level path as proof that optic capability or law was admitted.

## Decision 4: The ABI uses logical clocks only

The ABI names worldline ticks for append position, global ticks for runtime-cycle correlation, and run ids for control-plane generation. No ABI field depends on wall-clock time for semantics.

## Decision 5: Results use canonical CBOR envelopes

Byte-returning exports use CBOR result envelopes: `{ "ok": true, ... }` or `{ "ok": false, "code": u32, "message": string }`.

Implementation evidence: `crates/warp-wasm/src/lib.rs`, `crates/warp-wasm/src/kernel.rs`, `crates/echo-wasm-abi/src/*`, and `crates/echo-wasm-abi/tests/*`.
