// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! TTD Browser Engine: WASM bindings for the Time-Travel Debugger.
//!
//! This crate provides a stateful `TtdEngine` struct that wraps the TTD primitives
//! from `warp-core` into a JavaScript-friendly API. It is designed as a "pure MBUS
//! client" per the TTD architecture spec - it sends EINT intents and receives
//! `TruthFrames`, with minimal protocol logic.
//!
//! # Key Types
//!
//! - [`TtdEngine`]: The main entry point, managing cursors, sessions, and provenance.
//!
//! # Usage (from JavaScript)
//!
//! ```js
//! import init, { TtdEngine } from 'ttd-browser';
//!
//! await init();
//! const engine = new TtdEngine();
//!
//! // Create a cursor for a worldline
//! const cursorId = engine.create_cursor(worldlineIdBytes);
//!
//! // Seek to a specific tick
//! engine.seek_to(cursorId, 42n);
//!
//! // Get provenance data
//! const commitHash = engine.get_commit_hash(cursorId);
//! ```
//!
//! # Architecture Notes
//!
//! This crate is intentionally thin. The heavier protocol logic will live in
//! `ttd-controller` (Task 5.1) once Wesley delivers the generated types.
//! For now, we expose the playback, session, and provenance APIs that work
//! with existing `warp-core` infrastructure.

#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::cargo)]
#![allow(clippy::module_name_repetitions)]

use std::collections::BTreeMap;

use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

use ttd_protocol_rs::{
    ComplianceModel, ObligationReport, Snapshot, StepResult, StepResultKind, SCHEMA_SHA256,
};
use warp_core::materialization::ChannelId;
use warp_core::GraphStore;
use warp_core::{
    CursorId, CursorRole, LocalProvenanceStore, PlaybackCursor, PlaybackMode, ProvenanceStore,
    SeekThen, SessionId, StepResult as CoreStepResult, TruthSink, ViewSession, WorldlineId,
};

// ─── TtdEngine ───────────────────────────────────────────────────────────────

/// The main TTD browser engine.
///
/// This struct manages the lifecycle of cursors, sessions, and provenance data
/// for time-travel debugging in the browser. Each engine instance is isolated
/// and maintains its own state.
///
/// # Thread Safety
///
/// `TtdEngine` is designed for single-threaded use within a JavaScript context.
/// For Web Worker scenarios, create separate engine instances per worker.
#[wasm_bindgen]
pub struct TtdEngine {
    /// Provenance store holding worldline history.
    provenance: LocalProvenanceStore,

    /// Active playback cursors, keyed by handle ID.
    cursors: BTreeMap<u32, PlaybackCursor>,

    /// Active view sessions, keyed by handle ID.
    sessions: BTreeMap<u32, ViewSession>,

    /// Initial stores per worldline (for seek rebuilding from U0).
    initial_stores: BTreeMap<WorldlineId, GraphStore>,

    /// Truth sink for collecting frames during publish operations.
    truth_sink: TruthSink,

    /// Next cursor handle ID.
    next_cursor_id: u32,

    /// Next session handle ID.
    next_session_id: u32,

    /// Active transactions for receipt generation.
    transactions: BTreeMap<u64, Transaction>,

    /// Next transaction ID.
    next_tx_id: u64,
}

/// Internal transaction state for receipt generation.
struct Transaction {
    /// Cursor ID this transaction is associated with.
    cursor_id: u32,
    /// Tick at transaction start (reserved for future delta tracking).
    #[allow(dead_code)]
    start_tick: u64,
}

#[wasm_bindgen]
impl TtdEngine {
    // ─── Construction ────────────────────────────────────────────────────────

    /// Creates a new TTD engine instance.
    ///
    /// The engine starts with no worldlines, cursors, or sessions. Use
    /// `register_worldline` to add worldlines before creating cursors.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> Self {
        #[cfg(feature = "console-panic")]
        console_error_panic_hook::set_once();

