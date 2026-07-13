<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Causal Anchors

A causal-anchor value is an application-requested, canonical claim over a basis
in causal history. It is not a materialized snapshot, an admission receipt, or
proof that referenced history and roots exist. The current implementation binds
a subject, basis-frontier digest, root set, purpose, and receipt digest into a
deterministic value without admitting or publishing that value.

The short rule is:

```text
CAS addresses content. A causal-anchor value binds claimed meaning to supplied references.
A trusted admission receipt would prove Echo accepted that claim; no such API exists today.
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
| Causal-anchor value     | A canonical claim binds a subject, supplied basis-frontier digest, root sets, purpose, and supplied receipt digest.                                  |
| Trusted admitted anchor | A runtime authority verified and durably published an anchor claim. Echo has no such API today.                                                      |
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

type CausalAnchorFact = {
    kind: "echo.causal.Anchor";
    schemaVersion: number;
    anchorId: string;
    subject: CausalAnchorSubject;
    basisFrontier: CausalFrontier;
    retainedRoots: readonly CausalAnchorRoot[];
    materializationRoots?: readonly CausalAnchorRoot[];
    purpose: CausalAnchorPurpose;
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

`admittedByReceiptId` is a historical field name in the current value contract.
Its digest is supplied by the caller and committed into `anchorDigest`; the
constructor does not recover or authenticate that receipt.

## Value Construction And Trusted Admission Boundary

Applications construct anchor values because applications know when a domain
boundary is meaningful:

- An editor knows when a buffer was manually saved.
- A build system knows when an artifact frontier is reproducible.
- A mail app knows when a thread index should be retained.
- A debugger knows when a replay basis should be pinned.

The implemented flow is:

1. The application identifies a subject, basis frontier, retained roots, optional
   materialization roots, and purpose.
2. The application supplies a receipt digest with those references.
3. `CausalAnchorFact::from_request` validates subject and root-set shape,
   canonicalizes the roots, and derives the anchor digest and id.
4. The application receives a canonical value. No WAL record, retention pin, or
   runtime admission receipt is created by that operation.

The frontier, roots, and receipt digest are caller-provided references. No
current API verifies those references or publishes the value. Code that receives
a `CausalAnchorFact` must therefore not treat it as proof of admission.

A trusted admission authority would have to validate that the frontier is
admitted, each root exists under its declared kind, the request is lawful for the
subject authority, the retention policy can honor the request, and the receipt
provenance is recoverable. It would then publish the anchor evidence through the
runtime's durable commit boundary. Those requirements define the trust boundary;
they do not describe current behavior.

Applications may construct canonical anchor values, but they cannot confer Echo
admission on those values. Echo must not encode application semantics into the
generic value contract.

## Jim And Rope Checkpoints

For `jedit`, the domain checkpoint can be a thin fact over a causal-anchor value:

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

The constructed causal-anchor value says:

```text
This caller claimed this subject, basis frontier, and retained root set under
this purpose and receipt reference.
```

A Jim save anchor value can claim the rope head as retained authority:

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

Creating a rope checkpoint may construct:

- a canonical causal-anchor value;
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

The current canonical causal-anchor value contract enforces these invariants:

- The subject's `appId`, `subjectKind`, and `subjectId` are nonempty.
- At least one retained root is supplied.
- Retained and materialization root sets are sorted into canonical order and
  contain no duplicates.
- Materialization roots cannot declare authority.
- No root appears in both retained and materialization sets.
- The anchor digest commits to the supplied subject, basis frontier, roots,
  purpose, schema, and receipt digest.
- The anchor id is domain-separated from the anchor digest.

A trusted admitted anchor would additionally require proof that the basis
frontier is admitted causal history, every root exists with the declared kind,
the receipt is authentic and recoverable, the subject authority permits the
request, and retention policy explains each pin. The current value constructor
does not enforce those properties.

A domain checkpoint fact must separately validate domain semantics. For Jim, that
means the rope head exists, belongs to the named worldline, and matches the anchor
subject. Echo does not validate rope structure; Jim does.

## Relationship To WAL

The WAL remains the durable commit boundary for Echo runtime history. The current
causal-anchor value is not admitted through that authority and does not replace
WAL ordering or recovery. See [/topics/WAL](/topics/WAL) for the WAL boundary.

A trusted admission path could pin CAS roots, graph facts, manifests, indexes, or
materialized projection blobs. Constructing a `CausalAnchorFact` alone pins
nothing and creates no recoverable causal meaning.

## Implementation Evidence

The first Echo-owned value contract lives in
`crates/warp-core/src/causal_anchor.rs`. It defines the causal anchor subject,
frontier reference, purpose, typed root roles, canonical root-set validation,
anchor digest, and anchor id.

The public witness tests live in
`crates/warp-core/tests/causal_anchor_tests.rs`. They prove:

- retained and materialization root sets are canonicalized before digesting;
- duplicate roots are rejected after canonicalization;
- materialization roots cannot declare authority;
- anchor digests bind subject, basis frontier, purpose, and the supplied receipt
  digest;
- a Jim rope checkpoint retains the rope head as authority while flat text remains
  materialization;
- anchor ids are domain-separated from anchor digests.

This implementation is a canonical value contract only. No current API verifies
the supplied references or publishes the value under trusted runtime authority.

## Doctrine

The core distinction is:

```text
CAS object        = content is addressed; availability needs separate evidence
Projection cache  = derived work can be reused
Causal-anchor value = canonical claim over supplied references
Admitted anchor   = trusted authority verified and published the claim (not implemented)
Domain checkpoint = an app explains what the basis means
```

Echo must keep those responsibilities separate. If cached projection material
becomes causal authority by accident, the system has collapsed the distinction
between reading and reality.
