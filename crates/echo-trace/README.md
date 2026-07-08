<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# echo-trace

Canonical causal trace boundary for Echo graph delta streams.

`echo-trace` defines the small trait and data shapes used by trace sinks that
consume ordered graph deltas, seal deterministic chunks, and return receipts.
The crate is intentionally transport-neutral: it names the boundary between a
causal producer and a trace sink without owning storage, replay, or rendering.