        Self {
            provenance: LocalProvenanceStore::new(),
            cursors: BTreeMap::new(),
            sessions: BTreeMap::new(),
            initial_stores: BTreeMap::new(),
            truth_sink: TruthSink::new(),
            next_cursor_id: 1,
            next_session_id: 1,
            transactions: BTreeMap::new(),
            next_tx_id: 1,
        }
    }

    // ─── Worldline Management ────────────────────────────────────────────────

    /// Registers a worldline with the engine.
    ///
    /// This must be called before creating cursors for a worldline. The
    /// `worldline_id` and `warp_id` are 32-byte hashes.
    ///
    /// # Errors
    ///
    /// Returns an error if the worldline is already registered with a different
    /// warp ID.
    pub fn register_worldline(
        &mut self,
        worldline_id: &[u8],
        warp_id: &[u8],
    ) -> Result<(), JsError> {
        let worldline = parse_worldline_id(worldline_id)?;
        let warp = parse_warp_id(warp_id)?;

        self.provenance
            .register_worldline(worldline, warp)
            .map_err(|e| JsError::new(&e.to_string()))?;

        // Create and store initial empty GraphStore for this worldline
        let initial_store = GraphStore::new(warp);
        self.initial_stores.insert(worldline, initial_store);

        Ok(())
    }

    // ─── Cursor Management ───────────────────────────────────────────────────

    /// Creates a new playback cursor for a worldline.
    ///
    /// The cursor starts at tick 0 in Paused mode. Use `seek_to` to navigate
    /// to a specific tick, or `step` to advance.
    ///
    /// # Arguments
    ///
    /// * `worldline_id` - 32-byte worldline identifier
    ///
    /// # Returns
    ///
    /// A cursor handle ID (u32) for subsequent operations.
    ///
    /// # Errors
    ///
    /// Returns an error if the worldline is not registered.
    pub fn create_cursor(&mut self, worldline_id: &[u8]) -> Result<u32, JsError> {
        let wl_id = parse_worldline_id(worldline_id)?;

        let warp_id = self
            .provenance
            .u0(wl_id)
            .map_err(|e| JsError::new(&e.to_string()))?;

        let initial_store = self
            .initial_stores
            .get(&wl_id)
            .ok_or_else(|| JsError::new("worldline not registered"))?;

        // Determine pin_max_tick from provenance history length
        let history_len = self
            .provenance
            .len(wl_id)
            .map_err(|e| JsError::new(&e.to_string()))?;

        let cursor_id = CursorId(hash_from_u32(self.next_cursor_id));
        let cursor = PlaybackCursor::new(
            cursor_id,
            wl_id,
            warp_id,
            CursorRole::Reader,
            initial_store,
            history_len,
        );

        let handle = self.next_cursor_id;
        self.cursors.insert(handle, cursor);
        self.next_cursor_id = self.next_cursor_id.wrapping_add(1);

        Ok(handle)
    }

    /// Seeks a cursor to a specific tick.
    ///
    /// The cursor will apply patches from provenance to reach the target tick,
    /// verifying hashes along the way.
    ///
    /// # Arguments
    ///
    /// * `cursor_id` - Cursor handle from `create_cursor`
    /// * `tick` - Target tick number
    ///
    /// # Returns
    ///
    /// `true` if seek succeeded, `false` if the tick is unavailable.
    ///
    /// # Errors
    ///
    /// Returns an error if the cursor doesn't exist or verification fails.
    pub fn seek_to(&mut self, cursor_id: u32, tick: u64) -> Result<bool, JsError> {
        let cursor = self
            .cursors
            .get_mut(&cursor_id)
            .ok_or_else(|| JsError::new("cursor not found"))?;

        let initial_store = self
            .initial_stores
            .get(&cursor.worldline_id)
            .ok_or_else(|| JsError::new("worldline not registered"))?;

        match cursor.seek_to(tick, &self.provenance, initial_store) {
            Ok(()) => Ok(true),
            Err(
                warp_core::SeekError::HistoryUnavailable { .. }
                | warp_core::SeekError::PinnedFrontierExceeded { .. },
            ) => Ok(false),
            Err(e) => Err(JsError::new(&e.to_string())),
        }
    }

    /// Advances a cursor by one step according to its playback mode.
    ///
    /// # Returns
    ///
    /// CBOR-encoded `StepResult` object with fields:
    /// - `result`: `NoOp` | `Advanced` | `Seeked` | `ReachedFrontier`
    /// - `tick`: Current tick after step
    ///
    /// # Errors
    ///
    /// Returns an error if the cursor doesn't exist or step fails.
    pub fn step(&mut self, cursor_id: u32) -> Result<Uint8Array, JsError> {
        let cursor = self
            .cursors
            .get_mut(&cursor_id)
            .ok_or_else(|| JsError::new("cursor not found"))?;

        let initial_store = self
            .initial_stores
            .get(&cursor.worldline_id)
            .ok_or_else(|| JsError::new("worldline not registered"))?;

        let result = cursor
            .step(&self.provenance, initial_store)
            .map_err(|e| JsError::new(&e.to_string()))?;

        let step_result = StepResult {
            result: match result {
                CoreStepResult::NoOp => StepResultKind::NO_OP,
                CoreStepResult::Advanced => StepResultKind::ADVANCED,
                CoreStepResult::Seeked => StepResultKind::SEEKED,
                CoreStepResult::ReachedFrontier => StepResultKind::REACHED_FRONTIER,
            },
            tick: i32::try_from(cursor.tick).unwrap_or(i32::MAX),
        };

        encode_cbor(&step_result)
    }

    /// Gets the current tick of a cursor.
    ///
    /// # Errors
    ///
    /// Returns an error if the cursor doesn't exist.
    pub fn get_tick(&self, cursor_id: u32) -> Result<u64, JsError> {
        let cursor = self
            .cursors
            .get(&cursor_id)
            .ok_or_else(|| JsError::new("cursor not found"))?;
        Ok(cursor.tick)
    }

    /// Sets the playback mode for a cursor.
    ///
    /// # Arguments
    ///
    /// * `cursor_id` - Cursor handle
    /// * `mode` - One of: `Paused`, `Play`, `StepForward`, `StepBack`
    ///
    /// # Errors
    ///
    /// Returns an error if the cursor doesn't exist or mode is invalid.
    pub fn set_mode(&mut self, cursor_id: u32, mode: &str) -> Result<(), JsError> {
        let cursor = self
            .cursors
            .get_mut(&cursor_id)
            .ok_or_else(|| JsError::new("cursor not found"))?;

        cursor.mode = match mode {
            "Paused" => PlaybackMode::Paused,
            "Play" => PlaybackMode::Play,
            "StepForward" => PlaybackMode::StepForward,
            "StepBack" => PlaybackMode::StepBack,
            _ => return Err(JsError::new(&format!("invalid mode: {mode}"))),
        };

        Ok(())
    }

    /// Sets up a seek operation for a cursor.
    ///
    /// The seek will be performed on the next `step()` call.
    ///
    /// # Arguments
    ///
    /// * `cursor_id` - Cursor handle
    /// * `target` - Target tick to seek to
    /// * `then_play` - If true, transition to Play after seek; otherwise Paused
    ///
    /// # Errors
    ///
    /// Returns an error if the cursor doesn't exist.
    pub fn set_seek(
        &mut self,
        cursor_id: u32,
        target: u64,
        then_play: bool,
    ) -> Result<(), JsError> {
        let cursor = self
            .cursors
            .get_mut(&cursor_id)
            .ok_or_else(|| JsError::new("cursor not found"))?;

        cursor.mode = PlaybackMode::Seek {
            target,
            then: if then_play {
                SeekThen::Play
            } else {
                SeekThen::Pause
            },
        };

        Ok(())
    }

    /// Updates the pinned frontier for a cursor.
    ///
    /// The cursor cannot seek or step beyond this tick.
    ///
    /// # Errors
    ///
    /// Returns an error if the cursor doesn't exist.
    pub fn update_frontier(&mut self, cursor_id: u32, max_tick: u64) -> Result<(), JsError> {
        let cursor = self
            .cursors
            .get_mut(&cursor_id)
            .ok_or_else(|| JsError::new("cursor not found"))?;
        cursor.pin_max_tick = max_tick;
        Ok(())
    }

    /// Drops a cursor, freeing its resources.
    ///
    /// # Errors
    ///
    /// Returns an error if the cursor doesn't exist.
    pub fn drop_cursor(&mut self, cursor_id: u32) -> Result<(), JsError> {
        self.cursors
            .remove(&cursor_id)
            .ok_or_else(|| JsError::new("cursor not found"))?;
        Ok(())
    }

    // ─── Provenance Queries ──────────────────────────────────────────────────

    /// Gets the state root hash for a cursor's current position.
    ///
    /// # Returns
    ///
    /// 32-byte state root as `Uint8Array`.
    ///
    /// # Errors
    ///
    /// Returns an error if the cursor doesn't exist or tick is unavailable.
    pub fn get_state_root(&self, cursor_id: u32) -> Result<Uint8Array, JsError> {
        let cursor = self
            .cursors
            .get(&cursor_id)
            .ok_or_else(|| JsError::new("cursor not found"))?;

        if cursor.tick == 0 {
            // At initial state, no provenance entry yet
            return Ok(hash_to_uint8array(&[0u8; 32]));
        }

        let expected = self
            .provenance
            .expected(cursor.worldline_id, cursor.tick - 1)
            .map_err(|e| JsError::new(&e.to_string()))?;

        Ok(hash_to_uint8array(&expected.state_root))
    }

    /// Gets the commit hash for a cursor's current position.
    ///
    /// # Returns
    ///
    /// 32-byte commit hash as `Uint8Array`.
    ///
    /// # Errors
    ///
    /// Returns an error if the cursor doesn't exist or tick is unavailable.
    pub fn get_commit_hash(&self, cursor_id: u32) -> Result<Uint8Array, JsError> {
        let cursor = self
            .cursors
            .get(&cursor_id)
            .ok_or_else(|| JsError::new("cursor not found"))?;

        if cursor.tick == 0 {
            return Ok(hash_to_uint8array(&[0u8; 32]));
        }

        let expected = self
            .provenance
            .expected(cursor.worldline_id, cursor.tick - 1)
            .map_err(|e| JsError::new(&e.to_string()))?;

        Ok(hash_to_uint8array(&expected.commit_hash))
    }

    /// Gets the emissions digest for a cursor's current position.
    ///
    /// Note: This returns the patch digest since emissions digest is computed
    /// from the actual emissions data (which requires the full MBUS output).
    ///
    /// # Returns
    ///
    /// 32-byte digest as `Uint8Array`.
    ///
    /// # Errors
    ///
    /// Returns an error if the cursor doesn't exist or tick is unavailable.
    pub fn get_emissions_digest(&self, cursor_id: u32) -> Result<Uint8Array, JsError> {
        let cursor = self
            .cursors
            .get(&cursor_id)
            .ok_or_else(|| JsError::new("cursor not found"))?;

        if cursor.tick == 0 {
            return Ok(hash_to_uint8array(&[0u8; 32]));
        }

        let expected = self
            .provenance
            .expected(cursor.worldline_id, cursor.tick - 1)
            .map_err(|e| JsError::new(&e.to_string()))?;

        // Return patch_digest as a proxy; actual emissions_digest would need
        // to be computed from the MBUS outputs stored in provenance.
        Ok(hash_to_uint8array(&expected.patch_digest))
    }

    /// Gets the worldline history length.
    ///
    /// # Errors
    ///
    /// Returns an error if the worldline is not registered.
    pub fn get_history_length(&self, worldline_id: &[u8]) -> Result<u64, JsError> {
        let wl_id = parse_worldline_id(worldline_id)?;
        self.provenance
            .len(wl_id)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    // ─── Session Management ──────────────────────────────────────────────────

    /// Creates a new view session.
    ///
    /// Sessions manage channel subscriptions and truth frame delivery.
    /// A session must be associated with a cursor via `set_session_cursor`.
    ///
    /// # Returns
    ///
    /// A session handle ID (u32) for subsequent operations.
    pub fn create_session(&mut self) -> u32 {
        // Create with a placeholder cursor ID; must call set_session_cursor before use
        let session_id = SessionId(hash_from_u32(self.next_session_id));
        let cursor_id = CursorId([0u8; 32]);
        let session = ViewSession::new(session_id, cursor_id);

        let handle = self.next_session_id;
        self.sessions.insert(handle, session);
        self.next_session_id = self.next_session_id.wrapping_add(1);

        handle
    }

    /// Associates a session with a cursor.
    ///
    /// # Errors
    ///
    /// Returns an error if either the session or cursor doesn't exist.
    pub fn set_session_cursor(&mut self, session_id: u32, cursor_id: u32) -> Result<(), JsError> {
        let cursor = self
            .cursors
            .get(&cursor_id)
            .ok_or_else(|| JsError::new("cursor not found"))?;

        let session = self
            .sessions
            .get_mut(&session_id)
            .ok_or_else(|| JsError::new("session not found"))?;

        session.set_active_cursor(cursor.cursor_id);
        Ok(())
    }

    /// Subscribes a session to a channel.
    ///
    /// # Arguments
    ///
    /// * `session_id` - Session handle
    /// * `channel` - 32-byte channel identifier
    ///
    /// # Errors
    ///
    /// Returns an error if the session doesn't exist.
    pub fn subscribe(&mut self, session_id: u32, channel: &[u8]) -> Result<(), JsError> {
        let channel_id = parse_channel_id(channel)?;

        let session = self
            .sessions
            .get_mut(&session_id)
            .ok_or_else(|| JsError::new("session not found"))?;

        session.subscribe(channel_id);
        Ok(())
    }

    /// Unsubscribes a session from a channel.
    ///
    /// # Errors
    ///
    /// Returns an error if the session doesn't exist.
    pub fn unsubscribe(&mut self, session_id: u32, channel: &[u8]) -> Result<(), JsError> {
        let channel_id = parse_channel_id(channel)?;

        let session = self
            .sessions
            .get_mut(&session_id)
            .ok_or_else(|| JsError::new("session not found"))?;

        session.unsubscribe(channel_id);
        Ok(())
    }

    /// Publishes truth frames for a session's subscribed channels.
    ///
    /// This reads outputs from provenance for the cursor's current tick and
    /// publishes frames to the internal sink.
    ///
    /// # Errors
    ///
    /// Returns an error if the session or its cursor doesn't exist.
    pub fn publish_truth(&mut self, session_id: u32, cursor_id: u32) -> Result<(), JsError> {
        let cursor = self
            .cursors
            .get(&cursor_id)
            .ok_or_else(|| JsError::new("cursor not found"))?;

        let session = self
            .sessions
            .get(&session_id)
            .ok_or_else(|| JsError::new("session not found"))?;

        session
            .publish_truth(cursor, &self.provenance, &mut self.truth_sink)
            .map_err(|e| JsError::new(&e.to_string()))?;

        Ok(())
    }

    /// Drains all collected truth frames for a session.
    ///
    /// # Returns
    ///
    /// CBOR-encoded array of truth frames.
    ///
    /// # Errors
    ///
    /// Returns an error if the session doesn't exist.
    pub fn drain_frames(&mut self, session_id: u32) -> Result<Uint8Array, JsError> {
        let session = self
            .sessions
            .get(&session_id)
            .ok_or_else(|| JsError::new("session not found"))?;

        let frames = self.truth_sink.collect_frames(session.session_id);

        let js_frames: Vec<TruthFrameJs> = frames
            .iter()
            .map(|f| TruthFrameJs {
                channel: f.channel.0,
                value: f.value.clone(),
                value_hash: f.value_hash,
                tick: f.cursor.tick,
                commit_hash: f.cursor.commit_hash,
            })
            .collect();

        // Clear the sink after collecting
        self.truth_sink.clear();

        encode_cbor(&js_frames)
    }

    /// Drops a session, freeing its resources.
    ///
    /// # Errors
    ///
    /// Returns an error if the session doesn't exist.
    pub fn drop_session(&mut self, session_id: u32) -> Result<(), JsError> {
        self.sessions
            .remove(&session_id)
            .ok_or_else(|| JsError::new("session not found"))?;
        Ok(())
    }

    // ─── Transaction Control ─────────────────────────────────────────────────

    /// Begins a new transaction for a cursor.
    ///
    /// Transactions track operations for receipt generation. Call `commit`
    /// when done to generate a TTDR receipt.
    ///
    /// # Returns
    ///
    /// Transaction ID (u64).
    ///
    /// # Errors
    ///
    /// Returns an error if the cursor doesn't exist.
    pub fn begin(&mut self, cursor_id: u32) -> Result<u64, JsError> {
        let cursor = self
            .cursors
            .get(&cursor_id)
            .ok_or_else(|| JsError::new("cursor not found"))?;

        let tx_id = self.next_tx_id;
        self.transactions.insert(
            tx_id,
            Transaction {
                cursor_id,
                start_tick: cursor.tick,
            },
        );
        self.next_tx_id = self.next_tx_id.wrapping_add(1);

        Ok(tx_id)
    }

    /// Commits a transaction and generates a TTDR v2 Light receipt.
    ///
    /// # Returns
    ///
    /// Encoded TTDR v2 frame as `Uint8Array`.
    ///
    /// # Errors
    ///
    /// Returns an error if the transaction or cursor doesn't exist.
    pub fn commit(&mut self, tx_id: u64) -> Result<Uint8Array, JsError> {
        let tx = self
            .transactions
            .remove(&tx_id)
            .ok_or_else(|| JsError::new("transaction not found"))?;

        let cursor = self
            .cursors
            .get(&tx.cursor_id)
            .ok_or_else(|| JsError::new("cursor not found"))?;

        // Generate a Light mode receipt for the current cursor position
        if cursor.tick == 0 {
            return Err(JsError::new("cannot commit at tick 0"));
        }

        let expected = self
            .provenance
            .expected(cursor.worldline_id, cursor.tick - 1)
            .map_err(|e| JsError::new(&e.to_string()))?;

        // Use the existing wire codec to encode
        let flags = echo_session_proto::TtdrFlags::new(
            true, // has_state_root
            false,
            false,
            false,
            echo_session_proto::ReceiptMode::Light,
        );

        // Note: warp_id available via provenance.u0() for future use
        // Parse schema_hash from the protocol's SCHEMA_SHA256 hex string
        let schema_hash = parse_schema_hash();

        let header = echo_session_proto::TtdrHeader {
            version: echo_session_proto::TTDR_VERSION,
            flags,
            schema_hash,
            worldline_id: cursor.worldline_id.0,
            tick: cursor.tick,
            commit_hash: expected.commit_hash,
            state_root: expected.state_root,
            patch_digest: expected.patch_digest,
            emissions_digest: [0u8; 32], // Would need actual emissions data
            op_emission_index_digest: [0u8; 32],
            parent_count: 0,
            channel_count: 0,
        };

        let frame = echo_session_proto::TtdrFrame {
            header,
            parent_hashes: vec![],
            channel_digests: vec![],
        };

        let bytes =
            echo_session_proto::encode_ttdr_v2(&frame).map_err(|e| JsError::new(&e.to_string()))?;

        Ok(bytes_to_uint8array(&bytes))
    }

    // ─── Snapshot & Fork ─────────────────────────────────────────────────────

    /// Creates a snapshot of a cursor's current state.
    ///
    /// The snapshot can be used to create a fork via `fork_from_snapshot`.
    ///
    /// # Returns
    ///
    /// CBOR-encoded snapshot data.
    ///
    /// # Errors
    ///
    /// Returns an error if the cursor doesn't exist.
    pub fn snapshot(&self, cursor_id: u32) -> Result<Uint8Array, JsError> {
        let cursor = self
            .cursors
            .get(&cursor_id)
            .ok_or_else(|| JsError::new("cursor not found"))?;

        let snapshot = Snapshot {
            worldlineId: bytes_to_hex(&cursor.worldline_id.0),
            tick: i32::try_from(cursor.tick).unwrap_or(i32::MAX),
        };

        encode_cbor(&snapshot)
    }

    /// Creates a new worldline forked from a snapshot.
    ///
    /// # Arguments
    ///
    /// * `snapshot` - CBOR-encoded snapshot from `snapshot()`
    /// * `new_worldline_id` - 32-byte ID for the new worldline
    ///
    /// # Returns
    ///
    /// Cursor handle for the new fork.
    ///
    /// # Errors
    ///
    /// Returns an error if the snapshot is invalid or fork fails.
    pub fn fork_from_snapshot(
        &mut self,
        snapshot: &[u8],
        new_worldline_id: &[u8],
    ) -> Result<u32, JsError> {
        let snap: Snapshot =
            ciborium::from_reader(snapshot).map_err(|e| JsError::new(&e.to_string()))?;

        let source_wl_bytes = hex_to_bytes(&snap.worldlineId)
            .map_err(|e| JsError::new(&format!("invalid worldlineId: {e}")))?;
        let source_wl = WorldlineId(source_wl_bytes);
        let new_wl = parse_worldline_id(new_worldline_id)?;

        // Convert tick from i32 to u64 (protocol uses i32, internal uses u64)
        let tick = u64::try_from(snap.tick).unwrap_or(0);

        // Fork in provenance store
        self.provenance
            .fork(source_wl, tick.saturating_sub(1), new_wl)
            .map_err(|e| JsError::new(&e.to_string()))?;

        // Copy initial store
        if let Some(store) = self.initial_stores.get(&source_wl) {
            self.initial_stores.insert(new_wl, store.clone());
        }

        // Create cursor for the new worldline
        self.create_cursor(&new_wl.0)
    }

    // ─── Compliance (Stubs) ──────────────────────────────────────────────────

    /// Gets the compliance status for the current session.
    ///
    /// # Note
    ///
    /// This is a stub that returns an empty compliance model. Full compliance
    /// checking requires Wesley-generated schemas (Task 3.1).
    ///
    /// # Returns
    ///
    /// CBOR-encoded compliance model.
    ///
    /// # Errors
    ///
    /// Returns an error if CBOR encoding fails (should not happen in practice).
    pub fn get_compliance(&self) -> Result<Uint8Array, JsError> {
        let compliance = ComplianceModel {
            isGreen: true,
            violations: vec![],
        };
        encode_cbor(&compliance)
    }

    /// Gets the obligation status for the current session.
    ///
    /// # Note
    ///
    /// This is a stub that returns an empty obligation state. Full obligation
    /// tracking requires Wesley-generated contracts (Task 4.x).
    ///
    /// # Returns
    ///
    /// CBOR-encoded obligation state.
    ///
    /// # Errors
    ///
    /// Returns an error if CBOR encoding fails (should not happen in practice).
    pub fn get_obligations(&self) -> Result<Uint8Array, JsError> {
        let obligations = ObligationReport {
            pending: vec![],
            satisfied: vec![],
            violated: vec![],
        };
        encode_cbor(&obligations)
    }
}

