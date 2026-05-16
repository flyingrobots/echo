<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Basis-Bound Optic Admission

Status: implementation slice.
Scope: obstruction-only basis boundary for optic invocation admission.

## Doctrine

Admission decisions are evaluated against an explicit basis.

Basis selection is causal context, not caller folklore. A valid
`OpticArtifactHandle`, a matching operation id, and capability material that
covers registered artifact identity still do not authorize execution unless Echo
can resolve the requested basis.

This slice does not resolve basis requests successfully. It only makes basis
participation explicit:

```text
empty basis request -> MissingBasisRequest
identity covered but no basis resolver -> UnsupportedBasisResolution
```

Both outcomes remain obstructions. They are not admission tickets, not law
witnesses, not scheduler work, and not execution.

## Flow

```mermaid
flowchart TD
  Invocation[OpticInvocation]
  Registry[OpticArtifactRegistry]
  Handle[handle resolution]
  Operation[operation id check]
  Basis[basis request check]
  Presentation[presentation classification]
  Validator[CapabilityPresentationValidator]
  BasisResolution[basis resolution unavailable]
  Fact[GraphFact::OpticInvocationObstructed]
  Posture[OpticAdmissionTicketPosture]

  Invocation --> Registry
  Registry --> Handle
  Handle --> Operation
  Operation --> Basis
  Basis -->|empty| Fact
  Basis -->|non-empty| Presentation
  Presentation -->|structurally unavailable| Fact
  Presentation -->|structurally available| Validator
  Validator -->|identity covered| BasisResolution
  Validator -->|validation obstructed| Fact
  BasisResolution --> Fact
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
  Registry->>Registry: classify presentation shape
  alt presentation structurally available
    Registry->>Validator: validate_capability_presentation(artifact, invocation, presentation)
    Validator->>Facts: publish grant validation obstruction when validation fails
    Registry->>Registry: obstruct identity-covered material at basis boundary
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
    +capability_presentation
  }

  class OpticBasisRequest {
    +bytes
  }

  class OpticInvocationObstruction {
    MissingBasisRequest
    UnsupportedBasisResolution
    MissingCapability
    CapabilityValidationUnavailable
  }

  class OpticAdmissionTicketPosture {
    +basis_request
    +aperture_request
    +obstruction
  }

  OpticInvocation --> OpticBasisRequest
  OpticAdmissionTicketPosture --> OpticBasisRequest
  OpticAdmissionTicketPosture --> OpticInvocationObstruction
```

## Entity relationship

```mermaid
erDiagram
  OPTIC_INVOCATION ||--|| BASIS_REQUEST : names
  OPTIC_INVOCATION ||--|| APERTURE_REQUEST : names
  OPTIC_INVOCATION ||--|| INVOCATION_OBSTRUCTION_FACT : produces_when_refused
  INVOCATION_OBSTRUCTION_FACT ||--|| BASIS_REQUEST_DIGEST : records
  INVOCATION_OBSTRUCTION_FACT ||--|| APERTURE_REQUEST_DIGEST : records

  OPTIC_INVOCATION {
    string artifact_handle_id
    string operation_id
    bytes canonical_variables_digest
  }

  BASIS_REQUEST {
    bytes opaque_request
  }

  APERTURE_REQUEST {
    bytes opaque_request
  }

  INVOCATION_OBSTRUCTION_FACT {
    bytes basis_request_digest
    bytes aperture_request_digest
    string obstruction
  }
```

## Operating rule

Basis resolution is not ambient. If Echo cannot bind an invocation to an
explicit causal basis, the invocation remains refused even when artifact
identity and capability material line up.

## Non-goals

- no successful basis resolution;
- no successful invocation admission;
- no successful `AdmissionTicket`;
- no `LawWitness`;
- no scheduler work;
- no execution;
- no storage engine;
- no WASM ABI;
- no Continuum schema.
