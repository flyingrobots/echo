<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Causal Anchors

A causal-anchor claim is an application-requested, canonical claim over a basis
in causal history. It is not a materialized snapshot, an admission receipt, or
proof that referenced history and roots exist. The current implementation binds
a subject, basis-frontier digest, root set, and purpose into a deterministic
claim without admitting or publishing it.

The short rule is:

```text
CAS addresses content. A causal-anchor claim binds meaning to supplied references.
Only an Echo-owned transition may turn that claim into an admitted fact and receipt.
```

## Why Not Checkpoints

The word "checkpoint" is overloaded. In many systems it means a full materialized
state snapshot. That is not the primitive Echo needs.

Echo needs a generic causal anchor. Applications may use that anchor to implement
domain checkpoints, but Echo should not learn application nouns such as editor
buffer, rope head, mail thread, calendar event, build artifact, or dirty file.

The boundary is:

| Layer                   | Responsibility                                                                                                                                       |
| ----------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| CAS object              | A hash addresses bytes, facts, manifests, or projection material; availability requires separate CAS evidence.                                       |
| Projection cache        | Derived observer-relative materialization can be reused when its basis, aperture, observer authority, policy, schema, evaluator, and coverage match. |
| Causal-anchor claim     | A canonical claim binds a subject, supplied basis-frontier digest, root sets, and purpose without claiming admission.                                |
| Trusted admitted anchor | Echo committed a canonical claim and its admission receipt atomically through the causal WAL. The app-facing host API lands separately.            |
| Domain checkpoint       | An application explains what an Echo anchor means in domain terms.                                                                                   |

A graph-wide materialized checkpoint may exist as an export, backup, or diagnostic
artifact, but it must not be the normal meaning of an Echo anchor.

## Contract Shape

The precise wire type can evolve, but the domain shape should preserve these
fields:

```ts
type CausalAnchorPurpose =
    | "recovery"
    | "retention"
    | "export"
    | "user-save"
    | "autosave"
    | "debug"
    | "cache-warm";

type CausalAnchorSubject = {
    appId: string;
    subjectKind: string;
    subjectId: string;
};

type CausalAnchorRoot =
    | {
          kind: "echo.cas.Object";
          id: string;
          role: "materialization" | "manifest" | "index";
      }
    | {
          kind: "echo.graph.Fact";
          id: string;
          role: "authority" | "evidence" | "index";
      }
    | {
          kind: "app.subject.Root";
          appId: string;
          subjectKind: string;
          id: string;
          role: "authority" | "evidence";
      };

type CausalAnchorAdmissionRequest = {
    schemaVersion: number;
    subject: CausalAnchorSubject;
    basisFrontier: CausalFrontier;
    retainedRoots: readonly CausalAnchorRoot[];
    materializationRoots?: readonly CausalAnchorRoot[];
    purpose: CausalAnchorPurpose;
};

type CausalAnchorClaim = CausalAnchorAdmissionRequest & {
    claimDigest: string;
};

type CausalAnchorFact = CausalAnchorClaim & {
    kind: "echo.causal.Anchor";
    anchorId: string;
    admittedByReceiptId: string;
    anchorDigest: string;
};
```

`basisFrontier` is deliberate. A single tick id is too narrow for subjects whose
basis may include strands, branches, merges, imported suffixes, or multiple
receipt families. Linear applications can still supply a frontier with one head.
The current constructor accepts the frontier digest as an opaque caller-provided
reference; it does not prove that the frontier was admitted.

`retainedRoots` claim authority or evidence roots. `materializationRoots` claim
derived projection artifacts such as flattened text, line indexes, rendered
slices, or export blobs. Construction validates the declared roles and the shape
of the two sets, not the existence or authority of the referenced objects. A
materialized projection must not become authority just because it is convenient
to load. The two root sets are disjoint: a root cannot be both retained evidence
and a materialized projection in the same value.

