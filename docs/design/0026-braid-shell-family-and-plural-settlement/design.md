<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0026 — Braid Shell Family and Plural Settlement

_Make lawful plurality a retainable, replayable outcome: add the `Plural`
settlement arm and emit braid-scale results as in-graph holographic shells
(θ_braid) of the same family as tick receipts — so braiding becomes a WARP
optic lowering instead of a service-layer function call._

Legend: `PLATFORM`

Status: **approved with enhancements (James review, 2026-06-12) — RED next**

> A plural braid shell is not a note that plurality happened. It is the
> retained boundary that makes plurality replayable without reopening the
> strands. — review verdict

## Doctrine

AIΩN Paper VII (DOI 10.5281/zenodo.19751149):

- **Prop 3.5** — WARP is closed over its own witness-bearing outputs: tick
  receipts, braid shells, and import shells are **one shell family** of
  retained holographic boundaries living inside the causal graph. That
  containment is what makes replay cheap at every scale.
- **§4.2** — "irreducible plurality need not be treated as merge failure":
  the outcome algebra is `Derived ⊔ Plural ⊔ Conflict ⊔ Obstruction` and
  **those are the only arms**. Collapse is a witnessed _transition_ that
  produces a new `Derived` record; it is never a fifth arm.

Tracking issues: flyingrobots/echo#537 (shell-family doctrine, requirements
1–2 of 5), #538 (three-tier posture; E0-lite lands as the first commit of
this slice). Connective doctrine for #470 / #476 / #483 — this packet
establishes the shell family the others must reuse.

## Current state (verified @465cf61e)

- `SettlementDecision` (`crates/warp-core/src/settlement.rs#146`) has no
  plural arm; `ImportCandidate` lowers to `AdmissionOutcomeKind::Derived`
  (`settlement.rs#158`) and conflicts carry `ConflictReason`
  (`settlement.rs#36`). Settlement compares **one strand** against base.
- `AdmissionOutcomeKind::Plural` **already exists**
  (`crates/warp-core/src/admission.rs#150-152`) — the algebra is minted at
  admission scope; this slice extends it to settlement scope with no new
  ABI discriminant. Admission-side and settlement-side plurality are the
  **same doctrine at different optic scales**, never two meanings.
- Braid identities exist without substance: `OpticFocus::Braid`
  (`crates/warp-core/src/optic.rs#161`), `EchoCoordinate::Braid`
  (`optic.rs#297`), `SupportPin` geometry implemented and
  invariant-validated — but no reducer materializes a braid and nothing
  emits a braid-level shell.
- `BoundaryTransitionRecord` (`crates/warp-core/src/provenance_store.rs#626`)
  is the existing retained-shell mechanics — the family to extend as a
  subkind, **not** a pattern to duplicate.

## Hill

A settlement comparison over strands sharing a fork basis can end in a
**retained plural outcome**: a θ_braid shell, resident in the provenance
store, carrying the comparison basis, canonical member entries with compact
verdict snapshots, policy identity, outcome arm, witness digest, and
revelation posture — such that **the braid outcome replays from the shell
alone**, with member strand-history loaders replaced by panic stubs. That
hostile replay test is the definition of done. If replay needs member
histories, θ_braid is not a shell; it is a souvenir.

## Campaign map (this packet = E1)

