<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Aperture-Bound Optic Admission

Status: implementation slice.
Scope: obstruction-only aperture boundary for optic invocation admission.

## Doctrine

A basis answers which causal state is being evaluated.

An aperture answers what graph, window, or scope the invocation may see or
affect inside that resolved basis.

Basis alone is too broad. Without an aperture, a valid invocation could imply
ambient access to the whole resolved graph state. Aperture request presence is
therefore mandatory before Echo can continue toward authority, footprint, or
execution checks.

This slice does not resolve apertures successfully. It only makes aperture
participation explicit:

```text
empty basis request -> MissingBasisRequest
non-empty basis + empty aperture request -> MissingApertureRequest
non-empty basis + non-empty aperture + identity covered -> UnsupportedBasisResolution
```

`UnsupportedApertureResolution` exists as vocabulary for the future basis
resolved path. It is not reachable in this slice because aperture resolution is
defined only over a resolved basis.

Refusal remains causal evidence. Aperture obstruction facts are not
counterfactual candidates.

## Ordering

Presence checks happen before resolution checks. Basis resolution gates aperture
resolution.

```text
handle
-> operation
-> basis existence
-> aperture existence
-> basis resolution
-> aperture resolution
-> authority validation
-> footprint compatibility
-> execution
```

Echo must not evaluate aperture resolution before basis resolution exists.
Aperture is a constrained projection over a resolved basis, not a globally
meaningful graph region independent of causal state.

## Flow

```mermaid
flowchart TD
  Invocation[OpticInvocation]
  Registry[OpticArtifactRegistry]
  Handle[handle resolution]
  Operation[operation id check]
  BasisPresence[basis request present?]
  AperturePresence[aperture request present?]
  Presentation[presentation classification]
  Validator[CapabilityPresentationValidator]
  BasisResolution[basis resolution unavailable]
  FutureAperture[Aperture resolution future slot]
  Fact[GraphFact::OpticInvocationObstructed]
  Posture[OpticAdmissionTicketPosture]

  Invocation --> Registry
  Registry --> Handle
  Handle --> Operation
  Operation --> BasisPresence
  BasisPresence -->|missing| Fact
  BasisPresence -->|present| AperturePresence
  AperturePresence -->|missing| Fact
  AperturePresence -->|present| Presentation
  Presentation -->|structurally unavailable| Fact
  Presentation -->|structurally available| Validator
  Validator -->|identity covered| BasisResolution
  Validator -->|validation obstructed| Fact
  BasisResolution --> Fact
  BasisResolution -. future .-> FutureAperture
  FutureAperture -. future .-> Fact
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

  class OpticApertureRequest {
    +bytes
  }

  class OpticInvocationObstruction {
    MissingBasisRequest
    MissingApertureRequest
    UnsupportedBasisResolution
    UnsupportedApertureResolution
  }

  class OpticAdmissionTicketPosture {
    +basis_request
    +aperture_request
    +obstruction
  }

  OpticInvocation --> OpticBasisRequest
  OpticInvocation --> OpticApertureRequest
  OpticAdmissionTicketPosture --> OpticBasisRequest
  OpticAdmissionTicketPosture --> OpticApertureRequest
  OpticAdmissionTicketPosture --> OpticInvocationObstruction
```

## Entity relationship

```mermaid
erDiagram
  BASIS_REQUEST ||--|| APERTURE_REQUEST : gates
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

Basis establishes the universe. Aperture establishes the window.

Echo must not resolve a window before the universe exists. Until basis
resolution is real, `UnsupportedApertureResolution` remains future vocabulary,
not a reachable runtime branch.

## Non-goals

- no successful aperture resolution;
- no successful basis resolution;
- no successful invocation admission;
- no successful `AdmissionTicket`;
- no `LawWitness`;
- no scheduler work;
- no execution;
- no storage engine;
- no WASM ABI;
- no Continuum schema.
