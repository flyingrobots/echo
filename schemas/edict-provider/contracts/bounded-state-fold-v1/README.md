<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Bounded state fold contract publication

This exact schema/manifest pair comes from Amblicit merge `2269d81f257f7794b44270cf3f7ca330966a0784` ([PR #285](https://github.com/flyingrobots/amblicit/pull/285)). Select it explicitly with `ProviderContractPublicationV1::BoundedStateFold`. Previous selectors retain their pinned bytes and inventories.

| Artifact | SHA-256 |
| --- | --- |
| `edict-provider-contracts.cddl` | `2210f75386be76ea4182b967d2c29d1ff24c1c575ca4ffa7bbc1735f6e2c0284` |
| `manifest.json` | `48c08670e27bd722dc779488e5839f01252ff4ecc90c670043a509365bab6361` |

The twelve contracts, eight domains and five contract resources retain the source-closure inventory. The expression schema adds bounded folds to Core and Target IR. Schema validation does not establish helper typing, capability declarations, iteration costs or runtime execution. Core integrity and independent provider judgments own those checks.

`source_functions_publication_witness --bounded-state-fold` generates an explicitly selected package using the existing source-module semantic declarations. The checked components still reject `core.fold/v1`; selecting this schema does not make folds executable. Candidate components must be supplied explicitly and independently verified before execution. The default package is unchanged.

Docker checks: `cargo test -p echo-wesley-gen --test provider_bounded_fold_contract_pack --test provider_publication_binding`.
