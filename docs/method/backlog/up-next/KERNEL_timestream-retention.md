<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# TimeStream retention + spool compaction + wormhole density

Ref: #244

Define the storage lifecycle for time streams:

- Retention policy: how long do old ticks stay materialized?
- Spool compaction: when and how to compact the provenance log?
- Wormhole density: how often to create checkpoints for fast seek?

`ProvenanceStore::checkpoint_before()` exists as a trait method but
checkpoint creation and compaction are not implemented.

Affects warp-ttd — retention policy determines what the debugger
can replay. If old ticks are compacted away, the debugger can't
seek to them.
