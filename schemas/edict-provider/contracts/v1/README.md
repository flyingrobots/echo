<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Edict Provider Contract Pack v1

This directory vendors the exact Edict provider contract pack merged in
[Edict PR #162](https://github.com/flyingrobots/edict/pull/162) at commit
[`7cd8858c577fcfb6a05f0f617dfa821bb183c7df`](https://github.com/flyingrobots/edict/commit/7cd8858c577fcfb6a05f0f617dfa821bb183c7df):

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
