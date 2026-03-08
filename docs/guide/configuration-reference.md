<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Configuration Reference

This document describes how to configure the Echo engine at build time and
runtime. For the formal specification and hashing semantics, see
[spec-runtime-config.md](../spec-runtime-config.md).

## Engine Parameters (EngineBuilder)

The primary configuration surface is `EngineBuilder` in `warp-core`. All
parameters have sensible defaults; only override what you need.

| Parameter             | Type                     | Default                                                                            | Determinism Impact                                                                |
| --------------------- | ------------------------ | ---------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| `scheduler`           | `SchedulerKind`          | `Radix`                                                                            | None -- both variants produce identical results                                   |
| `policy_id`           | `u32`                    | `POLICY_ID_NO_POLICY_V0` (`b"NOP0"` as LE u32)                                     | **Critical** -- committed into `patch_digest` and `commit_id` v2                  |
| `worker_count`        | `usize`                  | `ECHO_WORKERS` env var, or `available_parallelism()`, capped at `NUM_SHARDS` (256) | None -- canonical merge order ensures identical output regardless of thread count |
| `telemetry`           | `Arc<dyn TelemetrySink>` | `NullTelemetrySink` (no-op)                                                        | None -- observability only                                                        |
| `materialization_bus` | `MaterializationBus`     | Fresh empty bus                                                                    | None -- channel registration only                                                 |

### SchedulerKind

| Variant  | Description                                                                                          |
| -------- | ---------------------------------------------------------------------------------------------------- |
| `Radix`  | Radix-based pending queue with O(n) drain and `GenSet` independence checks. Default and recommended. |
| `Legacy` | BTreeMap + Vec\<Footprint\> implementation. Kept for comparison benchmarks only.                     |

### Worker Count

Worker count controls how many threads are used for parallel shard execution.

- **Default:** `std::thread::available_parallelism()`, capped at `NUM_SHARDS` (256)
- **Override:** Set `ECHO_WORKERS=N` environment variable
- **Minimum:** Always at least 1 (values < 1 are clamped)
- **Serial mode:** `ECHO_WORKERS=1` forces single-threaded execution (useful for debugging)

Worker count does **not** affect determinism. The engine partitions work into
256 virtual shards and merges results in a canonical order, so the same inputs
produce the same outputs regardless of how many threads run concurrently.

### Policy ID

The policy ID is a 32-bit identifier committed into every tick's `patch_digest`
and `commit_id`. It makes the policy boundary explicit so that two engines
running different policy versions produce different hashes (intentional divergence).

Current value: `POLICY_ID_NO_POLICY_V0` = `b"NOP0"` interpreted as little-endian
u32. This placeholder will be replaced when Aion policy semantics land.

## Protocol Constants

These values are frozen and cannot be changed without a protocol version bump.

| Constant                 | Value        | Location                             | Notes                                                                 |
| ------------------------ | ------------ | ------------------------------------ | --------------------------------------------------------------------- |
| `NUM_SHARDS`             | 256          | `crates/warp-core/src/boaw/shard.rs` | Must be power of two. Routing formula: `LE_u64(node_id[0..8]) & 0xFF` |
| `POLICY_ID_NO_POLICY_V0` | `0x304F504E` | `crates/warp-core/src/constants.rs`  | Placeholder policy ID (`b"NOP0"` LE)                                  |

## Channel Policies (MaterializationBus)

Each materialization channel has a policy controlling how multiple emissions per
tick are resolved. All policies preserve confluence.

| Policy          | Behavior                                                      | Use Case                        |
| --------------- | ------------------------------------------------------------- | ------------------------------- |
| `Log` (default) | All emissions preserved in `EmitKey` order                    | Event streams, traces, logs     |
| `StrictSingle`  | Error if more than one emission per tick                      | Enforce single-writer semantics |
| `Reduce(op)`    | Merge emissions via a reduce operation (`Sum`, `First`, etc.) | Semantic coalescing             |

## Environment Variables

| Variable       | Purpose                              | Example                     |
| -------------- | ------------------------------------ | --------------------------- |
| `ECHO_WORKERS` | Override default worker thread count | `ECHO_WORKERS=8 cargo test` |

## Planned Configuration (spec-runtime-config.md)

The [runtime config spec](../spec-runtime-config.md) defines a planned
`echo.config.json` schema for project-level configuration. Key planned fields:

| Field               | Type                                      | Default     | Description                          |
| ------------------- | ----------------------------------------- | ----------- | ------------------------------------ |
| `mathMode`          | `"float32" \| "fixed32"`                  | `"float32"` | Scalar backend selection             |
| `chunkSize`         | `number`                                  | TBD         | ECS chunk size                       |
| `backpressureMode`  | `"throw" \| "dropOldest" \| "dropNewest"` | `"throw"`   | How to handle queue overflow         |
| `entropyWeights`    | `Record<string, number>`                  | `{}`        | Per-subsystem entropy metric weights |
| `inspector.enabled` | `boolean`                                 | `false`     | Enable inspector protocol            |
| `inspector.port`    | `number`                                  | TBD         | Inspector listen port                |

These are not yet implemented as runtime features. See the spec for the
planned load pipeline and canonical hashing.

## See Also

- [spec-runtime-config.md](../spec-runtime-config.md) -- formal specification
- [cargo-features.md](cargo-features.md) -- compile-time feature flags
- [start-here.md](start-here.md) -- getting started guide
