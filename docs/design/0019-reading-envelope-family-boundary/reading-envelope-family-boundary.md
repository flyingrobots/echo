<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Reading envelope family boundary

Status: Accepted and partially implemented.

Depends on:

- [0011 - Optic and observer runtime doctrine](../0011-optic-observer-runtime-doctrine/design.md)
- [0018 - Echo Optics API Design](../0018-echo-optics-api-design/design.md)

## Decision

Echo has one generic read-side family boundary:

```text
ObservationRequest
  -> observer plan / optional observer instance
  -> bounded runtime read
  -> ObservationArtifact {
       resolved coordinate,
       ReadingEnvelope,
       frame,
       projection,
       payload,
       artifact_hash
     }

ObserveOpticRequest
  -> ObservationRequest bridge, when the aperture maps to built-in reads
  -> OpticReading {
       ReadingEnvelope,
       ReadIdentity,
       ObservationPayload,
       optional RetainedReadingKey
     }
```

The reading envelope is not a UI wrapper and not a cache handle. It is the
runtime evidence envelope for an observer-relative reading. It names enough
read posture for downstream consumers to know what question was answered, under
which law, at which causal basis, and with which budget/rights/residual posture.

Echo does not expose a global graph result type. Echo emits bounded,
coordinate-anchored readings.

## Family Boundary

This design separates three families that must not collapse into one bag.

### Authored Family

The authored family names the read law a contract, adapter, or kernel observer
intends to use.

Current type anchors:

- `ReadingObserverPlan`
- `BuiltinObserverPlan`
- `AuthoredObserverPlan`
- `ObserverPlanId`

Rules:

- Built-in plans are kernel-owned and finite.
- Authored plans are identified by plan id plus artifact/schema/law hashes.
- Echo core must not import application nouns to understand an authored plan.
- If an authored plan is requested but not installed, the read obstructs.

### Compiled Or Installed Artifact Family

The compiled or installed artifact family names the generated code or law object
that can execute the authored read law.

Current type anchors:

- `AuthoredObserverPlan::artifact_hash`
- `AuthoredObserverPlan::schema_hash`
- `AuthoredObserverPlan::state_schema_hash`
- `AuthoredObserverPlan::update_law_hash`
- `AuthoredObserverPlan::emission_law_hash`
- `ObserverInstanceRef`

Rules:

- Generated artifacts may be Wesley-produced or produced by another lawful
  adapter, but Echo sees them generically.
- Runtime observer instances are optional. One-shot built-in reads use `None`.
- Stateful observer reads must name the observer instance and state hash.
- Echo must reject unsupported observer plans or instances instead of falling
  back to a built-in read.
- Built-in one-shot request helpers must fail closed when the frame/projection
  pair is invalid. They must not silently relabel an invalid request as
  `QueryBytes`, because the observer plan participates in the reading contract.

### Runtime Emitted Value Family

The runtime emitted value family is the actual read result produced by Echo.

Current type anchors:

- `ObservationArtifact`
- `ReadingEnvelope`
- `ObservationPayload`
- `ObservationHashInput`
- `OpticReading`
- `ReadIdentity`
- `RetainedReadingDescriptor`
- `RetainedReadingKey`

Rules:

- `ObservationArtifact` carries coordinate resolution, envelope, frame,
  projection, payload, and deterministic artifact hash.
- `ReadingEnvelope` carries read evidence posture.
- `OpticReading` pairs the existing envelope with `ReadIdentity`.
- Retained readings are keyed by semantic read identity plus byte identity, not
  by CAS content hash alone.

## Minimum Runtime Fields

Every emitted reading family must be able to name the following:

| Field                       | Current anchor                  | Why it matters                                               |
| --------------------------- | ------------------------------- | ------------------------------------------------------------ |
| Observer plan identity      | `ReadingObserverPlan`           | Names the read law.                                          |
| Optional observer instance  | `ObserverInstanceRef`           | Names stateful observer state when used.                     |
| Resolved coordinate         | `ResolvedObservationCoordinate` | Names what was actually observed.                            |
| Observer basis              | `ReadingObserverBasis`          | Names commit-boundary, recorded-truth, or query-view basis.  |
| Witness or shell refs       | `ReadingWitnessRef`             | Names evidence supporting the reading.                       |
| Parent/strand basis posture | `ObservationBasisPosture`       | Preserves strand-relative truth and revalidation needs.      |
| Budget posture              | `ReadingBudgetPosture`          | Prevents hidden full materialization.                        |
| Rights posture              | `ReadingRightsPosture`          | Names revelation/capability posture.                         |
| Residual posture            | `ReadingResidualPosture`        | Names complete, residual, plurality, or obstruction posture. |
| Payload                     | `ObservationPayload`            | Carries the bounded emitted value.                           |
| Read identity               | `ReadIdentity` for optic reads  | Names the semantic question answered.                        |

