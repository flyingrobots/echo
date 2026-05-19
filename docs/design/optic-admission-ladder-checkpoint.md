<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Optic Admission Ladder Checkpoint

Status: RuntimeSupport v0 boundary checkpoint.
Scope: refusal ladder with narrow controlled basis, aperture, budget, and
runtime-support fixtures.

## Doctrine

This checkpoint records the optic invocation admission ladder at the first
controlled runtime-support boundary.

Echo can now explain why an optic invocation is refused, but it cannot yet admit
one. There is no successful admission path in this checkpoint.

A registered artifact handle is not authority. A capability presentation slot is
not a validated grant. A basis request is not a resolved basis unless it matches
the narrow deterministic BasisResolution v0 fixture. A resolved basis is not
permission to act. An aperture request is not a resolved scope unless it matches
the narrow deterministic ApertureResolution v0 fixture after basis resolution.
Budget resolution exists only for the narrow deterministic BudgetResolution v0
fixture after aperture resolution.
A resolved aperture is not permission to act. A budget request is not spendable
runtime capacity. Runtime support is Echo-owned context recorded by the
registry; it is not caller-provided testimony. Resolved runtime support is not
permission to act.

Refusal is causal evidence. Refusal is not admission, not execution, not a law
witness, and not a counterfactual candidate.

## Current execution order

The current optic invocation admission path evaluates checks in this order:

1. Resolve the artifact handle internally.
2. Reject an unknown handle.
3. Reject an operation mismatch.
4. Require basis request presence.
5. Require aperture request presence.
6. Require budget request presence.
7. Classify capability presentation posture.
8. Optionally publish grant-validation obstruction evidence.
9. If capability validation returns identity-covered material, resolve the
   narrow BasisResolution v0 fixture or obstruct unsupported basis material.
10. If that basis fixture resolves, resolve the narrow ApertureResolution v0
    fixture or obstruct unsupported aperture material.
11. If that aperture fixture resolves, resolve the narrow BudgetResolution v0
    fixture or obstruct unsupported budget material.
12. If that budget fixture resolves, check Echo-owned RuntimeSupport v0 facts
    for the registered requirements or obstruct at `RuntimeSupportUnavailable`.
13. If RuntimeSupport v0 resolves, obstruct at
    `InvocationAdmissionUnavailable`.
14. Publish the invocation obstruction fact.

Presence checks come before resolution checks. Basis resolution gates aperture
resolution. Aperture resolution gates budget evaluation and runtime support
checks. The current invocation request fixture shapes are:

- BasisResolution v0: `basis-request:resolved-fixture:v0`
- ApertureResolution v0: `aperture-request:resolved-fixture:v0`
- BudgetResolution v0: `budget-request:resolved-fixture:v0`

The current Echo-owned runtime support fixture is
`runtime-support:resolved-fixture:v0`. It is recorded by the runtime registry
through an Echo-issued artifact handle for that artifact's registered
requirements. Artifact registration requires the stored requirements digest to
match the registered artifact requirements digest. Runtime support is not
carried by `OpticInvocation`.

## Obstruction reachability

| Obstruction                       | Reachability    | Meaning                                                                                                  |
| :-------------------------------- | :-------------- | :------------------------------------------------------------------------------------------------------- |
| `UnknownHandle`                   | Reachable today | Echo cannot resolve the runtime-local artifact handle.                                                   |
| `OperationMismatch`               | Reachable today | The invocation operation does not match registered artifact metadata.                                    |
| `MissingBasisRequest`             | Reachable today | The caller did not provide basis request material.                                                       |
| `MissingApertureRequest`          | Reachable today | Basis material is present, but aperture request material is absent.                                      |
| `MissingBudgetRequest`            | Reachable today | Basis and aperture material are present, but budget request material is absent.                          |
| `MissingCapability`               | Reachable today | Required invocation context is present, but no capability presentation was supplied.                     |
| `MalformedCapabilityPresentation` | Reachable today | Capability presentation material is present but not structurally usable.                                 |
| `UnboundCapabilityPresentation`   | Reachable today | Capability presentation material is structurally usable but not bound to the invocation.                 |
| `CapabilityValidationUnavailable` | Reachable today | A bound presentation exists, but no successful validation or admission has occurred yet.                 |
| `UnsupportedBasisResolution`      | Reachable today | Identity-covered material reaches the basis boundary, but the basis shape is outside BasisResolution v0. |
| `UnsupportedApertureResolution`   | Reachable today | BasisResolution v0 succeeded, but the aperture shape is outside ApertureResolution v0.                   |
| `UnsupportedBudgetResolution`     | Reachable today | ApertureResolution v0 succeeded, but the budget shape is outside BudgetResolution v0.                    |
| `RuntimeSupportUnavailable`       | Reachable today | BudgetResolution v0 succeeded, but Echo has no runtime support fact for the registered requirements.     |
| `InvocationAdmissionUnavailable`  | Reachable today | RuntimeSupport v0 succeeded, but invocation admission does not exist yet.                                |

