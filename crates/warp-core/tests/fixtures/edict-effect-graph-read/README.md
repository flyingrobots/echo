<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Public compiler graph-read effect program under `echo.dpo@2`

`source/` holds authored Amblicit input only: a lawpack whose adapter accepts
the `echo.dpo@2` effect target profile and binds `fx.outgoing` to
`echo.dpo@2.outgoing-edge-read`, with its three failures mapped to authored
obstructions, and `fx.ensureEdge` to `echo.dpo@2.ensure-edge`; and one program,
`FollowLink.edict`, that reads the `E` edges of the requested node and links
`fromId` to the target of the first edge it read, or to the requested node when
it read none (flyingrobots/echo#982). The coordinates under `consumer.links`
are neutral; Echo interprets none of them. The configuration carries the
`echo.dpo.intrinsics/v2` catalog the profile pins, a graph schema with one
mutable node type (`0x6b…`), one portal-owner type (`0x6c…`) and one edge type
(`0x6d…`), a footprint law that reads those nodes, scans `E` ranges and writes
one `E` edge (two write slots: the edge and its source's range), and a
`graphReads` declaration allowing eight edge scans.

The hexadecimal files are unchanged canonical CBOR written by the public
compiler, Amblicit `7dbcbc23d72705c5e224fc994a6c767881958912`: the package and
verifier report it wrote through the checked provider package in
`schemas/edict-provider/package/v1`, and the lawpack manifest, adapter and
configuration its lawpack build wrote. Nothing here was decoded and
re-encoded, and no Core or Target IR was authored by hand. `FollowLink.edict`
carries the digest of the lawpack the same compiler built from
`source/edict.lawpack.json`.

`scripts/verify-edict-provider-host-v1.sh` rebuilds these bytes in CI with
`xtask --example effect_step_publication --name link --check`, which also
requires Echo's lowerer to refuse the same program under `echo.dpo@1` and under
a substituted `echo.dpo@2` profile.

| File | Domain-framed digest |
| --- | --- |
| `link-executable-operation-package.hex` | `echo:operation-package:v1` `31c092bec1c0b3a061232e01ab0fa2943469f22324f5e240f2d30d6fb1dbe4c1` |

`crates/warp-core/tests/executable_operation_pipeline/public_graph_read_effect.rs`
fixes that pin independently of the bytes, admits the package under the
runtime owner's exact policy, installs it, runs it over graphs built there,
checks the link it derives against the edge it read, and checks the read image's
scanned bucket size for a bucket in a child warp.
