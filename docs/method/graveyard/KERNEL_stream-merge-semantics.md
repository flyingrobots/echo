<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Superseded merge semantics note

Folded from: #245

This old issue asked how debugger-era per-source admission records combine when
two forked worldlines merge. The old noun framing is obsolete. The remaining
valid concern is folded into
`docs/method/backlog/up-next/KERNEL_contract-strands-and-counterfactuals.md`:
settlement is a generic worldline/strand/braid admission law with typed
admitted, staged, plural, conflict, or obstructed outcomes.

git-warp uses CRDT convergence (OR-Set + LWW). Echo needs canonical
merge — one deterministic result, not eventual convergence. The
answer here shapes everything downstream: strand braiding, time
travel, and the debugger's counterfactual inspection.

Coordinate with warp-ttd — merge operations will be exercised
through the debugger's fork-merge workflow.
