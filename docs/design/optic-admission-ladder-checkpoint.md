<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Optic Admission Ladder Checkpoint

Status: checkpoint.
Scope: documentation-only refusal ladder before BasisResolution v0.

## Doctrine

This checkpoint records the optic invocation admission ladder exactly at the
last refusal-only boundary.

Echo can now explain why an optic invocation is refused, but it cannot yet admit
one. There is no successful admission path in this checkpoint.

A registered artifact handle is not authority. A capability presentation slot is
not a validated grant. A basis request is not a resolved basis. An aperture
request is not a resolved scope. A budget request is not spendable runtime
capacity.

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
9. Obstruct identity-covered material at unsupported basis resolution.
10. Publish the invocation obstruction fact.

Presence checks come before resolution checks. Basis resolution gates aperture
resolution. Aperture resolution gates budget evaluation and runtime support
checks.

## Obstruction reachability

| Obstruction                       | Reachability      | Meaning                                                                                                    |
| :-------------------------------- | :---------------- | :--------------------------------------------------------------------------------------------------------- |
| `UnknownHandle`                   | Reachable today   | Echo cannot resolve the runtime-local artifact handle.                                                     |
| `OperationMismatch`               | Reachable today   | The invocation operation does not match registered artifact metadata.                                      |
| `MissingBasisRequest`             | Reachable today   | The caller did not provide basis request material.                                                         |
| `MissingApertureRequest`          | Reachable today   | Basis material is present, but aperture request material is absent.                                        |
| `MissingBudgetRequest`            | Reachable today   | Basis and aperture material are present, but budget request material is absent.                            |
| `MissingCapability`               | Reachable today   | Required invocation context is present, but no capability presentation was supplied.                       |
| `MalformedCapabilityPresentation` | Reachable today   | Capability presentation material is present but not structurally usable.                                   |
| `UnboundCapabilityPresentation`   | Reachable today   | Capability presentation material is structurally usable but not bound to the invocation.                   |
| `CapabilityValidationUnavailable` | Reachable today   | Presentation material is identity-covered, but real grant validation does not exist yet.                   |
| `UnsupportedBasisResolution`      | Reachable today   | Identity-covered material reaches the resolution boundary, but lawful basis resolution does not exist yet. |
| `UnsupportedApertureResolution`   | Future vocabulary | Must remain unreachable until lawful basis resolution exists.                                              |
| `UnsupportedBudgetResolution`     | Future vocabulary | Must remain unreachable until lawful basis and aperture resolution exist.                                  |
| `RuntimeSupportUnavailable`       | Future vocabulary | Must remain unreachable until lawful basis, aperture, and budget resolution exist.                         |

`UnsupportedApertureResolution`, `UnsupportedBudgetResolution`, and
`RuntimeSupportUnavailable` are deliberately defined but not lawfully reachable
at this checkpoint.

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
- runtime support enforcement
- budget reservation
- aperture resolution
- basis resolution

The system remains obstruction-first. It records refusal; it does not authorize
work.

## Next transition point

The next transition point is BasisResolution v0.

BasisResolution v0 is the first controlled resolved-state boundary. It is the
first place where Echo may start saying something stronger than refusal about a
requested invocation context.

That transition must be narrow and explicit. It must not imply successful
admission, aperture resolution, budget spendability, runtime support, execution,
or authority validation.

## Tripwire

If a future slice makes `UnsupportedApertureResolution`,
`UnsupportedBudgetResolution`, or `RuntimeSupportUnavailable` reachable before a
lawful basis resolution boundary exists, the admission ladder is wrong.

If a future slice introduces a successful admission path before a resolved basis,
resolved aperture, evaluated budget, runtime support check, and validated grant
exist, the admission ladder is wrong.

This is the final refusal-only checkpoint before controlled resolved state.
