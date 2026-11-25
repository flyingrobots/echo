<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Typed-failure effect publication fixture

These authored inputs select the current `echo.dpo@2` profile. The lawpack
exports `ensureNode`, `createNode`, their types and one obstruction,
`RangeInverted`. The program declares `failure Underflow`, a helper
`distance(start, end)` that returns `end - start` when `start <= end` and
otherwise fails `Underflow`, and an intent that maps that failure with
`else fx.RangeInverted` before it ensures one node.

The `effect_step_publication` example accepts this directory with the output
name `typed`. It builds the lawpack, replaces the source import's digest with
the actual manifest digest, and builds the application twice. The retained
`typed-*.hex` outputs were written by that producer with a freshly built
Amblicit `2b09e4ab1ecf0041b4ff251693dc295ac2b649c2`; they are not authored
Core or Target IR. `scripts/verify-edict-provider-host-v1.sh` reruns the
producer with `--check` and requires the same bytes. The producer's controls
require the same source to be refused under `echo.dpo@1` and under a
substituted `echo.dpo@2` profile.

| File | Domain-framed digest |
| --- | --- |
| `typed-executable-operation-package.hex` | `echo:operation-package:v1` `0592572805487095102328d74b969763a1c5dad080c746d2ec96a0f8666d85b9` |

The runtime witness
`crates/warp-core/tests/executable_operation_pipeline/public_typed_failure_effect.rs`
admits and installs the package. With `start <= end` it buffers exactly the
one `EnsureNode`. With `start > end` it obstructs with exactly `fx.RangeInverted`
(the reason as Core names it) and an empty payload, and buffers nothing. No
graph commit, causal receipt or WAL publication is claimed by this fixture.
