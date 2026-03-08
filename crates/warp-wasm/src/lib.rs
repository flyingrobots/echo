// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! WASM boundary for the Echo deterministic simulation engine.
//!
//! This crate provides `wasm-bindgen` exports that delegate to a [`KernelPort`]
//! implementation. The boundary is app-agnostic: any kernel that implements
//! the trait can be installed via [`install_kernel`].
//!
//! # ABI Contract (v1)
//!
//! All exports return CBOR-encoded bytes wrapped in a success/error envelope:
//! - Success: `{ "ok": true, ...response_fields }`
//! - Error:   `{ "ok": false, "code": u32, "message": string }`
//!
//! See [`echo_wasm_abi::kernel_port`] for the full type definitions.
//!
//! # Initialization
//!
//! The host must call [`init`] (or install a kernel via [`install_kernel`])
//! before using any other export. Calling exports before initialization
//! returns a structured error (no panics).
// wasm_bindgen generates unsafe glue code; allow unsafe in this crate.
#![allow(unsafe_code)]

use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

use echo_wasm_abi::kernel_port::{
    self, AbiError, ErrEnvelope, KernelPort, OkEnvelope, RawBytesResponse,
};

use std::cell::RefCell;

// ---------------------------------------------------------------------------
// Kernel storage (module-scoped, single-threaded WASM)
// ---------------------------------------------------------------------------

thread_local! {
    static KERNEL: RefCell<Option<Box<dyn KernelPort>>> = const { RefCell::new(None) };
}

/// Install a kernel implementation into the WASM boundary.
///
/// This is the app-agnostic injection point. Call this from your app's
/// initialization code (or from [`init`] for the default engine kernel).
///
/// # Panics
///
/// Does not panic. Replaces any previously installed kernel.
pub fn install_kernel(kernel: Box<dyn KernelPort>) {
    KERNEL.with(|cell| {
        *cell.borrow_mut() = Some(kernel);
    });
}

/// Run a closure with a mutable reference to the installed kernel.
///
/// Returns an [`AbiError`] with code [`NOT_INITIALIZED`](kernel_port::error_codes::NOT_INITIALIZED)
/// if no kernel has been installed.
fn with_kernel<F, R>(f: F) -> Result<R, AbiError>
where
    F: FnOnce(&mut dyn KernelPort) -> Result<R, AbiError>,
{
    KERNEL.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let kernel = borrow.as_mut().ok_or_else(|| AbiError {
            code: kernel_port::error_codes::NOT_INITIALIZED,
            message: "kernel not initialized; call init() first".into(),
        })?;
        f(kernel.as_mut())
    })
}

/// Run a closure with an immutable reference to the installed kernel.
fn with_kernel_ref<F, R>(f: F) -> Result<R, AbiError>
where
    F: FnOnce(&dyn KernelPort) -> Result<R, AbiError>,
{
    KERNEL.with(|cell| {
        let borrow = cell.borrow();
        let kernel = borrow.as_ref().ok_or_else(|| AbiError {
            code: kernel_port::error_codes::NOT_INITIALIZED,
            message: "kernel not initialized; call init() first".into(),
        })?;
        f(kernel.as_ref())
    })
}

// ---------------------------------------------------------------------------
// CBOR envelope encoding
// ---------------------------------------------------------------------------

/// Encode a successful result as a CBOR Uint8Array with `{ ok: true, ...data }`.
fn encode_ok<T: serde::Serialize>(value: &T) -> Uint8Array {
    let envelope = OkEnvelope::new(value);
    match echo_wasm_abi::encode_cbor(&envelope) {
        Ok(bytes) => bytes_to_uint8array(&bytes),
        Err(_) => encode_err_raw(
            kernel_port::error_codes::CODEC_ERROR,
            "failed to encode response",
        ),
    }
}

/// Encode an error as a CBOR Uint8Array with `{ ok: false, code, message }`.
fn encode_err(err: &AbiError) -> Uint8Array {
    encode_err_raw(err.code, &err.message)
}

