// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Regression tests for Echo-owned capability grant storage.

use warp_core::{CapabilityGrant, CapabilityGrantRegistry, CapabilityGrantRegistryError};

fn fixture_grant(grant_id: &str) -> CapabilityGrant {
    CapabilityGrant {
        grant_id: grant_id.to_owned(),
        subject: "subject:jedit-session".to_owned(),
        artifact_hash: "artifact-hash:stack-witness-0001".to_owned(),
        operation_id: "operation:textWindow:v0".to_owned(),
        requirements_digest: "requirements-digest:stack-witness-0001".to_owned(),
        rights: vec!["optic.invoke".to_owned()],
        scope_bytes: b"scope:fixture".to_vec(),
        budget_bytes: b"budget:fixture".to_vec(),
    }
}

#[test]
fn capability_grant_registry_registers_and_resolves_grant() -> Result<(), String> {
    let mut registry = CapabilityGrantRegistry::new();
    let grant = fixture_grant("grant:fixture");

    registry
        .register_capability_grant(grant.clone())
        .map_err(|err| format!("fixture grant should register: {err:?}"))?;

    assert_eq!(
        registry
            .resolve_capability_grant("grant:fixture")
            .map_err(|err| format!("fixture grant should resolve: {err:?}"))?,
        &grant
    );
    Ok(())
}

#[test]
fn capability_grant_registry_rejects_duplicate_grant_id() -> Result<(), String> {
    let mut registry = CapabilityGrantRegistry::new();

    registry
        .register_capability_grant(fixture_grant("grant:duplicate"))
        .map_err(|err| format!("first fixture grant should register: {err:?}"))?;
    let duplicate_result = registry.register_capability_grant(fixture_grant("grant:duplicate"));
    let err = match duplicate_result {
        Ok(()) => return Err("duplicate grant id should reject".to_owned()),
        Err(err) => err,
    };

    assert_eq!(err, CapabilityGrantRegistryError::DuplicateGrantId);
    Ok(())
}

#[test]
fn capability_grant_registry_rejects_unknown_grant_lookup() -> Result<(), String> {
    let registry = CapabilityGrantRegistry::new();

    let err = match registry.resolve_capability_grant("grant:missing") {
        Ok(grant) => return Err(format!("unknown grant id should reject, got {grant:?}")),
        Err(err) => err,
    };

    assert_eq!(err, CapabilityGrantRegistryError::UnknownGrantId);
    Ok(())
}