impl Default for TtdEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Helper Types ────────────────────────────────────────────────────────────

/// Internal `TruthFrame` for CBOR serialization to JavaScript.
/// This differs from the protocol `TruthFrame` which is designed for event messages
/// and doesn't include the actual value bytes.
#[derive(serde::Serialize)]
struct TruthFrameJs {
    channel: [u8; 32],
    value: Vec<u8>,
    value_hash: [u8; 32],
    tick: u64,
    commit_hash: [u8; 32],
}

// ─── Helper Functions ────────────────────────────────────────────────────────

/// Error type for parsing 32-byte identifiers.
#[derive(Debug, Clone)]
struct ParseError(&'static str);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn parse_worldline_id_inner(bytes: &[u8]) -> Result<WorldlineId, ParseError> {
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| ParseError("worldline_id must be 32 bytes"))?;
    Ok(WorldlineId(arr))
}

fn parse_warp_id_inner(bytes: &[u8]) -> Result<warp_core::WarpId, ParseError> {
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| ParseError("warp_id must be 32 bytes"))?;
    Ok(warp_core::WarpId(arr))
}

fn parse_channel_id_inner(bytes: &[u8]) -> Result<ChannelId, ParseError> {
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| ParseError("channel must be 32 bytes"))?;
    Ok(warp_core::TypeId(arr))
}

