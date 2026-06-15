<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Braids And Strands Roadmap

Status: active hardening roadmap slice.

Last updated: 2026-06-15.

## Purpose

PR #545 moved strands, braid shells, proof envelopes, blinded member references,
and evolving braid logs from concept-level scaffolding into checked
`warp-core` surfaces. The next phase is not a feature grab bag. It is a
hardening sequence that carries the new surfaces from public data shapes into
law-bearing admission APIs, witnessed transitions, stable identity vectors,
and replayable audit tools.

This roadmap is ordered. Items 1-4 are hardening prerequisites. Items 5-9 MUST
NOT bypass them except for design-only exploration that creates no public
runtime, API, digest, privacy, or witness dependency.

No law-bearing object is born by accident.

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

## Progress Tracker

Check off a slice only in the PR that implements or lands that slice. Design
approval alone does not complete implementation slices unless the slice is
explicitly design-only. The roadmap stays as the live progress register for
this campaign.

### Goalpost 1: Lawful Construction And Typed Failures

Design:
[`goalpost-01-lawful-construction-and-typed-failures.md`](braids-and-strands-hardening/goalpost-01-lawful-construction-and-typed-failures.md)

- [x] GP1-S1: Make `Strand<P>` construction posture-aware and non-forgeable
      through public API.
- [x] GP1-S2: Replace public test construction with fixture builders.
- [x] GP1-S3: Replace proof validation strings with structured `ProofError`.
- [x] GP1-S4: Replace braid transition action strings with
      `BraidTransitionKind`.
- [x] GP1-S5: Add negative capability tests for forged strands and display
      string parsing.

### Goalpost 2: Stable Identity And Privacy Posture

Design:
[`goalpost-02-stable-identity-and-privacy-posture.md`](braids-and-strands-hardening/goalpost-02-stable-identity-and-privacy-posture.md)

- [ ] GP2-S1: Add golden vectors for replay-trace `ProofEnvelope` identity.
- [ ] GP2-S2: Add proofless and proof-bearing `BraidShell` identity vectors.
- [ ] GP2-S3: Add revealed and sealed `BraidMemberRef` vectors, including salt
      effect.
- [ ] GP2-S4: Mark vector compatibility classes and migration/versioning rules.
- [ ] GP2-S5: Document deterministic blinding salt risk in API docs and
      privacy-sensitive examples.

### Goalpost 3: Historical Membership And Replay

Design:
[`goalpost-03-historical-membership-and-replay.md`](braids-and-strands-hardening/goalpost-03-historical-membership-and-replay.md)

- [ ] GP3-S1: Promote append-only braid membership into an implementation
      design.
- [ ] GP3-S2: Add historical membership views by coordinate or event sequence.
- [ ] GP3-S3: Add membership diff facts for added, ended, revealed, and
      concealed changes.
- [ ] GP3-S4: Add replay/audit facts for member verdicts, posture floors,
      proof binding, retained support, frontier, and witness posture.
- [ ] GP3-S5: Define the Braid Flight Recorder and Causal X-Ray lower-mode
      output.

### Goalpost 4: Witness Receipts And Sealed Capabilities

Design:
[`goalpost-04-witness-receipts-and-sealed-capabilities.md`](braids-and-strands-hardening/goalpost-04-witness-receipts-and-sealed-capabilities.md)

- [ ] GP4-S1: Define `WitnessReceipt`, `WitnessKind`, and `WitnessBackend`.
- [ ] GP4-S2: Add deterministic witness simulator fixtures for supported,
      rejected, and unsupported outcomes.
- [ ] GP4-S3: Bind witness identity only through explicit compatibility rules.
- [ ] GP4-S4: Design purpose-bound sealed membership presentations.
- [ ] GP4-S5: Add disclosure budget labels and replay wording for sealed
      membership.

### Goalpost 5: Named Plurality Laws

Design:
[`goalpost-05-named-plurality-laws.md`](braids-and-strands-hardening/goalpost-05-named-plurality-laws.md)

- [ ] GP5-S1: Define the core plurality law registry shape.
- [ ] GP5-S2: Add machine-readable Law Cards.
- [ ] GP5-S3: Bind law name and version into witnessed readings.
- [ ] GP5-S4: Route adapter-provided law families without application nouns in
      Echo core.
