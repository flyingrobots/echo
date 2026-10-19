// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Pinned lawpack adapter correspondence, independent of instruction ordering.
use crate::edict_pure::{
    values::{array, bytes, exact_fields, field, map, require_text, text, text_field},
    EvaluationError as Error,
};
use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, digest_canonical_value_bytes_v1 as digest,
    CanonicalValueV1 as Value,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn validate<'a>(
    module: &Value,
    intent: &Value,
    target_artifact: &Value,
    target: &'a Value,
    exports: &'a Value,
    artifacts: &Value,
) -> Result<BTreeMap<&'a str, &'a Value>, Error> {
    let steps = array(field(target, "steps")?)?;
    if steps.is_empty() {
        return if artifacts == &Value::Null {
            Ok(BTreeMap::new())
        } else {
            Err(Error::InvalidArtifact)
        };
    }
    exact_fields(
        artifacts,
        &[
            "source_artifact",
            "lawpack_artifact",
            "adapter_artifact",
            "target_profile_artifact",
            "intrinsics_artifact",
        ],
    )?;
    let embedded = |key| decode(bytes(field(artifacts, key)?)?).map_err(|_| Error::InvalidArtifact);
    let source = embedded("source_artifact")?;
    let source = std::str::from_utf8(bytes(&source)?).map_err(|_| Error::InvalidArtifact)?;
    let lawpack = embedded("lawpack_artifact")?;
    let adapter = embedded("adapter_artifact")?;
    let target_profile = embedded("target_profile_artifact")?;
    let catalog = embedded("intrinsics_artifact")?;
    require_text(&target_profile, "apiVersion", "edict.target-profile/v1")?;
    let profile_coordinate = format!(
        "{}@{}",
        text_field(&target_profile, "id")?,
        text_field(&target_profile, "version")?
    );
    let profile_reference = field(target_artifact, "targetProfile")?;
    require_text(profile_reference, "id", &profile_coordinate)?;
    pinned(
        profile_reference,
        digest("edict.target-profile/v1", &target_profile).map_err(|_| Error::InvalidArtifact)?,
    )?;
    if !array(field(&target_profile, "acceptedCoreAbi")?)?
        .iter()
        .any(|value| text(value) == text_field(module, "apiVersion"))
    {
        return Err(Error::UnsupportedProgram);
    }
    require_text(&target_profile, "intrinsicNamespace", &profile_coordinate)?;
    let catalog_reference = field(&target_profile, "intrinsics")?;
    require_text(catalog_reference, "id", "echo.dpo.intrinsics/v1")?;
    pinned(
        catalog_reference,
        digest("echo.dpo.intrinsics/v1", &catalog).map_err(|_| Error::InvalidArtifact)?,
    )?;
    exact_fields(&catalog, &["apiVersion", "intrinsics"])?;
    require_text(&catalog, "apiVersion", "edict.target-profile.intrinsics/v1")?;
    let catalog = field(&catalog, "intrinsics")?;
    map(catalog)?;
    require_text(&lawpack, "apiVersion", "edict.lawpack/v1")?;
    require_text(&adapter, "apiVersion", "edict.lawpack-adapter/v1")?;
    require_text(&adapter, "class", "declarative")?;
    let coordinate = format!(
        "{}@{}",
        text_field(&lawpack, "id")?,
        text_field(&lawpack, "version")?
    );
    let identity = digest("edict.lawpack/v1", &lawpack).map_err(|_| Error::InvalidArtifact)?;
    let closure = field(target_artifact, "semanticClosure")?;
    let [reference] = array(field(closure, "lawpacks")?)? else {
        return Err(Error::UnsupportedProgram);
    };
    require_text(reference, "id", &coordinate)?;
    pinned(reference, identity)?;
    let mut imports = array(field(module, "imports")?)?
        .iter()
        .filter(|entry| text_field(entry, "kind") == Ok("lawpack"));
    let import = imports.next().ok_or(Error::InvalidArtifact)?;
    if imports.next().is_some() || field(import, "ref")? != reference {
        return Err(Error::UnsupportedProgram);
    }
    pinned(
        field(&lawpack, "exports")?,
        digest("edict.lawpack-exports/v1", exports).map_err(|_| Error::InvalidArtifact)?,
    )?;
    let mut matching = array(field(&lawpack, "targetAdapters")?)?
        .iter()
        .filter(|entry| {
            field(entry, "acceptedTargetProfile") == field(target_artifact, "targetProfile")
        });
    let entry = matching.next().ok_or(Error::InvalidArtifact)?;
    if field(&target_profile, "targetIr")? != field(entry, "acceptedTargetIr")? {
        return Err(Error::InvalidArtifact);
    }
    if matching.next().is_some() {
        return Err(Error::InvalidArtifact);
    }
    require_text(
        field(entry, "acceptedTargetIr")?,
        "id",
        text_field(target_artifact, "domain")?,
    )?;
    pinned(
        field(entry, "adapter")?,
        digest("edict.lawpack-adapter/v1", &adapter).map_err(|_| Error::InvalidArtifact)?,
    )?;
    let alias = alias(
        source,
        &coordinate,
        identity,
        text_field(module, "coordinate")?,
    )?;
    super::imported_types::validate(module, exports, &coordinate)?;
    let profile = unique_profile(&adapter, text_field(intent, "requiredOperationProfile")?)?;
    let authorized = array(field(profile, "semanticEffects")?)?;
    let nodes = array(field(field(intent, "body")?, "nodes")?)?;
    let mut signatures = BTreeMap::new();
    let steps = steps
        .iter()
        .map(|step| Ok((text_field(step, "id")?, step)))
        .collect::<Result<BTreeMap<_, _>, Error>>()?;
    for (core, id) in nodes.iter().zip(array(field(target, "executionOrder")?)?) {
        if text_field(core, "kind")? != "effect" {
            continue;
        }
        let lowered = *steps.get(text(id)?).ok_or(Error::InvalidArtifact)?;
        let name = resolve(text_field(core, "effect")?, &alias, &coordinate)?;
        if !authorized
            .iter()
            .any(|value| text(value) == Ok(name.as_str()))
        {
            return Err(Error::InvalidArtifact);
        }
        let implementation = field(field(&adapter, "effectImplementations")?, &name)?;
        let intrinsic_name = text_field(implementation, "targetIntrinsic")?;
        if !intrinsic_name
            .strip_prefix(&profile_coordinate)
            .is_some_and(|suffix| suffix.starts_with('.') && suffix.len() > 1)
        {
            return Err(Error::InvalidArtifact);
        }
        let intrinsic = field(catalog, intrinsic_name)?;
        require_text(intrinsic, "intrinsicClass", "effect")?;
        require_text(
            lowered,
            "targetIntrinsic",
            text_field(implementation, "targetIntrinsic")?,
        )?;
        let mut declarations = array(field(exports, "effects")?)?
            .iter()
            .filter(|effect| text_field(effect, "coordinate") == Ok(name.as_str()));
        let effect = declarations.next().ok_or(Error::InvalidArtifact)?;
        if declarations.next().is_some() {
            return Err(Error::InvalidArtifact);
        }
        super::intrinsic_contract::validate(intrinsic, effect, implementation)?;
        super::runtime_abi::validate(intrinsic_name, module, effect, intrinsic)?;
        require_text(
            field(core, "binding")?,
            "type",
            text_field(effect, "outputType")?,
        )?;
        signatures.insert(text(id)?, effect);
        let failures = field(effect, "effectFailures")?;
        let mappings = map(field(implementation, "failureMappings")?)?;
        let source_arms = field(core, "obstructionMap")?;
        let target_arms = field(lowered, "obstructionArms")?;
        if mappings.len() != map(failures)?.len()
            || mappings.len() != map(source_arms)?.len()
            || mappings.len() != map(target_arms)?.len()
        {
            return Err(Error::InvalidArtifact);
        }
        let mut seen = BTreeSet::new();
        for (failure, tag) in mappings {
            let tag = text(tag)?;
            if tag.is_empty() || !seen.insert(tag) {
                return Err(Error::InvalidArtifact);
            }
            let failure = text(failure)?;
            let arm = field(source_arms, failure)?;
            if arm != field(target_arms, tag)? {
                return Err(Error::InvalidArtifact);
            }
            require_text(
                field(arm, "binder")?,
                "type",
                text_field(field(failures, failure)?, "payloadType")?,
            )?;
            let reason = field(arm, "value")?;
            exact_fields(reason, &["kind", "callee", "args", "typeArgs"])?;
            require_text(reason, "kind", "call")?;
            if !array(field(reason, "args")?)?.is_empty()
                || !array(field(reason, "typeArgs")?)?.is_empty()
            {
                return Err(Error::UnsupportedProgram);
            }
            let coordinate = resolve(text_field(reason, "callee")?, &alias, &coordinate)?;
            let mut obstructions = array(field(exports, "obstructions")?)?
                .iter()
                .filter(|item| text_field(item, "coordinate") == Ok(coordinate.as_str()));
            let obstruction = obstructions.next().ok_or(Error::InvalidArtifact)?;
            if obstructions.next().is_some() {
                return Err(Error::InvalidArtifact);
            }
            let authority = text_field(field(failures, failure)?, "authorityClass")?;
            if !["domainMappable", "resourceFault", "integrityFault"].contains(&authority) {
                return Err(Error::InvalidArtifact);
            }
            require_text(obstruction, "authorityClass", authority)?;
        }
    }
    Ok(signatures)
}
fn pinned(reference: &Value, expected: [u8; 32]) -> Result<(), Error> {
    exact_fields(reference, &["id", "digest"])?;
    if text_field(reference, "id")?.is_empty() {
        return Err(Error::InvalidArtifact);
    }
    let [algorithm, value] = array(field(reference, "digest")?)? else {
        return Err(Error::InvalidArtifact);
    };
    if text(algorithm)? != "sha256" || bytes(value)? != expected {
        return Err(Error::InvalidArtifact);
    }
    Ok(())
}
fn resolve(name: &str, alias: &str, coordinate: &str) -> Result<String, Error> {
    if name
        .strip_prefix(coordinate)
        .is_some_and(|rest| rest.starts_with('.') && rest.len() > 1)
    {
        return Ok(name.to_owned());
    }
    let member = name
        .strip_prefix(alias)
        .and_then(|rest| rest.strip_prefix('.'))
        .filter(|rest| !rest.is_empty())
        .ok_or(Error::InvalidArtifact)?;
    Ok(format!("{coordinate}.{member}"))
}
fn unique_profile<'a>(adapter: &'a Value, core: &str) -> Result<&'a Value, Error> {
    let mut selected = None;
    for (_, profile) in map(field(adapter, "operationProfiles")?)? {
        if text_field(profile, "core")? == core && selected.replace(profile).is_some() {
            return Err(Error::InvalidArtifact);
        }
    }
    // The Core profile is already the lowered profile coordinate. Source aliases
    // are used only for effect names, never to select a different Core profile.
    selected.ok_or(Error::InvalidArtifact)
}
fn alias(
    source: &str,
    coordinate: &str,
    identity: [u8; 32],
    module: &str,
) -> Result<String, Error> {
    // Accept a bounded, explicit declaration preamble, not an import-looking
    // string inside a body or comment. A full source/Core proof remains separate.
    let mut lines = source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());
    let package = format!("package {module};");
    if lines.next() != Some(package.as_str()) {
        return Err(Error::UnsupportedProgram);
    }
    let expected = format!("\"sha256:{}\"", hex::encode(identity));
    let mut result = None;
    for line in lines {
        if !line.starts_with("use ") {
            break;
        }
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        let ["use", "lawpack", name, "digest", value, "as", alias] = tokens.as_slice() else {
            return Err(Error::UnsupportedProgram);
        };
        let alias = alias.strip_suffix(';').ok_or(Error::InvalidArtifact)?;
        if *name != coordinate
            || *value != expected
            || alias.is_empty()
            || !alias
                .bytes()
                .enumerate()
                .all(|(i, c)| c == b'_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit()))
            || result.replace(alias.to_owned()).is_some()
        {
            return Err(Error::InvalidArtifact);
        }
    }
    result.ok_or(Error::InvalidArtifact)
}