`RuntimeSupportUnavailable` is lawfully reachable after BasisResolution v0,
ApertureResolution v0, and BudgetResolution v0 all resolve when Echo has no
runtime-owned support fact for the registered requirements.

`InvocationAdmissionUnavailable` is lawfully reachable after RuntimeSupport v0
resolves. It is the current terminal refusal after Echo proves support but
before successful invocation admission exists.

`UnsupportedApertureResolution` is reachable only after the exact
BasisResolution v0 fixture resolves. For identity-covered material, unsupported
basis shapes must still stop at `UnsupportedBasisResolution`.

`UnsupportedBudgetResolution` is reachable only after the exact
ApertureResolution v0 fixture resolves. Unsupported aperture shapes must still
stop at `UnsupportedApertureResolution`.

## Non-behavior

This checkpoint does not introduce:

- successful admission
- `AdmissionTicket`
- `LawWitness`
- scheduler behavior
- execution behavior
- storage behavior
- WASM behavior
- Continuum behavior
- authority success
- caller-supplied runtime support testimony
- general runtime support enforcement
- budget reservation

The system remains obstruction-first. It records refusal; it does not authorize
work.

## BasisResolution v0

BasisResolution v0 is not general basis resolution. It recognizes exactly one
deterministic fixture shape:

```text
basis-request:resolved-fixture:v0
```

Resolving that fixture establishes only the causal state under evaluation. It
does not create authority, admission, aperture scope, budget capacity, runtime
support, scheduler work, or execution.

## ApertureResolution v0

ApertureResolution v0 is not general aperture resolution. It recognizes exactly
one deterministic fixture shape:

```text
aperture-request:resolved-fixture:v0
```

Resolving that fixture establishes only the bounded observation/action window
inside a resolved basis. It does not create authority, admission, budget
capacity, runtime support, scheduler work, or execution.

## BudgetResolution v0

BudgetResolution v0 is not general budget resolution. It recognizes exactly one
fixture after basis and aperture resolution both succeed:

```text
budget-request:resolved-fixture:v0
```

Budget resolution establishes a bounded resource envelope under consideration.
It does not create permission to act, reserve spendable capacity, validate a
grant, or admit an invocation. The next boundary is RuntimeSupport v0: absent
Echo-owned support obstructs at `RuntimeSupportUnavailable`; resolved support
advances to `InvocationAdmissionUnavailable`.

## RuntimeSupport v0

RuntimeSupport v0 is not general runtime support. It recognizes exactly one
Echo-owned fixture for registered requirements:

```text
runtime-support:resolved-fixture:v0
```

Runtime support establishes only that Echo has recorded runtime-owned support
evidence for the artifact handle's registered requirements digest. It is not an
invocation request field, not caller testimony, not authority, not admission,
not scheduler work, and not execution. If runtime support resolves, the only
lawful next refusal in this slice is `InvocationAdmissionUnavailable`.

## Next transition point

The next transition point is InvocationAdmission v0.

That transition must be narrow and explicit. It must not imply successful
execution, scheduler work, or unconstrained authority validation.

## Tripwire

If a future slice makes `RuntimeSupportUnavailable` reachable before a lawful
budget resolution boundary exists, the admission ladder is wrong.

If a future slice introduces a successful admission path before a resolved basis,
resolved aperture, evaluated budget, runtime support check, and validated grant
exist, the admission ladder is wrong.

If a future slice makes `InvocationAdmissionUnavailable` reachable before a
lawful Echo-owned runtime support fact exists, the admission ladder is wrong.

RuntimeSupport v0 is controlled resolved runtime context, not admission.