fn parse_worldline_id(bytes: &[u8]) -> Result<WorldlineId, JsError> {
    parse_worldline_id_inner(bytes).map_err(|e| JsError::new(e.0))
}

fn parse_warp_id(bytes: &[u8]) -> Result<warp_core::WarpId, JsError> {
    parse_warp_id_inner(bytes).map_err(|e| JsError::new(e.0))
}

fn parse_channel_id(bytes: &[u8]) -> Result<ChannelId, JsError> {
    parse_channel_id_inner(bytes).map_err(|e| JsError::new(e.0))
}

fn hash_from_u32(n: u32) -> [u8; 32] {
    let mut hash = [0u8; 32];
    hash[..4].copy_from_slice(&n.to_le_bytes());
    hash
}

fn hash_to_uint8array(hash: &[u8; 32]) -> Uint8Array {
    let arr = Uint8Array::new_with_length(32);
    arr.copy_from(hash);
    arr
}

// WASM targets are 32-bit; length cannot exceed u32::MAX.
#[allow(clippy::cast_possible_truncation)]
fn bytes_to_uint8array(bytes: &[u8]) -> Uint8Array {
    let arr = Uint8Array::new_with_length(bytes.len() as u32);
    arr.copy_from(bytes);
    arr
}

fn encode_cbor<T: serde::Serialize>(value: &T) -> Result<Uint8Array, JsError> {
    let mut buf = Vec::new();
    ciborium::into_writer(value, &mut buf)
        .map_err(|e| JsError::new(&format!("CBOR encode error: {e}")))?;
    Ok(bytes_to_uint8array(&buf))
}

