<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Invocation Grant Validation Obstruction Routing

Status: implementation slice.
Scope: validator-routed refusal evidence for optic invocation admission.

## Doctrine

Validation evidence refines refusal; it does not create authority.

An invocation may carry a bound capability presentation. Echo may ask a narrow
validator to inspect that presentation against the registered artifact identity.
The validator may publish sharper graph evidence such as
`GraphFact::CapabilityGrantValidationObstructed`, but invocation admission still
returns the conservative obstruction:

```text
OpticInvocationObstruction::CapabilityValidationUnavailable
```

Identity coverage is not admission. It only says recorded grant material names
the same artifact hash, operation id, and requirements digest as the registered
artifact. It does not issue an admission ticket, law witness, scheduler work, or
execution.

Failed presentation validation is causal refusal evidence, not a
counterfactual.

## Flow

```mermaid
flowchart TD
  Invocation[OpticInvocation]
  Registry[OpticArtifactRegistry]
  Artifact[RegisteredOpticArtifact]
  Presentation[OpticCapabilityPresentation]
  Validator[CapabilityPresentationValidator]
  ValidationFact[GraphFact::CapabilityGrantValidationObstructed]
  InvocationFact[GraphFact::OpticInvocationObstructed]
  Posture[OpticAdmissionTicketPosture]

  Invocation --> Registry
  Registry --> Artifact
  Invocation --> Presentation
  Presentation --> Validator
  Artifact --> Validator
  Validator -->|failed coverage| ValidationFact
  Validator -->|identity covered| Registry
  Registry --> InvocationFact
  Registry --> Posture
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
  Registry->>Registry: classify presentation shape
  Registry->>Validator: validate_capability_presentation(artifact, invocation, presentation)
  Validator->>Facts: publish CapabilityGrantValidationObstructed when validation fails
  Registry->>Facts: publish OpticInvocationObstructed
  Registry-->>Caller: Obstructed(CapabilityValidationUnavailable)
```

## Class diagram

```mermaid
classDiagram
  class OpticArtifactRegistry {
    +admit_optic_invocation(invocation)
    +admit_optic_invocation_with_capability_validator(invocation, validator)
  }

  class CapabilityPresentationValidator {
    <<trait>>
    +validate_capability_presentation(artifact, invocation, presentation)
  }

  class CapabilityGrantIntentGate {
    +validate_capability_presentation_for_artifact(presentation, artifact, expiry_posture)
  }

  class CapabilityGrantValidationOutcome {
    IdentityCovered
    Obstructed
  }

  OpticArtifactRegistry --> CapabilityPresentationValidator
  CapabilityGrantIntentGate ..|> CapabilityPresentationValidator
  CapabilityPresentationValidator --> CapabilityGrantValidationOutcome
```

## Operating rule

The validator is evidence infrastructure, not an authority oracle.

`CapabilityGrantValidationOutcome::IdentityCovered` must not be treated as
accepted authority. Until Echo has accepted grant material, admission tickets,
and law witnesses, invocation admission remains obstructed even when validation
finds identity coverage.

## Non-goals

- no successful invocation admission;
- no successful `AdmissionTicket`;
- no `LawWitness`;
- no accepted grant material;
- no delegation validation;
- no expiry parsing;
- no scheduler work;
- no execution;
- no WASM ABI;
- no Continuum schema.
