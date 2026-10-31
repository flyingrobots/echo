// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Lowerer-owned capability admission for the generic pure/read profiles.
use super::{
    as_map, required_array, required_map, text_field, CanonicalValueV1 as Value, ProviderRefusalV1,
};

pub(super) fn validate(core: &Value) -> Result<(), ProviderRefusalV1> {
    const SUBJECT: &str = "core.required-capabilities";
    let capabilities = required_array(core, "requiredCoreCapabilities", SUBJECT)?;
    let variant_supported = match capabilities.as_slice() {
        [] => false,
        [Value::Text(name)] if name == "core.variant/v1" => true,
        _ => {
            return Err(super::super::unsupported_semantics(
                SUBJECT,
                "required capabilities contain an unknown, duplicate or malformed entry",
            ))
        }
    };
    let types = required_map(core, "types", SUBJECT)?;
    let declares_variant = as_map(types).is_some_and(|types| {
        types
            .iter()
            .any(|(_, definition)| text_field(definition, "kind") == Some("Variant"))
    });
    if declares_variant && !variant_supported {
        return Err(super::super::unsupported_semantics(
            SUBJECT,
            "a variant type table requires core.variant/v1",
        ));
    }
    Ok(())
}
