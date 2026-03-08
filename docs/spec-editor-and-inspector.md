<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Inspector & Editor Protocol Specification (Phase 0.75)

> **Background:** For a gentler introduction, see [WARP Primer](guide/warp-primer.md).

Unifies Echo’s inspector data streams, transport contracts, and extension hooks for tooling.

---

## Goals

- Deliver deterministic, structured telemetry frames for visualization.
- Support multiple transports (IPC/WebSocket) without mutating simulation state.
- Provide plugin extensions for custom inspector frames.

---

## Frame Envelope

```ts
type FrameType =
    | "core"
    | "codex"
    | "bridge"
    | "capability"
    | "entropy"
    | "paradox";

interface InspectorEnvelope {
    frameType: FrameType;
    tick: ChronosTick;
    branch: KairosBranchId;
    payload: unknown;
}
```

> **Note:** The types above are proposed — they are not yet present in the generated protocol artifacts (`ttd-protocol-ts`): types.ts (missing: FrameType, InspectorEnvelope, InspectorCommand) and registry.ts (missing: corresponding registry entries). Treat this section as a draft contract.

- Frames emitted post `timeline_flush` each tick.
- **Frame ordering (normative):** Frames MUST be stable-sorted ascending by
  the composite key `(tick, frameType)`. `tick` is compared as an unsigned
  integer. `frameType` is compared lexicographically by UTF-8 byte order
  (e.g., `"bridge" < "capability" < "core"`). When two frames share the same
  `(tick, frameType)` pair, their relative order MUST match insertion order
  (i.e., the sort is _stable_). This ordering applies to both the in-memory
  frame buffer and the JSONL log written in deterministic mode.

---

## Transport

- Default: local WebSocket (`ws://localhost:<port>/echo-inspector`).
- CLI fallback: JSONL log for offline analysis.
- Remote inspector requires a signed session token (`ui:inspector` capability).

### Commands

```ts
interface InspectorCommand {
    op: "subscribe" | "unsubscribe" | "filter" | "snapshot";
    frameType?: FrameType;
    branch?: KairosBranchId;
    filter?: Record<string, unknown>;
}
```

Responses use `InspectorEnvelope`.

---

## Core Frames

- `InspectorFrame` – world metrics, system timings, entropy total.
- `CBInspectorFrame` – queue stats, latency metrics (from Codex’s Baby spec).
- `BridgeInspectorFrame` – pending events, retro records, paradox counts.
- `CapabilityInspectorFrame` – actor tokens, revocations.
- `EntropyFrame` / `ParadoxFrame` – entropy deltas, unresolved paradoxes.
- PLANNED: `StreamsFrame` – per-stream backlog, per-view cursors, and recent `StreamAdmissionDecision` records (see `spec-time-streams-and-wormholes.md`).

---

## Security

- Inspector is read-only; no mutation commands allowed.
- `ui:inspector` capability required for live feed.
- Session token includes allowed frame types; unauthorized frames omitted.

---

## Extensions

```ts
interface InspectorExtensionManifest {
    id: string;
    frameType: FrameType;
    schema: JSONSchema;
    producer(tick: ChronosTick): unknown;
}
```

Plugins register manifests via plugin system; frames included in sorted order.

---

This protocol standardizes inspector communications for Echo editors, debuggers, and remote tooling.
