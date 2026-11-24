<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Effect programs for exact admission under the shared-read schedule

Each file is the checked `executable-operation-package.cbor` that the public
compiler, Amblicit `65dc53624856f58d21404d65d5256779ce403d49`, wrote for the
unchanged authored source of an existing effect fixture, through the checked
provider package in `schemas/edict-provider/package/v1`, written as hexadecimal
by `xtask --example effect_v2_admission` (echo#1016):

| file | source |
| --- | --- |
| `read-executable-operation-package.hex` | `edict-effect-step/source` (node-atom read) |
| `link-executable-operation-package.hex` | `edict-effect-graph-read/source` (outgoing-edge read and `ensure-edge`) |
| `write-executable-operation-package.hex` | `edict-effect-mutations/source` (five mutation intrinsics) |
| `typed-executable-operation-package.hex` | `edict-effect-typed-failure/source` (an `attempt` of a failing helper, then `ensure-node`) |

Nothing here was decoded and re-encoded. They are new files: the packages
retained beside each source were written by earlier compilers and stay
unchanged.

`effect_programs_are_admitted_at_exactly_the_compiler_minimum` in
`crates/echo-edict-provider-lowerer/tests/effect_program_judgment.rs` gives each
package the shared-read capability and the compiler's own shared-read minimum
under Echo's `/v3` effect templates, (100, 1631), (244, 8240) and (176, 6796)
(steps, bytes), and requires Echo to admit it there, to refuse it one unit
below on either axis, and to measure the same minimum. They use the
configuration retained beside each source.

`an_attempt_is_charged_what_the_shared_runtime_charges` in the same file
measures Echo's shared-read minimum for the typed-failure package, (140, 2095).
Amblicit `65dc5362` reports (137, 1967) for it: that compiler charges an
`attempt` arm nothing, while warp-core's shared executor charges an
empty-payload arm (3, 128). Exact admission of this package waits for the
compiler change (amblicit#341).

`payload-attempt-executable-operation-package.hex` and
`payload-require-executable-operation-package.hex` were written the same way
by Amblicit `66f6ca4d9513ada331895a94facae4922f5dc5e1` from the sources in
`payload-attempt-source/` and `payload-require-source/`: the typed-failure
program whose `RangeInverted` payload schema is `Record<at:U64>`, with the
failure payload `{ at: input.start }` on its `attempt` arm, or on a
`require input.start <= input.end` in place of the `attempt`. They use the
typed-failure configuration. `a_failure_payload_field_is_charged_what_the_shared_runtime_charges`
pins Echo's shared-read minimums for them, (145, 2291) and (131, 2227),
which are what warp-core's shared executor charges for the payload, term by
term, as the test's constants explain.

`payload-attempt-two-executable-operation-package.hex` and
`payload-require-empty-executable-operation-package.hex` were written the same
way by Amblicit `80b52f956197fb94aeb2776abb844e482d3591e1` (main after
amblicit#348) from `payload-attempt-two-source/` (payload schema
`Record<at:U64,until:U64>`, arm payload `{ at: input.start, until: input.end }`)
and `payload-require-empty-source/` (`require input.start <= input.end else
fx.RangeInverted`, payload schema `Record<>`), with the typed-failure
configuration.

Under Amblicit `80b52f95` with Echo's `/v3` templates the compiler reports
these shared-read minimums, and refuses one unit below on either axis:
read (100, 1631), link (244, 8240), write (176, 6796), typed (140, 2095),
payload-attempt (145, 2291), payload-require (131, 2227), payload-attempt-two
(150, 2493), payload-require-empty (126, 2031). Its checked packages for the
first six are byte-identical to the files here.
`effect_programs_are_admitted_at_exactly_the_compiler_minimum` pins all eight;
`two_field_and_empty_failure_payload_minimums_equal_the_compiler_minimum` in
the verifier's `effect_program_judgment.rs` pins the verifier's minimums for
the last two.

The packages are private evaluation witnesses. They do not establish
installation, a causal Tick, WAL retention or a receipt.
