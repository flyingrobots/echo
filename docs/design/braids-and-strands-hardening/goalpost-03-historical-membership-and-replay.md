<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Goalpost 3: Historical Membership And Replay

Status: active. GP3-S1 and GP3-S2 implemented.

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

## Implementation Design

GP3-S1 establishes the membership-history source of truth without implementing
the later coordinate, diff, replay, or recorder surfaces.

The implementation boundary is:

```text
BraidEvent::MemberWoven
-> BraidMembershipEntry
-> Braid::membership_history()
-> Braid::frontier()
```

`BraidEvent` remains the authoritative append-only log. A
`BraidMembershipEntry` is a read projection over one accepted `MemberWoven`
event; it is not an admission token and constructing it does not weave a
member. `Braid::membership_history()` projects accepted weave events from the
log in event order. Rejected duplicate, incoherent, mixed-posture, or late
member events never enter the log and therefore never appear in membership
history.

`Braid::frontier()` remains the current membership projection.
`BraidMembershipCursor` names a half-open membership interval by event sequence:
`[0, next_sequence_num)`. `Braid::current_membership_cursor()` captures the
current cursor, and `Braid::membership_at(...)` projects accepted
`MemberWoven` facts visible at that historical cursor. This intentionally does
not reuse `BraidCoordinate`, which is already the shell-identity coordinate.
Later slices will add diffs and replay facts over the same event-log facts
instead of treating current membership as the substrate.

This preserves four boundaries:

1. Admission happens through `Braid::apply(...)`.
2. Membership history is derived from accepted events.
3. Current frontier is one projection, not the historical model.
4. Event-log membership cursors are distinct from braid shell coordinates.

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