/// Parses the `SCHEMA_SHA256` hex string into a [u8; 32] array.
fn parse_schema_hash() -> [u8; 32] {
    let mut result = [0u8; 32];
    for (i, chunk) in SCHEMA_SHA256.as_bytes().chunks(2).enumerate() {
        if i >= 32 {
            break;
        }
        let hex_str = std::str::from_utf8(chunk).unwrap_or("00");
        result[i] = u8::from_str_radix(hex_str, 16).unwrap_or(0);
    }
    result
}

/// Converts a byte array to a hex string.
fn bytes_to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;

    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
    out
}

/// Converts a hex string to a [u8; 32] array.
fn hex_to_bytes(hex: &str) -> Result<[u8; 32], &'static str> {
    if hex.len() != 64 {
        return Err("hex string must be 64 characters");
    }
    let mut result = [0u8; 32];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let hex_str = std::str::from_utf8(chunk).map_err(|_| "invalid UTF-8 in hex string")?;
        result[i] = u8::from_str_radix(hex_str, 16).map_err(|_| "invalid hex character")?;
    }
    Ok(result)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

// Tests are gated on wasm32 target to avoid wasm-bindgen panics on native.
// For native testing of the core logic, use warp-core's playback tests directly.
#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use super::*;

    fn test_worldline_id() -> [u8; 32] {
        [1u8; 32]
    }

    fn test_warp_id() -> [u8; 32] {
        [2u8; 32]
    }

    #[test]
    fn test_cursor_not_found() {
        let engine = TtdEngine::new();
        let result = engine.get_tick(999);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_helpers() {
        let wl = parse_worldline_id(&[0u8; 32]).unwrap();
        assert_eq!(wl.0, [0u8; 32]);

        let invalid = parse_worldline_id(&[0u8; 16]);
        assert!(invalid.is_err());
    }

    #[test]
    fn test_compliance_stub() {
        let engine = TtdEngine::new();
        let result = engine.get_compliance();
        assert!(result.is_ok());
    }

    #[test]
    fn test_obligations_stub() {
        let engine = TtdEngine::new();
        let result = engine.get_obligations();
        assert!(result.is_ok());
    }
}

