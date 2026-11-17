<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Option-value publication fixture

`source/OptionValue.edict` declares a helper `above(v, floor)` that returns
`some(v)` when `v > floor` and `none()` otherwise, and a helper `pick` that
tests the result with `isSome`, unwraps it in the refined `then` arm and
otherwise takes `default(o, 0u64)`. The intent returns
`pick(input.end, input.start)`. The program selects `echo.dpo@1` and shares the
`edict-effect-program` lawpack, as the function-precondition fixture does.

The compiled outputs are not retained yet. They are produced by the
`effect_program_publication` example with `--name option`, using an Amblicit
compiler that contains #316, after the provider components that judge
`core.option/v1` are promoted (flyingrobots/echo#995). Forged Option artifacts
are covered by the shared Option corpus in all three judgments.