- [ ] GP5-S5: Add obstruction evidence for unsupported or unauthorized law
      execution.

## North Star

Echo must feel less like a crate full of important structs and more like a
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

Every important causal claim must answer:

1. Who admitted this?
2. Under what authority?
3. With what posture?
4. Bound to what identity?
5. Replayable by whom, revealing how much?

The vocabulary stays sharp:

| Term               | Meaning                                                  |
| ------------------ | -------------------------------------------------------- |
| Struct             | Represents a fact.                                       |
| Constructor        | Admits a fact through a named door.                      |
| Witness            | Supports a fact with evidence.                           |
| Replay             | Explains a fact from recorded causes.                    |
| Law                | Interprets a fact under named authority and posture.     |
| Digest             | Preserves the identity of a fact.                        |
| Admission          | Decides whether a claim enters Echo history.             |
| Settlement         | Admits history; it is not merge semantics.               |
| Projection         | Materializes one bounded reading from admitted history.  |
| Sealed member      | Hides member identity behind authorized proof material.  |
| Revealed member    | Exposes member identity directly.                        |
| Posture floor      | Lowest causal posture the reading can honestly claim.    |
| Retained plurality | Preserved multiple claims not collapsed to one fact.     |
| Collapse law       | Named law that interprets or reduces retained plurality. |

Claim lifecycle:

```text
created -> admitted -> projected -> witnessed -> replayed -> interpreted
```

## Dependency Graph

This is a dependency chain, not a menu.

| Item                             | Depends on                              | Why                                                 |
| -------------------------------- | --------------------------------------- | --------------------------------------------------- |
| Private `Strand<P>` construction | PR #545 final model                     | prevents typestate-looking forgery                  |
| Structured errors                | current proof and braid APIs            | stops tests and callers from parsing strings        |
| Golden vectors                   | current digest and proof binding        | freezes identity semantics                          |
| Blinding salt docs and tests     | current sealed member API               | prevents reproducibility from being sold as privacy |
| Historical braid membership      | checked `BraidEvent` model              | turns event log into source of truth                |
| Replay optic                     | shell identity and historical views     | shows lawful reading                                |
| External witness receipts        | golden vectors and typed proof errors   | prevents witness model drift                        |
| Sealed membership capability     | blinding docs and historical membership | prevents privacy overclaim                          |
| Named plurality laws             | replay optic and witness boundary       | makes interpretation law visible                    |

## Ownership Labels

Each item owns a primary surface so issue triage stays narrow.

| Item | Surface                 |
| ---: | ----------------------- |
|    1 | API/runtime             |
|    2 | API/testability         |
|    3 | digest compatibility    |
|    4 | privacy/API             |
|    5 | runtime/history         |
|    6 | replay/agent inspection |
|    7 | witnessing/API          |
|    8 | privacy/capability      |
|    9 | law machinery/design    |

## Universal Definition Of Done

Every issue created from this roadmap MUST include:

1. One failing regression witness before the fix, unless the issue is
   explicitly design-only.
2. One focused implementation change.
3. One stable test, vector, fixture, replay fact, or deterministic integration
   proof.
4. Documentation when public semantics, privacy posture, digest identity, or
   replay output changes.
5. No hidden reliance on parsing display strings.
6. No public constructor that bypasses law-bearing validation.
7. No weakening of live runtime posture or registry checks.
8. No application-domain nouns in Echo core.

Design-only issues MUST name the later runtime, API, golden-vector, replay, or
deterministic integration witness that closes the claim.

## Issue Output Format

Every execution issue created from this roadmap MUST include:

1. Invariant being protected.
2. Failing witness.
3. Implementation surface.
4. Compatibility class affected.
5. Acceptance test.
6. Documentation, vector, or replay impact.

## Non-Goals

This roadmap slice does not include:

- Real ZK backend implementation.
- Application-domain law nouns in Echo core.
- Settlement-as-merge semantics.
- Any promise that deterministic member blinding defaults provide
  unlinkability.
- Replacement of runtime admission checks with typestate alone.
- Current-only braid history.
- Self-witness branding as independent attestation.