// Native tests that don't call methods returning JsError on failure paths.
// Tests that trigger error paths must run on wasm32 target.
#[cfg(test)]
mod tests {
    use super::*;

    fn test_worldline_id() -> [u8; 32] {
        [1u8; 32]
    }

    fn test_warp_id() -> [u8; 32] {
        [2u8; 32]
    }

    #[test]
    fn test_engine_creation() {
        let engine = TtdEngine::new();
        assert!(engine.cursors.is_empty());
        assert!(engine.sessions.is_empty());
    }

    #[test]
    fn test_worldline_registration() {
        let mut engine = TtdEngine::new();
        let result = engine.register_worldline(&test_worldline_id(), &test_warp_id());
        assert!(result.is_ok());
    }

    #[test]
    fn test_cursor_creation() {
        let mut engine = TtdEngine::new();
        engine
            .register_worldline(&test_worldline_id(), &test_warp_id())
            .unwrap();

        let cursor_id = engine.create_cursor(&test_worldline_id()).unwrap();
        assert_eq!(cursor_id, 1);

        let tick = engine.get_tick(cursor_id).unwrap();
        assert_eq!(tick, 0);
    }

    #[test]
    fn test_session_creation() {
        let mut engine = TtdEngine::new();
        let session_id = engine.create_session();
        assert_eq!(session_id, 1);
    }