/// Low-level error encoding that cannot itself fail (falls back to empty array).
fn encode_err_raw(code: u32, message: &str) -> Uint8Array {
    let envelope = ErrEnvelope::new(code, message.into());
    match echo_wasm_abi::encode_cbor(&envelope) {
        Ok(bytes) => bytes_to_uint8array(&bytes),
        Err(_) => Uint8Array::new_with_length(0),
    }
}

/// Encode a `Result<T, AbiError>` into a CBOR Uint8Array envelope.
fn encode_result<T: serde::Serialize>(result: Result<T, AbiError>) -> Uint8Array {
    match result {
        Ok(ref val) => encode_ok(val),
        Err(ref err) => encode_err(err),
    }
}

/// Helper to convert a byte slice into a JS `Uint8Array`.
///
/// WASM linear memory is 32-bit, so `bytes.len()` is guaranteed to fit in u32
/// on the target platform. On native (tests), we saturate to u32::MAX.
#[allow(clippy::cast_possible_truncation)]
fn bytes_to_uint8array(bytes: &[u8]) -> Uint8Array {
    let len = bytes.len().min(u32::MAX as usize) as u32;
    let arr = Uint8Array::new_with_length(len);
    arr.copy_from(bytes);
    arr
}

// ---------------------------------------------------------------------------
// WarpKernel: Engine-backed KernelPort (feature-gated)
// ---------------------------------------------------------------------------

#[cfg(feature = "engine")]
mod warp_kernel;

// ---------------------------------------------------------------------------
// Console panic hook
// ---------------------------------------------------------------------------

#[cfg(feature = "console-panic")]
#[wasm_bindgen(start)]
/// Initialize console panic hook for better error messages in browser.
pub fn init_console_panic_hook() {
    console_error_panic_hook::set_once();
}

// ---------------------------------------------------------------------------
// Frozen ABI exports (v1)
// ---------------------------------------------------------------------------

/// Initialize the default engine kernel.
///
/// When compiled with the `engine` feature, this creates a `WarpKernel`
/// backed by `warp-core::Engine`. Without the feature, this returns
/// a "not supported" error.
#[wasm_bindgen]
pub fn init() -> Uint8Array {
    #[cfg(feature = "engine")]
    {
        let kernel = warp_kernel::WarpKernel::new();
        let head = kernel.get_head().unwrap_or_else(|_| kernel_port::HeadInfo {
            tick: 0,
            state_root: Vec::new(),
            commit_id: Vec::new(),
        });
        install_kernel(Box::new(kernel));
        encode_ok(&head)
    }
    #[cfg(not(feature = "engine"))]
    {
        encode_err_raw(
            kernel_port::error_codes::NOT_SUPPORTED,
            "no engine feature enabled; install a kernel via install_kernel()",
        )
    }
}

/// Enqueue a canonical intent payload.
///
/// Returns CBOR-encoded [`DispatchResponse`](kernel_port::DispatchResponse)
/// on success, or an error envelope.
#[wasm_bindgen]
pub fn dispatch_intent(intent_bytes: &[u8]) -> Uint8Array {
    encode_result(with_kernel(|k| k.dispatch_intent(intent_bytes)))
}

/// Run deterministic steps up to a budget.
///
/// Returns CBOR-encoded [`StepResponse`](kernel_port::StepResponse).
/// A budget of 0 returns the current head without executing ticks.
#[wasm_bindgen]
pub fn step(step_budget: u32) -> Uint8Array {
    encode_result(with_kernel(|k| k.step(step_budget)))
}

/// Drain emitted ViewOps since the last drain.
///
/// Returns CBOR-encoded [`DrainResponse`](kernel_port::DrainResponse).
#[wasm_bindgen]
pub fn drain_view_ops() -> Uint8Array {
    encode_result(with_kernel(|k| k.drain_view_ops()))
}

/// Get current head info (tick, state_root, commit_id).
///
/// Returns CBOR-encoded [`HeadInfo`](kernel_port::HeadInfo).
#[wasm_bindgen]
pub fn get_head() -> Uint8Array {
    encode_result(with_kernel_ref(|k| k.get_head()))
}

