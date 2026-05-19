<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Budget and Runtime Support Optic Admission

Status: implementation slice.
Scope: obstruction-only budget request and runtime support boundary for optic
invocation admission.

## Doctrine

A basis request is not a resolved basis.

An aperture request is not a resolved scope.

A budget request is not spendable runtime capacity.

Runtime support is not caller-provided testimony.

Budget is invocation context supplied by the caller. It describes bounded
resource intent, but Echo has not evaluated or reserved any capacity in this
slice.

Runtime support is checked by Echo against registered artifact requirements and
Echo's own runtime support surface. The caller does not supply a support request
and Echo must not ask the caller whether Echo supports an operation.

This slice remains obstruction-only:

```text
empty basis request -> MissingBasisRequest
non-empty basis + empty aperture request -> MissingApertureRequest
non-empty basis + non-empty aperture + empty budget request -> MissingBudgetRequest
identity covered + unsupported basis -> UnsupportedBasisResolution
resolved basis + unsupported aperture -> UnsupportedApertureResolution
resolved aperture + unsupported budget -> UnsupportedBudgetResolution
resolved budget + no Echo-owned runtime support fact -> RuntimeSupportUnavailable
resolved runtime support + no Echo-owned admission fact -> InvocationAdmissionUnavailable
resolved invocation admission + no Echo-owned scheduler admission fact
  -> SchedulerAdmissionUnavailable
resolved scheduler admission -> SchedulerWorkUnavailable
```

`UnsupportedBudgetResolution` and `RuntimeSupportUnavailable` are current
obstruction vocabulary. They are reachable only after the earlier gates resolve:
basis resolution gates aperture resolution, aperture resolution gates budget
resolution, and budget resolution gates the Echo-owned runtime support check.

Refusal remains causal evidence. Budget and support obstruction facts are not
counterfactual candidates.

## Ordering

Presence checks happen before resolution checks.

Basis resolution gates aperture resolution.

Aperture resolution gates budget evaluation and runtime support checks.

```text
handle
-> operation
-> basis request presence
-> aperture request presence
-> budget request presence
-> basis resolution
-> aperture resolution
-> budget evaluation
-> runtime support evaluation
-> invocation admission evaluation
-> scheduler admission evaluation
-> scheduler work unavailable
```

This slice reaches the narrow fixture gates through SchedulerAdmission v0. It
still has no scheduler work and no execution.

## Flow

```mermaid
flowchart TD
  Invocation[OpticInvocation]
  Registry[OpticArtifactRegistry]
  Handle[handle resolution]
  Operation[operation id check]
  BasisPresence[basis request present?]
  AperturePresence[aperture request present?]
  BudgetPresence[budget request present?]
  Presentation[presentation classification]
  Validator[CapabilityPresentationValidator]
  BasisResolution[BasisResolution v0]
  ApertureResolution[ApertureResolution v0]
  BudgetResolution[BudgetResolution v0]
  RuntimeSupport[RuntimeSupport v0]
  InvocationAdmission[InvocationAdmission v0]
  SchedulerAdmission[SchedulerAdmission v0]
  Fact[GraphFact::OpticInvocationObstructed]
  Posture[OpticAdmissionTicketPosture]

  Invocation --> Registry
  Registry --> Handle
  Handle --> Operation
  Operation --> BasisPresence
  BasisPresence -->|missing| Fact
  BasisPresence -->|present| AperturePresence
  AperturePresence -->|missing| Fact
  AperturePresence -->|present| BudgetPresence
  BudgetPresence -->|missing| Fact
  BudgetPresence -->|present| Presentation
  Presentation -->|structurally unavailable| Fact
  Presentation -->|structurally available| Validator
  Validator -->|identity covered| BasisResolution
  Validator -->|validation obstructed| Fact
  BasisResolution -->|unsupported| Fact
  BasisResolution -->|resolved| ApertureResolution
  ApertureResolution -->|unsupported| Fact
  ApertureResolution -->|resolved| BudgetResolution
  BudgetResolution -->|unsupported| Fact
  BudgetResolution -->|resolved| RuntimeSupport
  RuntimeSupport -->|missing support fact| Fact
  RuntimeSupport -->|resolved| InvocationAdmission
  InvocationAdmission -->|missing admission fact| Fact
  InvocationAdmission -->|resolved| SchedulerAdmission
  SchedulerAdmission -->|missing scheduler admission fact| Fact
  SchedulerAdmission -->|resolved| Fact
  Fact --> Posture
```

