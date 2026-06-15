<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Braids And Strands Roadmap

Status: active roadmap slice.

Last updated: 2026-06-15.

## Purpose

PR #545 moved strands, braid shells, proof envelopes, blinded member references,
and evolving braid logs from concept-level scaffolding into checked
`warp-core` surfaces. The next phase is not a feature grab bag. It is a
hardening sequence that carries the new surfaces from public data shapes into
law-bearing admission APIs, witnessed transitions, stable identity vectors,
and replayable audit tools.

The destination is:

```text
Strands are not casually constructed.
Braid membership is historical, append-only, and queryable by coordinate.
Proof and witness claims are typed, bounded, and verifier-shaped.
Sealed membership is privacy-preserving by construction, not by convention.
Replay can show exactly what law admitted, retained, concealed, or rejected.
```

## Current Truth

- `Strand<P>` carries causal posture as a sealed typestate parameter, while
  live registry/runtime posture validation remains the final admission law.
- `Braid::apply` is checked: it rejects invalid lifecycle transitions,
  duplicate members, mixed member-reference postures, sequence overflow, empty
  settlement frontiers, and empty collapse witnesses.
- `ProofEnvelope::validate_shape` admits replay-trace envelope structure and
  public-input binding only. Cryptographic proof kinds remain reserved until
  verifier backends exist.
- `BraidShell::assemble_with_proof` binds proof-envelope identity into shell
  identity, so proof-bearing and proofless shells do not collide.
- Sealed braid members require secure query material and no longer leak behind
  generic member lookup APIs.
- Settlement member blinding has explicit salt input and deterministic default
  derivation for reproducible local flows.

## North Star

Echo should feel less like a crate full of important structs and more like a
causal operating system with lawful doors:

| Today                           | Target                                         |
| ------------------------------- | ---------------------------------------------- |
| Constructing a `Strand`         | admitted through posture-aware constructors    |
| Creating a `Braid`              | emitted as an authority-scoped event           |
| Adding a member                 | witnessed as `MemberWoven` history             |
| Settling                        | admitted as a shared settlement act            |
| Attaching proof evidence        | carried by verifier-shaped receipts            |
| Reading a shell                 | replayed through an audit optic                |
| Sealing a member                | backed by authority/capability-local blinding  |
| Interpreting retained plurality | governed by named settlement and collapse laws |

## Execution Order

### 1. Seal The Constructor Boundary

Goal: callers should not be able to fabricate law-looking strand values with
public struct literals.

Work:

1. Make `Strand<P>` fields private where possible.
2. Add accessors for read-only public data.
3. Add named constructors or builders for `DynamicPosture`, `Scratch`,
   `AuthorOnly`, and `Shared` paths.
4. Keep registry/runtime checks authoritative.
5. Move tests to fixture builders instead of public struct literals.

Acceptance:

- External callers cannot set `_marker` or `retention_posture` directly.
- Existing registry and settlement tests still prove stale or forged handles
  cannot bypass runtime posture validation.
- Public API docs state that typestate narrows normal construction but does
  not replace live admission checks.

### 2. Type The Error And Transition Vocabulary

Goal: validation and transition failures should be structured facts, not
stringly diagnostics.

Work:

1. Replace `ProofEnvelope::validate_shape(...) -> Result<(), String>` with a
   structured `ProofError`.
2. Split malformed replay-trace envelope, public-input mismatch, unsupported
   proof kind, and future backend rejection into distinct variants.
3. Replace braid transition action strings with a typed
   `BraidTransitionKind`.
4. Preserve stable display text for humans while exposing typed variants to
   callers and tests.

Acceptance:

- Tests assert exact `ProofError` variants.
- Tests assert exact invalid braid transition kinds.
- No caller depends on parsing error strings for behavior.

### 3. Lock Digest And Proof Identity With Golden Vectors

Goal: accidental identity drift should fail loudly and boringly.

Work:

1. Add golden vectors for `ProofEnvelope::digest`.
2. Add golden vectors for proofless and proof-bearing `BraidShell` identity.
3. Add vectors for revealed and sealed member references.
4. Keep vector fixtures small, hand-reviewable, and domain-separated.
5. Add a compatibility note for intentional vector changes.

Acceptance:

- A formatting-only or field-order refactor cannot silently change shell or
  proof identity.
- CI catches digest drift in a targeted test.
- The vector file explains which identities are public compatibility promises
  and which are internal E1 scaffolding.

### 4. Clarify Privacy Posture Around Blinding Salt

Goal: deterministic defaults must not be mistaken for unlinkability guarantees.

Work:

1. Document that the default member blinding salt is deterministic.
2. State that the default salt is not an unlinkability boundary across
   independent settlements.
3. Require or strongly route privacy-preserving sealed-member flows through
   authority-local, capability-local, or session-local blinding material.
4. Add tests showing caller-supplied salt changes sealed member commitments.

Acceptance:

- API docs say the quiet part clearly.
- Privacy-sensitive examples never use the deterministic default as their
  privacy boundary.
- Tests distinguish reproducible local defaults from unlinkability-oriented
  caller material.

### 5. Make Braid Membership Historical

Goal: braid membership changes should be append-only history, not current
state with a log nearby.

Work:

1. Promote the append-only braid membership backlog idea into an active design.
2. Treat `BraidEvent` as the source of truth for membership intervals.
3. Add historical membership views by braid coordinate or event sequence.
4. Preserve current membership as a projection.
5. Keep sealed member references lawful at historical coordinates.

Acceptance:

- A braid created with `s0` and `s1`, then woven with `s2`, reports `s2` only
  at coordinates after the weave event.
