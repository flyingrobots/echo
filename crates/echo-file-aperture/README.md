<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# echo-file-aperture

`echo-file-aperture` is the Echo-owned standard artifact for host-file
observation, basis tracking, content admission, and materialization
verification.

It is not a filesystem runtime and it is not a FUSE adapter. Callers provide
host capabilities such as reading bytes, writing bytes, and collecting path
evidence. This crate owns the deterministic contract that turns those host
facts into file sites, fingerprints, basis tokens, projections, receipts, and
obstructions.

File-site identity is resolved by authority tier:

- platform identity derives a `PlatformStable` `FileSiteId`;
- path-only evidence derives a weaker `PathBound` `FileSiteId`.

Path evidence is observation evidence, not durable authority. `FileSiteId`
binds local host-aperture observations; it is not `WorldlineId` and must not be
treated as portable WSC causal identity.

The first slice is intentionally in-memory. Later slices should attach these
receipts and retained materials to Echo WAL/WSC recovery surfaces.
