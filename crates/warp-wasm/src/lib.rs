// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! wasm-bindgen bindings that expose the motion rewrite spike to tooling.
//!
//! The exported `WasmEngine` mirrors the C ABI surface so browser clients can
//! create entities, drive transactions, and read deterministic hashes.
#![deny(missing_docs)]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::OnceLock;

use echo_registry_api::RegistryProvider;
use echo_wasm_abi::decode_cbor;
use js_sys::Uint8Array;
use warp_core::{
    build_motion_demo_engine,
    decode_motion_atom_payload,
    encode_motion_atom_payload,
    make_node_id,
    make_type_id,
    ApplyResult,
    AttachmentValue,
    Engine,
    NodeId,
    NodeRecord,
    TxId,
    MOTION_RULE_NAME,
};
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

/// Install an application-supplied registry provider. App code should call
/// this once at startup (see flyingrobots-echo-wasm).
pub fn install_registry(provider: &'static dyn RegistryProvider) {
    if REGISTRY.set(provider).is_err() {
        panic!("registry already installed");
    }
}

fn registry() -> Option<&'static dyn RegistryProvider> {
    REGISTRY.get().copied()
}

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

// Generates a 3D vector type with wasm_bindgen bindings.
macro_rules! wasm_vector_type {
    ($struct_doc:literal, $name:ident, $ctor_doc:literal, $x_doc:literal, $y_doc:literal, $z_doc:literal) => {
        #[wasm_bindgen]
        #[doc = $struct_doc]
        pub struct $name {
            x: f32,
            y: f32,
            z: f32,
        }

        #[wasm_bindgen]
        impl $name {
            #[wasm_bindgen(constructor)]
            #[doc = $ctor_doc]
            pub fn new(x: f32, y: f32, z: f32) -> $name {
                assert!(
                    x.is_finite(),
                    concat!(stringify!($name), " x component must be finite")
                );
                assert!(
                    y.is_finite(),
                    concat!(stringify!($name), " y component must be finite")
                );
                assert!(
                    z.is_finite(),
                    concat!(stringify!($name), " z component must be finite")
                );
                $name { x, y, z }
            }

            #[wasm_bindgen(getter)]
            #[doc = $x_doc]
            pub fn x(&self) -> f32 {
                self.x
            }

            #[wasm_bindgen(getter)]
            #[doc = $y_doc]
            pub fn y(&self) -> f32 {
                self.y
            }

            #[wasm_bindgen(getter)]
            #[doc = $z_doc]
            pub fn z(&self) -> f32 {
                self.z
            }
        }

        impl $name {
            pub(crate) fn components(&self) -> [f32; 3] {
                [self.x, self.y, self.z]
            }
        }
    };
}

/// Builds a fresh engine with the motion rule pre-registered.
fn build_engine() -> Engine {
    build_motion_demo_engine()
}

#[cfg(feature = "console-panic")]
#[wasm_bindgen(start)]
pub fn init_console_panic_hook() {
    console_error_panic_hook::set_once();
}

/// Converts a 32-byte buffer into a [`NodeId`].
fn bytes_to_node_id(bytes: &[u8]) -> Option<NodeId> {
    if bytes.len() != 32 {
        return None;
    }
    let mut id = [0u8; 32];
    id.copy_from_slice(bytes);
    Some(NodeId(id))
}

/// WASM-friendly wrapper around the deterministic engine.
#[wasm_bindgen]
pub struct WasmEngine {
    inner: Rc<RefCell<Engine>>,
}
wasm_vector_type!(
    "Position vector expressed in meters.\n\nProvides deterministic float32 components shared between host and Wasm callers. Callers must supply finite values; non-finite components will cause construction to panic.\n\n# Usage\nPass a `Position` reference to `WasmEngine::spawn_motion_entity` to seed an entity's initial transform.\n\n# Example\n```\nuse warp_wasm::Position;\nlet position = Position::new(1.0, 2.0, 3.0);\n```\n",
    Position,
    "Creates a new position vector.",
    "Returns the X component in meters.",
    "Returns the Y component in meters.",
    "Returns the Z component in meters."
);

wasm_vector_type!(
    "Velocity vector expressed in meters/second.\n\nEncapsulates deterministic float32 velocity components used by the motion demo rewrite. Callers must supply finite values; non-finite components will cause construction to panic.\n\n# Usage\nConstruct a `Velocity` and pass it by reference to `WasmEngine::spawn_motion_entity` alongside a `Position` to initialise entity motion.\n\n# Example\n```\nuse warp_wasm::Velocity;\nlet velocity = Velocity::new(0.5, -1.0, 0.25);\n```\n",
    Velocity,
    "Creates a new velocity vector.",
    "Returns the X component in meters/second.",
    "Returns the Y component in meters/second.",
    "Returns the Z component in meters/second."
);

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


#[wasm_bindgen]
impl WasmEngine {
    #[wasm_bindgen(constructor)]
    /// Creates a new engine with the motion rule registered.
    pub fn new() -> WasmEngine {
        WasmEngine {
            inner: Rc::new(RefCell::new(build_engine())),
        }
    }