`CausalAnchorAdmissionRequest` deliberately has no receipt field. The canonical
claim constructor cannot create `CausalAnchorFact`. Echo's trusted admission
transition derives `admittedByReceiptId` from the claim and WAL coordinate and
commits it with the claim. The application-facing host method that invokes this
transition lands in CA-01 Slice 3.

## Value Construction And Trusted Admission Boundary

Applications request anchor admission because applications know when a domain
boundary is meaningful:

- An editor knows when a buffer was manually saved.
- A build system knows when an artifact frontier is reproducible.
- A mail app knows when a thread index should be retained.
- A debugger knows when a replay basis should be pinned.

The implemented value and WAL flow is:

1. The application identifies a subject, basis frontier, retained roots, optional
   materialization roots, and purpose.
2. `CausalAnchorClaim::from_admission_request` validates subject and root-set
   shape, canonicalizes the roots, and derives the claim digest.
3. The application receives a canonical claim. No `CausalAnchorFact`, WAL
   record, retention pin, or runtime admission receipt is created by that
   operation.
4. Echo's admission-kernel WAL path derives the receipt identity from the claim
   and WAL coordinate, constructs the admitted fact and receipt, and places both
   in one `CausalAnchorAdmission` transaction.
5. Only a fully committed transaction appears in
   `recover_causal_anchor_admissions(...)`; recovery validates frame
   cardinality, payload integrity, cross-evidence identity, and WAL coordinates.

The frontier and roots are caller-provided references. No current
application-facing host API verifies those references and requests the WAL
transition. Code that receives a `CausalAnchorClaim` must not treat it as proof
of admission.

The remaining trusted-host admission authority must validate that the frontier
is admitted, the request is lawful for the subject, and the declared roots meet
generic support policy before invoking the WAL transition. The WAL layer owns
receipt derivation, atomic fact/receipt recording, and recovery; it does not
validate application-domain root semantics or imply a physical retention pin.

Applications may construct canonical anchor claims, but they cannot confer Echo
admission on those claims. Echo must not encode application semantics into the
generic claim contract.

## Jim And Rope Checkpoints

For `jedit`, the domain checkpoint can be a thin fact over an Echo-admitted
causal anchor:

```ts
type RopeCheckpointReason =
    | "manual-save"
    | "autosave"
    | "import"
    | "retention-boundary"
    | "export"
    | "test-fixture";

type RopeCheckpointFact = {
    kind: "jedit.text.RopeCheckpoint";
    schemaVersion: 1;
    checkpointId: string;
    worldlineId: string;
    headId: string;
    causalAnchorId: string;
    reason: RopeCheckpointReason;
};
```

Jim says:

```text
This rope head is the text-domain thing being checkpointed.
```

The canonical causal-anchor claim says:

```text
This caller claimed this subject, basis frontier, and retained root set for this
purpose.
```

An admitted Echo receipt additionally says that Echo accepted that exact claim
at its named causal basis. Jim must not invent that receipt locally.

A Jim save anchor claim can name the rope head as retained authority:

```ts
retainedRoots: [
    {
        kind: "app.subject.Root",
        appId: "jedit",
        subjectKind: "RopeHead",
        id: headId,
        role: "authority",
    },
];
```

An optional flat UTF-8 file projection belongs under `materializationRoots`, not
`retainedRoots`, unless the application explicitly models that projection as
authority.

Creating a rope checkpoint may request and, after Echo admission, receive:

- a canonical causal-anchor claim;
- an Echo-admitted anchor fact and receipt;
- a Jim `RopeCheckpointFact`;
- optional projection materialization evidence.

It must not mint:

- a new rope head;
- a rope rewrite;
- a rope diff;
- replacement blobs;
- text mutation evidence.

A checkpoint is a causal event. It is not a text edit.

## Projection Caches

Projection caches are separate from anchors. Holographic slicing is
observer-relative and aperture-relative, so cache entries must be keyed by enough
context to avoid serving the wrong materialization.

Useful cache key ingredients include:

- basis anchor id or basis frontier digest;
- subject root digest;
- aperture digest;
- observer authority digest;
- policy digest;
- evaluator version;
- schema version;
- materializer version;
- query parameter digest;
- coverage and completeness.