| Slice  | Scope                                                                                   | Status                             |
| :----- | :-------------------------------------------------------------------------------------- | :--------------------------------- |
| E0     | Full tier-posture system on strand creation (echo#538)                                  | E0-lite lands as E1's first commit |
| **E1** | **`SettlementDecision::Plural` + θ_braid shell family + hostile replay-from-shell**     | approved with enhancements         |
| E2     | Holographic strand origins — checkpoint-pinned basis ref, empty entry vector (echo#537) | next                               |
| E3     | Braid reducer/weave over N strands + collapse-policy library                            | after E2 (needs cheap strands)     |

**Honesty rule (N-strand):** E1 data structures are N-capable (`Vec`
members, canonical ordering); E1 _behavior_ is regression-pinned for
two-member braid settlement. General N-strand reducer/weave semantics
remain E3. This packet does not claim them.

## The two policies (never blurred)

1. **Plural-settlement policy** — permits multiple lawful alternatives to
   survive a settlement comparison.
2. **Collapse policy** — permits a retained plural result to become
   `Derived` via a new witnessed shell-family record.

The rules:

> A settlement may produce `Plural` only when plurality is lawful under an
> explicit plural-settlement policy or already-witnessed plural intent.
> Once `Plural` exists, it may become `Derived` only through an explicit,
> named, witnessed collapse policy. Absent collapse policy, `Plural`
> remains `Plural`. Absent plural-settlement policy, incompatible overlap
> remains `Conflict`.

No default winner. No first-alternative-wins. No stable-sort-picks-winner.
No UI-selected-top-one. Plural is not indecision; it is a lawful retained
outcome.

## Planned shape

Settlement decision (algebraic arms only; collapse is a transition):

```rust
enum SettlementDecision {
    Derived(DerivedSettlement),
    Plural(PluralSettlement),      // refs + canonical digests, no clones
    Conflict(ConflictSettlement),
    Obstruction(ObstructionSettlement),
}
```

θ_braid as a subkind of the existing retained shell family (θ_tick,
θ_braid, θ_import are siblings; no `BraidSettlementStore`, no parallel log,
no service-layer result cache — ever):

```rust
struct BraidShell {
    version: ShellVersion,
    coordinate: EchoCoordinate,    // Braid(...) — first real consumer
    basis: BraidBasis,
    members: Vec<BraidShellMember>, // canonically ordered
    policy: SettlementPolicyRef,
    outcome: BraidShellOutcome,
    witness: BraidWitness,
    posture: RevelationPosture,
    digest: ContentDigest,
}

struct BraidShellMember {
    strand_ref: StrandRef,
    support_pin_ref: SupportPinRef,
    support_pin_digest: ContentDigest,
    basis_digest: ContentDigest,
    frontier_digest: ContentDigest,
    footprint_digest: ContentDigest,
    claim_digest: ContentDigest,
    verdict: MemberVerdict,        // compact snapshot, not history
    verdict_digest: ContentDigest,
    posture: RevelationPosture,
}

enum BraidShellOutcome {
    Derived {
        result_ref: DerivedRef,
        collapse_policy: Option<PolicyRef>,
        collapsed_from: Option<BraidShellRef>, // plural→derived lineage
    },
    Plural { alternatives: Vec<PluralMemberRef> },
    Conflict { reasons: Vec<ConflictReason> },
    Obstruction { reason: ObstructionReason, witness: WitnessDigest },
}
```

The shell binds `basis + members + member verdicts + policy + outcome +
witness`. It contains **no entry vectors and no strand histories** — enough
compact, content-addressed facts that replay never calls the restaurant.

**E0-lite posture core (first commit of E1):**

```rust
enum RevelationPosture { Scratch, AuthorOnly, Shared }
```

Default `AuthorOnly`. Promotion to `Shared` is an explicit witnessed act.
Invariant: **a braid shell cannot reveal more than its least-revealed
member** unless a witnessed redaction/promotion transform exists. Posture
is load-bearing, not cosmetic: it affects query, replay digests, promotion,
and visibility.

**Shells are append-only.** Collapse never mutates the plural shell; it
creates a new `Derived` shell-family record with
`collapsed_from: Some(prior_shell_ref)`. The old plural shell remains true
forever:

```text
θ_braid_plural ──collapsed_by(policy)──▶ θ_braid_derived
```

**Determinism:** members are canonically ordered (sort key:
`basis_digest, strand_ref, support_pin_digest, claim_digest` — or a single
canonical `member_digest`). Digest domains are explicit and separated:
`echo.shell.tick.v1`, `echo.shell.braid.v1`, `echo.shell.import.v1`,
`echo.braid.member.v1`, `echo.braid.witness.v1`.

**Serialization honesty:** `SettlementDecision`'s canonical encoding uses
stable explicit variant tags (not derived-serializer ordinal accident).
No wire-format ABI breakage; Rust source exhaustiveness may require
downstream match updates unless the enum is already `non_exhaustive` or
internal — the compiler will tell it straight, and so does this packet.

**Conflict structure:** braid-scope conflict carries enough structure to
distinguish incompatible-rewrite, missing-plural-policy
(plurality-would-have-been-lawful), basis mismatch, invalid support pin,
frontier-fact mismatch, and policy obstruction. No flattening into one
reason.

## E1a hierarchy (checkpoint review, 2026-06-12)

The plural-arm checkpoint landed as per-entry residue plumbing. The
hierarchy must never blur:

> `PluralAlternative` is an E1a per-entry residue noun. θ_braid is the
> plural settlement boundary. The final replayable plural outcome is the
> `BraidShell`, not the individual `PluralAlternative` event.

Named debts from the checkpoint review:

1. **`ProvenanceEventKind::PluralArtifact { plural_id }` is a marker, not
   a body.** The durable replayable truth is the θ_braid shell record;
   the event kind points at it. Shell facts are never re-derived from
   in-memory drafts.
2. **`ConflictReason::PluralUpstream` is temporary residue shape.** Once
   `SettlementDecision::Obstruction` exists, suffix entries blocked by
   prior retained plurality become Obstruction/PluralDependency rather
   than Conflict.
3. **Posture witnesses need a quality bar.** `promote_posture` accepts
   any 32-byte value today; before shell promotion law lands, reject
   empty/null witness digests or introduce a `WitnessDigest` newtype.
   A witness must never be a 32-byte shrug.

## E1 landed laws (post-θ_braid review, 2026-06-12)

- **Empty-settlement law:** an empty settlement is not a braid-scope
  settlement act and emits no θ_braid (no claims means no braid outcome).
  Tested as `empty_settlement_emits_no_shell_by_law`.
- **No-leak law:** no failed settlement may leak a shell; the shell is the
  final fallible step of `settle_with_policy`. Tested as
  `failed_settlement_retains_no_shell`.
- **One boundary family in code:** `RetainedBoundaryRecord` /
  `RetainedBoundaryKind` are implemented by both
  `BoundaryTransitionRecord` (θ_tick) and `BraidShell` (θ_braid);
  θ_import joins later.
- **Canonical set order:** plural `alternative_ids` are a sorted set
  (member verdict digests bind the ordered transcript); support pins and
  overlap slots digest in canonical order.
- **Residue ↔ boundary, both directions:** shell outcome lists plural
  artifact ids; `braid_shell_for_plural` resolves residue → shell.
- **Collapse law:** `collapse_braid_shell` never mutates the plural
  parent; named witnessed policy → new `Derived` shell with full lineage
  (`collapse_policy` + `collapse_witness` + `collapsed_from`, all-or-none
  coherent); missing policy → retained `Obstruction` shell. `WitnessDigest`
  refuses zero/empty digests.

## Record-law remediations (Code Lawyer self-review, 2026-06-13)

A pedantic self-review surfaced eleven findings (0 critical, 1 major, 3
medium, 7 low); all resolved before merge:

- **M1 — no-leak is now a mechanism, not a convention.**
  `ProvenanceCheckpoint` snapshots the braid-shell and plural-index key
  sets; `ProvenanceService::restore` prunes any shell or residue binding
  retained after the checkpoint. A rolled-back settlement can no longer
  leak a shell describing vanished history, even if a future fallible step
  is added after shell append.
- **D1 — witnessed-promotion law enforced by the type system.**
  `WitnessDigest` moved to `revelation` (the witness-primitives module);
  `promote_posture` now takes `WitnessDigest`, so a shrug witness cannot
  reach it. One shrug-rejection implementation, shared with the braid
  shell family.
- **D2 — one event-kind digest scheme.** `BoundaryTransitionRecord`
  reuses the canonical `coordinator::hash_provenance_event_kind` instead
  of a parallel encoder.
- **D3 — member digests computed once** per `assemble`/`validate`
  (`sort_by_cached_key` + a single digest vector feeding coordinate,
  witness, and shell digests).
- **L1–L7:** accurate `# Errors` docs; obstruction lineage uses
  `InvalidLineageParent { parent }`; `take_braid_shells` is
  `#[doc(hidden)]`; collapse's no-policy ref-drop documented; the
  `witness_digest` self-witness scaffolding documented; trait method
  delegation deduped; export grouping clarified.

## Acceptance criteria (enhanced per review)

1. `SettlementDecision` gains a `Plural` arm carrying surviving
   alternatives by refs plus canonical member/verdict digests. It lowers
   to `AdmissionOutcomeKind::Plural`.
2. Existing single-strand `Derived`, `Conflict`, and `Obstruction`
   behavior remains byte-identical at the canonical serialization/digest
   layer, with golden fixtures proving it.
3. `BraidShell` / θ_braid is added as a subkind of the existing
   `BoundaryTransitionRecord` shell family, not as a parallel record
   family.
4. θ_braid records basis, canonical member refs, support-pin digests,
   member verdict summaries, policy ref/digest, outcome arm, witness
   digest, and revelation posture.
5. θ_braid outcome uses the same algebraic arms as settlement: `Derived`,
   `Plural`, `Conflict`, `Obstruction`. Collapse is represented as a
   witnessed transition producing `Derived`, not as a separate outcome
   arm.
6. Replay from θ_braid reproduces outcome arm, member verdicts, policy
   digest, and witness digest using only the shell and provenance-store
   shell records, without loading member strand histories.
7. Plurality is never silently collapsed. A plural result remains plural
   until a named, witnessed collapse policy emits a new shell-family
   record.
8. Missing collapse policy yields retained `Plural`; missing
   plural-settlement policy for incompatible overlapping rewrites yields
   existing `Conflict` behavior.
9. θ_braid defaults to author-only; promotion to shared is explicit,
   witnessed, and cannot reveal more than member postures permit.
10. Member ordering is canonical. Input strand permutation does not change
    shell digest or replay result.
11. θ_braid can be queried by shell digest, braid coordinate, basis ref,
    member strand ref, outcome arm, and posture.
12. Tampering with basis digest, member verdict digest, policy digest,
    posture witness, or outcome digest causes replay failure.

## Test plan (enhanced per review)

1. **Plural arm shape** — two lawful alternatives over the same bounded
   site under explicit plural-settlement policy produce
   `SettlementDecision::Plural { alternatives: [a, b], .. }` lowering to
   `AdmissionOutcomeKind::Plural`.
2. **Single-strand regression** — golden fixtures prove canonical digest
   and serialized output are unchanged for existing single-strand paths.
3. **Shell emission** — every braid-scope `Derived`, `Plural`, `Conflict`,
   or `Obstruction` emits exactly one θ_braid-family boundary record.
4. **Hostile replay-from-shell** (the sacred test) — member strand-history
   loaders replaced with panic stubs; replay from θ_braid still reproduces
   outcome arm, member verdicts, policy digest, witness digest, and shell
   digest. The test fails if replay touches `load_strand_history`-shaped
   surfaces. Instrumented brutally; cheating impossible.
5. **No silent collapse** — plural without collapse policy remains
   `Plural` across ticks; attempted collapse without named policy emits
   `Obstruction` with witness.
6. **Explicit collapse** — plural + named collapse policy emits a new
   `Derived` shell referencing the prior plural shell
   (`collapsed_from`); the original plural shell is unchanged.
7. **Conflict still conflicts** — overlapping incompatible rewrites
   without plural-settlement policy keep today's `Conflict` +
   `ConflictReason` behavior exactly.
8. **Posture default and promotion** — θ_braid defaults author-only;
   promotion to shared requires witnessed promotion; promotion fails or
   redacts when member posture forbids sharing.
9. **Deterministic ordering** — same members in different input order
   produce the same θ_braid digest and same replay result.
10. **Tamper resistance** — changing member verdict digest, basis digest,
    outcome arm, or policy digest makes replay fail.
11. **Queryability** — shell retrievable by digest, basis, braid
    coordinate, member strand, outcome arm, and posture.

## Design notes (COULD tier, adopt where cheap)

- `BraidShellReplayPlan { shell_ref, required_records, forbidden_loads }`
  as the internal structure that makes the hostile harness auditable.
- θ_braid as the first real consumer of `EchoCoordinate::Braid`:
  `BraidCoordinate = hash(basis_ref, canonical_member_digest_list,
settlement_policy_digest)`; the shell lives at that coordinate.
- `PluralSetDigest(ContentDigest)` over canonical alternatives — a stable
  identity for "these alternatives lawfully coexist" independent of shell
  wrapper details.
- Collapse-lineage graph affordances: "all derived outcomes that collapsed
  plural alternatives", "all plural shells not yet collapsed", "all
  collapses by policy X".

## Refusals (DON'T tier — hard lines)

- No parallel braid result store beside the provenance store.
- No silent plural→derived degradation by time, pressure, convenience, UI
  default, or last-writer-wins.
- No `Collapsed` as an algebra arm — event/witness/transition names only.
- No claimed N-strand reducer semantics (E3 owns them).
- No cosmetic posture field.

## Playback questions

1. Can a plural settlement outcome be retained, queried, and replayed from
   its shell — with strand-history loaders panicking — without
   rematerializing member strands?
2. Is the θ_braid shell demonstrably the same record family as the
   existing boundary-transition mechanics (one family, per Prop 3.5)?
3. Does any path silently collapse plurality?
4. Does posture observably gate query, replay digests, and promotion?

## Non-goals

- No braid reducer/weave over N>2 strands or collapse-policy library (E3).
- No suffix-transport shell θ_rep / import idempotence (later slice).
- No fork-mechanics change (E2 owns holographic origins).
- No session implementation (design 0025 owns that).
- No wire-format ABI breakage (source exhaustiveness updates permitted,
  stated honestly above).

## Resolved questions (James review, 2026-06-12)

1. **E0 first?** Not as a separate blocker — E0-lite (the
   `RevelationPosture` enum, author-only default, promotion witness,
   least-revealed-member invariant) lands as the **first commit of E1**.
   θ_braid never ships with implicit visibility.
2. **Member refs?** Neither extreme: refs + canonical support-pin digest +
   compact member verdict snapshot (shape in Planned shape above).
   Self-contained for replay of the settlement result; not self-contained
   for reconstruction of the strand. The middle way is the blade.
3. **ABI?** `AdmissionOutcomeKind::Plural` exists
   (`admission.rs#150-152@465cf61e`); settlement-side plurality is the
   same doctrine at a different optic scale — one meaning, two scopes.
