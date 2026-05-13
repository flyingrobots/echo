<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Optic Capability Grant Intent Boundary

Status: obstruction skeleton
Scope: Echo-owned capability grant intent intake and meta-authority shape only.

## Doctrine

Grant registration is causal authority intent. A grant is not authority until
Echo admits the grant intent into witnessed history.

No principal can mint authority from nowhere. Grant intent must be authorized by
prior authority, host root policy, quorum, or governance rule.

Policy evaluation that reads graph state is not a detached preflight query. It
is a basis-bound, aperture-bound, receipt-emitting atomic phase as described in
[`transaction-optic-atomicity-model.md`](transaction-optic-atomicity-model.md).
Grant intent refusals are causal obstruction records, not counterfactual grant
worlds, as described in
[`obstruction-receipt-boundary.md`](obstruction-receipt-boundary.md).

This slice only adds the shape and obstruction boundary. It does not implement a
real authority policy and therefore every grant intent remains obstructed.

The ladder is:

- registered handle is not authority;
- presentation slot is not validated grant;
- grant object is not admitted authority;
- grant intent is not accepted policy decision;
- policy shape is not trusted governance.

## System fit

The lawful optic path is converging through small boundaries:

1. Wesley compiles an `OpticArtifact`.
2. Echo registers the artifact and returns an `OpticArtifactHandle`.
3. An authority layer proposes bounded authority as `CapabilityGrantIntent`.
4. Echo evaluates the intent through an authority context and policy shape.
5. Echo returns `CapabilityGrantIntentPosture::Obstructed(...)` for every v0
   intent.
6. A caller may later present an invocation with an artifact handle and
   presentation.
7. Current Echo invocation admission still obstructs every presentation.
8. Future work admits grant intents into witnessed history, then validates
   invocation presentations against admitted grants.

```mermaid
flowchart LR
  App[Application]
  Wesley[Wesley compiler]
  ArtifactRegistry[Echo OpticArtifactRegistry]
  Authority[Authority layer]
  Intent[CapabilityGrantIntent]
  Context[AuthorityContext]
  Policy[AuthorityPolicy]
  Gate[CapabilityGrantIntentGate]
  Invocation[OpticInvocation]
  Admission[Invocation admission]
  FutureGrant[Future admitted grant]
  FutureTicket[Future AdmissionTicket]
  FutureWitness[Future LawWitness]

  App -->|GraphQL operation| Wesley
  Wesley -->|OpticArtifact + descriptor| ArtifactRegistry
  ArtifactRegistry -->|opaque OpticArtifactHandle| App
  Authority -->|proposes authority change| Intent
  Authority -->|issuer + policy shape| Context
  Context --> Policy
  Intent --> Gate
  Context --> Gate
  Gate -->|Obstructed posture| Authority
  Gate -. future witnessed admission .-> FutureGrant
  App -->|handle + vars + presentation| Invocation
  Invocation --> Admission
  FutureGrant -. future validation .-> Admission
  Admission -. later success path .-> FutureTicket
  FutureTicket -. later .-> FutureWitness
```

## Grant intent sequence

The gate checks structure, replay/duplicate posture, issuer authority presence,
policy identity, delegation posture, scope posture, and policy support. Since no
real policy exists in this slice, even a well-formed intent with issuer context
obstructs.

```mermaid
sequenceDiagram
  participant P as Principal / authority layer
  participant E as Echo
  participant G as CapabilityGrantIntentGate
  participant H as Future witnessed history

  P->>E: submit_grant_intent(intent, authority_context)
  E->>G: classify intent + authority context
  alt malformed intent
    G-->>E: Obstructed(MalformedGrantIntent)
    E-->>P: not authority
  else replay or duplicate intent id
    G-->>E: Obstructed(ReplayOrDuplicateIntent)
    E-->>P: not authority
  else missing issuer authority
    G-->>E: Obstructed(MissingIssuerAuthority)
    E-->>P: not authority
  else invalid delegation
    G->>G: record submitted intent id for replay/duplicate obstruction
    G-->>E: Obstructed(InvalidDelegation)
    E-->>P: not authority
  else scope escalation
    G->>G: record submitted intent id for replay/duplicate obstruction
    G-->>E: Obstructed(ScopeEscalation)
    E-->>P: not authority
  else no supported policy exists
    G->>G: record submitted intent id for replay/duplicate obstruction
    G-->>E: Obstructed(UnsupportedAuthorityPolicy)
    E-->>P: not authority
  end

  Note over G,H: Future work may admit grant intent into witnessed history.
  H--xG: no grant admission in this slice
```

## Invocation relationship

Capability presentation remains separate from grant intent submission. A
presentation may name a grant id, and a grant intent may have been submitted,
but neither fact authorizes invocation in this slice.

```mermaid
sequenceDiagram
  participant C as Caller
  participant A as OpticArtifactRegistry
  participant G as CapabilityGrantIntentGate
  participant B as Invocation admission boundary

  C->>A: resolve artifact handle
  A-->>B: registered artifact metadata
  C->>B: OpticInvocation + CapabilityPresentation
  B->>B: classify presentation posture
  B-->>C: OpticAdmissionTicketPosture(obstructed)

  Note over G,B: Grant intent lookup is intentionally not wired to invocation admission yet.
  G--xB: no grant validation in this slice
```

## Class model

