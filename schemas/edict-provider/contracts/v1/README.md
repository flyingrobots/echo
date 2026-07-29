<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Edict Provider Contract Pack v1

This directory vendors the exact Edict provider contract pack introduced in
[Edict PR #162](https://github.com/flyingrobots/edict/pull/162) and extended
with `edict.result-projection.artifact/v1` in
[Edict PR #174](https://github.com/flyingrobots/edict/pull/174) at commit
[`21c4400faadf107e68906463d67c95532563c2ed`](https://github.com/flyingrobots/edict/commit/21c4400faadf107e68906463d67c95532563c2ed):

- `edict-provider-contracts.cddl` is the assembled Edict-owned CDDL contract.
- `manifest.json` binds that CDDL and its contract resources to their published
  identities, digests, and provenance.

Both files are licensed under Apache-2.0, as declared by the upstream CDDL and
manifest. They are exact external inputs, not Echo-authored schemas and not
generated Echo outputs. Do not edit either file independently. An update must
replace the pair from one reviewed Edict publication and update this provenance
record in the same change.

The Echo generator receives these bytes explicitly. It does not discover this
directory, resolve a mutable coordinate, read a registry, or fetch the pack at
generation time. The checked path makes the selected publication reviewable;
admission of the supplied bytes is the executable authority boundary.