## Threat Model Notes

- Sealed member references hide global strand identity only when blinding
  material remains non-public and is not reused across unlinkability domains.
- The deterministic default salt is for reproducibility, not unlinkability.
  Privacy-preserving flows MUST provide authority-local, capability-local, or
  session-local blinding material.
- Authority-local and capability-local salts are the preferred privacy
  boundary.
- Replay MUST reveal what was lawfully proven without revealing concealed
  source chains beyond the requested aperture.
- `SelfWitness` means integrity-only local witness unless an external receipt
  says otherwise.
- Typestate is a guardrail. Runtime admission is the courthouse.

## Compatibility Classes

Golden vectors and replay fixtures MUST mark identity stability explicitly:

| Class                      | Rule                                                          |
| -------------------------- | ------------------------------------------------------------- |
| Public stable identity     | MUST NOT change without migration note and compatibility plan |
| E1 scaffolding identity    | Changes require an explicit compatibility note                |
| Test-only fixture identity | Carries no compatibility promise beyond the fixture           |

Intentional digest, proof, shell, witness, or replay identity changes MUST
state which compatibility class changed and how callers migrate.

Any public stable identity change MUST include a migration path, a version
bump, or an explicit declaration that no prior stable identity was published.

## Execution Order

### 1. Seal The Constructor Boundary

Surfaces: API/runtime.

Depends on: PR #545 final model.

Goal: callers MUST NOT fabricate law-looking strand values with public struct
literals.

Work:

1. Make `Strand<P>` fields private where possible.
2. Add accessors for read-only public data.
3. Add named constructors or builders for `DynamicPosture`, `Scratch`,
   `AuthorOnly`, and `Shared` paths.
4. Keep registry/runtime checks authoritative.
5. Move tests to fixture builders instead of public struct literals.
6. Ensure the public API cannot construct a value whose type-level posture and
   runtime retention posture disagree.

Acceptance:

- External callers cannot set `_marker` or `retention_posture` directly.
- Existing registry and settlement tests still prove stale or forged handles
  cannot bypass runtime posture validation.
- Public API docs state that typestate narrows normal construction but does
  not replace live admission checks.
- Negative tests prove `Strand<Shared>` cannot be publicly constructed with
  `AuthorOnly` retention posture.

### 2. Type The Error And Transition Vocabulary

Surfaces: API/testability.

Depends on: current proof and braid APIs.

Goal: validation and transition failures MUST be structured facts, not
stringly diagnostics.

Work:

1. Replace `ProofEnvelope::validate_shape(...) -> Result<(), String>` with a
   structured `ProofError`.
2. Split malformed replay-trace envelope, empty payload, public-input
   mismatch, unsupported proof kind, and future backend rejection into distinct
   variants.
3. Reserve this shape:

    ```rust
    pub enum ProofError {
        UnsupportedKind { kind: ProofKind },
        EmptyPayload,
        PublicInputsMismatch {
            expected: Hash,
            actual: Hash,
        },
        MalformedEnvelope,
        BackendRejected {
            kind: ProofKind,
            reason: VerificationFailureCode,
        },
    }
    ```

4. Replace braid transition action strings with a typed
   `BraidTransitionKind`.
5. Preserve stable display text for humans while exposing typed variants to
   callers and tests.

Acceptance:

- Tests assert exact `ProofError` variants.
- Tests assert exact invalid braid transition kinds.
- No caller depends on parsing error strings for behavior.
- Negative tests prove display text is not the behavior contract.

### 3. Lock Digest And Proof Identity With Golden Vectors

Surfaces: digest compatibility.

Depends on: current digest and proof binding.

Goal: accidental identity drift MUST fail loudly and boringly.

Work:

1. Add golden vectors for `ProofEnvelope::digest` for replay traces.
2. Add vectors for rejected or unsupported proof-kind shape where applicable.
3. Add golden vectors for proofless and proof-bearing `BraidShell` identity.
4. Add vectors for revealed and sealed `BraidMemberRef` identity.
5. Add vectors proving member blinding salt effect.
6. Keep vector fixtures small, hand-reviewable, and domain-separated.
7. Add a compatibility note for intentional vector changes.

Acceptance:

