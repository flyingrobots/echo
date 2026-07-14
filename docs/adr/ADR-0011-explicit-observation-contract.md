<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ADR-0011: Explicit Observation Contract

- **Status:** Partially superseded
- **Superseded in part by:** [ADR 0021](0021-public-optic-observation-boundary.md), which replaces the claim that `observe(...)` is the only canonical public read entrypoint
- **Date:** 2026-03-15
- **Amends:** ADR-0008, ADR-0010
- **Related:** ADR-0009

ADR 0011 remains authoritative for explicit coordinates, frames, projections,
read-only execution, deterministic serialization, and observation artifact
identity. ADR 0021 governs the public WARP-optic versus internal observation
boundary.

## Context

Echo's write path is already worldline-native:

- runtime ingress is explicit,
- provenance is entry-based,
- parent refs are stored rather than reconstructed,
- replay is grounded in recorded history,
- BTRs exist as deterministic contiguous provenance containers.

The read path still lags behind that architecture. Snapshot, head, truth-drain,
and query-shaped operations are currently exposed as separate surfaces with
different implicit coordinate stories. That leaves too much hidden:

- which worldline is being read,
- which historical coordinate is being observed,
- whether the read is a commit-boundary view or recorded-truth view,
- whether the read is reconstructive or current-frontier,
- and which parts of the runtime are allowed to mutate as a side effect.

The system already knows it lives on worldlines. Reads must stop pretending
otherwise.

## Decision

### 1. Observation is the canonical read contract

Echo reads are observations of a worldline at a coordinate under a declared
frame and projection.

The canonical internal entrypoint is:

```rust
ObservationService::observe(
    runtime: &WorldlineRuntime,
    provenance: &ProvenanceService,
    engine: &Engine,
    request: ObservationRequest,
) -> Result<ObservationArtifact, ObservationError>
```

All meaningful reads must flow through this path.

### 2. Observation is explicit about coordinate, frame, and projection

The v1 observation request surface is:

```rust
pub struct ObservationCoordinate {
    pub worldline_id: WorldlineId,
    pub at: ObservationAt,
}

pub enum ObservationAt {
    Frontier,
    Tick(u64),
}

pub enum ObservationFrame {
    CommitBoundary,
    RecordedTruth,
    QueryView,
}

pub enum ObservationProjection {
    Head,
    Snapshot,
    TruthChannels { channels: Option<Vec<ChannelId>> },
    Query { query_id: u32, vars_bytes: Vec<u8> },
}
```

The frame/projection validity matrix is closed and centralized:

- `CommitBoundary` → `Head`, `Snapshot`
- `RecordedTruth` → `TruthChannels`
- `QueryView` → `Query`
- all other combinations fail with deterministic `UnsupportedFrameProjection`

### 3. Observation is read-only by construction

Observation must not mutate:

- runtime frontier ticks,
- inbox state,
- committed-ingress ledgers,
- provenance history,
- worldline mirrors such as `tick_history`, `last_snapshot`, or recorded
  materialization fields.

Implementations should prefer immutable borrows all the way down:

- `&WorldlineRuntime`
- `&ProvenanceService`
- `&Engine`

If a helper cannot be expressed without mutation, it does not belong in this
phase.

### 4. Recorded truth means recorded truth

`RecordedTruth` observations read recorded outputs from provenance/history.

They do not re-run engine logic, recompute materialization, or synthesize truth
from current state under another name.

### 5. Resolved coordinates and artifact identity are first-class

Every observation returns explicit resolved coordinate metadata:

```rust
pub struct ResolvedObservationCoordinate {
    pub observation_version: u32,
    pub worldline_id: WorldlineId,
    pub requested_at: ObservationAt,
    pub resolved_tick: u64,
    pub state_root: Hash,
    pub commit_hash: Hash,
}
```

The observation artifact is identity-bearing:

```rust
pub struct ObservationArtifact {
    pub resolved: ResolvedObservationCoordinate,
    pub frame: ObservationFrame,
    pub projection: ObservationProjection,
    pub artifact_hash: Hash,
    pub payload: ObservationPayload,
}
```

### 6. Canonical serialization and hashing are normative

Observation artifact identity uses the repository's canonical CBOR rules.

`artifact_hash` is defined as:

```text
blake3("echo:observation-artifact:v2\0" || canonical_cbor(hash_input))
```

Where `hash_input` includes:

- observation version,
- resolved coordinate,
- frame,
- projection,
- canonical payload bytes,

and excludes `artifact_hash` itself.

Map-order dependence or serializer-specific field-order behavior is forbidden.

### 7. Query is reserved but intentionally unsupported

The only valid query-shaped pairing in v1 is:

- `QueryView + Query { ... }`

That pairing is still allowed to fail with deterministic `UnsupportedQuery`
until real query support exists.

Query resolution must lower through the explicit observation primitive. Public
application and adapter queries also obey ADR 0021's bounded optic boundary.

## Consequences

### Positive

- Reads become explicit about worldline and time.
- One canonical internal observation primitive replaces divergent implicit read
  semantics.
- Historical and current observations can share one deterministic identity model.
- Recorded truth becomes a real read contract rather than a side effect of
  mutable drain plumbing.

### Negative

- Adapters must name coordinates, frames, and projections instead of relying on
  implicit current state.
- Cursor or session helpers may accelerate reads but cannot become an
  independent causal or product read model.

## Non-Goals

This ADR does not introduce:

- rich observer profiles,
- governance or aperture-rights systems,
- translation-cost / observer-geometry machinery,
- multi-worldline coordinate models,
- implicit continuation from historical reads,
- `fork_from_observation(...)` itself.

## Historical Note

The original record included an ABI-phase migration diary, temporary adapter
list, deletion schedule, and future-work language. Those process artifacts
remain in Git history. ADR 0021 governs the current public optic boundary.
