<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# CI-003 — Append-Only Braid Membership

Legend: [WARP — Causal History]

## Idea

Model braids as append-only witnessed relationships over strand intervals,
not as binary pairings and not as permanent strand merges.

A braid can begin with multiple members and later weave additional strands
into the relation without pretending those strands were present from the
beginning:

```text
t0: braid B includes s0 and s1
t1: braid B weaves in s2
```

The source of truth should be a braid event log:

```text
BraidCreated { members: [s0, s1], ... }
BraidMemberWovenIn { member: s2, ... }
```

Materialized braid views can report current membership, but historical views
must preserve membership as of the requested coordinate.

## Why

1. **Doctrine:** Braided does not mean settled. Related does not mean admitted.
2. **Causality:** Weaving in `s2` at `t1` must not rewrite `t0` membership.
3. **Scale:** Real review/conflict/proposal workflows can involve more than
   two strands.
4. **Posture:** A braid may reveal a shared projection or relationship summary
   while sealed member source chains remain AuthorOnly.

## Acceptance Sketch

- Create braid `B` with `s0` and `s1` at `t0`.
- Weave `s2` into `B` at `t1`.
- Current braid view after `t1` includes `s0`, `s1`, and `s2`.
- Historical braid view before `t1` excludes `s2`.
- Braid membership changes are append-only events, not mutable list rewrites.
- A shared braid projection can reveal the relationship without revealing a
  sealed member source chain.
- Settlement can admit a braid projection without collapsing member strands.
- Weaving in an AuthorOnly member requires authority or records a sealed
  member reference.

## Effort

Medium-Large — requires braid event types, interval/member views, revelation
policy around sealed members, and settlement/projection integration.