- A formatting-only or field-order refactor cannot silently change shell or
  proof identity.
- CI catches digest drift in a targeted test.
- The vector file marks each identity as public stable, E1 scaffolding, or
  test-only fixture.
- Negative tests prove salt changes alter sealed member commitments where
  required.

### 4. Clarify Privacy Posture Around Blinding Salt

Surfaces: privacy/API.

Depends on: current sealed member API and golden vector plan.

Goal: deterministic defaults MUST NOT be mistaken for unlinkability
guarantees.

Work:

1. Put this sentence in API docs, examples, and this roadmap:

    ```text
    The deterministic default salt is for reproducibility, not unlinkability.
    Privacy-preserving flows MUST provide authority-local, capability-local, or
    session-local blinding material.
    ```

2. Document that the default member blinding salt is deterministic.
3. State that the default salt is not an unlinkability boundary across
   independent settlements.
4. Route privacy-preserving sealed-member flows through authority-local,
   capability-local, or session-local blinding material.
5. Add tests showing caller-supplied salt changes sealed member commitments.

Acceptance:

- API docs say the deterministic-default risk clearly.
- Privacy-sensitive examples never use the deterministic default as their
  privacy boundary.
- Tests distinguish reproducible local defaults from unlinkability-oriented
  caller material.
- Negative tests or docs examples prove deterministic salt reuse is surfaced as
  caller risk.

### 5. Make Braid Membership Historical

Surfaces: runtime/history.

Depends on: checked `BraidEvent` model.

Goal: braid membership changes MUST be append-only history, not current state
with a log nearby.

Work:

1. Promote the append-only braid membership backlog idea into an active design.
2. Treat `BraidEvent` as the source of truth for membership intervals.
3. Add historical membership views by braid coordinate or event sequence.
4. Preserve current membership as a projection.
5. Keep sealed member references lawful at historical coordinates.
6. Add `braid.diff_membership(from_coordinate, to_coordinate)` as a design
   target for replay, UI, and audit.

Acceptance:

- A braid whose initial interval includes `s0` and `s1`, then later weaves in
  `s2`, reports `s2` only at coordinates after the weave event.
- Current membership and historical membership are both deterministic
  projections over the same event log.
- Settlement can admit a braid projection without pretending member strands
  merged.
- Negative tests prove a current-only membership projection cannot satisfy a
  historical coordinate request.
- Historical membership diff can report added, ended, revealed, and
  concealed member changes.

### 6. Add Replay And Audit Optics

Surfaces: replay/agent inspection.

Depends on: shell identity and historical views.

Goal: the architecture MUST show its work.

Work:

1. Add a braid replay surface that explains member verdicts, posture floors,
   proof binding, retained support, settlement frontier, and witness posture.
2. Make replay output stable enough for tests and agent inspection.
3. Keep concealed member source chains concealed while reporting the lawful
   reason they are concealed.
4. Add lower-mode output suitable for CLI, JSON, and docs examples.
5. Define a Braid Flight Recorder artifact that records:

    ```text
    event log
    -> membership projection
    -> shell assembly
    -> proof binding
    -> witness reading
    -> replay verdict
    ```

6. Define a Causal X-Ray CLI target:

    ```text
    echo braid inspect <shell-digest>
    ```

7. Add generated audit examples in JSON form.

Acceptance:

- Given a braid shell digest or event log, Echo can render an audit reading
  without hand-inspecting internals.
- Replay distinguishes admitted, retained, concealed, conflicted, obstructed,
  and unsupported claims.
- Tests can assert replay facts without depending on ornamental formatting.
- Replay cannot treat `SelfWitness` as independent attestation.
- Audit examples include warnings such as deterministic blinding salt usage.

Example future audit output:

```json
{
    "braid": "bsh1...",
    "coordinate": "bc1...",
    "members": [
        {
            "reference": "sealed",
            "verdict": "Plural",
            "concealment": "AuthorityScoped"
        }
    ],
    "posture_floor": "AuthorOnly",
    "settlement_law": "AllowPluralOverFootprintOverlap",
    "proof": {
        "kind": "ReplayTrace",
        "binding": "Matched"
    },
    "witness": {
        "kind": "SelfWitness",
        "attestation": "IntegrityOnly"
    }
}
```

