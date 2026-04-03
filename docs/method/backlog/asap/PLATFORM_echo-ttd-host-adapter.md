<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo host adapter for warp-ttd

Implement `TtdHostAdapter` for Echo's WASM ABI so warp-ttd can
debug Echo substrates.

The git-warp adapter in warp-ttd is the reference pattern. Echo's
adapter would wrap the WASM exports (`init`, `observe`,
`dispatch_intent`, `scheduler_status`) and map them to warp-ttd's
protocol envelopes (`PlaybackFrame`, `ReceiptSummary`, etc.).

Key mapping work:

- Frame indexing: map Echo's Lamport ticks to warp-ttd frame indices
- Receipt mapping: map ProvenanceStore entries to ReceiptSummary
- Channel emissions: map FinalizedChannel to EffectEmissionSummary
- Playback control: map PlaybackCursor modes to step/seek commands

This adapter likely lives in warp-ttd (not Echo), but Echo needs to
ensure its WASM ABI surface is sufficient. Coordinate with warp-ttd
backlog item for the adapter itself.
