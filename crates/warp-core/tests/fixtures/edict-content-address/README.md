<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Public identity-intrinsic fixture

`source/ContentAddress.edict` calls the three `echo.dpo@2` identity
constructors through `use target echo.dpo@2 ... as echo`. It computes one
output per frozen `ContentAddressV1` vector (no edges, one edge, two edges in
both orders, a content edge and a slot edge), then ensures a content node at
the address `echo.contentAddress` computes for it. The lawpack is the
structured fixture's `ensureNode`/`createNode` lawpack under
`consumer.identity`, with a 512-byte result and 131,072-byte allocation
ceiling. The application requires the three contracts at
`echo.content-address/v1` through `target.pureIntrinsics`.

The runtime inputs are the frozen vectors' inputs (`warp` 03, `nodeType` 07,
`atomType` 02, `atom` `new`, `edgeA` 06, `edgeB` 08, `low` 04, `high` 05, each
id 32 repetitions of the byte), plus `storedType` 5b and `storedAtomType` 5c
for the ensured node, which the lawpack's graph schema admits.

## Provenance

The `identity-*.hex` files are the public compiler's outputs for this source,
not authored Target IR or packages. The `effect_step_publication` example
produces them with an unmodified Amblicit
`7dbcbc23d72705c5e224fc994a6c767881958912` binary and the checked provider
package; `scripts/verify-edict-provider-host-v1.sh` reruns it with `--check`.
That Amblicit revision sizes the provider-host fuel limit for real Echo
programs (amblicit#326); this program needs more than the earlier 50,000,000
units.
