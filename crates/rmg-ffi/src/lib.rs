//! C-compatible bindings for the motion rewrite spike.
//!
//! This module exposes a minimal ABI that higher-level languages (Lua, Python,
//! etc.) can use to interact with the deterministic engine without knowing the
//! internal Rust types.
#![deny(missing_docs)]

use std::ffi::CStr;
use std::os::raw::c_char;
use std::slice;

use rmg_core::{
    build_motion_demo_engine, decode_motion_payload, encode_motion_payload, make_node_id,
    make_type_id, ApplyResult, Engine, NodeId, NodeRecord, TxId, MOTION_RULE_NAME,
};

/// Opaque engine pointer exposed over the C ABI.
pub struct RmgEngine {
    inner: Engine,
}

/// 256-bit node identifier exposed as a raw byte array for FFI consumers.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct rmg_node_id {
    /// Raw bytes representing the hashed node identifier.
    pub bytes: [u8; 32],
}

/// Transaction identifier mirrored on the C side.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct rmg_tx_id {
    /// Native transaction value.
    pub value: u64,
}

/// Snapshot hash emitted after a successful commit.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct rmg_snapshot {
    /// Canonical hash bytes for the snapshot.
    pub hash: [u8; 32],
}

/// Creates a new engine with the motion rule registered.
///
/// # Safety
/// The caller assumes ownership of the returned pointer and must release it
/// via [`rmg_engine_free`] to avoid leaking memory.
// Rust 2024 requires `#[unsafe(no_mangle)]` as `no_mangle` is an unsafe attribute.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rmg_engine_new() -> *mut RmgEngine {
    Box::into_raw(Box::new(RmgEngine {
        inner: build_motion_demo_engine(),
    }))
}

/// Releases the engine allocation created by [`rmg_engine_new`].
///
/// # Safety
/// `engine` must be a pointer previously returned by [`rmg_engine_new`] that
/// has not already been freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rmg_engine_free(engine: *mut RmgEngine) {
    if engine.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(engine));
    }
}

/// Spawns an entity with encoded motion data and returns its identifier.
///
/// # Safety
/// `engine`, `label`, and `out_handle` must be valid pointers. `label` must
/// reference a null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rmg_engine_spawn_motion_entity(
    engine: *mut RmgEngine,
    label: *const c_char,
    px: f32,
    py: f32,
    pz: f32,
    vx: f32,
    vy: f32,
    vz: f32,
    out_handle: *mut rmg_node_id,
) -> bool {
    if engine.is_null() || label.is_null() || out_handle.is_null() {
        return false;
    }
    let engine = unsafe { &mut *engine };
    let label = unsafe { CStr::from_ptr(label) };
    let label_str = match label.to_str() {
        Ok(value) => value,
        Err(_) => return false,
    };

    let node_id = make_node_id(label_str);
    let entity_type = make_type_id("entity");
    let payload = encode_motion_payload([px, py, pz], [vx, vy, vz]);

    engine.inner.insert_node(
        node_id,
        NodeRecord {
            ty: entity_type,
            payload: Some(payload),
        },
    );

    unsafe {
        (*out_handle).bytes = node_id.0;
    }
    true
}

/// Starts a new transaction and returns its identifier.
///
/// # Safety
/// `engine` must be a valid pointer created by [`rmg_engine_new`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rmg_engine_begin(engine: *mut RmgEngine) -> rmg_tx_id {
    if engine.is_null() {
        return rmg_tx_id { value: 0 };
    }
    let engine = unsafe { &mut *engine };
    let tx = engine.inner.begin();
    rmg_tx_id { value: tx.0 }
}

/// Applies the motion rewrite to the provided entity within transaction `tx`.
///
/// # Safety
/// All pointers must be valid. `tx` must reference an active transaction.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rmg_engine_apply_motion(
    engine: *mut RmgEngine,
    tx: rmg_tx_id,
    node_handle: *const rmg_node_id,
) -> bool {
    let engine = match unsafe { engine.as_mut() } {
        Some(engine) => engine,
        None => return false,
    };
    if tx.value == 0 {
        return false;
    }
    let node_id = match handle_to_node_id(node_handle) {
        Some(id) => id,
        None => return false,
    };
    match engine
        .inner
        .apply(TxId(tx.value), MOTION_RULE_NAME, &node_id)
    {
        Ok(ApplyResult::Applied) => true,
        Ok(ApplyResult::NoMatch) => false,
        Err(_) => false,
    }
}

