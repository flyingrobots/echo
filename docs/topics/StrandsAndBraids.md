<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Strands and Braids

Strands and braids describe causal plurality without replacing history with a
mutable graph snapshot.

## Strands

A strand is an identified causal lane with an explicit fork basis. Its identity
binds to causal ancestry and law, not a display label or collection position.
Appending to a strand creates new witnessed history; it does not mutate the
fork basis.

Privacy and revelation posture are evidence-bearing properties of how a strand
may be observed or promoted. They are not UI visibility flags.

## Braids

A braid records historical membership and plurality among strands. Membership
history is append-only. Current projections may summarize that history, but
they cannot erase joins, departures, settlements, conflicts, or sealed
evidence.

Settlement is a named lawful act over explicit bases and participants. It may
produce supported outcomes, plurality, conflict residue, or obstruction.
Plurality is not an automatic merge failure; the governing law determines
whether multiple outcomes remain lawful.

## Shells, Proofs, and Replay

Retained shells preserve enough boundary evidence to explain identity,
membership, capability, settlement, and revelation without granting mutation
authority. Replay applies admitted facts and retained evidence; it does not
rerun application callbacks to rediscover history.

## Normative Boundary

`docs/invariants/STRAND-CONTRACT.md` remains the hard invariant for strand
construction and identity. Runtime types, receipts, and tests in `warp-core`
are executable truth when prose and implementation diverge.
