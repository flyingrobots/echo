// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Verifier-owned inventory of required capabilities, independent of lowering.
use super::{
    array_field, as_map, as_text, map_field, text_field, CanonicalValueV1 as Value,
    ProviderRefusalV1,
};
use std::collections::BTreeSet;

pub(super) fn validate(core: &Value) -> Result<(), ProviderRefusalV1> {
    let refused = || {
        super::super::unsupported_semantics(
            "core.required-capabilities",
            "required capability inventory is unsupported or incomplete",
        )
    };
    let mut supported = BTreeSet::new();
    for capability in array_field(core, "requiredCoreCapabilities").ok_or_else(refused)? {
        let capability = as_text(capability).ok_or_else(refused)?;
        if capability != "core.variant/v1" || !supported.insert(capability) {
            return Err(refused());
        }
    }
    let definitions = map_field(core, "types")
        .and_then(as_map)
        .ok_or_else(refused)?;
    for (_, definition) in definitions {
        if text_field(definition, "kind") == Some("Variant")
            && !supported.contains("core.variant/v1")
        {
            return Err(refused());
        }
    }
    Ok(())
}
