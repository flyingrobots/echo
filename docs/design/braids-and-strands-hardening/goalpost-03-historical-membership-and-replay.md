<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Goalpost 3: Historical Membership And Replay

Status: planned.

Roadmap:
[`../braids-and-strands-roadmap.md`](../braids-and-strands-roadmap.md)

## Decision Summary

Echo will make braid membership historical and replayable. Current membership
becomes one projection over append-only event history, while replay surfaces
explain member verdicts, posture floors, proof binding, retained support,
frontiers, and witness posture.

## Invariant

Braid membership changes are append-only history. Current-only views cannot
satisfy historical coordinate requests.

## Sponsored Human

A maintainer wants to answer who belonged to a braid at a coordinate without
pretending later members existed in earlier intervals.

## Sponsored Agent

An agent needs replay facts and lower-mode output so it can audit braid shell
readings without hand-inspecting internal state.

## Scope

This goalpost includes:

- append-only membership design;
- historical membership views by coordinate or event sequence;
- membership diff facts for added, ended, revealed, and concealed changes;
- replay facts for member verdicts and proof/witness posture;
- Braid Flight Recorder artifact shape;
- Causal X-Ray lower-mode output target.

## Non-Goals

This goalpost does not include:

- settlement-as-merge semantics;
- exposing sealed source chains beyond the requested aperture;
- external witness backend implementation;
- plurality law registry execution.

## Slices

| Slice  | Work                                    | Witness                                   |
| ------ | --------------------------------------- | ----------------------------------------- |
| GP3-S1 | Promote append-only membership design   | accepted design and invariant tests       |
| GP3-S2 | Add historical membership views         | coordinate-based membership tests         |
| GP3-S3 | Add membership diff facts               | added/ended/revealed/concealed diff tests |
| GP3-S4 | Add replay/audit facts                  | stable JSON or fact assertions            |
| GP3-S5 | Define recorder and Causal X-Ray output | fixture output and docs example           |

## Acceptance

- A braid whose initial interval includes `s0` and `s1`, then later weaves in
  `s2`, reports `s2` only after the weave coordinate.
- Current membership remains a deterministic projection over the same event
  log.
- Replay distinguishes admitted, retained, concealed, conflicted, obstructed,
  and unsupported claims.
- `SelfWitness` is displayed as integrity-only unless an external receipt says
  otherwise.
