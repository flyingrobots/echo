<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Goalpost 4: Witness Receipts And Sealed Capabilities

Status: planned.

Roadmap:
[`../braids-and-strands-roadmap.md`](../braids-and-strands-roadmap.md)

## Decision Summary

Echo will name the witness receipt boundary before real witness backends
arrive, then design purpose-bound sealed membership presentations on top of
historical membership and explicit privacy posture.

## Invariant

Self-witness is local integrity scaffolding, not independent attestation.
Sealed membership proves only the aperture-authorized membership claim for a
purpose and disclosure budget.

## Sponsored Human

A maintainer wants witness and sealed membership surfaces that are ready for
real backends without giving current scaffolding stronger semantics than it
has earned.

## Sponsored Agent

An agent needs simulator fixtures and disclosure labels so it can test
supported, rejected, and unsupported witness outcomes before cryptographic or
institutional backends exist.

## Scope

This goalpost includes:

- `WitnessReceipt`, `WitnessKind`, and `WitnessBackend`;
- deterministic witness backend simulator fixtures;
- explicit compatibility rules for witness identity;
- purpose-bound sealed membership presentation design;
- generic `PresentationPurpose` capability vocabulary;
- disclosure budget labels;
- replay wording for sealed membership.

## Non-Goals

This goalpost does not include:

- real ZK, threshold, or signature backend implementation;
- domain-specific purpose enums in Echo core;
- sealed membership before historical membership and salt vectors exist;
- treating self-witness as independent attestation.

## Slices

| Slice  | Work                                    | Witness                                      |
| ------ | --------------------------------------- | -------------------------------------------- |
| GP4-S1 | Define witness receipt boundary         | typed unsupported-backend tests              |
| GP4-S2 | Add witness simulator fixtures          | supported/rejected/unsupported fixture tests |
| GP4-S3 | Bind witness identity by compatibility  | compatibility class tests or docs assertions |
| GP4-S4 | Design sealed membership presentation   | design doc and capability fixture            |
| GP4-S5 | Add disclosure budget labels and replay | replay facts for sealed membership           |

## Acceptance

- Unsupported witness kinds fail as typed unsupported-backend outcomes.
- Simulator fixtures harden witness behavior before real backends exist.
- `PresentationPurpose` remains a generic capability purpose, not an
  application-domain enum.
- Replay records what was proven, what remained sealed, and which disclosure
  budget applied.
