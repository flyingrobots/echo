<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# PLATFORM - Neighborhood publication stack documentation

Echo now has enough real neighborhood publication truth that the stack needs a
single durable explanation, not scattered comments and chat archaeology.

This cycle should write down:

- the difference between commit-time emissions and read-time publications
- the role of `BoundedSite`, `ObservationArtifact`, `NeighborhoodSite`, and
  `NeighborhoodCore`
- the exact host/publication path through `echo-wasm-abi` and `warp-wasm`
- what the host should request when it wants raw observation, Echo-native local
  site truth, or the shared Continuum family projection

The goal is not new runtime law. The goal is to stop forcing maintainers and
host authors to reverse-engineer the publication stack from code.

Related:

- `docs/design/0005-echo-ttd-witness-surface.md`
- `docs/design/0007-braid-geometry-and-neighborhood-publication.md`
- `docs/design/0010-bounded-site-and-admission-policy.md`
- `docs/spec/SPEC-0009-wasm-abi-v3.md`
