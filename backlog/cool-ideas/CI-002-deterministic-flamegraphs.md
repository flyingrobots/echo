<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# CI-002 — Deterministic Rule Profiling (Flamegraphs)

Legend: [DX — Developer Experience]

## Idea

Traditional profiling is non-deterministic. Echo has the unique capability to know exactly which rule touched which graph region.

Integrate a deterministic profiler into the scheduler that records canonical rule cost units per emission rather than wall-clock time or host CPU cycles. Export this data as a "Causal Flamegraph" where the Y-axis is the rule dependency stack and the X-axis is the deterministic cost.

## Why

1. **Performance Debugging**: Allows builders to find "heavy rules" without wall-clock noise.
2. **Reproducibility**: Profiling results are identical across runs, making optimization verification a science.
3. **Auditability**: Provides a machine-readable "cost receipt" for every tick.

## Effort

Medium-Large — requires canonical cost instrumentation in the scheduler and a data-export adapter.
