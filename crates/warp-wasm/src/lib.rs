// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! wasm-bindgen bindings for warp-core engine.
//!
//! Provides WASM exports for browser clients to interact with the
//! deterministic engine and registry.
#![deny(missing_docs)]

use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

/// Placeholder ABI bytes for empty responses.
fn empty_bytes() -> Uint8Array {
    Uint8Array::new_with_length(0)
}

// -------------------------------------------------------------------------
// Registry provider
// -------------------------------------------------------------------------
//
// This crate intentionally avoids process-wide state. Apps should surface
// registry metadata and validation via their own WASM bindings instead.

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
    #[cfg(feature = "console-panic")]
    web_sys::console::error_1(
        &"execute_query: registry validation not wired in no-global build".into(),
    );
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
    #[cfg(feature = "console-panic")]
    web_sys::console::error_1(
        &"get_registry_info: registry metadata not wired in no-global build".into(),
    );
    empty_bytes()
}

/// Get the codec identifier from the installed registry.
#[wasm_bindgen]
pub fn get_codec_id() -> JsValue {
    JsValue::NULL
}

/// Get the registry version from the installed registry.
#[wasm_bindgen]
pub fn get_registry_version() -> JsValue {
    JsValue::NULL
}

/// Get the schema hash (hex) from the installed registry.
#[wasm_bindgen]
pub fn get_schema_sha256_hex() -> JsValue {
    JsValue::NULL
}

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
