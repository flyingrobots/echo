<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Optic Capability Grant Registry

Status: skeleton storage boundary  
Scope: Echo-owned capability grant registration only.

## Doctrine

A grant can exist before it is trusted. Registration stores authority material;
validation proves it applies.

The capability grant registry is the deterministic storage boundary for bounded
authority material. It is not admission, proof, execution, witness generation,
or permission by itself.

Two rules matter in this slice:

- registered artifact handle is not authority;
- registered capability grant is not validated authority.

The grant registry creates the place where future validation can look for
authority material. It does not decide whether that material covers an
invocation.

## System fit

The lawful optic path is converging through small boundaries:

1. Wesley compiles an `OpticArtifact`.
2. Echo registers the artifact and returns an `OpticArtifactHandle`.
3. An authority layer issues bounded grant material.
4. Echo stores that grant material in `CapabilityGrantRegistry`.
5. A caller presents an invocation with an artifact handle and presentation.
6. Current Echo admission still obstructs every presentation.
7. A later validation slice proves whether a grant covers the invocation.
8. Only after validation can Echo issue a successful admission ticket.

```mermaid
flowchart LR
  App[Application]
  Wesley[Wesley compiler]
  EchoArtifacts[Echo OpticArtifactRegistry]
  Authority[Authority layer]
  Grants[Echo CapabilityGrantRegistry]
  Invoke[OpticInvocation]
  Admit[Admission boundary]
  FutureTicket[Future AdmissionTicket]
  Witness[Future LawWitness]

  App -->|GraphQL operation| Wesley
  Wesley -->|OpticArtifact + descriptor| EchoArtifacts
  EchoArtifacts -->|opaque OpticArtifactHandle| App
  Authority -->|CapabilityGrant material| Grants
  App -->|handle + vars + presentation| Invoke
  Invoke --> Admit
  Grants -. future validation lookup .-> Admit
  Admit -. no success path in this slice .-> FutureTicket
  FutureTicket -. later .-> Witness
```

## Registration sequence

Grant registration is deliberately boring. It stores bounded material by grant
id and rejects duplicate ids.

```mermaid
sequenceDiagram
  participant A as Authority layer
  participant E as Echo
  participant R as CapabilityGrantRegistry
  participant F as Future validator

  A->>E: register CapabilityGrant
  E->>R: register_capability_grant(grant)
  alt grant id is new
    R-->>E: Ok(())
    E-->>A: stored
  else grant id already exists
    R-->>E: DuplicateGrantId
    E-->>A: reject duplicate
  end

  Note over R,F: Registration stores material only.
  F-->>R: future resolve_capability_grant(id)
  R-->>F: grant or UnknownGrantId
```

## Invocation relationship

The current invocation boundary only classifies presentation posture. A
registered grant does not change that behavior yet.

```mermaid
sequenceDiagram
  participant C as Caller
  participant A as OpticArtifactRegistry
  participant G as CapabilityGrantRegistry
  participant B as Admission boundary

  C->>A: resolve artifact handle
  A-->>B: registered artifact metadata
  C->>B: OpticInvocation + CapabilityPresentation
  B->>B: classify presentation posture
  B-->>C: OpticAdmissionTicketPosture(obstructed)

  Note over G,B: Grant lookup is intentionally not wired to admission yet.
  G--xB: no validation in this slice
```

## Class model

```mermaid
classDiagram
  class CapabilityGrant {
    +grant_id
    +subject
    +artifact_hash
    +operation_id
    +requirements_digest
    +rights
    +scope_bytes
    +budget_bytes
  }

  class CapabilityGrantRegistry {
    -grants_by_id
    +new()
    +register_capability_grant(grant)
    +resolve_capability_grant(grant_id)
    +len()
    +is_empty()
  }

  class CapabilityGrantRegistryError {
    <<enumeration>>
    DuplicateGrantId
    UnknownGrantId
  }

  class OpticCapabilityPresentation {
    +presentation_id
    +bound_grant_id
  }

  class OpticInvocation {
    +artifact_handle
    +operation_id
    +canonical_variables_digest
    +basis_request
    +aperture_request
    +capability_presentation
  }

  class OpticAdmissionTicketPosture {
    +kind
    +artifact_handle
    +operation_id
    +canonical_variables_digest
    +basis_request
    +aperture_request
    +obstruction
  }

  CapabilityGrantRegistry --> CapabilityGrant : stores
  CapabilityGrantRegistry --> CapabilityGrantRegistryError : returns
  OpticInvocation --> OpticCapabilityPresentation : may carry
  OpticAdmissionTicketPosture --> OpticInvocation : echoes context
```

## Entity relationship

```mermaid
erDiagram
  OPTIC_ARTIFACT ||--o{ CAPABILITY_GRANT : scoped_by
  CAPABILITY_GRANT_REGISTRY ||--o{ CAPABILITY_GRANT : stores
  OPTIC_INVOCATION }o--|| OPTIC_ARTIFACT_HANDLE : names
  OPTIC_INVOCATION }o--o| CAPABILITY_PRESENTATION : carries
  CAPABILITY_PRESENTATION }o--o| CAPABILITY_GRANT : claims
  OPTIC_INVOCATION ||--|| ADMISSION_POSTURE : obstructs_as

  OPTIC_ARTIFACT {
    string artifact_hash
    string operation_id
    string requirements_digest
  }

  CAPABILITY_GRANT {
    string grant_id
    string subject
    string artifact_hash
    string operation_id
    string requirements_digest
    string rights
    bytes scope_bytes
    bytes budget_bytes
  }

  CAPABILITY_PRESENTATION {
    string presentation_id
    string bound_grant_id
  }

  OPTIC_INVOCATION {
    string artifact_handle
    string operation_id
    bytes canonical_variables_digest
    bytes basis_request
    bytes aperture_request
  }

  ADMISSION_POSTURE {
    string kind
    string obstruction
  }
```

## Current grant shape

The current `CapabilityGrant` shape carries bounded material:

- grant id;
- subject;
- artifact hash;
- operation id;
- requirements digest;
- rights;
- opaque scope bytes;
- opaque budget bytes.

These fields are stored for a future validation slice. This slice intentionally
does not decide whether any registered grant authorizes any optic invocation.

## This slice does

- stores `CapabilityGrant` values by grant id;
- resolves a registered grant by grant id;
- rejects duplicate grant ids;
- rejects unknown grant lookups;
- uses deterministic `BTreeMap` storage.

## This slice does not

- validate invocation authority;
- issue successful `AdmissionTicket` values;
- emit `LawWitness` values;
- verify signatures;
- implement expiry semantics;
- implement delegation or revocation;
- execute runtime work;
- change scheduler, WASM, app, or Continuum surfaces.

## Boundary

Grant registration means Echo has authority material available for later
validation. It does not mean the grant applies to any invocation.

Capability presentation remains separate from grant registration. A
presentation may name a grant id, but it is not trusted until a future
validation boundary proves that the registered grant covers the artifact,
operation, requirements digest, subject, basis, aperture, rights, budget, and
time posture for that exact invocation.