## Sequence

```mermaid
sequenceDiagram
  participant Caller as caller
  participant Registry as OpticArtifactRegistry
  participant Validator as CapabilityPresentationValidator
  participant Facts as graph fact log

  Caller->>Registry: admit_optic_invocation_with_capability_validator(invocation, validator)
  Registry->>Registry: resolve artifact handle
  Registry->>Registry: check operation id
  Registry->>Registry: reject empty basis request
  Registry->>Registry: reject empty aperture request
  Registry->>Registry: reject empty budget request
  Registry->>Registry: classify presentation shape
  alt presentation structurally available
    Registry->>Validator: validate_capability_presentation(artifact, invocation, presentation)
    Validator->>Facts: publish grant validation obstruction when validation fails
    Registry->>Registry: resolve basis, aperture, budget, runtime support, admission
    Registry->>Registry: resolve scheduler admission fixtures
    Registry->>Registry: obstruct resolved scheduler admission before scheduler work
  else missing, malformed, or unbound presentation
    Registry->>Registry: skip validator
  end
  Registry->>Facts: publish OpticInvocationObstructed
  Registry-->>Caller: Obstructed(...)
```

## Class diagram

```mermaid
classDiagram
  class OpticInvocation {
    +artifact_handle
    +operation_id
    +canonical_variables_digest
    +basis_request
    +aperture_request
    +budget_request
    +capability_presentation
  }

  class OpticBudgetRequest {
    +bytes
  }

  class OpticInvocationObstruction {
    MissingBudgetRequest
    UnsupportedBudgetResolution
    RuntimeSupportUnavailable
    InvocationAdmissionUnavailable
    SchedulerAdmissionUnavailable
    SchedulerWorkUnavailable
  }

  class RegisteredOpticArtifact {
    +artifact_hash
    +operation_id
    +requirements_digest
    +requirements
  }

  class EchoRuntimeSupportSurface {
    +runtime-owned support facts
    +runtime-owned admission facts
    +runtime-owned scheduler admission facts
  }

  OpticInvocation --> OpticBudgetRequest
  RegisteredOpticArtifact --> EchoRuntimeSupportSurface
  EchoRuntimeSupportSurface --> OpticInvocationObstruction
```

## Entity relationship

```mermaid
erDiagram
  OPTIC_INVOCATION ||--|| BASIS_REQUEST : names
  OPTIC_INVOCATION ||--|| APERTURE_REQUEST : names
  OPTIC_INVOCATION ||--|| BUDGET_REQUEST : names
  OPTIC_INVOCATION ||--|| INVOCATION_OBSTRUCTION_FACT : produces_when_refused
  REGISTERED_OPTIC_ARTIFACT ||--|| RUNTIME_SUPPORT_SURFACE : checked_against
  INVOCATION_OBSTRUCTION_FACT ||--|| BUDGET_REQUEST_DIGEST : records

  OPTIC_INVOCATION {
    string artifact_handle_id
    string operation_id
    bytes canonical_variables_digest
  }

  BUDGET_REQUEST {
    bytes opaque_request
  }

  REGISTERED_OPTIC_ARTIFACT {
    string artifact_hash
    string operation_id
    string requirements_digest
  }

  INVOCATION_OBSTRUCTION_FACT {
    bytes basis_request_digest
    bytes aperture_request_digest
    bytes budget_request_digest
    string obstruction
  }
```

## Operating rule

Budget is caller context. Runtime support is Echo context.

Echo must not accept caller testimony about runtime support. Support checks
compare registered artifact requirements against Echo-owned runtime support
facts recorded by the registry.

As of InvocationAdmission v0, Echo records a narrow runtime-owned invocation
admission fixture through Echo-issued artifact handles. That admission fact
advances the ladder past `InvocationAdmissionUnavailable`, but only to
SchedulerAdmission v0.

As of SchedulerAdmission v0, Echo records a narrow runtime-owned scheduler
admission fixture through Echo-issued artifact handles. That scheduler
admission fact advances the ladder past `SchedulerAdmissionUnavailable`, but
only to `SchedulerWorkUnavailable`; it still does not schedule work or execute
an invocation.

## Non-goals

- no `MissingSupportRequest`;
- no `MissingSchedulerAdmissionRequest`;
- no support request bytes;
- no scheduler admission request bytes;
- no successful `AdmissionTicket`;
- no `LawWitness`;
- no scheduler work;
- no execution;
- no storage engine;
- no WASM ABI;
- no Continuum schema.
