<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Witnessed Suffix Admission Evaluator

_Classify one local witnessed suffix admission request into exactly one typed
outcome, without adding transport, sync, daemon behavior, or a new ABI model._

Status: Design

Owner: Echo / WARP runtime

Scope: local evaluator only

## Problem Statement

Echo now has the first witnessed suffix admission shell skeleton:

- `WitnessedSuffixShell`
- `WitnessedSuffixAdmissionRequest`
- `WitnessedSuffixAdmissionResponse`
- `WitnessedSuffixAdmissionOutcome`
- admitted, staged, plural, conflict, and obstructed outcome shapes

That skeleton deliberately stopped at vocabulary. It can represent a proposed
suffix and a typed response, but it does not yet contain the local judgment step
that turns a request into one outcome.

The next risk is straightforward. If Echo moves from shell vocabulary straight
to sync, transport, or import execution, admission law will be scattered across
whatever calls the shell first. That would make it harder for maintainers to
inspect decisions, harder for tools to explain outcomes, and easier for agents
to accidentally invent a peer protocol or a second status model.

The evaluator should be the boring local middle layer:

- input: one `WitnessedSuffixAdmissionRequest`
- local read-only basis/provenance evidence
- output: one `WitnessedSuffixAdmissionResponse`
- outcome: exactly one of `Admitted`, `Staged`, `Plural`, `Conflict`, or
  `Obstructed`

This cycle must stay local. The evaluator should classify shell evidence and
target basis posture. It must not fetch remote data, open endpoints, stream raw
patches, run a sync loop, or execute a full import.

## User Stories

### Human Users

- As a runtime maintainer, I want one local evaluator for witnessed suffix
  admission so that admission law is inspectable before import execution exists.
- As a debugging/tooling user, I want a proposed suffix to explain whether it
  was admitted, staged, plural, conflicted, or obstructed with compact evidence.
- As a reviewer, I want the evaluator to reuse shell, provenance, settlement,
  and reading-posture vocabulary so that it does not create another runtime
  status language.
- As a maintainer investigating a regression, I want deterministic evaluator
  fixtures so that a failed admission classification can be replayed without a
  peer or network stack.

### Agent Users

- As a coding agent, I want a narrow evaluator contract so I can write RED tests
  without inventing transport, sync, or import semantics.
- As a review agent, I want explicit outcome branches so I can prove every local
  classification returns one top-level outcome.
- As a test agent, I want non-goal guard tests that fail if evaluator work adds
  endpoint, peer, daemon, or raw patch-stream surfaces.
- As a future implementation agent, I want a single local extension point that
  transport can call later instead of inventing a parallel admission model.

## Hills

### Hill 1: Classify One Local Request

- **Who:** Runtime maintainers.
- **What:** Evaluate one `WitnessedSuffixAdmissionRequest` against local
  read-only basis/provenance evidence and return one
  `WitnessedSuffixAdmissionResponse`.
- **Wow:** A maintainer can replay an admission decision locally and see exactly
  why the suffix was admitted, staged, plural, conflicted, or obstructed.

### Hill 2: Preserve Typed Outcome Evidence

- **Who:** Debugging and tooling users.
- **What:** Report admitted, staged, plural, conflict, and obstructed outcomes
  as typed variants that carry compact evidence.
- **Wow:** Tools can render admission posture without parsing strings, scanning
  logs, or inferring from side effects.

### Hill 3: Prove the Evaluator Is Not Sync

- **Who:** Coding and review agents.
- **What:** Use golden, known-failure, edge, and non-goal tests to prove the
  evaluator is local and shape-complete.
- **Wow:** An agent can verify the feature adds admission judgment, not network
  sync, peer identity, raw patch streaming, or import execution.

## Non-Goals

This evaluator cycle explicitly does not include:

- network transport
- remote sync
- peer identity
- durable wire protocol
- daemon behavior
- raw patch streaming
- payload fetching
- full import execution
- conflict UI
- docs inventory
- broad ABI redesign

It also does not define trust policy, signatures, multi-peer reconciliation,
checkpoint exchange, compaction, retry scheduling, or any host-facing service
loop.

## Proposed Evaluator Contract

The evaluator should live near the core witnessed suffix vocabulary. It should
not create a new public DTO family unless RED tests prove the existing shell
types are insufficient.

Conceptual entry point:

```text
evaluate_witnessed_suffix_admission(request, local_context)
  -> WitnessedSuffixAdmissionResponse
```

The subject of evaluation is the `WitnessedSuffixAdmissionRequest`. The
`local_context` is not transport. It is a read-only view of local runtime facts
needed to judge the request, such as retained provenance, target-basis posture,
known shell digests, and compact settlement evidence.

The evaluator must:

- validate the source shell identity and witness digest
- resolve the target basis locally
- inspect source suffix entries and boundary witness posture
- reuse settlement basis evidence where present
- reuse overlap revalidation evidence for conflict classification where present
- reuse reading residual posture vocabulary for plural or obstructed outcomes
- produce exactly one top-level outcome
- avoid mutating worldlines, provenance stores, inboxes, or runtime scheduler
  state

