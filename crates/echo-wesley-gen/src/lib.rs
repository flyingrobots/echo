// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Adapter from Wesley-compiled runtime optic artifacts into Echo runtime types.
//!
//! Wesley owns compiled artifact truth: artifact hashes, schema ids, operation
//! ids, requirements digests, and registration descriptors. `warp-core` owns
//! runtime registration and opaque handles. This crate is the dependency seam
//! that may see both sides.
//!
//! The v0 adapter stores Wesley admission requirements as deterministic
//! `serde_json` bytes in `warp-core`. Those bytes are registry payload only;
//! enforcement, grant validation, admission tickets, witnesses, and execution
//! are intentionally out of scope for this adapter.

/// Adapter-local staging codec for imported Wesley admission requirements.
///
/// This is not runtime admission truth. It is a v0 canonical staging format so
/// Echo can store opaque requirements bytes while Wesley grows a durable
/// canonical requirements byte/codec surface.
pub const WESLEY_REQUIREMENTS_STAGING_CODEC_V0: &str =
    "echo-wesley-gen/wesley-requirements-canonical-json-staging/v0";

/// Imported Wesley runtime optic artifact ready for Echo registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedRuntimeOpticArtifact {
    /// Echo runtime artifact shape.
    pub artifact: warp_core::OpticArtifact,
    /// Wesley registration descriptor mirrored into Echo's registration shape.
    pub descriptor: warp_core::OpticRegistrationDescriptor,
}

/// Imports a Wesley-compiled runtime optic artifact into Echo runtime structs.
///
/// This does not register the artifact and does not issue authority. Echo still
/// verifies the descriptor through [`warp_core::OpticArtifactRegistry`] and
/// returns the opaque runtime-local handle only after registration succeeds.
pub fn import_runtime_optic_artifact(
    artifact: &wesley_core::OpticArtifact,
) -> anyhow::Result<ImportedRuntimeOpticArtifact> {
    let requirements_bytes = canonicalize_wesley_requirements_v0(&artifact.requirements)?;

    Ok(ImportedRuntimeOpticArtifact {
        artifact: warp_core::OpticArtifact {
            artifact_id: artifact.artifact_id.clone(),
            artifact_hash: artifact.artifact_hash.clone(),
            schema_id: artifact.schema_id.clone(),
            requirements_digest: artifact.requirements_digest.clone(),
            operation: warp_core::OpticArtifactOperation {
                operation_id: artifact.operation.operation_id.clone(),
            },
            requirements: warp_core::OpticAdmissionRequirements {
                bytes: requirements_bytes,
            },
        },
        descriptor: import_registration_descriptor(&artifact.registration),
    })
}

/// Canonicalizes Wesley admission requirements for adapter-local v0 staging.
///
/// The helper deliberately names the seam: these bytes are not permanent
/// artifact truth and `warp-core` must not interpret them for admission.
/// Durable admission should eventually consume Wesley-owned canonical
/// requirements bytes plus an explicit codec id.
pub fn canonicalize_wesley_requirements_v0(
    requirements: &wesley_core::OpticAdmissionRequirements,
) -> anyhow::Result<Vec<u8>> {
    let value = serde_json::to_value(requirements)?;
    let mut bytes = Vec::new();
    write_canonical_json_value(&value, &mut bytes)?;
    Ok(bytes)
}

/// Imports a Wesley registration descriptor into Echo's registration shape.
pub fn import_registration_descriptor(
    descriptor: &wesley_core::OpticRegistrationDescriptor,
) -> warp_core::OpticRegistrationDescriptor {
    warp_core::OpticRegistrationDescriptor {
        artifact_id: descriptor.artifact_id.clone(),
        artifact_hash: descriptor.artifact_hash.clone(),
        schema_id: descriptor.schema_id.clone(),
        operation_id: descriptor.operation_id.clone(),
        requirements_digest: descriptor.requirements_digest.clone(),
    }
}

fn write_canonical_json_value(
    value: &serde_json::Value,
    bytes: &mut Vec<u8>,
) -> anyhow::Result<()> {
    match value {
        serde_json::Value::Null => bytes.extend_from_slice(b"null"),
        serde_json::Value::Bool(true) => bytes.extend_from_slice(b"true"),
        serde_json::Value::Bool(false) => bytes.extend_from_slice(b"false"),
        serde_json::Value::Number(number) => {
            bytes.extend_from_slice(serde_json::to_string(number)?.as_bytes());
        }
        serde_json::Value::String(text) => {
            bytes.extend_from_slice(serde_json::to_string(text)?.as_bytes());
        }
        serde_json::Value::Array(values) => {
            bytes.push(b'[');
            for (index, item) in values.iter().enumerate() {
                if index > 0 {
                    bytes.push(b',');
                }
                write_canonical_json_value(item, bytes)?;
            }
            bytes.push(b']');
        }
        serde_json::Value::Object(object) => {
            let mut entries: Vec<_> = object.iter().collect();
            entries.sort_by(|(left_key, _), (right_key, _)| left_key.cmp(right_key));

            bytes.push(b'{');
            for (index, (key, item)) in entries.into_iter().enumerate() {
                if index > 0 {
                    bytes.push(b',');
                }
                bytes.extend_from_slice(serde_json::to_string(key)?.as_bytes());
                bytes.push(b':');
                write_canonical_json_value(item, bytes)?;
            }
            bytes.push(b'}');
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::write_canonical_json_value;

    #[test]
    fn canonical_json_writer_sorts_object_keys_recursively() -> anyhow::Result<()> {
        let left = serde_json::json!({
            "z": [{"b": 2, "a": 1}],
            "a": {"d": false, "c": true}
        });
        let right = serde_json::json!({
            "a": {"c": true, "d": false},
            "z": [{"a": 1, "b": 2}]
        });
        let mut left_bytes = Vec::new();
        let mut right_bytes = Vec::new();

        write_canonical_json_value(&left, &mut left_bytes)?;
        write_canonical_json_value(&right, &mut right_bytes)?;

        assert_eq!(left_bytes, right_bytes);
        assert_eq!(
            left_bytes,
            br#"{"a":{"c":true,"d":false},"z":[{"a":1,"b":2}]}"#
        );

        Ok(())
    }
}
