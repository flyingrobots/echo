<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Witnessed Suffix Admission Shell

_Define the first skeleton vocabulary for admitting witnessed suffixes without
turning the feature into transport, remote sync, or raw patch streaming._

Status: Design

Owner: Echo / WARP runtime

Scope: first skeleton only

## Problem Statement

Echo now has three important pieces of runtime and boundary evidence in place:

- settlement overlap revalidation compares overlapped slots instead of whole
  worldline roots, so clean overlap can be distinguished from false conflict
- settlement basis evidence is exposed through the ABI, so external consumers
  can inspect parent posture and overlap revalidation evidence
- reading residual posture is exposed through the ABI, so public readings can
  name complete, residual, plurality-preserved, or obstructed posture

That makes the next risk sharper. Echo can now explain settlement and reading
posture, but it still lacks one boring, typed shell for a proposed suffix before
that suffix becomes an import, conflict artifact, or retained plural result.
Without that shell, the next implementation could accidentally smuggle in a
network sync API, a raw patch stream, or stringly typed admission status fields.

The first skeleton should define the admission vocabulary only. It should let
runtime maintainers, tools, and agents represent a witnessed suffix admission
request and response with enough basis evidence to test shape and posture, while
deliberately refusing to define transport, remote synchronization, durable wire
protocol, or full import execution.

## User Stories

### Human Users

- As a runtime maintainer, I want to inspect a proposed remote/local suffix
  before accepting it so that I can distinguish admissible history from conflict
  or obstruction.
- As a debugging/tooling user, I want admission outcomes to explain whether a
  suffix was admitted, staged, plural, conflicted, or obstructed.
- As a reviewer, I want the shell vocabulary to reuse existing settlement and
  provenance evidence so that the feature does not create another basis model.

### Agent Users

- As a coding agent, I want a typed shell vocabulary so I can generate tests and
  implementations without inventing transport or sync semantics.
- As a review agent, I want explicit outcome types so I can verify the feature
  does not smuggle in network sync or raw patch streaming.
- As a testing agent, I want golden and known-failure cases that prove the
  skeleton has exactly one top-level outcome per response.

## Hills

### Hill 1: Represent Admission Without Transport

- **Who:** Runtime maintainers.
- **What:** Represent a witnessed suffix admission request/response using
  existing provenance and basis concepts, with no network endpoint or transport
  contract.
- **Wow:** A maintainer can inspect the source suffix, target basis, and
  declared witness shell before any import execution exists.

### Hill 2: Distinguish Admission Outcomes

- **Who:** Debugging and tooling users.
- **What:** Distinguish admitted, staged, plural, conflict, and obstructed
  outcomes as typed variants.
- **Wow:** A tool can explain why a suffix is not immediately admitted without
  parsing a free-form status string or guessing from side effects.

### Hill 3: Prove Shape Completeness

- **Who:** Coding and review agents.
- **What:** Prove the skeleton is shape-complete with golden tests and
  known-failure tests.
- **Wow:** An agent can confirm the feature is a vocabulary skeleton, not hidden
  network sync, by checking tests and absence of transport surfaces.

## Non-Goals

This first skeleton explicitly does not include:

- network transport
- remote sync
- durable wire protocol
- full import execution
- conflict UI
- docs inventory
- broad ABI redesign

It also does not define trust policy, peer identity, signature handling, payload
fetching, checkpoint exchange, compaction, or multi-peer reconciliation.

## Proposed Vocabulary

The names below are conceptual. Implementation may place them in `warp-core`,
`echo-wasm-abi`, or conversion modules, but it should preserve the vocabulary
and avoid parallel status models.

### `WitnessedSuffixShell`

A compact shell describing one proposed suffix and the witness material needed
to judge it against a target basis.

Conceptual fields:

- source worldline identity as `WorldlineId`
- source suffix start and end as `WorldlineTick` bounds
- ordered source provenance as `Vec<ProvenanceRef>`
- optional source boundary witness when the suffix has no importable entries yet
- compact witness digest or reference for shell identity
- basis evidence reused from settlement where available

The shell is not a patch stream. It may reference provenance and compact
evidence, but it should not expose raw transport frames or raw network payloads.

### `WitnessedSuffixAdmissionRequest`

A request to judge one `WitnessedSuffixShell` against a target basis.

Conceptual fields:

- source shell: `WitnessedSuffixShell`
- target worldline: `WorldlineId`
- target basis: `ProvenanceRef`
- optional settlement basis evidence when the target is a strand/parent
  realization case

The request must identify both source suffix and target basis without depending
on transport.

### `WitnessedSuffixAdmissionResponse`

A response containing exactly one `WitnessedSuffixAdmissionOutcome`.

Conceptual fields:

- request identity or shell identity
- resolved target basis: `ProvenanceRef`
- outcome: `WitnessedSuffixAdmissionOutcome`
- compact evidence refs used while judging the outcome

The response must not contain zero outcomes or multiple top-level outcomes.
Plurality belongs inside the typed `Plural` outcome.

### `WitnessedSuffixAdmissionOutcome`

The top-level algebra for admission posture.

Conceptual variants:

- `Admitted`
- `Staged`
- `Plural`
- `Conflict`
- `Obstructed`

This outcome family should line up with existing runtime vocabulary:

- settlement import candidates and conflict artifacts
- settlement basis reports and overlap revalidation evidence
- `ProvenanceRef`, `WorldlineId`, and `WorldlineTick`
- reading residual posture vocabulary where a suffix remains residual, plural,
  or obstructed instead of being cleanly derived

### `Admitted`

