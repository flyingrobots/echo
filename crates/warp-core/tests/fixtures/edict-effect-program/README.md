<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Public compiler flat effect program

`source/` holds authored Amblicit input only: a lawpack whose adapter selects
the `edict-effect-program/v1` configuration for the
`continuum.profile.graph-effects/v1` profile, and one flat program,
`FlatEffect.edict`, that binds one pure value and returns it. The coordinate
`consumer.effect` is neutral; Echo interprets none of it.

The configuration carries the exact intrinsic catalog that the checked
`echo.dpo@1` target profile pins, an empty graph schema and an empty footprint
law. The program invokes no effect intrinsic. That catalog declares only
`echo.dpo@1.replace`, and the checked profile names `echo.span-ir/v1` as its
Target IR, so an effect node cannot pass Echo's effect-program judgment through
the checked profile. Effect-bearing public programs need target resource
declarations that are not part of this fixture.

The hexadecimal files are unchanged canonical CBOR written by the public
compiler, Amblicit `66bf6bac369f67e32ddaee099c3198bd30f885a1`: the package and
verifier report it wrote through the checked provider package in
`schemas/edict-provider/package/v1`, and the lawpack manifest, adapter and
configuration its lawpack build wrote. Nothing here was decoded and
re-encoded, and no Core or Target IR was authored by hand. The program carries
the digest of the lawpack the same compiler built from
`source/edict.lawpack.json`.

`scripts/verify-edict-provider-host-v1.sh` rebuilds these bytes in CI with
`xtask --example effect_program_publication --check`. It builds the lawpack,
compiles the program twice, and requires the output to equal these files and
the predecessor provider package to refuse the program at its lowerer.

| File | Domain-framed digest |
| --- | --- |
| `flat-executable-operation-package.hex` | `echo:operation-package:v1` `2ac246d890ee7c965878a43f4da21166f0a2dab81c677f2fe29c5e84ce5a47e8` |

`crates/warp-core/tests/executable_operation_pipeline/public_flat_effect.rs`
fixes that pin independently of the bytes, admits the package under the runtime
owner's exact policy, installs it, runs the installed program, and refuses
another package or a re-bound one under the same policy. The provider tests in
`crates/echo-edict-provider-verifier/tests/public_effect_program.rs` rebuild
the lowering request from these files and require the native lowerer to
reproduce the package and the separate verifier to accept it and reject
substituted or re-bound packages. The compiler does not publish its result
projection, so that request carries a stand-in, which the effect route binds by
digest but never reads.

This is admission, installation and private execution evidence. It does not
establish an invocation Action, a causal Tick, WAL retention or a receipt.