/// Commits the transaction and writes the resulting snapshot hash.
///
/// # Safety
/// Pointers must be valid; `tx` must correspond to a live transaction.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rmg_engine_commit(
    engine: *mut RmgEngine,
    tx: rmg_tx_id,
    out_snapshot: *mut rmg_snapshot,
) -> bool {
    if engine.is_null() || out_snapshot.is_null() || tx.value == 0 {
        return false;
    }
    let engine = unsafe { &mut *engine };
    match engine.inner.commit(TxId(tx.value)) {
        Ok(snapshot) => {
            unsafe {
                (*out_snapshot).hash = snapshot.hash;
            }
            true
        }
        Err(_) => false,
    }
}

/// Reads the decoded position and velocity for an entity.
///
/// # Safety
/// Pointers must be valid; output buffers must have length at least three.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rmg_engine_read_motion(
    engine: *mut RmgEngine,
    node_handle: *const rmg_node_id,
    out_position: *mut f32,
    out_velocity: *mut f32,
) -> bool {
    let engine = match unsafe { engine.as_ref() } {
        Some(engine) => engine,
        None => return false,
    };
    if out_position.is_null() || out_velocity.is_null() {
        return false;
    }
    let node_id = match handle_to_node_id(node_handle) {
        Some(id) => id,
        None => return false,
    };
    let record = match engine.inner.node(&node_id) {
        Some(record) => record,
        None => return false,
    };
    let payload = match record.payload.as_ref() {
        Some(payload) => payload,
        None => return false,
    };
    let (position, velocity) = match decode_motion_payload(payload) {
        Some(values) => values,
        None => return false,
    };
    copy_vec3(out_position, &position);
    copy_vec3(out_velocity, &velocity);
    true
}

fn handle_to_node_id(handle: *const rmg_node_id) -> Option<NodeId> {
    // Helper used internally by the ABI; callers pass raw bytes from C.
    if handle.is_null() {
        return None;
    }
    let bytes = unsafe { (*handle).bytes };
    Some(NodeId(bytes))
}

fn copy_vec3(ptr: *mut f32, values: &[f32; 3]) {
    // Safety: callers guarantee `ptr` references a buffer with len >= 3.
    unsafe {
        let slice = slice::from_raw_parts_mut(ptr, 3);
        slice.copy_from_slice(values);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    unsafe fn spawn(engine: *mut RmgEngine, label: &str) -> rmg_node_id {
        let c_label = CString::new(label).unwrap();
        let mut handle = rmg_node_id { bytes: [0; 32] };
        let ok = unsafe {
            rmg_engine_spawn_motion_entity(
                engine,
                c_label.as_ptr(),
                1.0,
                2.0,
                3.0,
                0.5,
                -1.0,
                0.25,
                &mut handle as *mut _,
            )
        };
        assert!(ok);
        handle
    }

    #[test]
    fn ffi_motion_rewrite_is_deterministic() {
        unsafe {
            let engine_a = rmg_engine_new();
            let handle_a = spawn(engine_a, "entity-ffi");
            let tx_a = rmg_engine_begin(engine_a);
            assert!(rmg_engine_apply_motion(
                engine_a,
                tx_a,
                &handle_a as *const _
            ));
            let mut snap_a = rmg_snapshot { hash: [0; 32] };
            assert!(rmg_engine_commit(engine_a, tx_a, &mut snap_a as *mut _));

            let engine_b = rmg_engine_new();
            let handle_b = spawn(engine_b, "entity-ffi");
            let tx_b = rmg_engine_begin(engine_b);
            assert!(rmg_engine_apply_motion(
                engine_b,
                tx_b,
                &handle_b as *const _
            ));
            let mut snap_b = rmg_snapshot { hash: [0; 32] };
            assert!(rmg_engine_commit(engine_b, tx_b, &mut snap_b as *mut _));

            assert_eq!(snap_a.hash, snap_b.hash);

            let mut position = [0f32; 3];
            let mut velocity = [0f32; 3];
            assert!(rmg_engine_read_motion(
                engine_a,
                &handle_a as *const _,
                position.as_mut_ptr(),
                velocity.as_mut_ptr()
            ));
            assert_eq!(position, [1.5, 1.0, 3.25]);
            assert_eq!(velocity, [0.5, -1.0, 0.25]);

            rmg_engine_free(engine_a);
            rmg_engine_free(engine_b);
        }
    }

    #[test]
    fn ffi_apply_no_match_returns_false() {
        unsafe {
            let engine = rmg_engine_new();
            let tx = rmg_engine_begin(engine);
            let bogus = rmg_node_id { bytes: [1; 32] };
            assert!(!rmg_engine_apply_motion(engine, tx, &bogus as *const _));
            rmg_engine_free(engine);
        }
    }
}