    #[wasm_bindgen]
    /// Spawns an entity with encoded motion payload.
    ///
    /// * `label` – stable identifier used to derive the entity node id. Must be
    ///   unique for the caller's scope.
    /// * `position` – initial position in meters.
    /// * `velocity` – velocity components in meters/second.
    ///
    /// Returns the 32-byte node id as a `Uint8Array` for JavaScript consumers.
    pub fn spawn_motion_entity(
        &self,
        label: &str,
        position: &Position,
        velocity: &Velocity,
    ) -> Uint8Array {
        let mut engine = self.inner.borrow_mut();
        let node_id = make_node_id(label);
        let entity_type = make_type_id("entity");
        let payload = encode_motion_atom_payload(position.components(), velocity.components());

        if let Err(_err) = engine.insert_node_with_attachment(
            node_id,
            NodeRecord { ty: entity_type },
            Some(AttachmentValue::Atom(payload)),
        ) {
            #[cfg(feature = "console-panic")]
            web_sys::console::error_1(
                &format!("spawn_motion_entity failed for node {node_id:?}: {_err:?}").into(),
            );
            return Uint8Array::new_with_length(0);
        }

        Uint8Array::from(node_id.as_bytes().as_slice())
    }

    #[wasm_bindgen]
    /// Begins a new transaction and returns its identifier.
    pub fn begin(&self) -> u64 {
        self.inner.borrow_mut().begin().value()
    }

    #[wasm_bindgen]
    /// Applies the motion rewrite to the entity identified by `entity_id`.
    ///
    /// Returns `true` on success and `false` if the transaction id, entity id,
    /// or rule match is invalid. Future revisions will surface richer error
    /// information.
    pub fn apply_motion(&self, tx_id: u64, entity_id: &[u8]) -> bool {
        if tx_id == 0 {
            return false;
        }
        let node_id = match bytes_to_node_id(entity_id) {
            Some(id) => id,
            None => return false,
        };
        let mut engine = self.inner.borrow_mut();
        match engine.apply(TxId::from_raw(tx_id), MOTION_RULE_NAME, &node_id) {
            Ok(ApplyResult::Applied) => true,
            Ok(ApplyResult::NoMatch) => false,
            Err(_) => false,
        }
    }

    /// Commits the transaction and returns the resulting snapshot hash.
    #[wasm_bindgen]
    pub fn commit(&self, tx_id: u64) -> Option<Vec<u8>> {
        if tx_id == 0 {
            return None;
        }
        let mut engine = self.inner.borrow_mut();
        let snapshot = engine.commit(TxId::from_raw(tx_id)).ok()?;
        Some(snapshot.hash.to_vec())
    }

    /// Returns the sequential history of all committed ticks.
    #[wasm_bindgen]
    pub fn get_ledger(&self) -> JsValue {
        let engine = self.inner.borrow();
        let ledger = engine.get_ledger();
        let serializable: Vec<warp_core::SerializableTick> = ledger
            .iter()
            .map(|(s, r, p)| warp_core::SerializableTick::from_parts(s, r, p))
            .collect();
        serde_wasm_bindgen::to_value(&serializable).unwrap_or(JsValue::NULL)
    }

    /// Reads the decoded position/velocity tuple for the provided entity.
    pub fn read_motion(&self, entity_id: &[u8]) -> Option<Box<[f32]>> {
        let engine = self.inner.borrow();
        let node_id = bytes_to_node_id(entity_id)?;
        let payload = match engine.node_attachment(&node_id) {
            Ok(Some(value)) => value,
            Ok(None) => return None,
            Err(_) => return None,
        };
        let AttachmentValue::Atom(payload) = payload else {
            return None;
        };
        let (position, velocity) = decode_motion_atom_payload(payload)?;
        let mut data = Vec::with_capacity(6);
        data.extend_from_slice(&position);
        data.extend_from_slice(&velocity);
        Some(data.into_boxed_slice())
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    fn spawn(engine: &WasmEngine) -> Vec<u8> {
        let position = Position::new(1.0, 2.0, 3.0);
        let velocity = Velocity::new(0.5, -1.0, 0.25);
        engine
            .spawn_motion_entity("entity-wasm", &position, &velocity)
            .to_vec()
    }

    #[wasm_bindgen_test]
    fn wasm_motion_is_deterministic() {
        let engine_a = WasmEngine::new();
        let handle_a = spawn(&engine_a);
        let tx_a = engine_a.begin();
        assert!(engine_a.apply_motion(tx_a, &handle_a));
        let hash_a = engine_a.commit(tx_a).expect("snapshot");

        let engine_b = WasmEngine::new();
        let handle_b = spawn(&engine_b);
        let tx_b = engine_b.begin();
        assert!(engine_b.apply_motion(tx_b, &handle_b));
        let hash_b = engine_b.commit(tx_b).expect("snapshot");

        assert_eq!(hash_a, hash_b);

        let motion = engine_a.read_motion(&handle_a).expect("motion payload");
        assert!((motion[0] - 1.5).abs() < 1e-6);
        assert!((motion[1] - 1.0).abs() < 1e-6);
        assert!((motion[2] - 3.25).abs() < 1e-6);
        assert!((motion[3] - 0.5).abs() < 1e-6);
        assert!((motion[4] + 1.0).abs() < 1e-6);
        assert!((motion[5] - 0.25).abs() < 1e-6);
    }
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
