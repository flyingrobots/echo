# Two reads with independent addresses

Captured with the same diagnostic-only compiler instrumentation and revisions
as the parent fixture. This source reads `firstAddress`, guards its bytes, then
reads `secondAddress` and guards those bytes before returning the second result.
The source owns both addresses and guards. This exercises two read producers
and eight independently scoped failure binders. It adds no native application
semantics. Lawpack, adapter, exports, configuration, and target profile bytes
are identical to the parent fixture. The native test replaces only the four
changed compiler artifacts below.

Like the parent capture, the public build refuses the unpublished configuration;
this is not independently verified executable or runtime evidence.

| Artifact | Bytes | Raw SHA-256 |
| --- | ---: | --- |
| `core.hex` | 4119 | `c4fcfba58267e37515212cac7a4a638960822c12fb860b1a772345f3dde86c68` |
| `04-source.hex` | 1155 | `9f114d7d491104ea44c2310ac86d702832e264a79ed163f159d8ac1bc5e033a5` |
| `06-target-ir.hex` | 3731 | `bb6635dd77dbe81a65e1239c6dfc3b270844a5d85129b9f040e49c25cab911f6` |
| `07-result-projection.hex` | 238 | `e62bc371852e88c081bc16107e24f9f3cf56ed13a2fe3043983af580c58feea5` |