    // Use _inner functions for tests that check error paths
    #[test]
    fn test_parse_worldline_id_inner_valid() {
        let wl = parse_worldline_id_inner(&[0u8; 32]);
        assert!(wl.is_ok());
        assert_eq!(wl.unwrap().0, [0u8; 32]);
    }

    #[test]
    fn test_parse_worldline_id_inner_invalid_length() {
        let invalid = parse_worldline_id_inner(&[0u8; 16]);
        assert!(invalid.is_err());
    }

    #[test]
    fn test_parse_warp_id_inner_valid() {
        let wid = parse_warp_id_inner(&[1u8; 32]);
        assert!(wid.is_ok());
    }

    #[test]
    fn test_parse_channel_id_inner_valid() {
        let cid = parse_channel_id_inner(&[2u8; 32]);
        assert!(cid.is_ok());
    }

    #[test]
    fn test_hash_from_u32() {
        let hash = hash_from_u32(42);
        assert_eq!(&hash[..4], &42u32.to_le_bytes());
        assert_eq!(&hash[4..], &[0u8; 28]);
    }

    #[test]
    fn test_cursor_modes_success() {
        let mut engine = TtdEngine::new();
        engine
            .register_worldline(&test_worldline_id(), &test_warp_id())
            .unwrap();

        let cursor_id = engine.create_cursor(&test_worldline_id()).unwrap();

        // Test setting various modes - all success paths
        assert!(engine.set_mode(cursor_id, "Paused").is_ok());
        assert!(engine.set_mode(cursor_id, "Play").is_ok());
        assert!(engine.set_mode(cursor_id, "StepForward").is_ok());
        assert!(engine.set_mode(cursor_id, "StepBack").is_ok());
    }

