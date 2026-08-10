<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo × Keep Physical Content Integration Plan

- **Status:** Change-local implementation and evidence plan.
- **Decision:** Architecture approved; production replacement not approved.
- **Historical source:** The original feasibility exploration is preserved in
  Git commit `fa943a6c0`.
- **Boundary owner:**
  [Echo × Keep physical content boundary](../architecture/echo-keep-physical-content-boundary.md)
- **Keep contract:** External `flyingrobots/keep` document
  [`docs/invariants/authenticated-reconstruction/README.md` at revision
  `2b87899853b61d2f616f98a33b3d45657af3f621`](https://github.com/flyingrobots/keep/blob/2b87899853b61d2f616f98a33b3d45657af3f621/docs/invariants/authenticated-reconstruction/README.md)

This plan owns sequencing, conformance evidence, and cutover gates. It does not
own the durable architecture boundary, live priority, or release status. The
Draft pull request and GitHub work items own current progress.

## Verdict

Keep is ready to become an experimental Echo backend. It is not ready to
become Echo's sole durable content authority, and `echo-cas` must not be
removed yet.

The next milestone is not “make Keep implement `BlobStore`.” It is:

> Give Echo a receipt-bearing physical-content boundary, wrap `echo-cas`
> behind it, then prove Keep conforms.

## Scope

The first integration proves:

- complete-object reconstruction;
- expected staged ingestion;
- explicit publication;
- same-source Echo and Keep identity corroboration;
- complete reconstructed-source corroboration;
- quarantined output visibility;
- authenticated success versus evidenced refusal versus operational failure;
- backend-neutral behavior across existing `echo-cas` and Keep's
  non-durable `ReferenceStore`.

The first integration does not claim:

- durable Keep authority;
- range-read support at the Echo port;
- a persisted identity-binding format;
- end-to-end bounded WSC recovery memory;
- production retention or crash recovery;
- `DiskTier` replacement;
- `echo-cas` removal.

## Milestone 1 — identity-bridge decision

Before adapter code, record the exact identity relation:

| Question                                             | Required evidence                                    |
| ---------------------------------------------------- | ---------------------------------------------------- |
| What fields form Echo's current content identity?    | Exact typed structure and canonical input bytes      |
| What algorithm and domain does Echo hash?            | Source anchor and golden fixtures                    |
| Does Echo identity include logical length?           | Explicit yes or no                                   |
| Can an Echo ID become a Keep `BlobId` without bytes? | Proof or explicit no                                 |
| Is the conversion one-to-one?                        | Argument over exact preimages plus mutation fixtures |
| Which identity remains in Echo WAL and WSC?          | Explicit ownership decision                          |
| What happens when versions or algorithms differ?     | Typed refusal; no implicit conversion                |
| Are both identities retained during migration?       | Explicit carrier posture                             |

The expected initial answer is that both identities are retained and their
relation is established from the same exact bytes. No 32-byte digest cast is
lawful.

The exploratory logical binding is private and noncanonical:

```rust
struct EchoKeepLogicalBindingV1 {
    echo_content_id: EchoContentId,
    keep_blob_id: KeepBlobId,
    logical_length: ByteLength,
    binding_contract: BindingContractVersion,
}
```

One logical binding may have multiple realization bindings:

```rust
struct EchoKeepRealizationBindingV1 {
    logical: EchoKeepLogicalBindingV1,
    keep_layout_id: KeepLayoutId,
}
```

These pseudostructures are obligation checklists, not persisted formats or
public APIs.

### Identity witnesses

Run both identity calculations over one source stream, then reconstruct
through Keep and independently recalculate Echo identity and exact length.

Required cases:

- empty bytes;
- small text;
- deterministic binary ramp;
- chunk boundary and boundary-plus-one sizes;
- large deterministic virtual input;
- arbitrary short and interrupted reads;
- nearby edits with deterministic chunk reuse;
- mismatched Echo identity;
- mismatched length;
- mismatched Keep `BlobId`;
- wrong or corrupt layout;
- missing or corrupt chunk.

Acceptance requires both routes. Same-source computation alone does not prove
retained reconstruction. Reconstructed-source computation is route-independent
evidence, not an independent BLAKE3 implementation.

## Milestone 2 — Echo physical-content port

Define the Echo-owned port from the
[boundary contract](../architecture/echo-keep-physical-content-boundary.md).
Keep types must not appear in its ordinary consumer surface.

First implement the existing Echo backends:

1. `MemoryTier` adapter;
2. `DiskTier` adapter;
3. any WSC retained-content consumer needed by the conformance fixture.

This establishes that the port is consumer-shaped, not reverse-engineered
from Keep.

The adapters must expose their actual evidence posture. A current backend that
cannot prove pinned-view absence must return unavailable evidence rather than
convert `None` into an authenticated absence receipt.

### Output witness

Inject a sink failure after a deterministic prefix and prove:

- no application-visible content is promoted;
- no Echo content observation is emitted;
- the prefix exists only in the destination's private staging artifact;
- abort leaves the prior destination state visible and the staged artifact
  explicitly unpublished;
- operational failure does not become a refusal receipt.

Inject commit failure after successful reconstruction and identity
corroboration. Prove that the sealed artifact remains unpublished, the prior
destination state remains visible, and no Echo observation is emitted. A
destination without atomic commit support must fail with
`CapabilityUnavailable` before reconstruction begins.

## Milestone 3 — backend-neutral conformance

Add a single conformance suite that runs against the existing Echo adapters and
the experimental Keep `ReferenceStore` adapter.

Mandatory laws:

| Case                                        | Required result                                          |
| ------------------------------------------- | -------------------------------------------------------- |
| Same bytes through different Keep layouts   | Same Echo observation; different physical provenance     |
| Missing content in a complete view          | Evidenced absence, not generic `None`                    |
| Missing content in an incomplete view       | No content claim                                         |
| Corrupt content                             | Integrity refusal; never fallback bytes                  |
| Sink failure after a prefix                 | No Echo observation; no promoted output                  |
| Range receipt where whole proof is required | Rejected by type or adapter                              |
| Expected identity mismatch during ingestion | No publication                                           |
| Re-layout between independent reads         | Logical observation stable; physical evidence may change |
| Exact `LayoutId` requested but unavailable  | Refusal; no alternate-layout fallback                    |
| Operational timeout or resource refusal     | No authenticated absence claim                           |

The complete-view absence cases must use the boundary's single witness rule.
The fixture supplies a known Echo-to-Keep binding, pinned view identifier,
versioned completeness predicate, authenticated view-root commitment,
target-bound non-membership witness, and retention guard for the witness
closure. It must prove:

- a valid witness under the matching complete view yields evidenced absence;
- an incomplete view yields no content claim;
- a witness for another target, root, or generation is rejected;
- a missing root, index page, witness node, or retention guard is an
  operational failure; and
- a present target can never be admitted through a forged absence witness.

The Keep adapter lives in Echo or an interop crate above both projects. Keep
must never index by Echo hash or import Echo semantics.

The first Keep backend uses `ReferenceStore` only. Its process-memory state is
not durable evidence.

## Milestone 4 — durable Keep consumer contract

Before durable integration, Keep must expose a consolidated generic capability
with this semantic shape:

```text
admitted immutable Keep view
+ target BlobId
+ optional exact LayoutId
+ proof scope
+ caller-owned output
────────────────────────────
receipt or evidenced refusal
    or operational failure
```

The durable operation must:

- pin one immutable generation or catalog view;
- retain all required evidence for the read lifetime;
- verify retained closure required by the proof scope;
- stream exact logical bytes into the adapter-owned private staging writer
  without one adapter-owned whole-blob memory allocation;
- name the generation in its receipt;
- distinguish evidenced refusal from operation failure;
- state that unsuccessful ordinary output may contain an untrusted prefix that
  remains quarantined and cannot become application-visible.

Echo, not Keep, owns sealing, Echo identity verification, and the atomic
destination commit. A durable Keep receipt does not publish the staged artifact
or authorize an Echo observation.

Keep owns this API in Keep vocabulary. Echo does not supply WSC, causal,
semantic, or outbox concepts to Keep core.

Production experiments remain limited to Keep's explicitly admitted platform
profile. Unsupported platforms return typed posture rather than degraded
durability claims.

Echo currently uses Rust 1.90 and Keep requires Rust 1.96. The durable adapter
must make that boundary explicit through a toolchain upgrade, separate package
or CI lane, or independently supported Keep MSRV change. It must not arrive as
an incidental dependency update.

## Milestone 5 — cross-store publication protocol

Keep publication and Echo WAL publication require a durable operation identity
and a recovery state machine.

The governing invariant is:

> Orphaned physical content is acceptable. A committed Echo reference to
> unavailable content is not.

Proposed choreography:

```text
1. Echo assigns operation identity O.
2. Adapter establishes expected Echo and Keep identities.
3. Keep stages and verifies the exact content.
4. Keep durably publishes under a provisional anchor or lease for O.
5. Echo commits the causal reference or observation for O.
6. Keep finalizes long-term retention for O.
7. Recovery records or derives the reconciled completion posture.
```

### Crash matrix

| Crash point                                            | Lawful recovery                                           |
| ------------------------------------------------------ | --------------------------------------------------------- |
| Before Keep publication                                | Discard or resume staging                                 |
| After Keep publication, before Echo WAL                | Preserve as provisional orphan; eventually collect        |
| After Echo WAL, before final retention                 | Recover through provisional anchor and finalize           |
| After retention finalization                           | Complete                                                  |
| Echo committed but Keep evidence missing or unprovable | Integrity obstruction; never silent fallback              |
| Keep published but Echo state unreadable               | Preserve provisional evidence until Echo recovery decides |

Exercise before, during, and after every physical synchronization and Echo WAL
commit boundary. Retry must be idempotent by operation identity.

The binding carrier and canonical encoding are intentionally deferred until
this protocol identifies what recovery must retain. Freezing either requires a
production ADR.

## Milestone 6 — shadow, backfill, and cutover decision

Migration retains both Echo and Keep identities.

Backfill procedure:

1. Read existing content from the declared source backend.
2. Calculate Echo identity and exact length.
3. Stage expected Keep identity from the same bytes.
4. Reconstruct from Keep into quarantine.
5. Recalculate Echo identity and length.
6. Admit the binding only after all values agree.
7. Record backend provenance and migration outcome.

Shadow comparison must never silently repair the authoritative result. A
difference is an obstruction with retained evidence.

During an explicit migration posture, policy may permit recorded fallback:

```text
Try Keep under migration policy.
If evidence is unavailable, consult echo-cas explicitly.
Record fallback and backend provenance.
Verify bytes and backfill Keep.
```

After Keep becomes authoritative:

```text
Keep refusal
    ≠ permission to consult echo-cas silently
```

### Production decision gates

- identical Echo identity for every fixture;
- backend-neutral conformance suite green;
- complete output-quarantine evidence;
- pinned-generation durable reads;
- crash recovery at every cross-store boundary;
- binding recovery after process death;
- retained-closure verification;
- deterministic layout-selection policy;
- bounded transient-memory measurements;
- explicit platform and toolchain posture;
- existing-content migration and rollback rehearsal;
- no semantic, WSC, or causal-evidence drift;
- no silent fallback.

Only then may a production ADR decide whether Keep replaces `DiskTier` for a
specific supported posture. That decision does not replace Echo content
identity, WSC hashes, semantic coordinates, causal anchors, materialization
intent or observation, or in-memory CAS uses.

## Receipt durability questions

Every stored Echo reference to a Keep receipt must declare which posture it
expects:

1. ephemeral statement about one completed operation;
2. locator for replayable retained evidence;
3. portable self-contained proof.

Current Keep reconstruction receipts are primarily posture 1. They can support
posture 2 only when their generation and supporting evidence remain retained.
They are not automatically posture 3.

The adapter must not persist a receipt as durable causal evidence until its
supporting-evidence retention and revalidation contract is explicit.

## Validation commands

Documentation changes on this branch must pass:

```bash
cargo xtask docs-lint
tests/docs/test_adr_namespace.sh
git diff --check
```

Implementation milestones add the narrow executable witnesses described above
and the directly relevant workspace checks. A green documentation plan is not
implementation evidence.

## Stop conditions

Stop and require a separate decision before:

- freezing public port types;
- freezing persisted binding bytes or a digest domain;
- changing Echo or WSC content identity;
- introducing a Keep dependency into Echo's ordinary Rust 1.90 workspace;
- claiming authenticated absence from an incomplete view;
- treating a Keep receipt as an Echo observation;
- treating a range receipt as complete-object proof;
- removing or silently bypassing `echo-cas`;
- declaring Keep the sole durable content authority.
