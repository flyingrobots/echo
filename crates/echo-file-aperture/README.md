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

The first slice is intentionally in-memory. Later slices should attach these
receipts and retained materials to Echo WAL/WSC recovery surfaces.
