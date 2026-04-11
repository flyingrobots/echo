<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Braid geometry and neighborhood publication for strands

The current strand contract is a bootstrap, not the parity endpoint.

It gives Echo:

- speculative lane identity
- base provenance
- explicit ticking
- TTD lane typing and parentage

It does **not** yet give Echo the things `git-warp` already has or is much
closer to:

- read-only support overlays as real braid geometry
- participating-lane publication at a local site
- honest plurality for neighborhood browser/debugger surfaces

This slice should answer:

1. what the first non-empty `support_pins` contract looks like
2. how Echo publishes participating lanes at a local site without waiting for a
   perfect global braid model
3. which part belongs in kernel runtime truth versus adapter-level summary
4. how this feeds Continuum-aligned `NeighborhoodCore` publication instead of
   staying an Echo-local hidden trick

This is the missing leg between bootstrap strands and real conceptual parity
with `git-warp`.