## Reused Vocabulary

The evaluator should reuse the skeleton and evidence types that already exist:

- `WitnessedSuffixShell`
- `WitnessedSuffixAdmissionRequest`
- `WitnessedSuffixAdmissionResponse`
- `WitnessedSuffixAdmissionOutcome`
- `ProvenanceRef`
- `WorldlineId`
- `WorldlineTick`
- settlement basis evidence
- settlement overlap revalidation evidence
- conflict artifact and obstruction concepts
- `ReadingResidualPosture`

Core naming can continue to use the runtime settlement names where they already
exist, such as `StrandBasisReport` and `StrandOverlapRevalidation`, while the
ABI-facing DTOs continue to expose settlement basis and overlap evidence with
ABI-shaped names. The evaluator must not leak raw internal structs across ABI
boundaries.

## Outcome Classification

The evaluator must keep failure categories separate:

- impossible or invalid DTO shape fails during construction or decode
- well-formed requests with unverifiable local evidence evaluate to `Obstructed`
- well-formed requests with deterministic adverse admission law evaluate to
  `Conflict`

That split keeps `Obstructed` from becoming a catch-all failure bucket.

### `Admitted`

Use `Admitted` when the suffix is locally admissible on the target basis.

Initial classification shape:

- source shell identity is valid
- target basis resolves locally
- source suffix entries are present and ordered
- basis evidence is clean or absent because no basis drift is involved
- no conflict or obstruction evidence is found

The evaluator does not append imports. It only reports that the request is
admissible under the local evidence it was given.

### `Staged`

Use `Staged` when the shell is well-formed but should be retained for later
judgment instead of admitted now.

Initial classification shape:

- shell identity is valid
- target basis is known
- source suffix has no importable entries yet but carries a boundary witness, or
  local policy says the suffix is well-formed but incomplete
- no conflict evidence is present

Staged is a local retention posture. It is not a sync queue, retry daemon, or
transport state.

### `Plural`

Use `Plural` when the request preserves lawful plurality instead of collapsing
to one admitted result.

Initial classification shape:

- source shell identity is valid
- target basis resolves locally
- local evidence identifies multiple lawful candidate refs
- residual posture is `ReadingResidualPosture::PluralityPreserved`
- no conflict or obstruction evidence overrides the plural result

Plurality belongs inside the `Plural` outcome. The response still has exactly
one top-level outcome.

### `Conflict`

Use `Conflict` when the request conflicts with the target basis under current
admission law.

Initial classification shape:

- source shell identity is valid enough to name the conflicting source ref
- target basis resolves locally
- conflict reason is deterministic
- compact evidence digest is available
- overlap revalidation evidence is attached when footprint overlap caused the
  conflict

Conflict should reuse existing `ConflictReason` and settlement overlap
revalidation concepts. It must not silently drop the suffix.

### `Obstructed`

Use `Obstructed` when Echo cannot currently judge the request lawfully.

Initial classification shape:

- target basis is missing or unavailable locally
- required witness material is missing
- shell digest does not match compact shell evidence
- source suffix bounds are inconsistent
- reading posture is `ReadingResidualPosture::Obstructed`

Obstruction is not conflict. It means the evaluator cannot produce a lawful
admission, staging, or plurality decision from local evidence.

## Playback Questions

| Question                                                                                      | Target answer |
| --------------------------------------------------------------------------------------------- | ------------- |
| Can one request produce exactly one top-level admission outcome?                              | Yes           |
| Can the evaluator classify admitted, staged, plural, conflict, and obstructed locally?        | Yes           |
| Can the evaluator run without network transport, peer identity, or sync daemon state?         | Yes           |
| Can conflict classification carry compact overlap revalidation evidence where available?      | Yes           |
| Can plural and obstructed outcomes reuse `ReadingResidualPosture` instead of a status string? | Yes           |
| Can tests prove the evaluator does not execute a full import?                                 | Yes           |
| Can a future transport layer locate the local evaluator instead of inventing a parallel one?  | Yes           |

## Test Plan

### Golden Tests

- request with clean local basis evaluates to `Admitted`
- request with no importable entries and a boundary witness evaluates to
  `Staged`
- request with plural local candidate evidence evaluates to `Plural`
- request with compact conflict evidence evaluates to `Conflict`
- request with unavailable target basis evaluates to `Obstructed`
- every evaluator response converts to the existing ABI response shape
- every evaluator response carries the original source shell digest and resolved
  target basis

### Known Failure Tests

- malformed or impossible request DTO shape fails during construction or decode
- request with mismatched source shell digest evaluates to `Obstructed`
- request with inconsistent suffix tick bounds returns `Obstructed`
- request with source entries outside suffix bounds returns `Obstructed`
- request with unknown target basis returns `Obstructed`
- response construction cannot produce zero outcomes
- response construction cannot produce multiple top-level outcomes
- conflict classification without compact evidence rejects or becomes
  `Obstructed`

