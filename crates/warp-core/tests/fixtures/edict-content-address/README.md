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

## Provenance and known limit

The `identity-*.hex` files are the public compiler's outputs for this source,
not authored Target IR or packages. They were produced by Amblicit
`da1574f169cda98206440e4d8f785b1ea9e01fcf` with two diagnostic-only changes:
writing the Core and Target IR it sends to the lowerer, and a configurable
provider fuel limit, run at 200,000,000. Lowering this program needs about
60–70 million units of Wasm fuel and verifying it about 80–90 million; the
unmodified compiler allows 50,000,000 per invocation and refuses it with
`FuelExhausted`. The structured fixture needs about 30–40 and 40–50 million.

The provider components were local, non-designated builds of this branch
(lowerer `fe7189cc…`, verifier `b47981cc…`), not the promoted G4 components.
The verifier accepted the package. These outputs are provisional: after
component promotion they must be reproduced with the promoted package, and
with an unmodified compiler once the fuel limit is resolved.
