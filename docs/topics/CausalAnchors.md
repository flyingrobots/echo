<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Causal Anchors

A causal anchor is an application-requested, Echo-admitted durable basis over
causal history. It is not a materialized snapshot. It is the evidence that a
subject, basis frontier, and retained root set have been admitted as meaningful
for recovery, retention, export, save, replay, debug, or cache warming.

The short rule is:

```text
CAS proves content exists. An anchor proves Echo admitted what that content means.
```

## Why Not Checkpoints

The word "checkpoint" is overloaded. In many systems it means a full materialized
state snapshot. That is not the primitive Echo needs.

Echo needs a generic causal anchor. Applications may use that anchor to implement
domain checkpoints, but Echo should not learn application nouns such as editor
buffer, rope head, mail thread, calendar event, build artifact, or dirty file.

The boundary is:

| Layer | Responsibility |
| --- | --- |
| CAS object | Content-addressed bytes, facts, manifests, or projection material exist at a hash. |
| Projection cache | Derived observer-relative materialization can be reused when its basis, aperture, observer authority, policy, schema, evaluator, and coverage match. |
| Causal anchor | Echo admitted a durable subject, basis frontier, and retained root set under a named purpose. |
| Domain checkpoint | An application explains what an Echo anchor means in domain terms. |

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

`retainedRoots` name authority or evidence roots. `materializationRoots` name
derived projection artifacts such as flattened text, line indexes, rendered
slices, or export blobs. A materialized projection must not become authority just
because it is convenient to load. The two root sets are disjoint: a root cannot
be both retained evidence and a materialized projection in the same anchor.

## Admission Flow

Applications request anchors because applications know when a domain boundary is
meaningful:

- An editor knows when a buffer was manually saved.
- A build system knows when an artifact frontier is reproducible.
- A mail app knows when a thread index should be retained.
- A debugger knows when a replay basis should be pinned.

Echo admits anchors because Echo owns causal admission, receipt posture,
retention policy, and recovery semantics.

The flow is:

1. The application identifies a subject, basis frontier, retained roots, optional
   materialization roots, and purpose.
2. Echo validates that the frontier is admitted, roots exist under their declared
   kinds, the request is lawful for the subject authority, and retention policy
   can honor the request.
3. Echo records anchor evidence and returns an anchor receipt.
4. The application may record a domain fact that references the Echo anchor.

The application must not mint fake Echo anchors. Echo must not encode application
semantics into the generic anchor primitive.

## Jim And Rope Checkpoints

For `jedit`, the domain checkpoint is a thin fact over an Echo causal anchor:

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

Echo says:

```text
This subject, basis frontier, and retained root set were admitted as a durable
anchor.
```

A Jim save anchor would normally retain the rope head as authority:

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

Creating a rope checkpoint may mint:

- a causal anchor fact or receipt;
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
reuse paths remain caches unless Echo admits them as anchors.

## Validation Invariants

An admitted causal anchor must satisfy these invariants:

- The subject id is scoped by `appId` and `subjectKind`.
- The basis frontier is admitted causal history.
- Every retained root exists and has the declared root kind.
- Materialization roots are marked as derived artifacts unless explicitly
  declared as authority by the application domain.
- The anchor digest commits to subject, basis frontier, roots, purpose, schema,
  and admission receipt.
- The admission receipt exists and is recoverable through Echo's runtime evidence.
- Retention policy can explain why retained roots are pinned or when they may be
  collected.

A domain checkpoint fact must separately validate domain semantics. For Jim, that
means the rope head exists, belongs to the named worldline, and matches the anchor
subject. Echo does not validate rope structure; Jim does.

## Relationship To WAL

The WAL remains the durable commit boundary for Echo runtime history. Causal
anchors are facts admitted through that authority, not a replacement for WAL
ordering or recovery. See [/topics/WAL](/topics/WAL) for the WAL boundary.

An anchor may pin CAS roots, graph facts, manifests, indexes, or materialized
projection blobs. The anchor itself is the recoverable causal meaning of that pin,
not the storage mechanism.

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
- anchor digests bind subject, basis frontier, purpose, and admission receipt;
- a Jim rope checkpoint retains the rope head as authority while flat text remains
  materialization;
- anchor ids are domain-separated from anchor digests.

This is a value contract, not the final WAL publication API. A later slice must
record causal anchor admission under trusted runtime authority.

## Doctrine

The core distinction is:

```text
CAS object        = content exists
Projection cache  = derived work can be reused
Causal anchor     = Echo admitted a durable basis
Domain checkpoint = an app explains what the basis means
```

Echo must keep those responsibilities separate. If cached projection material
becomes causal authority by accident, the system has collapsed the distinction
between reading and reality.
