<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Option-value publication fixture

`source/OptionValue.edict` declares a helper `above(v, floor)` that returns
`some(v)` when `v > floor` and `none()` otherwise, and a helper `pick` that
tests the result with `isSome`, unwraps it in the refined `then` arm and
otherwise takes `default(o, 0u64)`. The intent returns
`pick(input.end, input.start)`. The program selects `echo.dpo@1` and shares the
`edict-effect-program` lawpack, as the function-precondition fixture does.

The `effect_program_publication` example accepts this directory with
`--name option`. It builds the lawpack, pins the import digest, and builds the
application twice through the checked provider package. The five retained
`option-*.hex` outputs were written by that producer with an unmodified Amblicit
`7dbcbc23d72705c5e224fc994a6c767881958912` binary installed from a clean
checkout of that public revision (binary SHA-256
`40fbef6fdfaf32d4b99831ee51ff63c35ce561a25e1240f8aa6dcef129f76518`), which
contains the Option compiler half (amblicit#316). They are not authored Target
IR. The producer also requires the predecessor provider at `d527f47` (before
Option support) to refuse the same source at Echo's lowerer.
`scripts/verify-edict-provider-host-v1.sh` reruns the producer with `--check`.

The runtime witness
`crates/warp-core/tests/executable_operation_pipeline/public_option_value.rs`
pins the package identity and checks results against the authored law. Forged
Option Core is covered by the shared Option corpus in all three judgments.
