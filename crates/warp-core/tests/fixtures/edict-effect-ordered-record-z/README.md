<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Ordered-record effect publication fixture (`z` first)

These authored inputs select the current `echo.dpo@2` profile. The lawpack
exports `ensureNode`, `createNode`, their types and two obstructions,
`FirstRaised` and `SecondRaised`. The program declares `failure First` and
`failure Second`, a helper `pair()` that returns `{ z: fail First, a: fail Second }`, and an intent that
maps `First` to `fx.FirstRaised` and `Second` to `fx.SecondRaised` before
it ensures one node. Because a field can fail, Amblicit emits the record as an
`orderedRecord` that keeps the authored field order. The sibling fixture
`edict-effect-ordered-record-a` authors the same fields in the other order.

The `effect_step_publication` example accepts this directory with the output
name `ordered-z`. The retained `ordered-z-*.hex` outputs were written by that
producer with a freshly built Amblicit
`7dbcbc23d72705c5e224fc994a6c767881958912`; they are not authored Core or
Target IR. `scripts/verify-edict-provider-host-v1.sh` reruns the producer with
`--check` and requires the same bytes.

The runtime witness
`crates/warp-core/tests/executable_operation_pipeline/public_ordered_record_effect.rs`
checks that the compiled helper result is an `orderedRecord` whose first entry
is `z`, admits and installs the package, and requires the run to obstruct
with exactly `fx.FirstRaised` (the first authored field fails `First`) and an empty
payload, buffering nothing. No graph commit, causal receipt or WAL publication
is claimed by this fixture.
