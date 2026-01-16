// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! wasm-bindgen bindings for warp-core engine.
//!
//! Provides WASM exports for browser clients to interact with the
//! deterministic engine and registry.
#![deny(missing_docs)]

use std::sync::OnceLock;

use echo_registry_api::RegistryProvider;
use echo_wasm_abi::{decode_cbor, encode_cbor};
use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

/// Placeholder ABI bytes for empty responses.
fn empty_bytes() -> Uint8Array {
    Uint8Array::new_with_length(0)
}

// -------------------------------------------------------------------------
// Registry provider (placeholder until app-supplied registry is linked).
// -------------------------------------------------------------------------

static REGISTRY: OnceLock<&'static dyn RegistryProvider> = OnceLock::new();

/// Install an application-supplied registry provider.
///
/// Must be called exactly once at application startup before any registry-dependent
/// operations (e.g., `execute_query`, `encode_command`). The provider must have
/// `'static` lifetime as it's stored in a global `OnceLock`.
///
/// # Panics
///
/// Panics if called more than once (registry already installed).
///
/// # Thread Safety
///
/// Uses `OnceLock` internally, so the first successful call wins in concurrent scenarios.
pub fn install_registry(provider: &'static dyn RegistryProvider) {
    if REGISTRY.set(provider).is_err() {
        panic!("registry already installed");
    }
}

fn registry() -> Option<&'static dyn RegistryProvider> {
    REGISTRY.get().copied()
}

/// Validates a serde value against an argument definition list.
///
/// Returns `true` if `value` is a map where every key corresponds to an `ArgDef`,
/// all required fields are present, and all values pass type checks against their
/// declared types (including enum validation).
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

/// Checks if a scalar value matches the expected GraphQL type.
///
/// Handles String, ID, Boolean, Int, Float, and enum types. For enums, validates
/// that the string value is a member of the enum's defined values.
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

#[cfg(feature = "console-panic")]
#[wasm_bindgen(start)]
/// Initialize console panic hook for better error messages in browser.
pub fn init_console_panic_hook() {
    console_error_panic_hook::set_once();
}

// -----------------------------------------------------------------------------
// Frozen ABI exports (website kernel spike)
// -----------------------------------------------------------------------------

/// Enqueue a canonical intent payload (opaque bytes). Placeholder: currently no-op.
#[wasm_bindgen]
pub fn dispatch_intent(_intent_bytes: &[u8]) {
    // TODO: wire to ingest_inbox_event once kernel plumbing lands in warp-wasm.
}

/// Run deterministic steps up to a budget. Placeholder: returns empty StepResult bytes.
#[wasm_bindgen]
pub fn step(_step_budget: u32) -> Uint8Array {
    empty_bytes()
}

/// Drain emitted ViewOps since last drain. Placeholder: returns empty array.
#[wasm_bindgen]
pub fn drain_view_ops() -> Uint8Array {
    empty_bytes()
}

/// Get head info (tick/seq/state root). Placeholder: returns empty bytes.
#[wasm_bindgen]
pub fn get_head() -> Uint8Array {
    empty_bytes()
}

/// Execute a read-only query by ID with canonical vars.
#[wasm_bindgen]
pub fn execute_query(_query_id: u32, _vars_bytes: &[u8]) -> Uint8Array {
    let reg = registry().expect("registry not installed");
    let Some(op) = reg.op_by_id(_query_id) else {
        #[cfg(feature = "console-panic")]
        web_sys::console::error_1(&format!("execute_query: unknown op_id {_query_id}").into());
        return empty_bytes();
    };

    // PURITY GUARD: Refuse to execute Mutations via execute_query.
    if op.kind != echo_registry_api::OpKind::Query {
        #[cfg(feature = "console-panic")]
        web_sys::console::error_1(
            &format!(
                "execute_query purity violation: op '{}' is a Mutation",
                op.name
            )
            .into(),
        );
        return empty_bytes();
    }

    // Decode and validate vars against schema
    let Ok(value) = decode_cbor::<serde_value::Value>(_vars_bytes) else {
        #[cfg(feature = "console-panic")]
        web_sys::console::error_1(&"execute_query: failed to decode CBOR vars".into());
        return empty_bytes();
    };
    if !validate_object_against_args(&value, op.args, reg.all_enums()) {
        #[cfg(feature = "console-panic")]
        web_sys::console::error_1(
            &format!(
                "execute_query: schema validation failed for op '{}'",
                op.name
            )
            .into(),
        );
        return empty_bytes();
    }

    // TODO: execute against read-only graph once available.
    empty_bytes()
}

/// Snapshot at a tick (sandbox replay). Placeholder: returns empty bytes.
#[wasm_bindgen]
pub fn snapshot_at(_tick: u64) -> Uint8Array {
    empty_bytes()
}

/// Render a snapshot to ViewOps. Placeholder: returns empty bytes.
#[wasm_bindgen]
pub fn render_snapshot(_snapshot_bytes: &[u8]) -> Uint8Array {
    empty_bytes()
}

/// Return registry metadata (schema hash, codec id, registry version).
#[wasm_bindgen]
pub fn get_registry_info() -> Uint8Array {
    let reg = registry().expect("registry not installed");
    let info = reg.info();
    #[derive(serde::Serialize)]
    struct Info<'a> {
        codec_id: &'a str,
        registry_version: u32,
        schema_sha256_hex: &'a str,
    }
    let dto = Info {
        codec_id: info.codec_id,
        registry_version: info.registry_version,
        schema_sha256_hex: info.schema_sha256_hex,
    };
    match encode_cbor(&dto) {
        Ok(bytes) => Uint8Array::from(bytes.as_slice()),
        Err(_) => empty_bytes(),
    }
}

#[wasm_bindgen]
/// Get the codec identifier from the installed registry.
pub fn get_codec_id() -> JsValue {
    registry()
        .map(|r| JsValue::from_str(r.info().codec_id))
        .unwrap_or_else(|| JsValue::NULL)
}

#[wasm_bindgen]
/// Get the registry version from the installed registry.
pub fn get_registry_version() -> JsValue {
    registry()
        .map(|r| JsValue::from_f64(r.info().registry_version as f64))
        .unwrap_or_else(|| JsValue::NULL)
}

#[wasm_bindgen]
/// Get the schema hash (hex) from the installed registry.
pub fn get_schema_sha256_hex() -> JsValue {
    registry()
        .map(|r| JsValue::from_str(r.info().schema_sha256_hex))
        .unwrap_or_else(|| JsValue::NULL)
}

#[cfg(test)]
mod schema_validation_tests {
    use super::*;
    use echo_registry_api::{ArgDef, EnumDef};

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
        let val = serde_json::json!({"path":"ok","extra":1});
        let val_sv: serde_value::Value = serde_value::to_value(val).unwrap();
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
        let val = serde_json::json!({});
        let val_sv: serde_value::Value = serde_value::to_value(val).unwrap();
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
        let val = serde_json::json!({"mode":"WRONG"});
        let val_sv: serde_value::Value = serde_value::to_value(val).unwrap();
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
        let val = serde_json::json!({"obj":{"routePath":"/"}});
        let val_sv: serde_value::Value = serde_value::to_value(val).unwrap();
        assert!(!validate_object_against_args(
            &val_sv,
            &args,
            &enums_theme(),
        ));
    }
}
