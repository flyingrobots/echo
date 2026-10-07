<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# echo-cas

Content-addressed blob store for Echo.

`MemoryTier` provides infallible in-process content-addressed storage.
`DiskTier` provides a fallible filesystem-backed retained blob tier for material
that must survive process reconstruction while preserving content-only BLAKE3
hash semantics. Because filesystem writes can fail, `DiskTier` exposes fallible
methods directly instead of hiding I/O behind the infallible `BlobStore` trait.

## Complete-object physical-content port

`physical_content` adds fallible expected staging, explicit backend publication, borrowed read views, and atomic destination promotion. Both MemoryTier and DiskTier pass one shared suite. Staging requires explicit per-object byte bounds; it does not enforce aggregate MemoryTier capacity. Missing content and unsupported output capabilities return operational errors rather than authenticated absence. Receipts establish no synchronization or crash durability. Existing consumers keep their current APIs.

[The physical-content boundary](../../docs/architecture/echo-keep-physical-content-boundary.md) owns the evidence and visibility contract.