The suffix is admissible on the named target basis.

Conceptual fields:

- admitted target worldline: `WorldlineId`
- admitted suffix range or resulting target refs
- receipt or compact admission witness
- basis evidence used to decide cleanliness

This does not execute a full import in the skeleton. It only names the clean
admission result shape.

### `Staged`

The suffix is well-formed but not admitted yet.

Conceptual fields:

- staging reason
- source shell identity
- target basis
- compact witness explaining what remains unresolved

Staged is not failure. It is the "retained for later judgment" posture.

### `Plural`

The suffix produces lawful plurality rather than one admitted result.

Conceptual fields:

- source shell identity
- plural candidate refs
- basis evidence proving plurality was preserved rather than collapsed
- optional link to reading residual posture `plurality_preserved`

Plurality must not be encoded as multiple top-level response outcomes.

### `Conflict`

The suffix conflicts with the target basis under current admission law.

Conceptual fields:

- conflict reason
- source provenance ref implicated in the conflict
- compact evidence, reusing settlement conflict artifact concepts where possible
- optional overlap revalidation evidence when footprint overlap caused the
  conflict

Conflict should remain explicit residue, not a skipped writer or silent drop.

### `Obstructed`

The suffix cannot be judged or admitted because a required witness, basis, or
lawful replay condition is unavailable.

Conceptual fields:

- obstruction reason
- source or target ref implicated in the obstruction
- compact evidence explaining missing or unusable witness material
- optional link to reading residual posture `obstructed`

Obstruction is not conflict. It means Echo cannot currently produce a lawful
admission decision.

## Playback Questions

| Question                                                                                                    | Target answer |
| ----------------------------------------------------------------------------------------------------------- | ------------- |
| Can a request identify the source suffix and target basis without transport?                                | Yes           |
| Can every response report exactly one top-level outcome?                                                    | Yes           |
| Can admitted, staged, plural, conflict, and obstructed be represented without stringly typed status fields? | Yes           |
| Can tests prove the skeleton does not implement network sync?                                               | Yes           |
| Can an agent locate the exact type to extend later for transport without inventing a parallel model?        | Yes           |

## Test Plan

### Golden Tests

- request round-trips with source/target provenance refs
- response round-trips for admitted
- response round-trips for staged
- response round-trips for plural
- response round-trips for conflict
- response round-trips for obstructed

### Known Failure Tests

- missing source suffix identity rejects
- missing target basis rejects
- response with zero outcomes rejects
- response with multiple outcomes rejects
- raw transport endpoint is not present
- network sync API is not present

### Edge Tests

- empty suffix shell
- single-entry suffix shell
- suffix with boundary witness only
- conflict outcome with compact evidence
- obstruction outcome with compact evidence

### Non-Goal Guard Tests

- no public network endpoint appears in the shell skeleton
- no sync daemon or peer loop type appears in the shell skeleton
- no raw patch stream field is required to build a request
- no string status field can replace `WitnessedSuffixAdmissionOutcome`
- no broad ABI redesign is required for the first DTO skeleton

## Implementation Plan

### RED 1

Type-shape tests fail because no shell vocabulary exists.

Expected failing tests:

- `witnessed_suffix_request_shape_names_source_and_target`
- `witnessed_suffix_response_requires_one_outcome`
- `witnessed_suffix_outcome_names_all_first_skeleton_variants`

### RED 2

Outcome round-trip tests fail.

Expected failing tests:

- `witnessed_suffix_response_round_trips_admitted`
- `witnessed_suffix_response_round_trips_staged`
- `witnessed_suffix_response_round_trips_plural`
- `witnessed_suffix_response_round_trips_conflict`
- `witnessed_suffix_response_round_trips_obstructed`

### RED 3

Invalid response shape tests fail.

Expected failing tests:

- `witnessed_suffix_response_rejects_zero_outcomes`
- `witnessed_suffix_response_rejects_multiple_outcomes`
- `witnessed_suffix_request_rejects_missing_source_suffix`
- `witnessed_suffix_request_rejects_missing_target_basis`

### GREEN 1

Add core/ABI DTO skeleton.

Expected implementation:

- define `WitnessedSuffixShell`
- define `WitnessedSuffixAdmissionRequest`
- define `WitnessedSuffixAdmissionResponse`
- define `WitnessedSuffixAdmissionOutcome`
- define admitted, staged, plural, conflict, and obstructed payload shapes

### GREEN 2

Add conversions.

Expected implementation:

- map core DTOs to ABI DTOs
- reuse `ProvenanceRef`, `WorldlineId`, `WorldlineTick`, and settlement evidence
  types where they already fit
- avoid raw internal structs that are not ABI-shaped

### GREEN 3

Add validation helpers if needed.

Expected implementation:

- validate required source suffix identity
- validate required target basis
- validate exactly one top-level response outcome
- keep transport and network sync out of validation

### GREEN 4

Pass golden and known-failure tests.

Expected implementation:

- all outcome round-trips pass
- invalid shapes reject deterministically
- non-goal guard tests prove no transport/sync surface was added

## Playback After GREEN

- [ ] Hill 1 met?
- [ ] Hill 2 met?
- [ ] Hill 3 met?
- [ ] Any non-goals violated?
- [ ] Any vocabulary drift?

## Verification Commands

```sh
cargo fmt --all -- --check
cargo test -p echo-wasm-abi --lib witnessed_suffix
cargo test -p warp-core --lib witnessed_suffix
cargo clippy -p warp-core --all-targets -- -D warnings -D missing_docs
pnpm docs:build
```
