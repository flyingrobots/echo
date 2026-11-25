<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Public identity intrinsics in helpers and fold steps

`source/HelperIdentity.edict` calls `echo.dpo@2.contentAddress` from a source
helper, `leafId`, and from a fold step, `parentStep`, which addresses a node
whose one `edgeA` edge points at the previous id. It returns the helper's id
for the input types and the id after two fold steps, and ensures a node at
the address the helper computes under the stored types
(flyingrobots/echo#1005). The lawpack and application are the
`edict-content-address` fixture's.

The runtime inputs are that fixture's: `warp` 03, `nodeType` 07, `atomType`
02, `atom` `new`, `edgeA` 06, `storedType` 5b, `storedAtomType` 5c, each id
32 repetitions of the byte. With them the helper's id is the frozen
`node-zero-edges` vector.

## Provenance

The `helper-*.hex` files are the public compiler's outputs for this source,
not authored Target IR or packages. The `effect_step_publication` example
produces them with an unmodified Amblicit
`2b09e4ab1ecf0041b4ff251693dc295ac2b649c2` binary and the checked provider
package; `scripts/verify-edict-provider-host-v1.sh` reruns it with `--check`,
and requires the lowerer to refuse the same program under `echo.dpo@1` and
under a substituted `echo.dpo@2` profile.

| File | Domain-framed digest |
| --- | --- |
| `helper-executable-operation-package.hex` | `echo:operation-package:v1` `b346c00eab5c1fe244531fdb26717546c4726f526db3436323e86de6078ebdc6` |
