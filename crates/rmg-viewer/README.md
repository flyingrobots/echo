<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# rmg-viewer (scaffold)

Interactive renderer for Echo RMG snapshots. This initial version opens a wgpu window with an egui overlay; next steps are to stream snapshots from `rmg-cli`, render skeleton edges/nodes with instancing, and draw region hulls / wormhole polygons inspired by ADR-0003/4/5/6.

Run locally:

```bash
cargo run -p rmg-viewer
```

Roadmap:
- ingest `Snapshot` exports from `rmg-cli` (JSON/CBOR or live IPC)
- force-layout on CPU, draw nodes/edges via instanced meshes
- region hull fill + collapsible wormholes
- zoom levels aligned to ADR-0004 scale invariance
- overlays for provenance payloads and tick playback
