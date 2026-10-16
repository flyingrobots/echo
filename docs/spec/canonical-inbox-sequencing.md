<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Canonical Inbox Sequencing

_Define intent identity and tick-boundary ordering so arrival order does not become causality._

Depends on:

- [WARP Rewrite Scheduler](scheduler-warp-core.md)
- [WARP Tick Patch](warp-tick-patch.md)
- [Merkle Commit](merkle-commit.md)

## Purpose

Echo admits intents from outside the kernel. Arrival order is an observer phenomenon; it must not silently become causal order inside a deterministic tick. The inbox contract turns submitted bytes into content-addressed ingress entries and settles pending work in canonical order.

Retries and shuffled submissions must converge. Submitting the same set of
intent bytes in any order produces the same pending set, canonical tick order,
and committed state when tick membership is held fixed.

Intake is idempotent for automated clients: retrying a submitted envelope
returns the same identity without duplicating ledger state.

## Decision 1: Intent identity is content identity

Two intents are the same when their canonical bytes are byte-identical: `intent_id = H(intent_bytes)`. Sequence numbers, arrival time, and transport metadata are not identity.

Refined by Decision 6 for intents admitted from Continuum: a declared Continuum occurrence is not transport metadata, and it is part of ingress identity. Local intents keep this rule unchanged.

## Decision 2: Ingress is idempotent per resolved writer head

The pending inbox is keyed by `ingress_id` for the resolved writer head. Re-ingesting an already pending or already committed intent must not create a second live entry.

## Decision 3: Tick membership is a causality boundary

Permutation-invariance claims require the same set of intents to belong to the same tick. If tick membership differs, the causal history differs.

## Decision 4: Pending work is ordered at the tick boundary

For a pending set: deduplicate by content identity, sort by ingress/id byte order, and derive audit sequence numbers from that sorted order. The runtime must not use arrival order, hash-map iteration, thread scheduling, or platform timing.

## Decision 5: Ledgers are append-only; queues are maintenance state

Ingress event records are immutable once committed. Processing an event removes or updates pending membership; it does not delete the historical event.

## Decision 6: A Continuum occurrence is part of ingress identity

Decision 1 makes content the identity of a local intent. Continuum separates
*what* is intended from *which happening* it is: an `AppliedIntent` has an
`appliedDigest`, and an `OccurredIntent` adds an `OccurrenceRef` whose
`occurrenceKeyDigest = hash({ issuer, localId })`. Two occurrences of identical
content are two acts, and Echo must not collapse them into one submission.

An envelope built with `IngressEnvelope::continuum_occurred_intent` carries a
`ContinuumOccurrence`: Continuum's occurrence key digest and applied digest,
exactly as Continuum computed them. Its ingress identity wraps the content
identity of Decision 1:

```text
content_id = ingress id of the same kind, bytes and causal parents
ingress_id = BLAKE3("echo.ingress.continuum-occurrence.v1\0" || occurrence_key || content_id)
```

The occurrence domain deliberately does not begin with `"ingress:"`. A local
intent hashes `"ingress:" || kind || bytes`, and its kind may be any 32 bytes,
so a domain with that prefix could be reproduced exactly by a crafted local
intent and give it an occurrence's ingress ID.

The submission ID keeps its formula,
`H("echo.intent-submission.v0", worldline, head, ingress_id)`, so it remains
Echo's local submission identity, and attribution and outcome lookup name it
as before. Envelopes without an occurrence keep their ingress and submission
IDs unchanged; a local submission is never matched to an occurrence.

Admission applies Continuum's binding law within the target worldline, which
is the coordinate lineage an occurrence may be admitted to only once:

| Presented again | Echo's answer |
| --- | --- |
| Same occurrence key, same content, applied digest and writer head | Retry: the existing submission (`Duplicate`) |
| Different occurrence key, identical content | A separate submission, decided and attributed on its own |
| Same occurrence key, different content, applied digest or writer head | `RuntimeError::ContinuumOccurrenceConflict`; nothing is admitted and the existing submission is unchanged |

The admission mapping from `(worldline, occurrence key)` to submission is
derived from retained envelope material. Retained envelopes with an occurrence
use format `EINGR003` (the `EINGR002` layout followed by the two digests);
envelopes without one are still written as `EINGR002`. Restoring witnessed
submissions rebuilds the mapping and refuses two retained submissions that
claim one occurrence. The trusted host refuses to reopen a WAL with missing
envelope material, so a recovered mapping is complete.

`ContinuumOccurrence::from_occurred_digests` also checks a presented
`occurredDigest` against Continuum's law, SHA-256 of
`{"appliedDigest":"sha256:…","occurrenceKeyDigest":"sha256:…"}`. That check
covers only the relationship between the three digests: it does not
authenticate the issuer, and it does not prove that the applied digest
describes the intent bytes being admitted. Echo does not recompute the
occurrence key; issuer and local id stay with Continuum, which validates them
when it builds the reference. Echo adds no nonce.

Retained occurrence material is immutable. Because the applied digest is not
part of the ingress ID, a restored record can carry the submission ID of an
existing submission with a different applied digest; restore refuses it, as
it refuses two records in one snapshot that disagree, and changes nothing.
Record-only replay (`replay_witnessed_submissions`) restores submissions
without envelopes, so the mapping cannot be known complete. Until their
envelopes are restored, admitting any occurrence on that worldline fails with
`RuntimeError::ContinuumOccurrenceCoverageUnavailable`: unknown coverage is
not an unused key. Ingress without an occurrence is unaffected.

**Strands inherit occurrences ([#886](https://github.com/flyingrobots/echo/issues/886)).**
A strand's causal past is the prefix it copied at its fork basis
(`ForkBasisRef`: source worldline and fork tick). An act already present in
that past does not become a new act because the worldline changed, so the
occurrence lineage of a strand includes its inherited prefix:

| Presented on the strand | Echo's answer |
| --- | --- |
| Occurrence decided in the inherited prefix, same binding | `AlreadyPresentInInheritedHistory`, naming the original submission, its head, the fork basis and its receipt when ticketed; nothing is admitted or written |
| Same inherited occurrence key, different binding | `RuntimeError::ContinuumOccurrenceConflict` |
| Occurrence absent from the inherited prefix | Ordinary admission on the strand |
| Occurrence decided on the source after the fork basis | Not inherited |
| Occurrence accepted on the source but undecided at the fork basis | Not inherited: acceptance before the fork is not part of its causal past |

When a strand forks, live or while recovery replays the fork, Echo records the
submissions its copied ticks attribute. Admission on a strand with no binding of
its own walks the fork ancestry, and an ancestor's binding counts only when its
submission is in that set, so nested forks inherit through every level with
each level's fork tick as the bound. Unknown coverage on an ancestor obstructs
admission. A prefix tick older than attribution does not say what it decided, so
there the evidence is the submission's receipt correlation, from the decision
log the trusted host attests complete (#887): it counts only when it lies on the
ancestor at or before the last tick the child inherits, and a bound submission
without one was not decided. Final graph state is never treated as evidence. (An
occurrence cannot have been decided by such a tick in practice: occurrence
envelopes exist only since #880, and every writer since #873 attributes its
ticks.) A strand registered without forking has no recorded prefix and
obstructs occurrence admission (`AdmissionCoverageUnestablished`): an unknown
causal past is not an empty one. Redelivery returns the same original decision
however the parent advances, and changes no retained receipt on either lane. Reconciling occurrences admitted independently on divergent
branches is a separate merge question.

Decision record: [#880](https://github.com/flyingrobots/echo/issues/880).
