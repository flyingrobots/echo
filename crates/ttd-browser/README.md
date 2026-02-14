<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ttd-browser

TTD Browser Engine: WASM bindings for the Echo Time-Travel Debugger.

## Overview

This crate provides a stateful `TtdEngine` struct that wraps the TTD primitives from `warp-core` into a JavaScript-friendly API. It compiles to WebAssembly via `wasm-bindgen` and exposes:

- **Cursor Management**: Create, seek, step, and manage playback cursors
- **Session Management**: Subscribe to channels and receive truth frames
- **Provenance Queries**: Get state roots, commit hashes, and emissions digests
- **Fork Support**: Snapshot and fork worldlines for "what-if" exploration
- **Transaction Control**: Generate TTDR v2 receipts

## Architecture

Per the TTD spec (docs/plans/ttd-app.md Part 7.1), `ttd-browser` is designed as a "pure MBUS client" - it sends EINT intents and receives TruthFrames, with minimal protocol logic. The heavier protocol logic lives in `ttd-controller` (Task 5.1).

```text
┌─────────────────────────────────────────────────────────────────┐
│  JavaScript (TTD App)                                           │
├─────────────────────────────────────────────────────────────────┤
│  ttd-browser (TtdEngine)                                        │
│    ├── Cursor management (create, seek, step)                   │
│    ├── Session management (subscribe, drain_frames)             │
│    └── Provenance queries (digests, receipts)                   │
├─────────────────────────────────────────────────────────────────┤
│  warp-core (PlaybackCursor, ViewSession, LocalProvenanceStore)  │
└─────────────────────────────────────────────────────────────────┘
```

## Usage

### JavaScript

```js
import init, { TtdEngine } from "ttd-browser";

await init();
const engine = new TtdEngine();

// Register a worldline
engine.register_worldline(worldlineId, warpId);

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

- `register_worldline(worldline_id, warp_id)` - Register a worldline

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

### Session Management

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
- **echo-ttd**: Compliance engine
- **warp-core**: Playback cursors, sessions, provenance store

## License

Apache-2.0
