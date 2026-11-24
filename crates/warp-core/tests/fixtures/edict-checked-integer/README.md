<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Checked-integer publication fixture

`source/CheckedInteger.edict` declares `apply(op, a, b)`, which selects one of
`checkedAdd`, `checkedSub`, `checkedMul`, `checkedDiv`, `checkedRem`, `min` or
`max` on two `U64` inputs, and `boundsU8`/`boundsU32`, which return 0 when
every checked form and `min`/`max` behaves as specified at the given width
value and otherwise the first failing check. The intent returns
`run(op, start, end, small, mid)`: the `U64` result, or 42 for an absent one,
for `op` 0 to 6; `boundsU8(small)` for 7; and `boundsU32(mid)` for 8. The
program selects `echo.dpo@1` and shares the `edict-effect-program` lawpack.

The `effect_program_publication` example accepts this directory with
`--name checked`. The five retained `checked-*.hex` outputs were written by that
producer with an unmodified Amblicit `7dbcbc23d72705c5e224fc994a6c767881958912`
binary installed fresh from that public revision (binary SHA-256
`861f36e2f941f09b76d2fc78b3b6db5b9785a465890a20e65f79a823c75fb1fc`), which
contains the checked-integer compiler half (amblicit#321). They are not
authored Target IR. The producer also requires the provider before
checked-integer support (`9141e795`) to refuse the same source at Echo's
lowerer. `scripts/verify-edict-provider-host-v1.sh` reruns it with `--check`.

The runtime witness
`crates/warp-core/tests/executable_operation_pipeline/public_checked_integer.rs`
pins the package identity and checks every `U64` form at the type bounds and
the `U8` and `U32` bound checks at each width's maximum. Forged Core is covered
by the shared checked-integer corpus in all three judgments.
