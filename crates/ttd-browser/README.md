<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ttd-browser

Echo browser host bridge: WASM bindings for browser-hosted observation,
playback, and receipt surfaces.

## Overview

This crate provides a stateful `TtdEngine` struct that wraps current Echo
playback and provenance primitives into a JavaScript-friendly API. It compiles
to WebAssembly via `wasm-bindgen`.

Today it still exposes a fairly rich compatibility surface:

- **Cursor management**: create, seek, step, and manage playback cursors
- **Session-style frame draining**: subscribe to channels and receive truth frames
- **Provenance queries**: state roots, commit hashes, and emissions digests
- **Fork support**: snapshot and fork worldlines for local exploration
- **Receipt generation**: generate TTDR v2 receipts

That compatibility surface is useful, but the ownership split is now explicit:

- Echo owns runtime truth and browser-hostable WASM substrate
- `warp-ttd` owns debugger session semantics and browser debugger product

So `ttd-browser` should be read as a browser host bridge and migration layer,
not as the long-term home of browser debugger semantics.

## Architecture

`ttd-browser` is currently the narrowest reusable Echo-side browser bridge.
Long-term, Browser TTD should sit above it as a `warp-ttd` delivery adapter.

```text
┌─────────────────────────────────────────────────────────────────┐
│  Browser UI / Browser TTD delivery adapter                      │
├─────────────────────────────────────────────────────────────────┤
│  ttd-browser (Echo browser host bridge)                         │
│    ├── Browser-safe playback / provenance access                │
│    ├── TTDR / EINT bridge encoding                              │
│    └── Transitional compatibility surface                       │
├─────────────────────────────────────────────────────────────────┤
│  warp-core (PlaybackCursor, ViewSession, LocalProvenanceStore)  │
└─────────────────────────────────────────────────────────────────┘
```

## Long-term role

Keep in this crate:

- browser-hosted access to Echo runtime state
- browser-safe observation / playback handles
- deterministic frame / receipt encoding needed by a host adapter

Do not keep growing here:

- browser-only debugger session semantics
- canonical neighborhood browser logic
- product-defining browser debugger UI concepts

## Usage

### JavaScript

```js
import init, { TtdEngine } from "ttd-browser";

await init();
const engine = new TtdEngine();

// Register a canonical empty worldline
engine.register_empty_worldline(worldlineId, warpId);

// Create a cursor and navigate
const cursorId = engine.create_cursor(worldlineId);
engine.seek_to(cursorId, 42n);

// Get provenance data
const commitHash = engine.get_commit_hash(cursorId);

// Create a session and subscribe
const sessionId = engine.create_session();
engine.set_session_cursor(sessionId, cursorId);
engine.subscribe(sessionId, channelId);

// Publish and drain truth frames
engine.publish_truth(sessionId, cursorId);
const frames = engine.drain_frames(sessionId); // CBOR-encoded
```

## API Summary

### Construction

- `new()` - Create a new engine instance

### Worldline Management

- `register_empty_worldline(worldline_id, warp_id)` - Register a canonical empty worldline

### Cursor Management

- `create_cursor(worldline_id) → cursor_id`
- `seek_to(cursor_id, tick) → bool`
- `step(cursor_id) → StepResult (CBOR)`
- `get_tick(cursor_id) → tick`
- `set_mode(cursor_id, mode)` - Paused, Play, StepForward, StepBack
- `set_seek(cursor_id, target, then_play)`
- `update_frontier(cursor_id, max_tick)`
- `drop_cursor(cursor_id)`

### Provenance Queries

- `get_state_root(cursor_id) → Uint8Array`
- `get_commit_hash(cursor_id) → Uint8Array`
- `get_emissions_digest(cursor_id) → Uint8Array`
- `get_history_length(worldline_id) → u64`

### Session / Frame Compatibility

- `create_session() → session_id`
- `set_session_cursor(session_id, cursor_id)`
- `subscribe(session_id, channel)`
- `unsubscribe(session_id, channel)`
- `publish_truth(session_id, cursor_id)`
- `drain_frames(session_id) → Uint8Array (CBOR)`
- `drop_session(session_id)`

### Transaction Control

- `begin(cursor_id) → tx_id`
- `commit(tx_id) → Uint8Array (TTDR v2)`

### Fork Support

- `snapshot(cursor_id) → Uint8Array (CBOR)`
- `fork_from_snapshot(snapshot, new_worldline_id) → cursor_id`

### Compliance (Stubs)

- `get_compliance() → Uint8Array (CBOR)` - Stub until Wesley Task 3.1
- `get_obligations() → Uint8Array (CBOR)` - Stub until Wesley Task 4.x

## Building

```bash
# Check
cargo check -p ttd-browser

# Test (native)
cargo test -p ttd-browser

# Build WASM
wasm-pack build crates/ttd-browser --target web
```

## Related

- **warp-wasm**: Low-level TTD WASM bindings (digests, compliance, wire codecs)
- **echo-ttd**: Runtime-side compliance and receipt validation
- **warp-core**: Playback cursors, sessions, provenance store
- **warp-ttd**: Canonical debugger/session semantics and future Browser TTD delivery adapter

## License

Apache-2.0
