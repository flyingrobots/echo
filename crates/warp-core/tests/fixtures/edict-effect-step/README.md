<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Public compiler read-effect program under `echo.dpo@2`

`source/` holds authored Amblicit input only: a lawpack whose adapter accepts
the `echo.dpo@2` effect target profile and binds its `readFact` effect to
`echo.dpo@2.node-atom-read`, and one program, `ReadFact.edict`, that reads one
atom and returns it, mapping each read failure to a declared obstruction. The
coordinates under `consumer.effects` are neutral; Echo interprets none of them.
The configuration carries the `echo.dpo.intrinsics/v2` catalog the profile
pins, a graph schema with one node type (`0x5b…`) holding one atom type
(`0x5c…`), and a footprint law that reads that node type and writes nothing.

The hexadecimal files are unchanged canonical CBOR written by the public
compiler, Amblicit `0999db831422f97bd013d3d89a38fbb6f9e8ce96`, and reproduced
unchanged by `0034605467fcace397e7e30e4219a74aca57f183`: the package and
verifier report it wrote through the checked provider package in
`schemas/edict-provider/package/v1`, and the lawpack manifest, adapter and
configuration its lawpack build wrote. Nothing here was decoded and
re-encoded, and no Core or Target IR was authored by hand. `ReadFact.edict`
carries the digest of the lawpack the same compiler built from
`source/edict.lawpack.json`.

`scripts/verify-edict-provider-host-v1.sh` rebuilds these bytes in CI with
`xtask --example effect_step_publication --name read --check`. It builds the lawpack,
compiles the program twice, and requires the output to equal these files. It
then compiles the same program under `echo.dpo@1` (which names `echo.span-ir/v1`
and whose catalog declares none of the runtime intrinsics) and under a
substituted `echo.dpo@2` profile, and requires Echo's lowerer to refuse each.
The lowerer reports every judgment refusal with one message, so these controls
show that each case is refused, not which check refuses it. The native tests
in `crates/echo-edict-provider-verifier/tests/public_read_effect.rs` pin that
the `echo.dpo@1` profile refuses the `echo.dpo.intrinsics/v2` catalog this
program's configuration carries. It also compiles
`controls/zero-failure-read/ReadFact.edict`, which calls the read without a
failure map, against the same lawpack with every failure removed from its
exports and adapter; the compiler accepts that claim, and Echo's lowerer must
refuse it because `node-atom-read` pins four failures. The shared verdict
corpus carries two synthetic relatives, which the lowerer, the verifier and the
runtime each refuse: `read-without-failures` (exports and adapter without
failures, catalog and lowered arms with four) and
`read-catalog-without-failures` (every failure set empty, so only Echo's fixed
failure table for `node-atom-read` disagrees).

| File | Domain-framed digest |
| --- | --- |
| `read-executable-operation-package.hex` | `echo:operation-package:v1` `7a3955e1d08a5d16444c6374cb744fa073da7e197ea98ec6806a1158f745871b` |

`crates/warp-core/tests/executable_operation_pipeline/public_read_effect.rs`
fixes that pin independently of the bytes, admits the package under the
runtime owner's exact policy, installs it, runs it over a graph that holds the
fact and over one that does not, and refuses another package under the same
policy. `crates/echo-edict-provider-verifier/tests/public_read_effect.rs`
rebuilds the lowering request from these files and requires the native lowerer
to reproduce the package and the separate verifier to accept it. The compiler
does not publish its result projection, so that request carries a stand-in,
which the effect route binds by digest but never reads.

This is admission, installation and private execution evidence. It does not
establish an invocation Action, a causal Tick, WAL retention or a receipt.
