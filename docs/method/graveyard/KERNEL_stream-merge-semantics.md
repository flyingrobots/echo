<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Merge semantics for admitted stream facts across worldlines

Ref: #245

When two forked worldlines are merged, how do admitted stream facts
(channel emissions, provenance entries) combine? This is the hardest
design question in Echo's strand/braiding story.

git-warp uses CRDT convergence (OR-Set + LWW). Echo needs canonical
merge — one deterministic result, not eventual convergence. The
answer here shapes everything downstream: strand braiding, time
travel, and the debugger's counterfactual inspection.

Coordinate with warp-ttd — merge operations will be exercised
through the debugger's fork-merge workflow.