## Identity Rules

`ObservationArtifact::artifact_hash` is computed from
`ObservationHashInput`, which includes:

- resolved coordinate,
- `ReadingEnvelope`,
- frame,
- projection,
- payload.

That means envelope posture is part of observation identity. A payload emitted
with different budget, rights, witness, observer, or residual posture is not the
same artifact.

`ReadIdentity` is the semantic identity of an optic read question. It includes:

- optic id,
- focus digest,
- coordinate,
- aperture digest,
- projection version,
- reducer version where present,
- witness basis,
- rights posture,
- budget posture,
- residual posture.

`RetainedReadingKey` is derived from:

- `ReadIdentity`,
- content hash,
- codec id,
- byte length.

CAS hashes bytes. `ReadIdentity` names the question those bytes answer.

## Current Implementation

Implemented for built-in one-shot observations:

- `ObservationArtifact::reading`
- `ReadingEnvelope`
- `ReadingObserverPlan`
- `ReadingObserverBasis`
- `ReadingWitnessRef`
- `ReadingBudgetPosture`
- `ReadingRightsPosture`
- `ReadingResidualPosture`
- `ObservationHashInput::reading`
- `ReadIdentity`
- `RetainedReadingKey`
- `ObserveOpticRequest` for bounded head and snapshot metadata reads
- fail-closed obstructions for unsupported apertures, missing witness basis,
  unsupported authored observer plans, unsupported observer instances, and
  capability-scoped rights without an installed checker

Still open:

- authored observer plans
- hosted/stateful observer instances
- app-specific budget and rights enforcement
- obstruction/plurality variants beyond the current `complete` posture
- QueryView contract observers

## Consumer Contract

Downstream consumers should depend on this family rather than inventing custom
reading wrappers.

Allowed:

- inspect `ReadingEnvelope` before trusting or rendering a payload;
- retain payload bytes by `RetainedReadingKey`;
- reveal retained bytes only when the requested `ReadIdentity` matches;
- adapt the envelope into debugger/editor/replay UI terminology outside Echo.

Rejected:

- treating payload bytes as canonical state;
- treating a CAS hash as sufficient semantic identity;
- silently materializing beyond the requested aperture;
- converting unsupported query/view reads into empty successful payloads;
- adding application-specific nouns to Echo core.

## Tests

Current test anchors:

- `ordinary_worldline_observation_reports_worldline_posture`
- `explicit_bounded_observer_request_returns_bounded_reading_artifact`
- `authored_observer_plan_obstructs_without_hidden_builtin_fallback`
- `hosted_observer_instance_obstructs_without_stateful_fallback`
- `builtin_one_shot_rejects_invalid_frame_projection`
- `capability_scoped_observer_rights_obstruct_without_public_fallback`
- `bounded_head_optic_returns_read_identity`
- `read_identity_is_stable_for_same_read_question`
- `read_identity_changes_when_question_or_witness_changes`
- `retained_reading_key_requires_content_hash_and_read_identity`
- `live_tail_read_identity_names_checkpoint_plus_tail`
- `reading_envelope_posture_participates_in_artifact_identity`

These tests keep the boundary honest: envelope posture participates in artifact
identity, optic reads have semantic identity, retained readings are not keyed by
bytes alone, and unsupported richer observer paths fail closed.

## Closure Criteria

- one packet names the minimum reading-envelope fields Echo should emit
- the boundary clearly distinguishes:
    - authored family
    - compiled artifacts
    - runtime-emitted values
- downstream repos can depend on one named family instead of reconstructing
  their own "reading result" wrappers
- the family stays narrow enough to be shared by Echo, Continuum, and debugger
  consumers

## Repo evidence

- `crates/echo-wasm-abi/src/kernel_port.rs`
- `crates/warp-core/src/observation.rs`
- `crates/warp-core/src/optic.rs`
- `docs/architecture/WARP_DRIFT.md`
- `docs/design/0006-echo-continuum-alignment/design.md`
- `docs/design/0009-witnessed-causal-suffix-sync/design.md`