### Edge Tests

- empty suffix shell with boundary witness
- empty suffix shell without boundary witness
- single-entry suffix shell
- suffix whose start tick equals end tick
- suffix with target basis equal to its boundary witness
- conflict outcome with clean overlap revalidation absent
- conflict outcome with conflicting overlap revalidation present
- plural outcome with `ReadingResidualPosture::PluralityPreserved`
- obstruction outcome with `ReadingResidualPosture::Obstructed`

### Non-Goal Guard Tests

- no public network endpoint appears in evaluator code
- no sync daemon, peer loop, or retry scheduler type appears in evaluator code
- no raw transport endpoint field is added to request or response DTOs
- no raw patch stream is required to evaluate a request
- evaluator tests do not start an async runtime or socket listener
- evaluator does not mutate worldline state or append provenance
- evaluator does not introduce a string status field beside
  `WitnessedSuffixAdmissionOutcome`
- evaluator does not bump or redesign ABI DTOs unless a later RED test proves an
  existing shape is insufficient

## Implementation Plan

### RED 1

Evaluator API tests fail because no local admission evaluator exists.

Expected failing tests:

- `witnessed_suffix_evaluator_accepts_request`
- `witnessed_suffix_evaluator_returns_one_response_outcome`
- `witnessed_suffix_evaluator_preserves_source_digest_and_target_basis`

### RED 2

Golden outcome tests fail.

Expected failing tests:

- `witnessed_suffix_evaluator_admits_clean_suffix`
- `witnessed_suffix_evaluator_stages_boundary_only_suffix`
- `witnessed_suffix_evaluator_preserves_plural_outcome`
- `witnessed_suffix_evaluator_reports_conflict`
- `witnessed_suffix_evaluator_reports_obstruction`

### RED 3

Known-failure and edge tests fail.

Expected failing tests:

- `witnessed_suffix_evaluator_obstructs_mismatched_digest`
- `witnessed_suffix_evaluator_obstructs_inconsistent_bounds`
- `witnessed_suffix_evaluator_obstructs_unknown_target_basis`
- `witnessed_suffix_evaluator_rejects_conflict_without_evidence`

### RED 4

Non-goal guard tests fail if evaluator work adds forbidden surfaces.

Expected failing tests:

- `witnessed_suffix_evaluator_has_no_transport_endpoint`
- `witnessed_suffix_evaluator_has_no_sync_daemon`
- `witnessed_suffix_evaluator_does_not_execute_import`
- `witnessed_suffix_evaluator_has_no_string_status`

### GREEN 1

Add the local evaluator contract.

Expected implementation:

- add a core evaluator module or helper next to `witnessed_suffix`
- define a read-only local context or trait for target basis/provenance evidence
- keep request and response DTOs unchanged unless tests prove otherwise
- return `WitnessedSuffixAdmissionResponse`

### GREEN 2

Add deterministic request validation.

Expected implementation:

- validate source suffix bounds
- validate digest posture
- validate boundary witness or entry presence
- validate target basis availability through local context
- fail impossible DTO shapes during construction or decode
- classify well-formed but unverifiable local evidence as obstruction
- classify deterministic adverse admission law as conflict

### GREEN 3

Add initial local classification.

Expected implementation:

- admitted branch for clean locally admissible suffixes
- staged branch for well-formed but incomplete local evidence
- plural branch for preserved plurality
- conflict branch for deterministic conflict evidence
- obstructed branch for missing or unusable witness/basis evidence

### GREEN 4

Add conversion and evidence checks.

Expected implementation:

- use existing `WitnessedSuffixAdmissionResponse::to_abi`
- reuse settlement basis evidence and overlap revalidation evidence
- reuse `ReadingResidualPosture` for plural and obstructed outcomes
- avoid raw internal structs in ABI-facing responses

### GREEN 5

Pass golden, known-failure, edge, and non-goal guard tests.

Expected implementation:

- all five initial outcomes can be produced locally
- invalid local evidence fails deterministically
- no transport, sync, import execution, UI, or ABI redesign appears

## Playback After GREEN

- [ ] Hill 1 met?
- [ ] Hill 2 met?
- [ ] Hill 3 met?
- [ ] Any non-goals violated?
- [ ] Any vocabulary drift?
- [ ] Did evaluator code stay local and read-only?
- [ ] Did every response contain exactly one outcome?
- [ ] Did tests prove no transport or sync surface was added?

## Verification Commands

Design-only gate for this document:

```sh
pnpm exec prettier --check docs/design/witnessed-suffix-admission-evaluator.md
pnpm exec markdownlint-cli2 docs/design/witnessed-suffix-admission-evaluator.md
pnpm docs:build
```

Expected implementation gates for the future RED/GREEN cycle:

```sh
cargo fmt --all -- --check
cargo test -p echo-wasm-abi --lib witnessed_suffix
cargo test -p warp-core --lib witnessed_suffix
cargo clippy -p warp-core --all-targets -- -D warnings -D missing_docs
pnpm docs:build
```
