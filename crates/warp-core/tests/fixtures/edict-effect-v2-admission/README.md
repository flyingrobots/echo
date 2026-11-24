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

The packages are private evaluation witnesses. They do not establish
installation, a causal Tick, WAL retention or a receipt.