/// Execute a read-only query by ID with canonical vars.
///
/// Returns CBOR-encoded `{ ok: true, data: <bytes> }` or error envelope.
#[wasm_bindgen]
pub fn execute_query(query_id: u32, vars_bytes: &[u8]) -> Uint8Array {
    let result = with_kernel_ref(|k| {
        k.execute_query(query_id, vars_bytes)
            .map(|bytes| RawBytesResponse { data: bytes })
    });
    encode_result(result)
}

/// Replay to a specific tick and return the snapshot.
///
/// Returns CBOR-encoded `{ ok: true, data: <bytes> }` or error envelope.
#[wasm_bindgen]
pub fn snapshot_at(tick: u64) -> Uint8Array {
    let result = with_kernel(|k| {
        k.snapshot_at(tick)
            .map(|bytes| RawBytesResponse { data: bytes })
    });
    encode_result(result)
}

/// Render a snapshot to ViewOps for visualization.
///
/// Returns CBOR-encoded `{ ok: true, data: <bytes> }` or error envelope.
#[wasm_bindgen]
pub fn render_snapshot(snapshot_bytes: &[u8]) -> Uint8Array {
    let result = with_kernel_ref(|k| {
        k.render_snapshot(snapshot_bytes)
            .map(|bytes| RawBytesResponse { data: bytes })
    });
    encode_result(result)
}

/// Return registry metadata (schema hash, codec id, registry version).
///
/// Returns CBOR-encoded [`RegistryInfo`](kernel_port::RegistryInfo).
#[wasm_bindgen]
pub fn get_registry_info() -> Uint8Array {
    encode_result(with_kernel_ref(|k| Ok(k.registry_info())))
}

/// Get the codec identifier from the installed registry.
#[wasm_bindgen]
pub fn get_codec_id() -> JsValue {
    let result = with_kernel_ref(|k| Ok(k.registry_info()));
    match result {
        Ok(info) => match info.codec_id {
            Some(id) => JsValue::from_str(&id),
            None => JsValue::NULL,
        },
        Err(_) => JsValue::NULL,
    }
}

/// Get the registry version from the installed registry.
#[wasm_bindgen]
pub fn get_registry_version() -> JsValue {
    let result = with_kernel_ref(|k| Ok(k.registry_info()));
    match result {
        Ok(info) => match info.registry_version {
            Some(v) => JsValue::from_str(&v),
            None => JsValue::NULL,
        },
        Err(_) => JsValue::NULL,
    }
}

/// Get the schema hash (hex) from the installed registry.
#[wasm_bindgen]
pub fn get_schema_sha256_hex() -> JsValue {
    let result = with_kernel_ref(|k| Ok(k.registry_info()));
    match result {
        Ok(info) => match info.schema_sha256_hex {
            Some(h) => JsValue::from_str(&h),
            None => JsValue::NULL,
        },
        Err(_) => JsValue::NULL,
    }
}

// ---------------------------------------------------------------------------
// Schema validation helpers (test-only, retained from previous impl)
// ---------------------------------------------------------------------------

#[cfg(test)]
fn validate_object_against_args(
    value: &serde_value::Value,
    args: &[echo_registry_api::ArgDef],
    enums: &[echo_registry_api::EnumDef],
) -> bool {
    let obj = match value {
        serde_value::Value::Map(map) => map,
        _ => return false,
    };

    // Unknown keys?
    for key in obj.keys() {
        let serde_value::Value::String(s) = key else {
            return false;
        };
        if !args.iter().any(|a| a.name == s.as_str()) {
            return false;
        }
    }

    // Required + type checks
    for arg in args {
        let v = obj.get(&serde_value::Value::String(arg.name.to_string()));
        let Some(v) = v else {
            if arg.required {
                return false;
            }
            continue;
        };
        // Type check
        let ok = if arg.list {
            match v {
                serde_value::Value::Seq(items) => {
                    items.iter().all(|item| scalar_type_ok(item, arg.ty, enums))
                }
                _ => false,
            }
        } else {
            scalar_type_ok(v, arg.ty, enums)
        };
        if !ok {
            return false;
        }
    }
    true
}

