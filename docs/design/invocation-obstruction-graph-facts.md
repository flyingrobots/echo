<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Invocation Obstruction Graph Facts

Status: implementation slice.
Scope: in-memory causal graph fact publication for optic invocation refusal.

## Doctrine

Registered handle does not imply authority.

Invocation obstruction is causal refusal evidence. It is not a successful
admission ticket, not a law witness, not execution, not scheduler output, and
not a counterfactual candidate.

```text
registered artifact handle
  -> invocation attempted
  -> authority/presentation unavailable or invalid
  -> obstruction posture
  -> GraphFact::OpticInvocationObstructed
```

Receipts explain graph outcomes. They do not replace graph facts.

## Fact model

`GraphFact::OpticInvocationObstructed` records:

- `artifact_handle_id`;
- `operation_id`;
- `canonical_variables_digest`;
- `basis_request_digest`;
- `aperture_request_digest`;
- `obstruction`.

The basis and aperture request fields are stored as deterministic digests of
opaque request bytes. Echo does not interpret those request bytes in this slice.

## Flow

```mermaid
flowchart TD
  Caller[Caller]
  Invocation[OpticInvocation]
  Registry[OpticArtifactRegistry]
  Resolve[Resolve handle]
  Operation[Check operation id]
  Capability[Classify capability presentation]
  Posture[OpticAdmissionTicketPosture]
  Fact[GraphFact::OpticInvocationObstructed]
  Digest[FactDigest]

  Caller --> Invocation
  Invocation --> Registry
  Registry --> Resolve
  Resolve -->|unknown| Posture
  Resolve -->|known| Operation
  Operation -->|mismatch| Posture
  Operation -->|match| Capability
  Capability --> Posture
  Posture --> Fact
  Fact --> Digest
```

## Sequence

```mermaid
sequenceDiagram
  participant Caller as caller
  participant Registry as OpticArtifactRegistry
  participant Facts as in-memory fact log

  Caller->>Registry: admit_optic_invocation(invocation)
  Registry->>Registry: resolve handle
  Registry->>Registry: classify obstruction
  Registry->>Facts: append OpticInvocationObstructed
  Registry-->>Caller: obstructed admission posture
```

## Class diagram

```mermaid
classDiagram
  class OpticArtifactRegistry {
    +admit_optic_invocation(invocation)
    +published_graph_facts()
  }

  class GraphFact {
    OpticInvocationObstructed
    +digest()
  }

  class OpticInvocation {
    +artifact_handle
    +operation_id
    +canonical_variables_digest
    +basis_request
    +aperture_request
    +capability_presentation
  }

  class InvocationObstructionKind {
    UnknownHandle
    OperationMismatch
    MissingCapability
    MalformedCapabilityPresentation
    UnboundCapabilityPresentation
    CapabilityValidationUnavailable
  }

  OpticArtifactRegistry --> OpticInvocation
  OpticArtifactRegistry --> GraphFact
  GraphFact --> InvocationObstructionKind
```

## Entity relationship

```mermaid
erDiagram
  OPTIC_INVOCATION ||--|| OPTIC_ARTIFACT_HANDLE : names
  OPTIC_ARTIFACT_REGISTRY ||--o{ PUBLISHED_GRAPH_FACT : publishes
  PUBLISHED_GRAPH_FACT ||--|| FACT_DIGEST : has
  PUBLISHED_GRAPH_FACT ||--|| INVOCATION_OBSTRUCTION : records

  INVOCATION_OBSTRUCTION {
    string artifact_handle_id
    string operation_id
    bytes canonical_variables_digest
    bytes basis_request_digest
    bytes aperture_request_digest
    string obstruction
  }
```

## Non-goals

- no success admission;
- no `AdmissionTicket`;
- no `LawWitness`;
- no grant validation inside `admit_optic_invocation`;
- no execution;
- no scheduler;
- no persistence;
- no Continuum schema.

## Operating rule

Only legally admitted but unselected rewrites can become counterfactual
candidates. Invocation obstruction facts are refusal records, not unrealized
legal worlds.
