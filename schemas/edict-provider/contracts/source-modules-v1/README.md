<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Source-module contract publication

These are the exact Edict-owned contract-pack bytes from merge
`b57136309d29c702a8096a78af9e16c2d5e2ec71`, [Edict #255](https://github.com/flyingrobots/edict/pull/255).
Echo selects this pair only through `ProviderContractPublicationV1::SourceModules`.
Earlier selectors retain their existing bytes, bounds and inventories.

| Artifact                        | SHA-256                                                            |
| ------------------------------- | ------------------------------------------------------------------ |
| `edict-provider-contracts.cddl` | `8f8db4052e1bb85c4f728a0a453c4eb20904bcf2ce81530ffdbfce9351e209bc` |
| `manifest.json`                 | `29927f6db5e0fe1e5e7475bef3fb2c6e7f0f84ea56304fd97bd1299f1c2bb4be` |

The publication has twelve contracts, eight domains and five unchanged
contract resources. It adds `source-closure` and `edict.source-closure/v1`.
The CDDL checks a closed envelope, 1–64 modules and typed SHA-256 digests.
Placeholder digests can satisfy that shape. Import pins, root membership,
reachability, topology, UTF-8 and aggregate source budgets require semantic
validation and are not established by schema admission.

The generator still defaults to its original publication. Selecting the schema alone does not grant executable support. The native
provider models handle the distinct source form and the bounded-read decoder
rechecks its source domain. A consumer still needs a matching semantic variant,
fresh matching provider components and an independently verified package.
Structural package admission does not establish an execution witness.

Run `cargo test -p echo-wesley-gen --test provider_source_module_contract_pack
--test provider_publication_binding` for exact-pair, crossed-publication,
substitution, schema-boundary and generation-provenance checks.
