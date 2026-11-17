<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Current upstream provider contract pack

This is the exact schema/manifest pair from Amblicit merge
`1196396b8cbf30967a943ae170aa8edd972d28ef`, [PR #318](https://github.com/flyingrobots/amblicit/pull/318), published upstream as `function-preconditions-v1`.
The API and coordinate remain `edict.provider-contract-pack/v1` and
`edict.provider-contract-pack.cddl@1`; exact bytes identify this publication.

| Artifact | SHA-256 |
| --- | --- |
| `edict-provider-contracts.cddl` | `d83bb51133f7b7fc03f94dde4d3cc6dee165da31db77c6f82570b468223e1a1f` |
| `manifest.json` | `31ebc222b80be213d4b240a7dee85d0c6e781851706294965d8d025684910d62` |

Both files are Apache-2.0 upstream inputs. Replace them together; do not edit
one independently. The sole production admission API pins this pair, its
86,830-byte manifest ceiling, twelve contracts, eight domains and five resources.
Earlier pairs are rejected. Historical bytes survive only in Git and test-only
rejection fixtures, without a production selector or historical decoder.

Relative to `bounded-state-fold-v1`, this publication adds only the optional
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
