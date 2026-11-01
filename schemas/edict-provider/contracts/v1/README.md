<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Current upstream provider contract pack

This is the exact schema/manifest pair from Amblicit merge
`2269d81f257f7794b44270cf3f7ca330966a0784`, [PR #285](https://github.com/flyingrobots/amblicit/pull/285), published upstream as `bounded-state-fold-v1`.
The API and coordinate remain `edict.provider-contract-pack/v1` and
`edict.provider-contract-pack.cddl@1`; exact bytes identify this publication.

| Artifact | SHA-256 |
| --- | --- |
| `edict-provider-contracts.cddl` | `2210f75386be76ea4182b967d2c29d1ff24c1c575ca4ffa7bbc1735f6e2c0284` |
| `manifest.json` | `48c08670e27bd722dc779488e5839f01252ff4ecc90c670043a509365bab6361` |

Both files are Apache-2.0 upstream inputs. Replace them together; do not edit
one independently. The sole production admission API pins this pair, its
84,340-byte manifest ceiling, twelve contracts, eight domains and five resources.
Earlier pairs are rejected. Historical bytes survive only in Git and test-only
rejection fixtures, without a production selector or historical decoder.

Relative to the prior source-module pair, this publication adds the fold
expression to Core and the shared Target expression schema. It also includes
the upstream structured Target IR family (`structuredTargetIrArtifact`,
`echo.span-ir/v3`, branch and bounded-for control entries) and tightens phased
Target domains so `echo.span-ir/v2` and `echo.span-ir/v3` cannot masquerade as
phased artifacts. The source-closure envelope remains structurally bounded to
1–64 modules with typed SHA-256 digests. Source authentication, typing,
capability declarations, total costs and execution remain semantic obligations.

The checked provider components still reject `core.fold/v1`. Schema admission
and package generation do not establish fold execution, structured control
execution, admission, or authored-rope completion. The consumer publication
witness uses this same current pair and accepts explicit candidate components;
it has no historical-publication selector flags.