### 7. Name External Witness Receipts

Surfaces: witnessing/API.

Depends on: golden vectors and typed proof errors.

Goal: the current self-witness MUST remain honest E1 scaffolding and MUST NOT
become the permanent witness model by accident.

Work:

1. Define `WitnessReceipt`, `WitnessKind`, and a verifier-shaped
   `WitnessBackend` boundary.
2. Keep `SelfWitness` as deterministic local/test evidence.
3. Reserve shapes for signed witness, threshold witness, runtime attestation,
   replay-trace receipt, ZK verifier receipt, and vector-opening receipt.
4. Bind witness receipt identity into braid shell or retained evidence identity
   only through an explicit compatibility rule.
5. Build a witness backend simulator before real witness backends:
   `SelfWitness`, `SignedWitnessFixture`, `ThresholdWitnessFixture`,
   `RejectedWitnessFixture`, and `UnsupportedWitnessFixture`.
6. Reserve a migration hook shape:

    ```rust
    pub enum CompatibilityRule {
        StableV1,
        E1Scaffold,
        RequiresMigration { from: u32, to: u32 },
    }
    ```

Acceptance:

- Docs and APIs distinguish self-witness integrity from independent
  attestation.
- Adding a real witness backend does not require changing braid shell member
  semantics.
- Unsupported witness kinds fail as typed unsupported-backend outcomes.
- Simulator fixtures harden the boundary before real cryptographic backends
  arrive.

### 8. Build Sealed Membership Capabilities

Surfaces: privacy/capability.

Depends on: blinding docs and historical membership.

Goal: a holder MUST be able to prove authorized membership for a purpose
without revealing global strand identity or the source chain.

Work:

1. Model a purpose-bound sealed membership presentation.
2. Bind presentations to authority domain, braid coordinate, purpose, and
   blinding material.
3. Let reviewers or settlement authorities verify membership without learning
   more than the aperture permits.
4. Keep revealed-member and sealed-member paths visibly different in API and
   replay output.
5. Add disclosure budget labels:

    ```rust
    pub enum DisclosureBudget {
        Public,
        AuthorityScoped,
        CapabilityScoped,
        HolderOnly,
        ZeroKnowledge,
    }
    ```

6. Preserve the target token shape. `PresentationPurpose` is a generic
   capability purpose, not an application-domain purpose enum:

    ```rust
    pub struct SealedMembershipPresentation {
        pub braid_coordinate: BraidCoordinate,
        pub purpose: PresentationPurpose,
        pub authority_domain: AuthorityDomainRef,
        pub member_commitment: Hash,
        pub proof_or_receipt: MembershipEvidence,
        pub disclosure_budget: DisclosureBudget,
    }
    ```

Acceptance:

- A verifier can check "this actor is an authorized member of braid B for
  purpose P" without requiring global strand-id revelation.
- Reusing material across independent settlements is either impossible by API
  shape or explicitly visible as caller risk.
- Replay records what was proven and what remained sealed.
- Replay says not just that a member is sealed, but why and under which
  disclosure budget.

### 9. Introduce Named Plurality Laws

Surfaces: law machinery/design.

Depends on: replay optic and witness boundary.

Goal: retained plural strand claims MUST be interpreted by named law, not
hidden caller policy.

Work:

1. Define a registry shape for settlement, collapse, conflict-preserving,
   quorum, authority, and adapter-provided law families.
2. Make the law name and version part of the witnessed reading.
3. Attach capability, support, budget, and evidence posture to law execution.
4. Keep application nouns out of Echo core; authored contracts provide
   domain-specific laws through generated adapters.
5. Add Law Cards:

    ```text
    name = "allow-plural-over-footprint-overlap"
    version = 1
    requires = ["support-pins", "frontier-digest", "posture-floor"]
    emits = ["PluralArtifact"]
    conceals = ["sealed-member-source-chain"]
    ```

Acceptance:

- A retained braid reading states which law interpreted plurality.
- Two different laws over the same retained support produce distinct witnessed
  readings.
- Unsupported or unauthorized law execution yields typed obstruction evidence.
- Law cards are machine-readable without baking application-domain nouns into
  Echo core.

