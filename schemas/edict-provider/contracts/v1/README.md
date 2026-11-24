<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Current upstream provider contract pack

This is the exact schema/manifest pair from Amblicit merge
`0734b499adb7c79ddbf6b8290256c83d5240731e`, [PR #317](https://github.com/flyingrobots/amblicit/pull/317), published upstream as `typed-failures-v1`. The bytes are unchanged at
Amblicit `da1574f169cda98206440e4d8f785b1ea9e01fcf`, whose own current-pack test
asserts them.
The API and coordinate remain `edict.provider-contract-pack/v1` and
`edict.provider-contract-pack.cddl@1`; exact bytes identify this publication.

| Artifact | SHA-256 |
| --- | --- |
| `edict-provider-contracts.cddl` | `ed60e6ad6e0a80e23f915a8f791342b9e97cadd03034541865e754ef30352233` |
| `manifest.json` | `ef57e32542bea8c07f4629e54174c1598fb1852378dc147ee2da505fdaf069fa` |

Both files are Apache-2.0 upstream inputs. Replace them together; do not edit
one independently. The sole production admission API pins this pair, its
92,284-byte manifest ceiling, twelve contracts, eight domains and five resources.
Earlier pairs are rejected. Historical bytes survive only in Git and test-only
rejection fixtures, without a production selector or historical decoder.

Relative to `function-preconditions-v1` (Amblicit `1196396b`, PR #318, now a
rejection fixture), this publication adds intrinsic catalog v2 and typed
failures. Catalog v2 (`intrinsics-document`) requires API
`edict.target-profile.intrinsics/v2`, a complete monomorphic contract
(`contractVersion`, `costRule`) for every pure intrinsic, and an optional
`types` map; it no longer admits a v1 catalog. The frozen `echo.dpo@1`
catalog therefore keeps its original owning rules under the Echo-generated
`echo-frozen-catalog-v1-` namespace (see
`crates/echo-wesley-gen/tests/provider_frozen_catalog_schema.rs`). Typed
failures (`fail`, `attempt`) are schema-admitted only; Echo's providers and
runtime do not yet accept them.

Relative to `bounded-state-fold-v1`, the preconditions publication added only the optional
`preconditions` list of a Core source function, a nonempty list of
`precondition-predicate` (`true`, `false`, `not`, `all`, `any` and comparisons).
That pair, from Amblicit `2269d81f` (PR #285), is now a rejection fixture.
Relative to the source-module pair before it, the fold publication added the fold
expression to Core and the shared Target expression schema. It also includes
the upstream structured Target IR family (`structuredTargetIrArtifact`,
`echo.span-ir/v3`, branch and bounded-for control entries) and tightens phased
Target domains so `echo.span-ir/v2` and `echo.span-ir/v3` cannot masquerade as
phased artifacts. The source-closure envelope remains structurally bounded to
1–64 modules with typed SHA-256 digests. Source authentication, typing,
capability declarations, total costs and execution remain semantic obligations.

The checked provider components judge function preconditions (see
`docs/topics/GeneratedRules.md`, "Function preconditions"). They include `core.fold/v1` judgments for pure and
bounded-read operations, with source-owned invariant steps and bounded costs.
Schema admission and package generation alone do not establish fold execution,
structured control execution, admission, or authored-rope completion. The consumer publication
witness uses this same current pair and accepts explicit candidate components;
it has no historical-publication selector flags.

Amblicit is the renamed Edict compiler repository and crate family. Published
Edict wire-domain identifiers remain the exact protocol identifiers in this pair.
