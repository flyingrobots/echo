<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Shared-read Core cost corpus (`edict.core-cost/v2`)

This corpus is shared by Echo's three cost judges: the `warp-core` runtime
meter, the provider lowerer's static cost and the provider verifier's charge
check. Each judge reads the same Core bytes, so all three are measured
against one compiler minimum.

The producer `tests/edict-core-cost-v2-corpus` compiles each `.amblicit`
source with Amblicit `da1574f169cda98206440e4d8f785b1ea9e01fcf` under
`CoreCostSchedule::SharedReads`. It searches for the smallest step, allocation
and output budgets the compiler accepts, and requires native Target's step
and allocation minimums to be equal. It writes:

- `<name>.core.cbor.hex`: the canonical Core module compiled at exactly those
  minimums; it requires `core.cost.shared-reads/v1`;
- `minimums.txt`: the program's intent and its three minimum budgets.

`scripts/verify-edict-provider-host-v1.sh` reruns the producer with `--check`
and requires the same bytes. Regenerate the corpus only with the producer:

```sh
cargo +1.96.0 run --release --locked \
  --manifest-path tests/edict-core-cost-v2-corpus/Cargo.toml \
  -- crates/warp-core/tests/fixtures/edict-core-cost-v2
```

## Sources

The sources are the adversarial programs from Amblicit's
`crates/amblicit-syntax/tests/core_cost_schedules.rs` (amblicit#324, PR #325),
and the bounded-descent rope `fixtures/lang/bounded-descent/avl-rope.amblicit`,
copied at `da1574f1`. Echo's judges do not yet support two of the constructs
they use, so the Echo copies differ as follows:

| Program | Change from the Amblicit source | Reason |
| --- | --- | --- |
| `reads` | `s.cursor < 32u64` becomes `s.cursor <= 31u64` | Echo's unsigned-addition proof reads `<=` facts only |
| `fill` | `s.i < 32u64` becomes `s.i <= 31u64`; the leading `if s.acc == s.source then s` is removed | As above; Echo compares only Bool, unsigned and byte values, not lists |
| `nested` | `s.n < 1000u64` and `s.rounds < 100u64` become `<= 999u64` and `<= 99u64` | As above |
| `matches` | None | |
| `rope` | None | |

`integers` is Echo's own program (echo#1016). It exercises every checked
integer call, `min` and `max` in a fold, and the `Option` consumers `isSome`
and `unwrap`. Its output is total because Echo's intent output has no
`Option` encoding.

The rope's intent maps a typed failure with `attempt` (`let … else { … }`).
Echo admits `attempt` only on the effect-program route, which has no
shared-read rules, so no Echo judge reads `rope.core.cbor.hex` yet. Its
minimums are recorded so that the rope can join the corpus without
regenerating it.
