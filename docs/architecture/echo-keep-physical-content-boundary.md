<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo × Keep Physical Content Boundary

- **Status:** Accepted for experimental conformance; production adoption is
  not accepted.
- **Decision date:** 2026-08-09
- **Implementation posture:** No Echo physical-content port or Keep adapter is
  implemented on this branch.
- **Refines:** [Retained reading storage and proof boundary](../adr/0020-retained-reading-storage-and-proof-boundary.md)
- **Depends on:** [Durable external-action settlement](../adr/0026-durable-external-action-settlement.md)
- **Related:** [Keep authenticated reconstruction contract](https://github.com/flyingrobots/keep/blob/3bf7b9179db41e90620e6d1875c2d40222a2330b/docs/architecture/authenticated-reconstruction-contract.md)

## Decision

> Keep establishes physical content truth. Echo decides what that truth means
> causally.

Keep may become an experimental physical backend for Echo. It must not replace
`echo-cas` until the identity bridge, consumer port, durable read aperture,
cross-store crash protocol, migration, and rollback gates have executable
evidence.

Echo owns the substitution boundary:

```text
echo-core
    │
    ▼
Echo-owned physical-content port
    ├──────────────▶ existing echo-cas adapter
    │
    └──────────────▶ echo-keep adapter ─────────▶ Keep
```

Keep must not depend on Echo, expose Echo concepts, or adopt the current weak
`BlobStore` contract as its foundational public API. Keep-specific types remain
inside the adapter and a physical-evidence envelope; they do not cross ordinary
Echo semantic APIs.

[Echo issue #722](https://github.com/flyingrobots/echo/issues/722) owns
change-local integration sequencing and evidence gates. This page owns the
boundary regardless of which implementation phase is active.

## Division of responsibility

| Echo owns                                     | Keep owns                            | Adapter owns                                           |
| --------------------------------------------- | ------------------------------------ | ------------------------------------------------------ |
| Echo content identity and WSC hashes          | Versioned logical `BlobId`           | Witnessed Echo-ID to Keep-ID binding                   |
| Semantic coordinates and application meaning  | `LayoutId` and physical realizations | Translation between Echo requests and Keep coordinates |
| Causal basis, law, and observer aperture      | Chunking, segments, and catalogs     | Quarantined output and receipt validation              |
| Materialization intent and observation        | Physical publication and recovery    | Physical-evidence envelope for Echo                    |
| Retry, admissibility, and history publication | Retention generations and closure    | Cross-store operation reconciliation                   |

Keep receipts never grant Echo authority. Echo observations never prove Keep
presence, retention, or durability without matching Keep evidence.

```text
Keep receipt
    "These exact bytes were authenticated from this physical evidence."

Echo observation
    "Under this causal basis and law, this physical result matters this way."
```

One must not silently become the other.

## Echo-owned consumer shape

The first substitution boundary is a fallible, complete-object,
caller-output-oriented port. The following pseudostructure names semantic
obligations, not a frozen Rust ABI:

```rust
fn reconstruct(
    &self,
    view: &PhysicalContentView,
    target: EchoContentId,
    destination: &mut dyn TransactionalContentDestination,
) -> Result<ReconstructionDecision, ReconstructionOperationError>;
```

`PhysicalContentView` is an Echo-owned opaque capability. A Keep adapter binds
it to one pinned Keep generation or immutable view. An `echo-cas` adapter may
support a weaker posture initially, but it must report unsupported evidence
rather than manufacture generation or absence claims.

`TransactionalContentDestination` is likewise an Echo-owned semantic
capability, not an ordinary `Write` alias or a frozen ABI. It supplies private
staging plus an atomic commit operation. A destination that cannot guarantee
all-or-nothing visibility reports `CapabilityUnavailable` before
reconstruction.

The initial port includes:

- complete-object reconstruction;
- expected staged ingestion;
- explicit physical publication;
- authenticated success receipts;
- evidenced refusal only where the backend can prove it;
- operational failures that carry no content claim;
- quarantined output semantics.

The initial port excludes:

- range reads;
- an optional range argument;
- consumer-controlled Keep layouts unless required by a real consumer;
- generalized compaction controls;
- a broad retention-management trait;
- implicit backend fallback.

Echo may retain a bounded materializing helper implemented over this port. The
helper is not Keep's foundational contract and must require an explicit byte
limit.

## Output visibility

An ordinary `Write` sink can fail after accepting a prefix. Keep may therefore
leave an untrusted prefix in the writer supplied by its adapter. That writer is
always the private staging writer of the transactional destination, never an
application-visible sink.

The Echo adapter enforces:

> No complete receipt, no application-visible bytes.

The adapter follows one visibility protocol:

1. Begin a destination transaction and obtain its private staging writer.
2. Reconstruct through the backend into that writer.
3. On any failure, abort; a staged prefix may remain but is never visible.
4. Seal the staging artifact and verify the complete receipt, Echo identity,
   and exact length.
5. Atomically commit the sealed artifact or return an operational failure.
6. Emit an Echo content observation only after commit succeeds.

Commit changes the destination from its prior state directly to the complete
sealed artifact. A failed commit leaves the prior state visible and the new
artifact unpublished. Implementations may use an atomic same-filesystem rename
or return a sealed immutable handle; copying into an arbitrary live writer is
not a conforming commit.

```text
backend reconstruction
    │
    ▼
unpublished temporary artifact
    │
    ├── failure ──▶ discard; reveal nothing
    │
    └── receipt ──▶ verify Echo identity; promote
```

Quarantine may be a bounded memory buffer for small content or a temporary
artifact for larger content. The rule must not imply unbounded whole-object
memory allocation. Crash cleanup may collect unpublished staging artifacts,
but recovery must never infer publication from their existence.

## Decisions and failures

Content truth and operational success are separate result planes:

```rust
enum ReconstructionDecision {
    Authenticated(ReconstructionReceipt),
    Refused(ReconstructionRefusalReceipt),
}

enum ReconstructionOperationError {
    InputOutput,
    ResourceLimit,
    Cancellation,
    CapabilityUnavailable,
}
```

These names are conceptual and not a frozen ABI.

An authenticated refusal must bind enough evidence to support its proposition.
Absence is evidenced only for a known Keep `BlobId` binding and one pinned view
whose named membership-index contract certifies that it covers every published
logical `BlobId` admitted by that view. No Echo-to-Keep binding means
`CapabilityUnavailable`, not physical absence.

A complete-view absence receipt binds:

- the Echo identity, Keep `BlobId`, and exact pinned-view identifier;
- the versioned completeness predicate and authenticated view-root commitment;
- a target-bound non-membership witness under that root; and
- the retention guard or anchor that kept the root and witness closure valid
  through verification.

The backend may realize the witness with an authenticated ordered index, a
Merkle non-membership proof, or a complete canonical scan. The adapter must
verify the format-specific witness, completeness predicate, target, and view
root before constructing the backend-neutral refusal. Missing or unreadable
root records, index pages, retention evidence, or witness material are
operational failures. A timeout, exhausted resource limit, cancellation, or
unavailable capability likewise teaches Echo nothing about content truth.

Echo may admit an evidenced refusal as an observation only under an Echo law
that explicitly accepts that refusal class and its physical aperture. A Rust
error alone is not a witnessed refusal.

## Identity bridge

Echo `BlobHash` and Keep `BlobId` are distinct typed identities. They must not
be cast, substituted, or equated because both currently use 32-byte BLAKE3
digests.

The adapter establishes their relation by applying both identity laws to one
exact source stream and then verifying reconstruction:

```text
                    ┌── Echo identity law ──▶ EchoContentId
one exact stream ───┤
                    └── Keep identity law ──▶ Keep BlobId
```

The resulting logical binding retains both identities and the exact logical
length. A separate realization binding names one or more lawful Keep layouts.
The persisted carrier, canonical bytes, and digest domain remain undecided.

Keep's `stage_expected` can verify an expected Keep `BlobId`; it cannot by
itself prove an Echo-to-Keep identity relation. The adapter owns the second
identity calculation and the binding witness.

## Semantic observation and physical evidence

Ordinary Echo semantics and Keep realization provenance remain separate:

```rust
struct ContentObservation {
    content_id: EchoContentId,
    logical_length: ByteLength,
    proof_scope: CompleteObject,
}

struct PhysicalEvidence {
    keep_blob_id: KeepBlobId,
    keep_generation: KeepGenerationId,
    keep_layout_id: KeepLayoutId,
    receipt_version: ReceiptVersion,
}
```

These are conceptual ownership examples, not frozen records.

Different lawful Keep layouts for the same exact bytes produce the same
ordinary Echo content observation and different physical provenance. Echo must
not hash `LayoutId` into ordinary semantic state merely because a receipt
contains it.

If no exact layout was requested, the Keep adapter may select any admitted
realization under a deterministic policy. If an exact layout was requested, it
must use that layout or refuse; it may not fall back to another layout.

## Physical observer aperture

Each Echo execution frame or work unit should hold one physical-content view:

```text
Echo execution frame
    ├── causal basis and graph view
    └── PhysicalContentView
            └── pinned Keep generation capability
```

Every physical read in the frame resolves through that same view. The adapter
must retain the underlying evidence for the view's lifetime and must not
silently advance it.

New content produced during a frame remains staged or appears through an
explicit Echo-owned overlay. It becomes part of a later physical view only
after the causal and physical publication protocol completes.

This prevents mixed-generation reads, mutable-`HEAD` coupling, and receipts
whose physical evidence aperture is unclear.

## Cross-store invariant

Keep publication and Echo WAL publication are separate durable transitions.
Their integration requires an explicit operation identity, provisional
physical retention, and recovery reconciliation.

The governing invariant is:

> Orphaned physical content is acceptable. A committed Echo reference to
> unavailable content is not.

Echo must commit the authorized physical-publication request and claim before
Keep performs the publication effect. Keep then publishes under a durable
reconciliation anchor keyed by that operation identity, and Echo records the
result as a settlement or observation before any execution resumes on it. This
is the request-before-effect and settlement-before-resumption law from
[ADR 0026](../adr/0026-durable-external-action-settlement.md).

The reconciliation anchor is not a time-expiring lease. Keep must retain it
until an explicit recovery transition proves that long-term retention is
durable or that Echo durably settled abandonment before publication. Echo
unavailability cannot make the anchor expire. Finalization makes long-term
retention durable before releasing the provisional anchor.

Issue #722 owns the executable crash matrix. The read port must not conceal the
cross-store state machine or convert “write Keep, then write Echo” into an
implicit protocol.

## Fallback

During an explicitly declared migration posture, Echo may consult the old
backend after a Keep miss only when policy records the fallback, re-verifies
the bytes, and schedules or performs an explicit backfill.

Once Keep is authoritative, a Keep refusal is not permission to ask the old
store silently. Silent fallback would conceal the absence, corruption, and
retention failures the boundary exists to expose.

## Current source posture

Echo's current [`BlobStore`](../../crates/echo-cas/src/lib.rs) is synchronous
and materializing. Its `get` path collapses ordinary absence into `Option` and
returns a complete `Arc<[u8]>`. [`DiskTier`](../../crates/echo-cas/src/disk.rs)
already needs a separate fallible API, demonstrating that the existing trait
is too weak for durable physical evidence.

The current [WSC CAS port](../../crates/warp-core/src/wsc/store.rs) returns
`Option<Vec<u8>>` and likewise collapses failure posture while requiring full
materialization.

[ADR 0020](../adr/0020-retained-reading-storage-and-proof-boundary.md) already
requires byte identity, semantic reading identity, and proof identity to remain
distinct. This boundary extends that law to the Keep integration without
changing WSC wire identity.

## Production adoption gate

A production Keep backend, persisted binding format, changed WSC identity, or
permanent `echo-cas` replacement requires a separately reviewable accepted
change to Echo's named physical-content documentation after the conformance and
crash evidence exists. That change must refine or supersede this experimental
boundary explicitly and govern:

- exact Echo and Keep identity preimages;
- binding carrier, encoding, versioning, and recovery;
- physical-view lifetime and retention;
- success and refusal receipt formats;
- output quarantine and promotion;
- cross-store crash recovery;
- toolchain and supported platforms;
- backfill, rollback, and fallback removal;
- compatibility and release posture.

Until then, Keep is an experimental Echo backend, not Echo's sole durable
content authority.