#[cfg(test)]
fn scalar_type_ok(v: &serde_value::Value, ty: &str, enums: &[echo_registry_api::EnumDef]) -> bool {
    match ty {
        "String" | "ID" => matches!(v, serde_value::Value::String(_)),
        "Boolean" => matches!(v, serde_value::Value::Bool(_)),
        "Int" => matches!(
            v,
            serde_value::Value::I8(_)
                | serde_value::Value::I16(_)
                | serde_value::Value::I32(_)
                | serde_value::Value::I64(_)
                | serde_value::Value::U8(_)
                | serde_value::Value::U16(_)
                | serde_value::Value::U32(_)
                | serde_value::Value::U64(_)
        ),
        "Float" => matches!(
            v,
            serde_value::Value::F32(_)
                | serde_value::Value::F64(_)
                | serde_value::Value::I8(_)
                | serde_value::Value::I16(_)
                | serde_value::Value::I32(_)
                | serde_value::Value::I64(_)
                | serde_value::Value::U8(_)
                | serde_value::Value::U16(_)
                | serde_value::Value::U32(_)
                | serde_value::Value::U64(_)
        ),
        other => {
            // enum check
            if let Some(def) = enums.iter().find(|e| e.name == other) {
                if let serde_value::Value::String(s) = v {
                    def.values.contains(&s.as_str())
                } else {
                    false
                }
            } else {
                false // unknown type -> reject to prevent schema drift
            }
        }
    }
}

#[cfg(test)]
mod schema_validation_tests {
    use super::*;
    use echo_registry_api::{ArgDef, EnumDef};
    use serde_value::Value as SV;
    use std::collections::BTreeMap;

    fn sv_map(entries: Vec<(SV, SV)>) -> SV {
        let mut map = BTreeMap::new();
        for (k, v) in entries {
            map.insert(k, v);
        }
        SV::Map(map)
    }

    fn enums_theme() -> Vec<EnumDef> {
        vec![EnumDef {
            name: "Theme",
            values: &["LIGHT", "DARK", "SYSTEM"],
        }]
    }

    #[test]
    fn reject_unknown_keys() {
        let args = vec![ArgDef {
            name: "path",
            ty: "String",
            required: true,
            list: false,
        }];
        let val_sv = sv_map(vec![
            (SV::String("path".into()), SV::String("ok".into())),
            (SV::String("extra".into()), SV::I64(1)),
        ]);
        assert!(!validate_object_against_args(
            &val_sv,
            &args,
            &enums_theme(),
        ));
    }

    #[test]
    fn reject_missing_required() {
        let args = vec![ArgDef {
            name: "path",
            ty: "String",
            required: true,
            list: false,
        }];
        let val_sv = sv_map(vec![]);
        assert!(!validate_object_against_args(
            &val_sv,
            &args,
            &enums_theme(),
        ));
    }

    #[test]
    fn reject_enum_mismatch() {
        let args = vec![ArgDef {
            name: "mode",
            ty: "Theme",
            required: true,
            list: false,
        }];
        let val_sv = sv_map(vec![(
            SV::String("mode".into()),
            SV::String("WRONG".into()),
        )]);
        assert!(!validate_object_against_args(
            &val_sv,
            &args,
            &enums_theme(),
        ));
    }

    #[test]
    fn reject_unknown_type() {
        let args = vec![ArgDef {
            name: "obj",
            ty: "AppState",
            required: true,
            list: false,
        }];
        let val_sv = sv_map(vec![(
            SV::String("obj".into()),
            sv_map(vec![(
                SV::String("routePath".into()),
                SV::String("/".into()),
            )]),
        )]);
        assert!(!validate_object_against_args(
            &val_sv,
            &args,
            &enums_theme(),
        ));
    }
}
