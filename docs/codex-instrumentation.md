# Codex’s Baby Instrumentation Plan

This document defines the telemetry, logging, and debugging hooks for Codex’s Baby. Instrumentation must be deterministic-friendly and cheap enough for production builds, with richer introspection available in development.

---

## Metrics

### Counters (per phase)
- `enqueued[phase]` – total envelopes enqueued per tick.
- `dispatched[phase]` – envelopes delivered to handlers.
- `dropped[phase]` – envelopes dropped due to backpressure.
- `flushDuration[phase]` – cumulative time spent flushing the queue (ns).
- `handlerDuration[kind]` – total & average execution time per command kind.
- `bridgeDeliveries` – number of cross-branch messages delivered.
- `paradoxFlags` – count of envelopes flagged by paradox guard.

All counters reset each tick but accumulated into rolling averages for inspector display (e.g., exponentially weighted moving average).

### Gauges
- Queue high-water mark (max size reached).
- Current queue size per phase (for HUD display).
- Backpressure severity level (0 = normal, 1 = warning, 2 = critical).

---

## Tracing

### Event Trace Buffer
- Ring buffer storing up to N envelopes (configurable, default 256).
- Each entry logs:
  - `timestamp` (Chronos tick + frame-relative index)
  - `phase`
  - `kind`
  - `kairos`
  - `metadata` subset (filtered)
  - Optional handler outcome (success, dropped, error)
- Buffer can be sampled by inspector or exported for offline debugging.

### Debug Hooks
- `codex.onEnqueue(fn)` – receives envelope metadata (without payload unless flag set).
- `codex.onDispatch(fn)` – after handler returns, receives duration + result.
- Hooks only active in dev mode; no-op in production.

---

## Backpressure Alerts
- When queue exceeds 80% capacity, emit warning event (once per tick) to instrumentation system.
- At 95%, escalate to error, optionally trigger gameplay callbacks (e.g., slow down spawn rate).
- Provide optional “dropOldest” logging with summary of dropped kinds.

---

## UI Integration
- Timeline inspector panel:
  - Graph of enqueued vs dispatched per phase.
  - Table of top command kinds by handler duration.
  - Indicator for cross-branch traffic (counts, entropy contribution).
- HUD overlay (optional) showing real-time queue occupancy.

---

## Configuration
```ts
interface CodexInstrumentationOptions {
  readonly traceCapacity?: number;
  readonly capturePayloads?: boolean;    // defaults false, enables deep logging
  readonly enableDevAssertions?: boolean;
  readonly highWaterThreshold?: number;  // default 0.8
  readonly criticalThreshold?: number;   // default 0.95
}
```
- Engine options pass these into Codex’s Baby on initialization.
- Capture payloads only in secured dev builds to avoid leaking sensitive info.

---

## Determinism Safeguards
- Timestamps use deterministic sequence numbers, not wall-clock time.
- Traces stored per branch; merging traces should maintain order by Chronos/Kairos.
- All instrumentation writes go through dedicated buffer to avoid interfering with queue order.
- No random sampling; use deterministic sampling intervals (e.g., log every Nth envelope).

---

## Tasks
- [ ] Implement metrics struct and per-phase counters.
- [ ] Add `onEnqueue` / `onDispatch` hooks (dev only).
- [ ] Build ring buffer trace with configurable capacity.
- [ ] Expose metrics via inspector API.
- [ ] Add tests covering counter increments and backpressure alerts.
