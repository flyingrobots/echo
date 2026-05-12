<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo North Star

Echo is the runtime substrate for lawful, witnessed execution.

The immediate product pressure comes from jedit. The durable runtime shape is
not "make Echo know editors." It is:

```text
Applications hold product-facing capabilities.
Wesley compiles lawful optic claims.
Echo registers, admits, obstructs, instruments, and witnesses those claims.
```

Echo must stay runtime-shaped. Product nouns belong above Echo. Compiler
authority belongs to Wesley. Protocol publication belongs later, after the seam
has been proven locally.

The graph is not the substrate. The graph is a reading. The substrate is
witnessed causal history.

## Doctrine

Product pressure determines architecture truth.

One witnessed editor story is worth more than a generalized protocol that no
product path has forced into existence. Echo grows by making one executable
claim honest at a time, not by expanding ontology ahead of the witness.

The current grounding story is Stack Witness 0001:

```text
createBuffer
-> replaceRange("hello")
-> textWindow(0..5)
-> ReadingEnvelope + QueryBytes("hello")
```

This witness is not mythology. It is a small executable pressure test for the
runtime boundary.

## Ownership boundaries

| Object                        | Owner                          | Echo posture                                                                 |
| :---------------------------- | :----------------------------- | :--------------------------------------------------------------------------- |
| `OpticArtifact`               | Wesley                         | Echo verifies and stores it                                                  |
| `OpticRegistrationDescriptor` | Wesley                         | Echo accepts it as registration input, not authority                         |
| `OpticArtifactHandle`         | Echo                           | Echo returns it after verified registration                                  |
| `OpticRequirements`           | Wesley-authored, Echo-enforced | Echo stores internally and checks at invocation                              |
| `CapabilityGrant`             | Host, user, or quorum          | Echo validates it against stored artifact requirements                       |
| `CapabilityPresentation`      | Caller                         | Echo checks it at invocation time                                            |
| `LawWitness`                  | Echo                           | Echo emits runtime evidence                                                  |
| `ReadingEnvelope`             | Echo                           | Echo returns observed runtime evidence and payload posture                   |
| WARP-TTD                      | Debug observer                 | Inspects admission chains, witnesses, receipts, readings, and causal history |
| WARPDrive                     | Materializer                   | Exposes filesystem-compatible readings for legacy tools                      |

The handle proves registration, not authority.

That distinction is load-bearing. A caller holding an artifact handle has not
therefore proven permission to invoke the operation. Authority comes from a
valid capability grant and presentation that Echo checks against the registered
requirements.

## Platform flow

```text
Application declares GraphQL operation
  -> Wesley compiles OpticArtifact
  -> Wesley emits artifact hash and requirements digest
  -> App registers artifact with Echo
  -> Echo verifies artifact identity
  -> Echo stores requirements internally
  -> Echo returns opaque OpticArtifactHandle
  -> User, host, or quorum issues CapabilityGrant
  -> Caller invokes with handle, variables, and capability presentation
  -> Echo resolves handle internally
  -> Echo checks identity, authority, permissions, expiry, basis, and budget
  -> Echo admits or obstructs
  -> Echo instruments actual runtime access
  -> Echo emits LawWitness and receipt
```

Echo's job is not to trust the caller's story. Echo's job is to resolve the
registered artifact, enforce the stored requirements, observe actual runtime
access, and emit evidence about what happened.

## Echo must never accept caller-supplied runtime law

At invocation time, callers must not supply:

- requirements
- footprints
- law claims
- forbidden resources
- authority policy
- budget policy
- runtime coordinate substitutions

Those values are derived from registered artifacts, grants, and Echo-owned
runtime state. Caller-supplied variables are inputs to the operation, not
permission to redefine the operation's law.

## Required identity binding

Capability grants must bind to more than an artifact hash.

Minimum durable binding:

```text
artifact_hash
operation_id
requirements_digest
subject
issuer
scope
expiry
rights
budget constraints
basis constraints
```

The reason is the No Silent Upgrade Law:

```text
If requirements widen, old grants must not silently authorize the new operation.
```

`requirements_digest` is the field that keeps widened requirements from hiding
behind an unchanged product story.

## Echo-owned registration handle

`OpticArtifactHandle` is Echo-owned and runtime-local.

It should be opaque and small:

```text
kind = "optic-artifact-handle"
id = runtime-local opaque identifier
```

It should not contain caller-editable requirements, policy, or authority. Echo
uses the handle to find the verified registered artifact and its stored
requirements.

## Invocation posture

Invocation shape:

```text
OpticInvocation {
  artifact_handle
  operation_id
  variables_bytes
  variables_digest
  basis_request
  capability_presentation
}
```

Echo resolves the handle internally, checks the presentation against the stored
requirements, then admits or obstructs. Obstruction is a first-class outcome,
not an exceptional afterthought.

Examples of valid obstruction causes:

- unknown artifact handle
- artifact digest mismatch
- operation mismatch
- requirements digest mismatch
- missing capability
- expired capability
- subject mismatch
- unsupported basis
- rights violation
- budget exceeded
- forbidden runtime access
- insufficient support evidence

## Witness posture

Echo emits runtime evidence. It should prefer inspectable law witnesses over a
single vague boolean.

Long-term direction:

```text
artifact.registered.v1       satisfied | obstructed | unknown
identity.bound.v1            satisfied | obstructed | unknown
capability.covers.v1         satisfied | obstructed | unknown
budget.covered.v1            satisfied | obstructed | unknown
footprint.closed.v1          satisfied | obstructed | unknown
runtime.access.instrumented  satisfied | obstructed | unknown
```

Receipts can bundle witnesses. Individual witnesses should stay specific enough
to explain what was checked and why it passed, failed, or could not be proven.

## jedit boundary

jedit app-facing code should see product capabilities:

```text
optic.applyIntent(...)
optic.textWindow(...)
TextWindowReading
```

jedit app-facing code should not see:

- artifact hashes
- artifact handles
- requirements digests
- capability chain internals
- worldline ids
- head ids
- basis refs
- scheduler internals
- footprint checker internals

Those belong below the optic/session adapter boundary.

The product contract is stable. The transport and runtime machinery below it
are replaceable.

## Immediate Echo sequence

Next Echo work should proceed in this order:

1. Land the artifact registration boundary.
2. Accept a Wesley-style artifact registration descriptor.
3. Verify artifact hash, schema id, operation id, and requirements digest.
4. Store requirements internally.
5. Return an opaque Echo-owned `OpticArtifactHandle`.
6. Add an invocation skeleton using handle, variables, basis/aperture request, and capability presentation.
7. Obstruct unknown handles, mismatched operation ids, missing capabilities, and expired capabilities.
8. Emit specific `LawWitness` records for registration and admission checks.
9. Keep jedit unchanged until Echo exposes the new boundary.
10. Update jedit only below the optic/session adapter line.

## Non-goals

Do not implement these before the registration and invocation boundary is
witnessed:

- Continuum schema publication
- public package or crate release
- generalized plugin loading
- dynamic contract execution
- full collaboration semantics
- generalized capability delegation
- editor-specific Echo APIs
- Echo-owned jedit product nouns

## Operating rule

If Echo needs product nouns to make progress, the boundary is wrong or the
witness is not ready.

If jedit needs runtime coordinates to make progress, the boundary is wrong or
the witness is not ready.

If Continuum is needed before Echo and jedit can prove the seam locally, the
protocol is too early.
