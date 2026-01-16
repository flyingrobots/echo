// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! C-compatible bindings for the warp-core engine.
//!
//! This module exposes a minimal ABI that higher-level languages (Rhai host modules, Python,
//! etc.) can use to interact with the deterministic engine without knowing the
//! internal Rust types.
#![deny(missing_docs)]

use warp_core::{Engine, TxId};

/// Opaque engine pointer exposed over the C ABI.
pub struct WarpEngine {
    inner: Engine,
}

/// 256-bit node identifier exposed as a raw byte array for FFI consumers.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct warp_node_id {
    /// Raw bytes representing the hashed node identifier.
    pub bytes: [u8; 32],
}

/// Transaction identifier mirrored on the C side.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct warp_tx_id {
    /// Native transaction value.
    pub value: u64,
}

/// Snapshot hash emitted after a successful commit.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct warp_snapshot {
    /// Canonical hash bytes for the snapshot.
    pub hash: [u8; 32],
}

/// Releases the engine allocation.
///
/// # Safety
/// `engine` must be a pointer previously returned by an engine constructor that
/// has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn warp_engine_free(engine: *mut WarpEngine) {
    if engine.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(engine));
    }
}

/// Starts a new transaction and returns its identifier.
///
/// # Safety
/// `engine` must be a valid pointer to a `WarpEngine`.
#[no_mangle]
pub unsafe extern "C" fn warp_engine_begin(engine: *mut WarpEngine) -> warp_tx_id {
    if engine.is_null() {
        return warp_tx_id { value: 0 };
    }
    let engine = unsafe { &mut *engine };
    let tx = engine.inner.begin();
    warp_tx_id { value: tx.value() }
}

/// Commits the transaction and writes the resulting snapshot hash.
///
/// # Safety
/// Pointers must be valid; `tx` must correspond to a live transaction.
#[no_mangle]
pub unsafe extern "C" fn warp_engine_commit(
    engine: *mut WarpEngine,
    tx: warp_tx_id,
    out_snapshot: *mut warp_snapshot,
) -> bool {
    if engine.is_null() || out_snapshot.is_null() || tx.value == 0 {
        return false;
    }
    let engine = unsafe { &mut *engine };
    match engine.inner.commit(TxId::from_raw(tx.value)) {
        Ok(snapshot) => {
            unsafe {
                (*out_snapshot).hash = snapshot.hash;
            }
            true
        }
        Err(_) => false,
    }
}
