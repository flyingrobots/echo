<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Inspector & Editor Protocol Specification (Phase 0.75)
> **Background:** For a gentler introduction, see [WARP Primer](/guide/warp-primer).


Unifies Echo’s inspector data streams, transport contracts, and extension hooks for tooling.

---

## Goals
- Deliver deterministic, structured telemetry frames for visualization.
- Support multiple transports (IPC/WebSocket) without mutating simulation state.
- Provide plugin extensions for custom inspector frames.

---

## Frame Envelope

```ts
type FrameType = "core" | "codex" | "bridge" | "capability" | "entropy" | "paradox";

interface InspectorEnvelope {
  frameType: FrameType;
  tick: ChronosTick;
  branch: KairosBranchId;
  payload: object;
}
```

- Frames emitted post `timeline_flush` each tick.
- Order stable: sorted by `(tick, frameType)`.
- Frames written to JSONL log in deterministic mode.

---

## Transport
- Default: local WebSocket (`ws://localhost:<port>/echo-inspector`).
- CLI fallback: JSONL log for offline analysis.
- Remote inspector requires signed session token (`ui:inspector` capability).

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
- PLANNED: `StreamsFrame` – per-stream backlog, per-view cursors, and recent `StreamAdmissionDecision` records (see `docs/spec-time-streams-and-wormholes.md`).

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
  producer(tick: ChronosTick): object;
}
```

Plugins register manifests via plugin system; frames included in sorted order.

---

This protocol standardizes inspector communications for Echo editors, debuggers, and remote tooling.