## Required PR Sequence

Follow-up implementation work MUST be split into small PRs:

| PR  | Scope                                                  | Risk if skipped                                  |
| --- | ------------------------------------------------------ | ------------------------------------------------ |
| A   | Private `Strand<P>` construction plus fixture builders | typestate-looking forgery remains API-shaped     |
| B   | `ProofError` plus `BraidTransitionKind`                | callers parse strings                            |
| C   | Golden vectors                                         | digest drift goes undetected                     |
| D   | Blinding salt docs and tests                           | reproducibility gets mistaken for privacy        |
| E   | Historical braid membership views                      | event history degrades into current-only state   |
| F   | Replay and audit optic                                 | law-bearing readings remain hard to inspect      |
| G   | `WitnessReceipt` boundary plus simulator fixtures      | self-witness scaffolding becomes permanent model |
| H   | Sealed membership capability design                    | sealed membership turns into privacy theater     |
| I   | Named plurality law registry design                    | interpretation policy hides in callers           |

PRs A-D are the hardening foundation. PRs E-I MUST NOT introduce runtime or
public API dependencies that bypass A-D.

## Mandatory Negative Capability Tests

The execution issues MUST include negative witnesses for these claims:

- Cannot construct `Strand<Shared>` with `AuthorOnly` retention posture through
  public API.
- Cannot parse behavior from `ProofError` display text.
- Cannot silently reuse deterministic salt in a privacy-sensitive example.
- Cannot satisfy a historical membership coordinate with current-only
  membership.
- Cannot treat `SelfWitness` as independent attestation.
- Cannot introduce a plurality law without a law name, version, and evidence
  posture.

## Bad-Code Register

These are not revert-class defects. They are hardening debt that MUST be closed
before more public APIs depend on the new shapes.

| Priority | Area                   | Debt                                    | Required next move                   |
| -------: | ---------------------- | --------------------------------------- | ------------------------------------ |
|        1 | `Strand<P>`            | public construction weakens typestate   | private fields plus fixture builders |
|        2 | `ProofEnvelope`        | validation returns strings              | structured `ProofError`              |
|        3 | digest identity        | no golden vectors yet                   | proof and shell vector fixtures      |
|        4 | `Braid::apply`         | transition names are strings            | typed `BraidTransitionKind`          |
|        5 | settlement blinding    | deterministic default invites overtrust | sharper docs and salt-path tests     |
|        6 | braid shell witnessing | self-witness is only E1 scaffolding     | named witness receipt design         |

## Promoted Design Targets

These ideas are promoted into the roadmap as ordered extensions. They are
mandatory design targets, but they MUST respect the dependency graph.

| Rank | Idea                                 | Required placement |
| ---: | ------------------------------------ | ------------------ |
|    1 | Braid Flight Recorder                | replay optic       |
|    2 | Causal X-Ray CLI                     | replay optic       |
|    3 | Privacy Budget Labels                | sealed capability  |
|    4 | Law Cards                            | plurality laws     |
|    5 | Witness Backend Simulator            | witness receipts   |
|    6 | Historical Membership Diff           | historical views   |
|    7 | Sealed Membership Presentation Token | sealed capability  |

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
- Do not implement witness backends before typed proof errors, witness receipt
  identity, and simulator fixtures exist.
- Do not implement sealed membership capability before blinding docs,
  salt-effect vectors, and historical membership are in place.

## Issue Seeds

Live backlog is GitHub Issues. These are ready to turn into issues when this
roadmap is pulled into execution:

1. Private `Strand<P>` construction and fixture builders.
2. Structured `ProofError` and typed braid transition kinds.
3. Golden vectors for proof envelope and braid shell identity.
4. Blinding salt privacy docs and salt-path tests.
5. Append-only braid membership historical views and membership diff.
6. Braid replay, Braid Flight Recorder, and Causal X-Ray optic.
7. External witness receipt boundary and witness backend simulator.
8. Purpose-bound sealed membership capability with disclosure budgets.
9. Named plurality law registry with Law Cards.

Each issue MUST carry one executable witness. Design-only issues MUST state
which later runtime, API, golden-vector, replay, or deterministic integration
proof closes the claim.
