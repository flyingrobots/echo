//! wasm-bindgen bindings that expose the motion rewrite spike to tooling.
//!
//! The exported `WasmEngine` mirrors the C ABI surface so browser clients can
//! create entities, drive transactions, and read deterministic hashes.
#![deny(missing_docs)]

use std::cell::RefCell;
use std::rc::Rc;

use js_sys::Uint8Array;
use rmg_core::{
    ApplyResult, Engine, MOTION_RULE_NAME, NodeId, NodeRecord, TxId, build_motion_demo_engine,
    decode_motion_payload, encode_motion_payload, make_node_id, make_type_id,
};
use wasm_bindgen::prelude::*;

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
    "Position vector expressed in meters.\n\nProvides deterministic float32 components shared between host and Wasm callers. Each component must remain finite and encodes world-space coordinates in metres.\n\n# Usage\nPass a `Position` reference to `WasmEngine::spawn_motion_entity` to seed an entity's initial transform.\n\n# Example\n```\nlet position = Position::new(1.0, 2.0, 3.0);\n```\n",
    Position,
    "Creates a new position vector.",
    "Returns the X component in meters.",
    "Returns the Y component in meters.",
    "Returns the Z component in meters."
);

wasm_vector_type!(
    "Velocity vector expressed in meters/second.\n\nEncapsulates deterministic float32 velocity components used by the motion demo rewrite. Components must remain finite and describe linear velocity for each axis in metres/second.\n\n# Usage\nConstruct a `Velocity` and pass it by reference to `WasmEngine::spawn_motion_entity` alongside a `Position` to initialise entity motion.\n\n# Example\n```\nlet velocity = Velocity::new(0.5, -1.0, 0.25);\n```\n",
    Velocity,
    "Creates a new velocity vector.",
    "Returns the X component in meters/second.",
    "Returns the Y component in meters/second.",
    "Returns the Z component in meters/second."
);

impl Default for WasmEngine {
    fn default() -> Self {
        Self::new()
    }
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
        let payload = encode_motion_payload(position.components(), velocity.components());

        engine.insert_node(
            node_id,
            NodeRecord {
                ty: entity_type,
                payload: Some(payload),
            },
        );

        Uint8Array::from(node_id.0.as_slice())
    }

    #[wasm_bindgen]
    /// Begins a new transaction and returns its identifier.
    pub fn begin(&self) -> u64 {
        self.inner.borrow_mut().begin().0
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
        match engine.apply(TxId(tx_id), MOTION_RULE_NAME, &node_id) {
            Ok(ApplyResult::Applied) => true,
            Ok(ApplyResult::NoMatch) => false,
            Err(_) => false,
        }
    }

    #[wasm_bindgen]
    /// Commits the transaction and returns the resulting snapshot hash.
    pub fn commit(&self, tx_id: u64) -> Option<Vec<u8>> {
        if tx_id == 0 {
            return None;
        }
        let mut engine = self.inner.borrow_mut();
        let snapshot = engine.commit(TxId(tx_id)).ok()?;
        Some(snapshot.hash.to_vec())
    }

    #[wasm_bindgen]
    /// Reads the decoded position/velocity tuple for the provided entity.
    pub fn read_motion(&self, entity_id: &[u8]) -> Option<Box<[f32]>> {
        let engine = self.inner.borrow();
        let node_id = bytes_to_node_id(entity_id)?;
        let record = engine.node(&node_id)?;
        let payload = record.payload.as_ref()?;
        let (position, velocity) = decode_motion_payload(payload)?;
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
