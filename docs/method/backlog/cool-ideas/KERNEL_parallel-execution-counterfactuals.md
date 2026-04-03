<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Expose parallel execution counterfactuals

Echo's parallel executor runs rewrites on independent shards and then
performs a canonical merge. During merge, some shard-local results may
be superseded. These intermediate results are counterfactuals — "what
would have happened if this shard's result stood alone."

warp-ttd already has first-class counterfactual inspection (rejected
receipts are inspectable, not discarded). Echo could expose shard-level
intermediate results through the same mechanism.

This is unique to Echo — git-warp doesn't have parallel execution.
It would give the debugger a view into _why_ a merge produced the
result it did, not just _what_ the result is.

Requires coordination with warp-ttd on how to surface shard-level
provenance through the protocol.
