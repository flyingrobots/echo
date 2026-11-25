<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Current upstream provider contract pack

This is the exact schema/manifest pair from Amblicit merge
`aa8c7cbface8816cd930dca57252487eaa86f008`, [PR #352](https://github.com/flyingrobots/amblicit/pull/352), published upstream as `effect-cost-catalog-v1`.
PR #352 replaced the first publication of that pack (`a21de8c2`, PR #351),
whose recursive cost-rule schema the provider artifact-schema registry refused
(`SchemaCompileFailed` for `echo.dpo.intrinsics/v3`). The #351 pair never
shipped on Echo `main`, so it is not kept as a rejection fixture.
The API and coordinate remain `edict.provider-contract-pack/v1` and
`edict.provider-contract-pack.cddl@1`; exact bytes identify this publication.

| Artifact | SHA-256 |
| --- | --- |
| `edict-provider-contracts.cddl` | `ca33e99928edf8e0eaebdd3032ca1782c07eb84fd9e733a138711d7fcb4e53b8` |
| `manifest.json` | `6837fbf0b3fdc5ece9be31b5b4e1c4cb6017fa8dc002789c394691091c4ade96` |

Both files are Apache-2.0 upstream inputs. Replace them together; do not edit
one independently. The sole production admission API pins this pair, its
114,170-byte manifest ceiling, twelve contracts, eight domains and five resources.
Earlier pairs are rejected. Historical bytes survive only in Git and test-only
rejection fixtures, without a production selector or historical decoder.

Relative to `typed-failures-v1` (Amblicit `0734b499`, PR #317, now a
rejection fixture), this publication changes one root: `intrinsics-document`
also admits `edict.target-profile.intrinsics/v3` catalogs (amblicit#341). A
`/v3` catalog is the `/v2` shape plus a required `effectCostRule`
(`{ steps, bytes }` cost expressions) on every effect intrinsic and an
optional `parameters` map naming where each template parameter lives in the
operation configuration. Cost rules are written out level by level to a
nesting depth of 10, the same limit the compiler's decoder enforces; Echo's
catalog nests to level 6. `echo.dpo@2` pins a `/v3` catalog; Echo publishes no
`/v2` catalog.

The typed-failures publication, relative to `function-preconditions-v1` (Amblicit `1196396b`, PR #318, also a
rejection fixture), added intrinsic catalog v2 and typed
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
