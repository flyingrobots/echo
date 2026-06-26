<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Goalpost 3: Historical Membership And Replay

Status: implemented.

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

GP3-S1 established the membership-history source of truth. GP3-S2 through
GP3-S5 now layer historical cursors, diffs, replay facts, and recorder output
over that source without changing the admission path.

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
`Braid::diff_membership(...)` compares two cursor projections and returns
deterministic added and ended membership facts. The diff shape reserves
revealed and concealed fact slots, but the current append-only event model
does not infer sealed/revealed equivalence without explicit disclosure
evidence.
`audit_braid_shell(...)` validates the same retained shell and lineage
constraints as replay, then emits stable replay/audit facts for member
verdicts, member support and frontier digests, posture floor, proof binding,
and the current self-witness integrity-only posture.

The Braid Flight Recorder and Causal X-Ray lower-mode target are defined below
as stable design/output surfaces over these fact APIs. This goalpost does not
ship a CLI command.

This preserves six boundaries:

1. Admission happens through `Braid::apply(...)`.
2. Membership history is derived from accepted events.
3. Current frontier is one projection, not the historical model.
4. Event-log membership cursors are distinct from braid shell coordinates.
5. Reveal/conceal facts require explicit evidence, not reference-shape guesses.
6. Audit facts do not reopen member strand histories.

## Flight Recorder And Causal X-Ray Output

The Braid Flight Recorder is a durable audit artifact shape, not a separate
admission path. It records the interpreted path:

```text
event log
-> membership projection
-> membership diff
-> shell assembly
-> proof binding
-> witness reading
-> replay verdict
```

The recorder consumes existing fact surfaces:

| Stage                 | Source API                                           |
| --------------------- | ---------------------------------------------------- |
| event log             | `Braid::events()`                                    |
| membership projection | `Braid::membership_at(...)`                          |
| membership diff       | `Braid::diff_membership(...)`                        |
| shell assembly        | retained `BraidShell`                                |
| proof binding         | `BraidShellAudit::proof_binding`                     |
| witness reading       | `BraidShellAudit::witness_posture`                   |
| replay verdict        | `replay_braid_shell(...)` / `audit_braid_shell(...)` |

The lower-mode Causal X-Ray target is a stable, assertion-friendly object that
can later back a command such as:

```text
echo braid inspect <shell-digest>
```

No command ships in this slice. The current target output is:

```json
{
    "artifact": "braid-flight-recorder",
    "version": 1,
    "braid": {
        "id": "hex:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "status": "active",
        "membership_cursor": {
            "next_sequence_num": 3
        }
    },
    "membership": {
        "projection": [
            {
                "reference": "revealed",
                "sequence_num": 0
            },
            {
                "reference": "revealed",
                "sequence_num": 1
            },
            {
                "reference": "revealed",
                "sequence_num": 2
            }
        ],
        "diff": {
            "from_next_sequence_num": 1,
            "to_next_sequence_num": 3,
            "added": [1, 2],
            "ended": [],
            "revealed": [],
            "concealed": []
        }
    },
    "shell": {
        "digest": "hex:bsh",
        "coordinate": "hex:bc",
        "outcome": "plural",
        "posture_floor": "author_only",
        "shell_posture": "author_only",
        "settlement_frontier": ["revealed:0", "revealed:1", "revealed:2"]
    },
    "members": [
        {
            "reference": "revealed",
            "verdict": "plural",
            "support_pin_digest": "hex:21",
            "frontier_digest": "hex:23"
        }
    ],
    "proof": {
        "binding": "matched",
        "kind": "replay_trace"
    },
    "witness": {
        "kind": "self_witness",
        "attestation": "integrity_only"
    },
    "warnings": ["self_witness_is_not_independent_attestation"]
}
```

Fixture fields use symbolic digest strings because this is lower-mode output,
not a golden identity vector. Golden identity changes remain governed by
Goalpost 2 vector rules.

## Non-Goals

This goalpost does not include:

- settlement-as-merge semantics;
- exposing sealed source chains beyond the requested aperture;
- external witness backend implementation;
- plurality law registry execution;
- shipping a Causal X-Ray CLI command.

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
