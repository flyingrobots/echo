<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Public compiler mutation program under `echo.dpo@2`

`source/` holds authored Amblicit input only: a lawpack whose adapter accepts
the `echo.dpo@2` effect target profile and binds five effects, each with an
empty failure set and no failure mappings, to `echo.dpo@2.ensure-node`,
`create-node`, `replace-atom`, `ensure-edge` and `replace-edge`; and one
program, `WriteGraph.edict`, that calls them in that order without a failure
map (flyingrobots/amblicit#311). The coordinates under `consumer.writes` are
neutral; Echo interprets none of them. The configuration carries the
`echo.dpo.intrinsics/v2` catalog the profile pins, a graph schema with one node
type (`0x5b…`) holding one atom type (`0x5c…`) and one edge type (`0x5d…`)
between such nodes, and a footprint law that reads and writes them.

The hexadecimal files are unchanged canonical CBOR written by the public
compiler, Amblicit `0034605467fcace397e7e30e4219a74aca57f183`: the package and
verifier report it wrote through the checked provider package in
`schemas/edict-provider/package/v1`, and the lawpack manifest, adapter and
configuration its lawpack build wrote. Nothing here was decoded and
re-encoded, and no Core or Target IR was authored by hand. `WriteGraph.edict`
carries the digest of the lawpack the same compiler built from
`source/edict.lawpack.json`. Each lowered mutation step carries present, empty
`obstructionArms` and `obstructionFailures`, and each Core effect node an
empty `obstructionMap`.

`scripts/verify-edict-provider-host-v1.sh` rebuilds these bytes in CI with
`xtask --example effect_step_publication --name write --check`, which also
requires Echo's lowerer to refuse the same program under `echo.dpo@1` and under
a substituted `echo.dpo@2` profile.

| File | Domain-framed digest |
| --- | --- |
| `write-executable-operation-package.hex` | `echo:operation-package:v1` `06f14e50c1b46f71f2b413b4262ac9622877ac0e6895cca00a36b33c45daa55f` |

`crates/warp-core/tests/executable_operation_pipeline/public_mutation_effect.rs`
fixes that pin independently of the bytes, admits the package under the
runtime owner's exact policy, installs it, runs it, and requires the five
mutation intents in source order.
`crates/echo-edict-provider-verifier/tests/public_mutation_effect.rs` rebuilds
the lowering request from these files and requires the native lowerer to
reproduce the package and the separate verifier to accept it.

This is admission, installation and private execution evidence. It does not
derive or apply a patch, or establish an invocation Action, a causal Tick, WAL
retention or a receipt.
