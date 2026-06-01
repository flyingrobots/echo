<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Expose parallel execution counterfactuals

Status: active cool idea. `warp-core` has shard-based parallel execution,
per-worker/per-shard `TickDelta`s, canonical merge, poisoned-delta handling,
and tick receipts for accepted/rejected candidates, but no public artifact that
preserves shard-level intermediate deltas as debugger-inspectable
counterfactuals.

Echo's parallel executor runs rewrites on independent shards and then
performs a canonical merge. Today those per-worker or per-shard deltas are an
internal execution artifact: identical ops can be deduped, divergent writes
become merge errors, and the debugger does not get a first-class "what did this
shard produce before merge?" view.

Echo could expose shard-level intermediate results as debugger-inspectable
counterfactuals — "what would this shard's result have contributed if inspected
before canonical merge?"

This is unique to Echo — git-warp doesn't have parallel execution.
It would give the debugger a view into _why_ a merge produced the
result it did, not just _what_ the result is.

Requires coordination with TTD/provenance consumers on how to surface
shard-level execution evidence through the protocol.