- Current membership and historical membership are both deterministic
  projections over the same event log.
- Settlement can admit a braid projection without pretending member strands
  merged.

### 6. Add Replay And Audit Optics

Goal: the architecture should be able to show its work.

Work:

1. Add a braid replay surface that explains member verdicts, posture floors,
   proof binding, retained support, settlement frontier, and witness posture.
2. Make replay output stable enough for tests and agent inspection.
3. Keep concealed member source chains concealed while reporting the lawful
   reason they are concealed.
4. Add lower-mode output suitable for CLI, JSON, and docs examples.

Acceptance:

- Given a braid shell digest or event log, Echo can render an audit reading
  without hand-inspecting internals.
- Replay distinguishes admitted, retained, concealed, conflicted, obstructed,
  and unsupported claims.
- Tests can assert replay facts without depending on ornamental formatting.

### 7. Name External Witness Receipts

Goal: the current self-witness should remain honest E1 scaffolding, not become
the permanent witness model by accident.

Work:

1. Define `WitnessReceipt`, `WitnessKind`, and a verifier-shaped
   `WitnessBackend` boundary.
2. Keep `SelfWitness` as deterministic local/test evidence.
3. Reserve shapes for signed witness, threshold witness, runtime attestation,
   replay-trace receipt, ZK verifier receipt, and vector-opening receipt.
4. Bind witness receipt identity into braid shell or retained evidence identity
   only through an explicit compatibility rule.

Acceptance:

- Docs and APIs distinguish self-witness integrity from independent
  attestation.
- Adding a real witness backend does not require changing braid shell member
  semantics.
- Unsupported witness kinds fail as typed unsupported-backend outcomes.

### 8. Build Sealed Membership Capabilities

Goal: a holder should be able to prove authorized membership for a purpose
without revealing global strand identity or the source chain.

Work:

1. Model a purpose-bound sealed membership presentation.
2. Bind presentations to authority domain, braid coordinate, purpose, and
   blinding material.
3. Let reviewers or settlement authorities verify membership without learning
   more than the aperture permits.
4. Keep revealed-member and sealed-member paths visibly different in API and
   replay output.

Acceptance:

- A verifier can check "this actor is an authorized member of braid B for
  purpose P" without requiring global strand-id revelation.
- Reusing material across independent settlements is either impossible by API
  shape or explicitly visible as caller risk.
- Replay records what was proven and what remained sealed.

### 9. Introduce Named Plurality Laws

Goal: retained plural strand claims should be interpreted by named law, not
hidden caller policy.

Work:

1. Define a registry shape for settlement, collapse, conflict-preserving,
   quorum, editorial, and authority laws.
2. Make the law name and version part of the witnessed reading.
3. Attach capability, support, budget, and evidence posture to law execution.
4. Keep application nouns out of Echo core; authored contracts may provide
   domain-specific laws through generated adapters.

Acceptance:

- A retained braid reading states which law interpreted plurality.
- Two different laws over the same retained support produce distinct witnessed
  readings.
- Unsupported or unauthorized law execution yields typed obstruction evidence.

## Bad-Code Register

These are not revert-class defects. They are hardening debt that should be
closed before more public APIs depend on the new shapes.

| Priority | Area                   | Debt                                     | Recommended next move                |
| -------: | ---------------------- | ---------------------------------------- | ------------------------------------ |
|        1 | `Strand<P>`            | public construction weakens typestate    | private fields plus fixture builders |
|        2 | `ProofEnvelope`        | validation returns strings               | structured `ProofError`              |
|        3 | digest identity        | no golden vectors yet                    | proof and shell vector fixtures      |
|        4 | `Braid::apply`         | transition names are strings             | typed `BraidTransitionKind`          |
|        5 | settlement blinding    | deterministic default may be overtrusted | sharper docs and salt-path tests     |
|        6 | braid shell witnessing | self-witness is only E1 scaffolding      | named witness receipt design         |

## Cool-Idea Queue

Preserve these without letting them jump the hardening queue.

| Rank | Idea                         | Why it matters                                  |
| ---: | ---------------------------- | ----------------------------------------------- |
|    1 | braid replay explorer        | forces the model to show its work               |
|    2 | append-only membership views | makes braided history actually historical       |
|    3 | external witness plugins     | opens adversarial and institutional deployment  |
|    4 | sealed membership capability | unlocks privacy-preserving authorization proofs |
|    5 | plurality law registry       | turns interpretation policy into witnessed law  |

## Sequencing Guardrails

- Do not implement real ZK or vector-opening verification until the verifier
  boundary and golden vectors are in place.
- Do not make self-witness mean independent attestation.
- Do not treat typestate as the only authority; live runtime posture checks
  remain final.
- Do not rewrite settlement as merge semantics. Settlement admits history;
  it does not pretend braided strands became one published truth.
- Do not let application nouns enter Echo core while adding plurality laws or
  replay tools.
- Do not make current braid membership the only queryable shape once event
  history exists.

## Issue Seeds

Live backlog is GitHub Issues. These are ready to turn into issues when this
roadmap is pulled into execution:

1. Private `Strand<P>` construction and fixture builders.
2. Structured `ProofError` and typed braid transition kinds.
3. Golden vectors for proof envelope and braid shell identity.
4. Blinding salt privacy docs and salt-path tests.
5. Append-only braid membership historical views.
6. Braid replay and audit optic.
7. External witness receipt boundary.
8. Purpose-bound sealed membership capability.
9. Named plurality law registry.

Each issue should carry one executable witness. Design-only issues must state
which later runtime, API, golden-vector, replay, or DIND proof will close the
claim.
