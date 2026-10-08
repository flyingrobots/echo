// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Independent identity-preserving Core-to-Target relation for the pure v1 route.
//!
//! This route permits no effects, guards, rewriting, or reordered bindings.
//! Owning-schema admission remains the host's responsibility; artifact hashes
//! alone cannot establish the semantic correspondence checked here.

use echo_edict_canonical::CanonicalValueV1 as Value;

use crate::{array_field, map_field, text_field};

pub(super) fn preserves(core: &Value, target: &Value, intent_name: &str) -> bool {
    let Some(Value::Map(intents)) = map_field(target, "intents") else {
        return false;
    };
    if intents.len() != 1 {
        return false;
    }
    let Some(target) = map_field(target, "intents").and_then(|v| map_field(v, intent_name)) else {
        return false;
    };
    let Some(body) = map_field(core, "body") else {
        return false;
    };
    let (Some(nodes), Some(constraints), Some(requirements)) = (
        array_field(body, "nodes"),
        array_field(core, "inputConstraints"),
        array_field(target, "requirements"),
    ) else {
        return false;
    };
    if !requirements.is_empty()
        || array_field(target, "inputConstraints") != Some(constraints)
        || map_field(core, "basis") != map_field(target, "basis")
        || !equal_present(body, "result", target, "result")
    {
        return false;
    }
    // Canonical v1 omits pureBindings when the source has no let nodes.
    let bindings = match map_field(target, "pureBindings") {
        Some(Value::Array(bindings)) => bindings.as_slice(),
        None => &[],
        _ => return false,
    };
    nodes.len() == bindings.len()
        && nodes
            .iter()
            .zip(bindings)
            .enumerate()
            .all(|(index, (node, binding))| {
                text_field(node, "kind") == Some("let")
                    && text_field(binding, "id")
                        == Some(format!("{intent_name}.binding.{index}").as_str())
                    && equal_present(node, "binding", binding, "binding")
                    && equal_present(node, "value", binding, "value")
            })
}

fn equal_present(left: &Value, left_key: &str, right: &Value, right_key: &str) -> bool {
    map_field(left, left_key).is_some_and(|value| Some(value) == map_field(right, right_key))
}