Exact repeat queries are the easiest reuse case, but not the only one. Echo can
also reuse CAS objects, structural subtrees, aperture fragments, observer-class
equivalent projections, and anchor-plus-replay suffix materializations. Those
reuse paths remain caches unless a trusted runtime authority admits them.

## Validation Invariants

The current canonical causal-anchor claim contract enforces these invariants:

- The subject's `appId`, `subjectKind`, and `subjectId` are nonempty.
- Application-root `appId`, `subjectKind`, and `id` fields are nonempty.
- At least one retained root is supplied.
- Retained and materialization root sets are sorted into canonical order and
  contain no duplicates.
- Materialization roots cannot declare authority.
- No root appears in both retained and materialization sets.
- The claim digest commits to the supplied subject, basis frontier, roots,
  purpose, and schema.
- Unsupported schema versions are rejected rather than canonicalized under
  unknown rules.
- Decoded claim bytes must exactly equal canonical re-encoding; recovery does
  not normalize noncanonical root order into authority.
- The application request and claim contain no Echo admission receipt identity.

An application-facing trusted admission additionally requires proof that the
basis frontier is admitted causal history, the subject authority permits the
request, and Echo-owned roots have generic support under declared policy. The
WAL path authenticates and recovers the receipt; the claim constructor does not.
Application root semantics and physical retention pins remain separate.

A domain checkpoint fact must separately validate domain semantics. For Jim, that
means the rope head exists, belongs to the named worldline, and matches the anchor
subject. Echo does not validate rope structure; Jim does.

## Relationship To WAL

The WAL remains the durable commit boundary for Echo runtime history. The claim
constructor is not admission and does not replace WAL ordering or recovery.
Echo reserves transaction code `7`, record codes `23` and `24`, and frontier
code `8` for atomic causal-anchor admission. See [WAL](WAL.md) for the WAL
boundary.

A separate retention policy may pin CAS roots, graph facts, manifests, indexes,
or materialized projection blobs named by an admitted anchor. Constructing a
`CausalAnchorClaim` pins nothing and creates no recoverable causal meaning.

## Implementation Evidence

The Echo-owned claim and admitted-evidence contracts live in
`crates/warp-core/src/causal_anchor.rs`. It defines the causal anchor subject,
frontier reference, purpose, typed root roles, canonical root-set validation,
claim digest, opaque admitted fact, and Echo admission receipt. ADR 0022
reserves admitted fact and receipt construction for the trusted Echo transition.

The durable transaction and recovery path lives in
`crates/warp-core/src/causal_wal.rs`. Its deterministic witnesses live in
`crates/warp-core/tests/causal_anchor_wal_tests.rs` and prove stable persisted
codes, exact fact/receipt frame cardinality, corruption refusal, cross-evidence
validation, canonical frame order, WAL-coordinate binding, and uncommitted-tail
invisibility.

The public witness tests live in
`crates/warp-core/tests/causal_anchor_tests.rs`. They prove:

- retained and materialization root sets are canonicalized before digesting;
- duplicate roots are rejected after canonicalization;
- materialization roots cannot declare authority;
- claim digests bind subject, basis frontier, roots, purpose, and schema;
- application requests cannot supply an Echo admission receipt identity;
- a Jim rope checkpoint retains the rope head as authority while flat text remains
  materialization.

The implementation has a canonical claim contract and an internal WAL-backed
admission/recovery path. No current application-facing trusted-host API verifies
supplied references and requests that admission; that is CA-01 Slice 3.

## Doctrine

The core distinction is:

```text
CAS object        = content is addressed; availability needs separate evidence
Projection cache  = derived work can be reused
Causal-anchor claim = canonical claim over supplied references
Admitted anchor   = WAL-committed fact + Echo receipt over the exact claim
Domain checkpoint = an app explains what the basis means
```

Echo must keep those responsibilities separate. If cached projection material
becomes causal authority by accident, the system has collapsed the distinction
between reading and reality.
