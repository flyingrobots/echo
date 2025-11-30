<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Runtime Diagnostics Plan (Phase 0.5)

Outlines logging, tracing, crash recovery, and inspector data streams for Echo runtime.

---

## Logging Levels
- `TRACE` – verbose diagnostics (disabled in production).
- `DEBUG` – subsystem insights (branch tree, Codex’s Baby).
- `INFO` – major lifecycle events (fork, merge, replay start).
- `WARN` – recoverable anomalies (drop records, entropy spikes).
- `ERROR` – determinism faults (capability denial, PRNG mismatch).

Logs are structured JSON: `{ timestamp?, tick, branch, level, event, data }`. Timestamps optional and excluded from hashes.

---

## Crash Recovery
- On `ERROR`, emit synthetic timeline node with `errorCode`, `nodeId`, `diffId`.
- Persist crash report (JSON) including last inspector frames and capability state.
- Provide CLI `echo diagnostics --last-crash` to display report.

---

## Tracing
- Optional per-phase tracing (`TRACE` level) capturing start/end of scheduler phases, system durations.
- Output to separate trace buffer for tooling (`trace.jsonl`).

---

## Inspector Streams
- `InspectorFrame` (core metrics)
- `CBInspectorFrame` (Codex’s Baby)
- `BridgeInspectorFrame` (Temporal Bridge)
- `CapabilityInspectorFrame`

Frames emitted each tick after `timeline_flush`, appended to ring buffer (configurable size). Debug tools subscribe over IPC/WebSocket.

---

## Diagnostic CLI
- `echo inspect --tick <n>` – dump inspector frames.
- `echo entropy --branch <id>` – show entropy history.
- `echo diff <node>` – print diff summary.
- `echo replay --verify` – reuse replay contract.

---

## CI Integration
- Pipeline collects inspector frames for failing tests, attaches to artifacts.
- Warnings escalate to failures when thresholds exceeded (entropy > threshold without observer, repeated paradox quarantine).

---

This plan provides consistent observability without compromising determinism.
