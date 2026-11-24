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
    intent: &'a Value,
    target_artifact: &Value,
    target: &'a Value,
    exports: &'a Value,
    artifacts: &Value,
) -> Result<BTreeMap<&'a str, &'a Value>, Error> {
    let steps = array(field(target, "steps")?)?;
    let structured =
        text_field(target_artifact, "domain")? == super::control_order::STRUCTURED_TARGET_IR;
    if steps.is_empty() {
        // Without an effect step there are no lowering artifacts, so no
        // profile is authenticated. Keep this path on the frozen @1 profile;
        // @2 requires profile authentication from the effect artifacts.
        let profile = field(target_artifact, "targetProfile")?;
        if structured || text_field(profile, "id")? != "echo.dpo@1" || calls_identity(module, 0)? {
            return Err(Error::UnsupportedProgram);
        }
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
    let sources = super::source_modules::Sources::new(&source, text_field(module, "coordinate")?)?;
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
    // Only Echo's two effect namespaces name runtime intrinsics; each pins
    // its own catalog coordinate, which is also the catalog's digest domain.
    let namespace =
        super::intrinsic_names::namespace(&profile_coordinate).ok_or(Error::UnsupportedProgram)?;
    // `echo.dpo@1` is frozen; only `echo.dpo@2` carries structured control
    // (owner decision on flyingrobots/echo#947).
    if structured && !namespace.structured {
        return Err(Error::UnsupportedProgram);
    }
    let catalog_reference = field(&target_profile, "intrinsics")?;
    require_text(catalog_reference, "id", namespace.catalog)?;
    pinned(
        catalog_reference,
        digest(namespace.catalog, &catalog).map_err(|_| Error::InvalidArtifact)?,
    )?;
    // Catalog v3 carries its template `parameters` and may also publish the
    // named types its pure contracts reference; the frozen v1 catalog has
    // exactly two fields.
    let templated = namespace.catalog_api == "edict.target-profile.intrinsics/v3";
    if templated {
        let typed = field(&catalog, "types").is_ok();
        let fields: &[&str] = if typed {
            &["apiVersion", "intrinsics", "parameters", "types"]
        } else {
            &["apiVersion", "intrinsics", "parameters"]
        };
        exact_fields(&catalog, fields)?;
        map(field(&catalog, "parameters")?)?;
        if typed {
            map(field(&catalog, "types")?)?;
        }
    } else {
        exact_fields(&catalog, &["apiVersion", "intrinsics"])?;
    }
    require_text(&catalog, "apiVersion", namespace.catalog_api)?;
    if !identity_bound(namespace.coordinate, &catalog, module, profile_reference)?
        && calls_identity(module, 0)?
    {
        return Err(Error::UnsupportedProgram);
    }
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
    owner_pinned(field(&lawpack, "exports")?, exports)?;
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
    owner_pinned(field(entry, "adapter")?, &adapter)?;
    let preamble =
        |module, source| preamble(source, &coordinate, identity, module, profile_reference);
    let (alias, root_pins) = preamble(text_field(module, "coordinate")?, sources.root())?;
    let alias = alias.ok_or(Error::InvalidArtifact)?;
    sources.require_pinned(root_pins, |module, source| {
        preamble(module, source).map(|(_, pins)| pins)
    })?;
    super::imported_types::validate(module, exports, &coordinate)?;
    let profile = unique_profile(&adapter, text_field(intent, "requiredOperationProfile")?)?;
    let authorized = array(field(profile, "semanticEffects")?)?;
    let mut signatures = BTreeMap::new();
    for (core, lowered) in super::control_order::instructions(intent, target)? {
        if text_field(core, "kind")? != "effect" {
            continue;
        }
        let id = text_field(lowered, "id")?;
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
        super::intrinsic_contract::validate(
            intrinsic_name,
            intrinsic,
            effect,
            implementation,
            templated,
        )?;
        super::runtime_abi::validate(intrinsic_name, module, effect, intrinsic)?;
        if super::mutations::MutationKind::from_intrinsic(intrinsic_name).is_some() {
            require_text(
                intent,
                "requiredOperationProfile",
                "continuum.profile.graph-effects/v1",
            )?;
        }
        require_text(
            field(core, "binding")?,
            "type",
            text_field(effect, "outputType")?,
        )?;
        signatures.insert(id, effect);
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
/// A lawpack pin: the compiler frames the exports and adapter digests under
/// the coordinate each reference names, not under a fixed artifact domain.
fn owner_pinned(reference: &Value, value: &Value) -> Result<(), Error> {
    let owner = text_field(reference, "id")?;
    pinned(
        reference,
        digest(owner, value).map_err(|_| Error::InvalidArtifact)?,
    )
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
/// One module's declaration preamble: its `package` line, then only `use`
/// lines. A `use target` names exactly the selected, pinned profile, at most
/// once; a `use lawpack` names exactly the bound lawpack, at most once; a
/// `use module` pins a source module, which [`super::source_modules`]
/// resolves. Returns the lawpack alias, if declared, and the module pins.
fn preamble<'s>(
    source: &'s str,
    coordinate: &str,
    identity: [u8; 32],
    module: &str,
    profile_reference: &Value,
) -> Result<(Option<String>, Vec<super::source_modules::Pin<'s>>), Error> {
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
    // The only target declaration accepted names the selected, pinned profile.
    let [scheme, target_digest] = array(field(profile_reference, "digest")?)? else {
        return Err(Error::InvalidArtifact);
    };
    if text(scheme)? != "sha256" {
        return Err(Error::InvalidArtifact);
    }
    let target = format!(
        "{} \"sha256:{}\"",
        text_field(profile_reference, "id")?,
        hex::encode(bytes(target_digest)?)
    );
    let mut targeted = false;
    let mut result = None;
    let mut pins = Vec::new();
    for line in lines {
        if !line.starts_with("use ") {
            break;
        }
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        let [_, kind, name, "digest", value, "as", alias] = tokens.as_slice() else {
            return Err(Error::UnsupportedProgram);
        };
        let alias = alias.strip_suffix(';').ok_or(Error::InvalidArtifact)?;
        let valid_alias = !alias.is_empty()
            && alias.bytes().enumerate().all(|(i, c)| {
                c == b'_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit())
            });
        match *kind {
            "target" => {
                if format!("{name} {value}") != target || alias.is_empty() || targeted {
                    return Err(Error::InvalidArtifact);
                }
                targeted = true;
            }
            "lawpack" => {
                if *name != coordinate
                    || *value != expected
                    || !valid_alias
                    || result.replace(alias.to_owned()).is_some()
                {
                    return Err(Error::InvalidArtifact);
                }
            }
            "module" => {
                if !valid_alias || pins.iter().any(|(pinned, _)| pinned == name) {
                    return Err(Error::InvalidArtifact);
                }
                pins.push((*name, *value));
            }
            _ => return Err(Error::UnsupportedProgram),
        }
    }
    Ok((result, pins))
}

/// The runtime's statement of the identity contracts a catalog must carry
/// before a program may call them: operand ceilings, base steps and step
/// coefficients, matching [`crate::edict_pure`]'s own metering.
const IDENTITY: [(&str, &[&str], i128, &[i128]); 4] = [
    (
        "echo.dpo@2.contentAddress",
        &[
            "Bytes<exact=32>",
            "Bytes<exact=32>",
            "Bytes<exact=32>",
            "Bytes<max=1048576>",
            "List<echo.dpo@2.OutgoingEdge,max=16>",
        ],
        71,
        &[0, 0, 0, 1, 67],
    ),
    (
        "echo.dpo@2.contentEdgeId",
        &["Bytes<exact=32>"; 4],
        70,
        &[0; 4],
    ),
    (
        "echo.dpo@2.slotEdgeId",
        &["Bytes<exact=32>"; 3],
        69,
        &[0; 3],
    ),
    (
        "echo.dpo@2.attachmentValueDigest",
        &["Bytes<exact=32>", "Bytes<max=1048576>"],
        68,
        &[0, 1],
    ),
];

/// Whether the pinned catalog authorizes identity calls: under `echo.dpo@2`
/// it carries all four exact contracts and the `OutgoingEdge` record, and
/// Core's single target import is the selected profile. A partial or altered
/// set of contracts is refused, never ignored.
fn identity_bound(
    profile: &str,
    catalog: &Value,
    module: &Value,
    profile_reference: &Value,
) -> Result<bool, Error> {
    let intrinsics = field(catalog, "intrinsics")?;
    let present = IDENTITY
        .iter()
        .filter(|(name, ..)| field(intrinsics, name).is_ok())
        .count();
    let mut targets = 0;
    for import in array(field(module, "imports")?)? {
        if text_field(import, "kind")? == "target" {
            targets += 1;
            if targets > 1 || field(import, "ref")? != profile_reference {
                return Err(Error::InvalidArtifact);
            }
        }
    }
    if present == 0 {
        return Ok(false);
    }
    if profile != "echo.dpo@2" || present != IDENTITY.len() {
        return Err(Error::InvalidArtifact);
    }
    require_text(
        field(catalog, "types")?,
        "echo.dpo@2.OutgoingEdge",
        "Record<edgeType:Bytes<exact=32>,to:Bytes<exact=32>>",
    )?;
    let texts =
        |values: &[&str]| Value::Array(values.iter().map(|v| Value::Text((*v).into())).collect());
    let integers =
        |values: &[i128]| Value::Array(values.iter().copied().map(Value::Integer).collect());
    for (name, arguments, base, coefficients) in IDENTITY {
        let entry = field(intrinsics, name)?;
        exact_fields(
            entry,
            &[
                "intrinsicClass",
                "typeParameters",
                "argumentTypes",
                "returnType",
                "guardSupport",
                "footprintTemplate",
                "costTemplate",
                "writeClass",
                "contractVersion",
                "costRule",
            ],
        )?;
        let rule = field(entry, "costRule")?;
        exact_fields(rule, &["base", "perArgumentMax"])?;
        require_text(entry, "intrinsicClass", "pure")?;
        require_text(entry, "returnType", "Bytes<exact=32>")?;
        require_text(entry, "writeClass", "none")?;
        require_text(entry, "contractVersion", "echo.content-address/v1")?;
        if field(entry, "typeParameters")? != &Value::Array(Vec::new())
            || field(entry, "guardSupport")? != &Value::Bool(false)
            || field(entry, "argumentTypes")? != &texts(arguments)
            || field(rule, "base")? != &Value::Integer(base)
            || field(rule, "perArgumentMax")? != &integers(coefficients)
            || text_field(entry, "footprintTemplate")?.is_empty()
            || text_field(entry, "costTemplate")?.is_empty()
        {
            return Err(Error::InvalidArtifact);
        }
    }
    Ok(targets == 1)
}

/// Whether any call in `value` names an identity intrinsic.
fn calls_identity(value: &Value, depth: usize) -> Result<bool, Error> {
    // Core nests at most 64 expression levels inside a bounded envelope.
    if depth > 256 {
        return Err(Error::UnsupportedProgram);
    }
    Ok(match value {
        Value::Map(entries) => {
            if text_field(value, "kind") == Ok("call")
                && text_field(value, "callee")
                    .is_ok_and(|callee| IDENTITY.iter().any(|(name, ..)| *name == callee))
            {
                return Ok(true);
            }
            for (_, entry) in entries {
                if calls_identity(entry, depth + 1)? {
                    return Ok(true);
                }
            }
            false
        }
        Value::Array(items) => {
            for item in items {
                if calls_identity(item, depth + 1)? {
                    return Ok(true);
                }
            }
            false
        }
        _ => false,
    })
}