    #[test]
    fn test_set_seek() {
        let mut engine = TtdEngine::new();
        engine
            .register_worldline(&test_worldline_id(), &test_warp_id())
            .unwrap();

        let cursor_id = engine.create_cursor(&test_worldline_id()).unwrap();

        // Set up a seek - success paths only
        assert!(engine.set_seek(cursor_id, 10, false).is_ok());
        assert!(engine.set_seek(cursor_id, 5, true).is_ok());
    }

    #[test]
    fn test_begin_transaction() {
        let mut engine = TtdEngine::new();
        engine
            .register_worldline(&test_worldline_id(), &test_warp_id())
            .unwrap();

        let cursor_id = engine.create_cursor(&test_worldline_id()).unwrap();
        let tx_id = engine.begin(cursor_id).unwrap();
        assert_eq!(tx_id, 1);

        // Second transaction gets next ID
        let tx_id2 = engine.begin(cursor_id).unwrap();
        assert_eq!(tx_id2, 2);
    }

    #[test]
    fn test_drop_cursor_success() {
        let mut engine = TtdEngine::new();
        engine
            .register_worldline(&test_worldline_id(), &test_warp_id())
            .unwrap();

        let cursor_id = engine.create_cursor(&test_worldline_id()).unwrap();
        assert!(engine.drop_cursor(cursor_id).is_ok());

        // Cursor should be gone
        assert!(!engine.cursors.contains_key(&cursor_id));
    }

    #[test]
    fn test_drop_session_success() {
        let mut engine = TtdEngine::new();
        let session_id = engine.create_session();

        assert!(engine.drop_session(session_id).is_ok());

        // Session should be gone
        assert!(!engine.sessions.contains_key(&session_id));
    }

    #[test]
    fn test_update_frontier() {
        let mut engine = TtdEngine::new();
        engine
            .register_worldline(&test_worldline_id(), &test_warp_id())
            .unwrap();

        let cursor_id = engine.create_cursor(&test_worldline_id()).unwrap();

        // Update frontier
        assert!(engine.update_frontier(cursor_id, 100).is_ok());
        assert_eq!(engine.cursors.get(&cursor_id).unwrap().pin_max_tick, 100);
    }

    #[test]
    fn test_session_cursor_association() {
        let mut engine = TtdEngine::new();
        engine
            .register_worldline(&test_worldline_id(), &test_warp_id())
            .unwrap();

        let cursor_id = engine.create_cursor(&test_worldline_id()).unwrap();
        let session_id = engine.create_session();

        // Associate session with cursor
        assert!(engine.set_session_cursor(session_id, cursor_id).is_ok());
    }

    #[test]
    fn test_subscribe_unsubscribe() {
        let mut engine = TtdEngine::new();
        let session_id = engine.create_session();
        let channel = [42u8; 32];

        assert!(engine.subscribe(session_id, &channel).is_ok());
        assert!(engine.unsubscribe(session_id, &channel).is_ok());
    }

    #[test]
    fn test_get_history_length() {
        let mut engine = TtdEngine::new();
        engine
            .register_worldline(&test_worldline_id(), &test_warp_id())
            .unwrap();

        let len = engine.get_history_length(&test_worldline_id()).unwrap();
        assert_eq!(len, 0);
    }
}
