// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Explicit catalog transport and profile binding, without execution authority.

use super::{
    as_map, canonical_text, invalid_artifact, map_field, require_exact_fields, text_field,
    validate_bound, BoundArtifact, CanonicalValueV1 as Value, ProviderRefusalV1, CONFIGURATION_ABI,
};
use echo_edict_canonical::{decode_canonical_cbor_v1, digest_canonical_value_bytes_v1};

const SUBJECT: &str = "effect-target-resources";
const CATALOG_DOMAIN: &str = "echo.dpo.intrinsics/v1";
const MAX_CATALOG_BYTES: usize = 16 * 1024 * 1024;

pub(super) fn is_configuration(value: &Value) -> bool {
    text_field(value, "apiVersion") == Some(CONFIGURATION_ABI)
        && text_field(value, "programKind") == Some("edict-effect-program/v1")
}

fn invalid() -> ProviderRefusalV1 {
    invalid_artifact(
        SUBJECT,
        "the explicit canonical intrinsic catalog must match the selected target profile",
    )
}

pub(super) fn validate(
    configuration: &Value,
    profile: &BoundArtifact,
) -> Result<(), ProviderRefusalV1> {
    let Some(Value::Bytes(bytes)) = map_field(configuration, "intrinsicCatalog") else {
        return Err(invalid());
    };
    if bytes.is_empty() || bytes.len() > MAX_CATALOG_BYTES {
        return Err(invalid());
    }
    let catalog = decode_canonical_cbor_v1(bytes).map_err(|_| invalid())?;
    require_exact_fields(&catalog, &["apiVersion", "intrinsics"], SUBJECT)
        .map_err(|_| invalid())?;
    if text_field(&catalog, "apiVersion") != Some("edict.target-profile.intrinsics/v1")
        || map_field(&catalog, "intrinsics").and_then(as_map).is_none()
    {
        return Err(invalid());
    }
    let profile = validate_bound(profile, "edict.target-profile/v1")?;
    let reference = map_field(&profile, "intrinsics").ok_or_else(invalid)?;
    require_exact_fields(reference, &["id", "digest"], SUBJECT).map_err(|_| invalid())?;
    let identity =
        digest_canonical_value_bytes_v1(CATALOG_DOMAIN, &catalog).map_err(|_| invalid())?;
    let expected = Value::Array(vec![
        canonical_text("sha256"),
        Value::Bytes(identity.to_vec()),
    ]);
    if text_field(reference, "id") != Some(CATALOG_DOMAIN)
        || map_field(reference, "digest") != Some(&expected)
    {
        return Err(invalid());
    }
    Ok(())
}