```mermaid
classDiagram
  class PrincipalRef {
    +id
  }

  class AuthorityPolicy {
    +policy_id
  }

  class AuthorityPolicyEvaluation {
    <<enumeration>>
    InvalidDelegation
    ScopeEscalation
    Unsupported
  }

  class AuthorityContext {
    +issuer
    +policy
    +policy_evaluation
  }

  class CapabilityGrantIntent {
    +intent_id
    +proposed_by
    +subject
    +artifact_hash
    +operation_id
    +requirements_digest
    +rights
    +scope_bytes
    +expiry_bytes
    +delegation_basis_bytes
  }

  class CapabilityGrantIntentGate {
    -intents_by_id
    +new()
    +submit_grant_intent(intent, authority_context)
    +len()
    +is_empty()
  }

  class CapabilityGrantIntentOutcome {
    <<enumeration>>
    Obstructed
  }

  class CapabilityGrantIntentPosture {
    +kind
    +intent_id
    +proposed_by
    +subject
    +obstruction
  }

  class CapabilityGrantIntentObstruction {
    <<enumeration>>
    MissingIssuerAuthority
    MalformedGrantIntent
    InvalidDelegation
    ScopeEscalation
    ReplayOrDuplicateIntent
    UnsupportedAuthorityPolicy
  }

  class OpticCapabilityPresentation {
    +presentation_id
    +bound_grant_id
  }

  CapabilityGrantIntent --> PrincipalRef : proposed_by
  CapabilityGrantIntent --> PrincipalRef : subject
  AuthorityContext --> PrincipalRef : issuer
  AuthorityContext --> AuthorityPolicy : policy
  AuthorityContext --> AuthorityPolicyEvaluation : classifies
  CapabilityGrantIntentGate --> CapabilityGrantIntent : records submitted
  CapabilityGrantIntentGate --> AuthorityContext : evaluates with
  CapabilityGrantIntentGate --> CapabilityGrantIntentOutcome : returns
  CapabilityGrantIntentOutcome --> CapabilityGrantIntentPosture : carries
  CapabilityGrantIntentPosture --> CapabilityGrantIntentObstruction : explains
```

## Entity relationship

```mermaid
erDiagram
  PRINCIPAL ||--o{ CAPABILITY_GRANT_INTENT : proposes
  PRINCIPAL ||--o{ CAPABILITY_GRANT_INTENT : subject_of
  AUTHORITY_POLICY ||--o{ AUTHORITY_CONTEXT : selected_by
  CAPABILITY_GRANT_INTENT_GATE ||--o{ CAPABILITY_GRANT_INTENT : records_submitted
  OPTIC_ARTIFACT ||--o{ CAPABILITY_GRANT_INTENT : scoped_by
  CAPABILITY_GRANT_INTENT ||--|| GRANT_INTENT_POSTURE : obstructs_as
  CAPABILITY_PRESENTATION }o--o| CAPABILITY_GRANT_INTENT : claims
  OPTIC_INVOCATION }o--o| CAPABILITY_PRESENTATION : carries

  PRINCIPAL {
    string id
  }

  AUTHORITY_POLICY {
    string policy_id
  }

  AUTHORITY_CONTEXT {
    string issuer
    string policy_id
    string policy_evaluation
  }

  OPTIC_ARTIFACT {
    string artifact_hash
    string operation_id
    string requirements_digest
  }

  CAPABILITY_GRANT_INTENT {
    string intent_id
    string proposed_by
    string subject
    string artifact_hash
    string operation_id
    string requirements_digest
    string rights
    bytes scope_bytes
    bytes expiry_bytes
    bytes delegation_basis_bytes
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

  GRANT_INTENT_POSTURE {
    string kind
    string obstruction
  }
```

## Current grant intent shape

The current `CapabilityGrantIntent` shape carries proposed authority material:

- intent id;
- proposing principal;
- subject principal;
- artifact hash;
- operation id;
- requirements digest;
- rights;
- opaque scope bytes;
- opaque expiry bytes;
- opaque delegation-basis bytes.

`AuthorityContext` carries the issuer, selected policy shape, and
`policy_evaluation` posture used to classify obstruction vocabulary. The
evaluation field is policy-shaped evidence only; no trusted governance policy is
implemented in this slice.

## This slice does

- defines `PrincipalRef`;
- defines `AuthorityPolicy` and `AuthorityContext`;
- defines `CapabilityGrantIntent`;
- defines `CapabilityGrantIntentPosture`;
- classifies malformed grant intents;
- classifies replay/duplicate grant intents as `ReplayOrDuplicateIntent`;
- classifies missing issuer authority;
- classifies invalid delegation;
- classifies scope escalation;
- classifies unsupported authority policy;
- records well-formed unique submitted intent ids deterministically;
- keeps all grant intent submissions obstructed.

## This slice does not

- validate invocation authority;
- admit grant intents into witnessed history;
- make any grant authority;
- issue successful `AdmissionTicket` values;
- emit `LawWitness` values;
- verify signatures;
- implement expiry semantics;
- implement delegation or revocation;
- execute runtime work;
- change scheduler, WASM, app, or Continuum surfaces.

## Boundary

Grant intent submission means Echo has seen proposed authority material. It does
not mean the grant applies to any invocation.

Capability presentation remains separate from grant intent submission. A
presentation may name a grant id, but it is not trusted until a future
validation boundary proves that an admitted grant covers the artifact,
operation, requirements digest, subject, basis, aperture, rights, budget,
expiry, delegation posture, and issuer authority for that exact invocation.
