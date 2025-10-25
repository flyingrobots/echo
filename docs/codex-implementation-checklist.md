# Codex's Baby Implementation Checklist

A step-by-step guide for turning the event bus spec into working code.

---

## 1. Core Data Structures
- [ ] Define `CommandEnvelope` type (generics, metadata defaults).
- [ ] Implement `CommandQueue` ring buffer with growable capacity.
- [ ] Create handler registry (`Map<phase, Map<kind, Handler[]>>`).
- [ ] Add metrics struct (counters, gauges placeholders).

## 2. Initialization
- [ ] Accept configuration (queue capacities, backpressure policy, instrumentation options).
- [ ] Instantiate queues for each scheduler phase.
- [ ] Wire instrumentation hooks (no-op stubs when disabled).

## 3. Enqueue Path
- [ ] Implement `enqueue(phase, envelope)` with capacity checks and growth.
- [ ] Update metrics + tracing ring buffer.
- [ ] Handle immediate channel (`enqueueImmediate`).

## 4. Flush & Dispatch
- [ ] `flushPhase` iterating queue FIFO order.
- [ ] Dispatch to handlers with deterministic sorting (priority desc, registration order).
- [ ] Support once-handlers (auto-unregister).
- [ ] Record handler duration (dev mode only).

## 5. Handler Registration API
- [ ] Public `registerHandler` / `unregisterHandler` functions.
- [ ] Validate phase alignment and duplicate detection.
- [ ] Provide scoped registration helper for systems (auto unregister on system remove).

## 6. Inter-Branch Bridge
- [ ] Implement bridge buffer keyed by `(branchId, chronos)`.
- [ ] Validate chronology; spawn retro branch via callback.
- [ ] Delivery integration in `timeline_flush`.
- [ ] Track entropy/paradox flags.

## 7. Instrumentation
- [ ] Metrics update points (enqueued, dispatched, dropped).
- [ ] Backpressure alerts (threshold checks).
- [ ] Trace ring buffer with configurable capacity / payload capture.
- [ ] Dev hooks (`onEnqueue`, `onDispatch`).

## 8. Testing
- [ ] Unit tests for queue wraparound & growth.
- [ ] Handler ordering determinism tests.
- [ ] Backpressure behavior (throw/drop).
- [ ] Bridge delivery test (cross-branch message).
- [ ] Instrumentation toggle tests (metrics increments).

## 9. Integration
- [ ] Wire into scheduler phases (`pre_update`, `update`, etc.).
- [ ] Expose API on `EchoEngine` (context injection into systems/handlers).
- [ ] Document usage in developer guide.

## 10. Follow-up
- [ ] Add inspector panel to display metrics.
- [ ] Extend `docs/decision-log.md` with a bus-event template (optional).
- [ ] Profile throughput with scheduler benchmarks.
