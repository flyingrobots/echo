# ADR-0010: Observational Seek, Explicit Snapshots, and Administrative Rewind

- **Status:** Proposed
- **Date:** 2026-03-10
- **Amends:** ADR-0008
- **Related:** ADR-0009

## Context

ADR-0008 correctly establishes that:

- seek/jump is head-local,
- reader heads replay provenance and never mutate the frontier,
- global rewind is not the default playback API.

That text works well when one imagines head-local replay state. But the
implementation plan for ADR-0008 and ADR-0009 intentionally adopts **one mutable
frontier state per worldline**. Under that model, a generic writer `seek(head, t)`
is easy to misimplement: rewinding a writer head can accidentally become a rewind
of the shared live worldline.

That is not a harmless API wrinkle. It collides with ADR-0008's own invariants:

- seek must be head-local,
- seeking must not globally rewind unrelated worldlines,
- replay observes provenance; it does not become the live mutation mechanism.

The implementation plan therefore needs an explicit clarification.

## Decision

### 1. Seek is observational

`seek(...)` is an observational operation over provenance-backed historical state.

It is valid for:

- reader heads,
- debugger sessions,
- historical snapshots,
- test and verification workflows.

It is **not** the default mechanism for rewinding a live shared writer frontier.

### 2. Reader seek is the primary seek API

The runtime surface should expose an explicit reader-oriented API:

```rust
seek_reader(reader_head, target_tick)
```

Semantics:

- rebuild reader-local view from provenance,
- clamp to frontier if `target_tick > frontier`,
- never synthesize future state,
- never mutate any live worldline frontier.

### 3. Historical snapshots are first-class

The runtime should expose an explicit snapshot API:

```rust
snapshot_at(worldline_id, target_tick)
```

This returns a read-only reconstructed historical view for debuggers, tools,
tests, and comparison workflows.

Historical inspection should not require mutating a live writer head.

### 4. Fork is the sanctioned way to continue execution from the past

If the caller wants to **inspect the past and then continue execution from it**,
the correct primitive is:

```rust
fork(worldline_id, fork_tick, new_worldline_id)
```

The new worldline gets reconstructed state at `fork_tick` and an independent
future. This preserves append-only provenance and avoids destructive rewind of a
shared live frontier.

### 5. Administrative rewind is separate and explicit

If a destructive rewind of a live worldline is truly required for maintenance,
testing, or migration, it must use a distinct administrative API such as:

```rust
rewind_worldline(worldline_id, target_tick)
```

Requirements:

- explicit capability gating,
- unavailable by default in ordinary runtime/app flows,
- clearly marked as administrative/testing behavior.

### 6. Replay helpers never drive live writer advancement

Replay/apply helpers remain valid for:

- reader seek,
- snapshot construction,
- worldline rebuild,
- fork reconstruction.

They are never the mechanism by which a live writer head advances the frontier.
Live mutation continues to flow through deterministic commit only.

## Consequences

### Positive

- Aligns the API surface with the single-frontier-state-per-worldline design.
- Removes ambiguity around writer `seek`.
- Makes debugger and tool workflows cleaner via explicit snapshot semantics.
- Preserves append-only provenance and fork-first branching semantics.

### Negative

- Existing call sites expecting a generic `seek(head, t)` API must migrate.
- Some testing helpers may need to move from destructive rewind to snapshot/fork.
- Administrative rewind becomes a visibly privileged path instead of a casual utility.

## Implementation Guidance

The implementation plan should therefore prefer the following API family:

- `seek_reader(...)`
- `jump_to_frontier(...)`
- `snapshot_at(...)`
- `fork(...)`
- `rewind_worldline(...)` (admin/testing only)

and should deprecate generic global rewind helpers such as `jump_to_tick()`.

## Non-Goals

This ADR does not:

- forbid all administrative rewind forever,
- change ADR-0008's acceptance of fork as a core runtime primitive,
- alter ADR-0009 transport or conflict semantics,
- prescribe any particular snapshot serialization format.

## Supersession Note

This ADR clarifies the operational reading of ADR-0008 Section 6.
If accepted, implementations should treat generic head `seek(...)` wording as
refined by the explicit observational/admin split described here.