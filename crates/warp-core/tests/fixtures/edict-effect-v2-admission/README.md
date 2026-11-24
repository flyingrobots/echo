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

The four `payload-*-record-*` and `payload-loop-*` packages were written the
same way by Amblicit `615d2e59170c4ed82d2c294302dff2a4c24d2440` (main after
amblicit#349) from their `-source/` directories: the one-field payload as
`{ at: { value: input.start } }` (schema `Record<at:Record<value:U64>>`) on
the `attempt` arm and on the `require`, and a `require item <= input.end`
inside `for item in input.items bounded 3` with the scalar payload
`{ at: item }` and the record payload `{ at: { value: item } }`. Under
615d2e59 the eight packages above are byte-identical and their minimums
unchanged. The compiler reports attempt-record (151, 2621), require-record
(137, 2557), loop-scalar (306, 3584) and loop-record (324, 4574). Echo's
lowerer and verifier agree on the first two and measure (300, 3200) and
(318, 4190) on the loops.

Amblicit `e306ecc9ed22c0eb166351bb9d766150dca617f2` (main after amblicit#350)
no longer charges a loop iteration the binder copy bytes or the loop body's
`Null` result, which the shared runtime never charges. Under it all twelve
packages are byte-identical, the first ten minimums are unchanged, and the
loops are (300, 3200) and (318, 4190), equal to Echo.
`effect_programs_are_admitted_at_exactly_the_compiler_minimum` pins all twelve
to that compiler.

Four more loop packages were written the same way by Amblicit `e306ecc9` from
their `-source/` directories, with the typed-failure configuration:
`payload-loop-nested-*` (a 3 by 3 nested loop of `require item <= input.end`,
payload `{ at: row }`), `payload-loop-entry-*` (items of a two-field record
`{ value: U64, limit: U64 }`, `require item.value <= item.limit`),
`payload-loop-bytes-*` (items `Bytes<max=8>`, `require len(item) <= input.end`)
and `payload-loop-mixed-*` (a body of a `let`, a `require` and an
`ensureNode`). The compiler reports (783, 5144), (336, 3998), (1134, 3608)
and (339, 4232); Echo's lowerer and verifier measure the same, and the
lowerer admits each at exactly that budget and refuses it one unit below.
`the_loop_programs_run_within_the_compiler_minimum_on_every_path` in
warp-core runs all six loop programs at that budget under shared reads, with
every item passing and with the first, second or third item failing: the
result and mutations equal the copying run's and the charge stays within the
bound on both axes.

`payload-loop-attempt-source/` holds an `attempt` of the failing helper
inside a loop bounded 3. The compiler reports (342, 3392) for it, but Echo's
checked lowerer refuses the program (`effect-program.failure-sets`):
Echo's failure-set judge (`failure_sets.rs`, identical in warp-core, the
lowerer and the verifier) accepts an `attempt` only as a top-level node of an
intent body, so an `attempt` inside a loop is outside the checked profile.
No package is retained and nothing is pinned for it.

The packages are private evaluation witnesses. They do not establish
installation, a causal Tick, WAL retention or a receipt.
