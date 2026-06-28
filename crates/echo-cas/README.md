<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# echo-cas

Content-addressed blob store for Echo.

`MemoryTier` provides infallible in-process content-addressed storage.
`DiskTier` provides a fallible filesystem-backed retained blob tier for material
that must survive process reconstruction while preserving content-only BLAKE3
hash semantics. Because filesystem writes can fail, `DiskTier` exposes fallible
methods directly instead of hiding I/O behind the infallible `BlobStore` trait.
