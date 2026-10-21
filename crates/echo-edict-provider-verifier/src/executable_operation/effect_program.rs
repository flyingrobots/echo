// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Verifier-owned judgment of a compiler-produced authored effect program.
//!
//! The verifier establishes, without consulting the lowerer or the runtime,
//! that the program is bound to its own artifacts, that each effect is
//! authorized by its pinned adapter and conforms to its intrinsic's runtime ABI,
//! that every failure arm names a declared obstruction through the
//! version-scoped failure table, that Core and Target IR agree node for node,
//! and that the body is typed, proven and within its declared budget.
use super::source_functions::{judge_effect_intent, EffectSite};
use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, digest_canonical_value_bytes_v1 as digest,
    encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value,
};
use std::collections::{BTreeMap, BTreeSet};

type Check<T> = Result<T, ()>;

const PROFILE_GRAPH_EFFECTS: &str = "continuum.profile.graph-effects/v1";
const MAX_DEPTH: usize = 64;
const MAX_NODES: usize = 65_536;

/// One failure of a read intrinsic: its catalog identifier, the runtime tag
/// Echo records, and its authority class.
struct FailureName {
    catalog: &'static str,
    runtime: &'static str,
    authority: &'static str,
}

const NODE_ATOM_READ_V1: [FailureName; 4] = [
    FailureName {
        catalog: "atomMissing",
        runtime: "echo.atom-missing/v1",
        authority: "resourceFault",
    },
    FailureName {
        catalog: "atomRequired",
        runtime: "echo.atom-required/v1",
        authority: "integrityFault",
    },
    FailureName {
        catalog: "atomTypeMismatch",
        runtime: "echo.atom-type-mismatch/v1",
        authority: "integrityFault",
    },
    FailureName {
        catalog: "atomByteBoundExceeded",
        runtime: "echo.atom-byte-bound-exceeded/v1",
        authority: "integrityFault",
    },
];

/// The failure contract of each runtime intrinsic, keyed by its versioned
/// coordinate. An intrinsic outside this table is refused.
fn failures(intrinsic: &str) -> Option<&'static [FailureName]> {
    match intrinsic {
        "echo.dpo@1.node-atom-read" => Some(&NODE_ATOM_READ_V1),
        "echo.dpo@1.ensure-node"
        | "echo.dpo@1.create-node"
        | "echo.dpo@1.replace-atom"
        | "echo.dpo@1.ensure-edge"
        | "echo.dpo@1.replace-edge" => Some(&[]),
        _ => None,
    }
}

/// The effect kind, which is also the write class, of each intrinsic.
fn effect_kind(intrinsic: &str) -> Option<&'static str> {
    Some(match intrinsic {
        "echo.dpo@1.node-atom-read" => "read",
        "echo.dpo@1.create-node" => "create",
        "echo.dpo@1.ensure-node" | "echo.dpo@1.ensure-edge" => "ensure",
        "echo.dpo@1.replace-atom" | "echo.dpo@1.replace-edge" => "replace",
        _ => return None,
    })
}

/// What the judged program binds a package to.
pub(crate) struct Judged {
    pub(crate) core_identity: Value,
    pub(crate) target_identity: Value,
    pub(crate) exports_identity: Value,
    pub(crate) operation: String,
    pub(crate) output: String,
}

/// Judges compiler-produced effect-program bytes.
pub(crate) fn judge(compiled_program: &[u8]) -> Check<Judged> {
    let bundle = decode(compiled_program).map_err(|_| ())?;
    exact(
        &bundle,
        &[
            "schema",
            "intent",
            "core_artifact",
            "target_ir_artifact",
            "lawpack_exports_artifact",
            "lowering_artifacts",
        ],
    )?;
    equal_text(
        &bundle,
        "schema",
        "echo.compiler-produced-effect-program/v1",
    )?;
    let intent_name = nonempty(get(&bundle, "intent")?)?;
    let embedded = |key| decode(bytes(get(&bundle, key)?)?).map_err(|_| ());
    let module = embedded("core_artifact")?;
    let target = embedded("target_ir_artifact")?;
    let exports = embedded("lawpack_exports_artifact")?;
    artifacts(&module, &target, &exports)?;
    let intent = get(get(&module, "intents")?, intent_name)?;
    let lowered = get(get(&target, "intents")?, intent_name)?;
    binding(intent, lowered)?;
    let sites = relation(
        &module,
        intent,
        &target,
        lowered,
        &exports,
        get(&bundle, "lowering_artifacts")?,
    )?;
    correspondence(intent, lowered)?;
    judge_effect_intent(&module, &exports, intent, &sites)?;
    Ok(Judged {
        core_identity: identity("edict.core.module/v1", &module)?,
        target_identity: identity("edict.target-ir.artifact/v1", &target)?,
        exports_identity: identity("edict.lawpack-exports/v1", &exports)?,
        operation: format!("{}.{intent_name}", string(get(&module, "coordinate")?)?),
        output: string(get(intent, "output")?)?.to_owned(),
    })
}

/// The three embedded artifacts and the Core pin the Target IR carries.
fn artifacts(module: &Value, target: &Value, exports: &Value) -> Check<()> {
    exact(
        target,
        &[
            "domain",
            "kind",
            "intents",
            "targetProfile",
            "semanticClosure",
            "sourceCoreCoordinate",
        ],
    )?;
    equal_text(target, "domain", "echo.span-ir/v2")?;
    equal_text(target, "kind", "orderedTargetIrArtifact")?;
    exact(
        exports,
        &[
            "types",
            "effects",
            "constants",
            "obstructions",
            "pureFunctions",
            "operationProfiles",
        ],
    )?;
    list(get(exports, "types")?)?;
    members(get(exports, "operationProfiles")?)?;
    for key in ["effects", "constants", "obstructions", "pureFunctions"] {
        list(get(exports, key)?)?;
    }
    let mut fields = vec![
        "apiVersion",
        "coordinate",
        "imports",
        "types",
        "intents",
        "requiredCoreCapabilities",
    ];
    if map_field(module, "functions").is_some() {
        fields.push("functions");
    }
    exact(module, &fields)?;
    equal_text(module, "apiVersion", "edict.core/v1")?;
    let coordinate = nonempty(get(module, "coordinate")?)?;
    if members(get(module, "intents")?)?.is_empty() {
        return Err(());
    }
    list(get(module, "imports")?)?;
    members(get(module, "types")?)?;
    if let Some(functions) = map_field(module, "functions") {
        if members(functions)?.is_empty() {
            return Err(());
        }
    }
    for capability in list(get(module, "requiredCoreCapabilities")?)? {
        string(capability)?;
    }
    equal_text(target, "sourceCoreCoordinate", coordinate)?;
    let source = get(get(target, "semanticClosure")?, "sourceCore")?;
    exact(source, &["id", "digest"])?;
    equal_text(source, "id", coordinate)?;
    pinned(source, identity("edict.core.module/v1", module)?)
}

/// The selected intent and its lowered form agree on everything the runtime
/// executes besides the body's instructions.
fn binding(intent: &Value, lowered: &Value) -> Check<()> {
    for key in ["inputConstraints", "coreEvaluationBudget"] {
        if get(lowered, key)? != get(intent, key)? {
            return Err(());
        }
    }
    equal_text(
        lowered,
        "operationProfile",
        string(get(intent, "requiredOperationProfile")?)?,
    )?;
    if map_field(lowered, "basis") != map_field(intent, "basis") {
        return Err(());
    }
    let mut fields = vec![
        "input",
        "output",
        "requiredOperationProfile",
        "inputConstraints",
        "coreEvaluationBudget",
        "body",
    ];
    if map_field(intent, "basis").is_some() {
        fields.push("basis");
    }
    if map_field(intent, "optic").is_some() {
        // Optic-scoped programs are outside the authored-effect route.
        return Err(());
    }
    exact(intent, &fields)?;
    nonempty(get(intent, "input")?)?;
    nonempty(get(intent, "output")?)?;
    nonempty(get(intent, "requiredOperationProfile")?)?;
    let budget = get(intent, "coreEvaluationBudget")?;
    exact(budget, &["maxSteps", "maxAllocatedBytes", "maxOutputBytes"])?;
    if number(get(budget, "maxSteps")?)? == 0 {
        return Err(());
    }
    let body = get(intent, "body")?;
    exact(body, &["locals", "nodes", "result"])?;
    if get(lowered, "result")? != get(body, "result")? {
        return Err(());
    }
    Ok(())
}

/// The pinned lawpack, adapter, target profile and intrinsic catalog that
/// authorize each effect node, and each effect's ABI and failure contract.
#[allow(
    clippy::too_many_lines,
    reason = "Each pin is checked where the next artifact relies on it."
)]
fn relation<'a>(
    module: &'a Value,
    intent: &'a Value,
    target_artifact: &'a Value,
    lowered: &'a Value,
    exports: &'a Value,
    lowering: &'a Value,
) -> Check<BTreeMap<usize, EffectSite>> {
    let steps = list(get(lowered, "steps")?)?;
    if steps.is_empty() {
        return if lowering == &Value::Null {
            Ok(BTreeMap::new())
        } else {
            Err(())
        };
    }
    exact(
        lowering,
        &[
            "source_artifact",
            "lawpack_artifact",
            "adapter_artifact",
            "target_profile_artifact",
            "intrinsics_artifact",
        ],
    )?;
    let embedded = |key| decode(bytes(get(lowering, key)?)?).map_err(|_| ());
    let source = embedded("source_artifact")?;
    let source = std::str::from_utf8(bytes(&source)?).map_err(|_| ())?;
    let lawpack = embedded("lawpack_artifact")?;
    let adapter = embedded("adapter_artifact")?;
    let profile = embedded("target_profile_artifact")?;
    let catalog = embedded("intrinsics_artifact")?;
    equal_text(&profile, "apiVersion", "edict.target-profile/v1")?;
    let profile_coordinate = format!(
        "{}@{}",
        string(get(&profile, "id")?)?,
        string(get(&profile, "version")?)?
    );
    let profile_reference = get(target_artifact, "targetProfile")?;
    equal_text(profile_reference, "id", &profile_coordinate)?;
    pinned(
        profile_reference,
        identity("edict.target-profile/v1", &profile)?,
    )?;
    let core_abi = get(module, "apiVersion")?;
    if !list(get(&profile, "acceptedCoreAbi")?)?
        .iter()
        .any(|value| value == core_abi)
    {
        return Err(());
    }
    equal_text(&profile, "intrinsicNamespace", &profile_coordinate)?;
    let catalog_reference = get(&profile, "intrinsics")?;
    equal_text(catalog_reference, "id", "echo.dpo.intrinsics/v1")?;
    pinned(
        catalog_reference,
        identity("echo.dpo.intrinsics/v1", &catalog)?,
    )?;
    exact(&catalog, &["apiVersion", "intrinsics"])?;
    equal_text(&catalog, "apiVersion", "edict.target-profile.intrinsics/v1")?;
    let catalog = get(&catalog, "intrinsics")?;
    members(catalog)?;
    equal_text(&lawpack, "apiVersion", "edict.lawpack/v1")?;
    equal_text(&adapter, "apiVersion", "edict.lawpack-adapter/v1")?;
    equal_text(&adapter, "class", "declarative")?;
    let lawpack_coordinate = format!(
        "{}@{}",
        string(get(&lawpack, "id")?)?,
        string(get(&lawpack, "version")?)?
    );
    let lawpack_identity = identity("edict.lawpack/v1", &lawpack)?;
    let [reference] = list(get(get(target_artifact, "semanticClosure")?, "lawpacks")?)? else {
        return Err(());
    };
    equal_text(reference, "id", &lawpack_coordinate)?;
    pinned(reference, lawpack_identity.clone())?;
    let mut imports = list(get(module, "imports")?)?
        .iter()
        .filter(|entry| map_field(entry, "kind") == Some(&text("lawpack")));
    let import = imports.next().ok_or(())?;
    if imports.next().is_some() || get(import, "ref")? != reference {
        return Err(());
    }
    pinned(
        get(&lawpack, "exports")?,
        identity("edict.lawpack-exports/v1", exports)?,
    )?;
    let mut matching = list(get(&lawpack, "targetAdapters")?)?
        .iter()
        .filter(|entry| map_field(entry, "acceptedTargetProfile") == Some(profile_reference));
    let entry = matching.next().ok_or(())?;
    if matching.next().is_some() || get(&profile, "targetIr")? != get(entry, "acceptedTargetIr")? {
        return Err(());
    }
    equal_text(
        get(entry, "acceptedTargetIr")?,
        "id",
        string(get(target_artifact, "domain")?)?,
    )?;
    pinned(
        get(entry, "adapter")?,
        identity("edict.lawpack-adapter/v1", &adapter)?,
    )?;
    let alias = alias(
        source,
        &lawpack_coordinate,
        &lawpack_identity,
        string(get(module, "coordinate")?)?,
    )?;
    imported_types(module, exports, &lawpack_coordinate)?;
    let operation_profile = string(get(intent, "requiredOperationProfile")?)?;
    let authorized = list(get(
        unique_profile(&adapter, operation_profile)?,
        "semanticEffects",
    )?)?;
    let implementations = get(&adapter, "effectImplementations")?;
    let nodes = list(get(get(intent, "body")?, "nodes")?)?;
    let lowered_steps = steps
        .iter()
        .map(|step| Ok((string(get(step, "id")?)?, step)))
        .collect::<Check<BTreeMap<_, _>>>()?;
    let order = list(get(lowered, "executionOrder")?)?;
    if order.len() != nodes.len() {
        return Err(());
    }
    let mut sites = BTreeMap::new();
    for (index, (node, id)) in nodes.iter().zip(order).enumerate() {
        if string(get(node, "kind")?)? != "effect" {
            continue;
        }
        let step = *lowered_steps.get(string(id)?).ok_or(())?;
        let name = resolve(string(get(node, "effect")?)?, &alias, &lawpack_coordinate)?;
        if !authorized.iter().any(|value| value == &text(&name)) {
            return Err(());
        }
        let implementation = get(implementations, &name)?;
        let intrinsic_name = string(get(implementation, "targetIntrinsic")?)?;
        if !intrinsic_name
            .strip_prefix(profile_coordinate.as_str())
            .is_some_and(|suffix| suffix.starts_with('.') && suffix.len() > 1)
        {
            return Err(());
        }
        equal_text(step, "targetIntrinsic", intrinsic_name)?;
        let kind = effect_kind(intrinsic_name).ok_or(())?;
        if kind != "read" && operation_profile != PROFILE_GRAPH_EFFECTS {
            return Err(());
        }
        let intrinsic = get(catalog, intrinsic_name)?;
        let mut declarations = list(get(exports, "effects")?)?
            .iter()
            .filter(|effect| map_field(effect, "coordinate") == Some(&text(&name)));
        let effect = declarations.next().ok_or(())?;
        if declarations.next().is_some() {
            return Err(());
        }
        contract(intrinsic_name, kind, intrinsic, effect, implementation)?;
        let output = string(get(effect, "outputType")?)?;
        equal_text(get(node, "binding")?, "type", output)?;
        arms(
            node,
            step,
            effect,
            implementation,
            exports,
            &alias,
            &lawpack_coordinate,
        )?;
        sites.insert(
            index,
            EffectSite {
                intrinsic: intrinsic_name.to_owned(),
                input: string(get(effect, "inputType")?)?.to_owned(),
                output: output.to_owned(),
            },
        );
    }
    if sites.len() != steps.len() {
        return Err(());
    }
    Ok(sites)
}

/// The catalog entry, exported effect and adapter implementation agree with
/// each other and with the intrinsic's runtime ABI and failure table.
fn contract(
    name: &str,
    kind: &str,
    intrinsic: &Value,
    effect: &Value,
    implementation: &Value,
) -> Check<()> {
    exact(
        intrinsic,
        &[
            "intrinsicClass",
            "argumentTypes",
            "returnType",
            "typeParameters",
            "effectKind",
            "writeClass",
            "guardSupport",
            "canParticipateInAtomicGuard",
            "costTemplate",
            "footprintTemplate",
            "effectFailures",
        ],
    )?;
    equal_text(intrinsic, "intrinsicClass", "effect")?;
    equal_text(effect, "executionClass", "runtime")?;
    let abi_input = format!("{name}.input");
    let [argument] = list(get(intrinsic, "argumentTypes")?)? else {
        return Err(());
    };
    if string(argument)? != abi_input
        || string(get(intrinsic, "returnType")?)? != format!("{name}.result")
    {
        return Err(());
    }
    if !list(get(intrinsic, "typeParameters")?)?.is_empty()
        || !list(get(effect, "typeParameters")?)?.is_empty()
    {
        return Err(());
    }
    equal_text(intrinsic, "effectKind", kind)?;
    equal_text(intrinsic, "writeClass", kind)?;
    equal_text(implementation, "writeClass", kind)?;
    let guard = boolean(get(intrinsic, "guardSupport")?)?;
    boolean(get(intrinsic, "canParticipateInAtomicGuard")?)?;
    if boolean(get(effect, "guardSupport")?)? && !guard {
        return Err(());
    }
    for key in ["costTemplate", "footprintTemplate"] {
        nonempty(get(intrinsic, key)?)?;
    }
    let declared = failures(name).ok_or(())?;
    let catalog_failures = members(get(intrinsic, "effectFailures")?)?;
    let exported = get(effect, "effectFailures")?;
    let mappings = members(get(implementation, "failureMappings")?)?;
    if catalog_failures.len() != declared.len()
        || mappings.len() != declared.len()
        || members(exported)?.len() != declared.len()
    {
        return Err(());
    }
    let mut mapped = BTreeSet::new();
    for (source, catalog_name) in mappings {
        let catalog_name = string(catalog_name)?;
        let failure = declared
            .iter()
            .find(|failure| failure.catalog == catalog_name)
            .ok_or(())?;
        if !mapped.insert(failure.runtime) {
            return Err(());
        }
        let actual = get(get(intrinsic, "effectFailures")?, catalog_name)?;
        let expected = get(exported, string(source)?)?;
        exact(actual, &["authorityClass", "payloadType"])?;
        exact(expected, &["authorityClass", "payloadType"])?;
        equal_text(actual, "authorityClass", failure.authority)?;
        equal_text(expected, "authorityClass", failure.authority)?;
        equal_text(actual, "payloadType", &abi_input)?;
        equal_text(expected, "payloadType", string(get(effect, "inputType")?)?)?;
    }
    Ok(())
}

/// Every failure arm of an effect node names exactly one declared
/// obstruction with the failure's authority class, and Core and Target IR
/// carry the same arms under the catalog's failure names.
fn arms(
    node: &Value,
    step: &Value,
    effect: &Value,
    implementation: &Value,
    exports: &Value,
    alias: &str,
    lawpack: &str,
) -> Check<()> {
    let failures = get(effect, "effectFailures")?;
    let mappings = members(get(implementation, "failureMappings")?)?;
    let source_arms = get(node, "obstructionMap")?;
    let target_arms = get(step, "obstructionArms")?;
    if members(source_arms)?.len() != mappings.len()
        || members(target_arms)?.len() != mappings.len()
    {
        return Err(());
    }
    let declared_failures = list(get(step, "obstructionFailures")?)?
        .iter()
        .map(string)
        .collect::<Check<BTreeSet<_>>>()?;
    if declared_failures.len() != mappings.len() {
        return Err(());
    }
    for (failure, catalog_name) in mappings {
        let failure = string(failure)?;
        let catalog_name = string(catalog_name)?;
        let arm = get(source_arms, failure)?;
        if Some(arm) != map_field(target_arms, catalog_name)
            || !declared_failures.contains(catalog_name)
        {
            return Err(());
        }
        exact(arm, &["binder", "value"])?;
        equal_text(
            get(arm, "binder")?,
            "type",
            string(get(get(failures, failure)?, "payloadType")?)?,
        )?;
        let reason = get(arm, "value")?;
        let coordinate = resolve(string(get(reason, "callee")?)?, alias, lawpack)?;
        let mut obstructions = list(get(exports, "obstructions")?)?
            .iter()
            .filter(|item| map_field(item, "coordinate") == Some(&text(&coordinate)));
        let obstruction = obstructions.next().ok_or(())?;
        if obstructions.next().is_some() {
            return Err(());
        }
        equal_text(
            obstruction,
            "authorityClass",
            string(get(get(failures, failure)?, "authorityClass")?)?,
        )?;
    }
    Ok(())
}

/// Core nodes and Target IR instructions correspond one for one, in the
/// lowered execution order, with identical kinds and contents.
fn correspondence(intent: &Value, lowered: &Value) -> Check<()> {
    let mut fields = vec![
        "operationProfile",
        "inputConstraints",
        "coreEvaluationBudget",
        "steps",
        "requirements",
        "result",
        "executionOrder",
    ];
    for key in ["basis", "pureBindings"] {
        if map_field(lowered, key).is_some() {
            fields.push(key);
        }
    }
    exact(lowered, &fields)?;
    let mut instructions = BTreeMap::new();
    for (collection, kind) in [
        ("steps", "effect"),
        ("requirements", "require"),
        ("pureBindings", "let"),
    ] {
        let Some(items) = map_field(lowered, collection) else {
            continue;
        };
        for item in list(items)? {
            let id = nonempty(get(item, "id")?)?;
            if instructions.insert(id, (kind, item)).is_some() {
                return Err(());
            }
        }
    }
    let nodes = list(get(get(intent, "body")?, "nodes")?)?;
    let order = list(get(lowered, "executionOrder")?)?;
    if nodes.len() != order.len() || nodes.len() != instructions.len() {
        return Err(());
    }
    for (node, id) in nodes.iter().zip(order) {
        let (kind, instruction) = instructions.remove(string(id)?).ok_or(())?;
        equal_text(node, "kind", kind)?;
        let keys: &[&str] = match kind {
            "let" => {
                exact(instruction, &["id", "binding", "value"])?;
                &["binding", "value"]
            }
            "require" => {
                exact(instruction, &["id", "predicate", "onFailure"])?;
                &["predicate", "onFailure"]
            }
            _ => {
                exact(
                    instruction,
                    &[
                        "id",
                        "binding",
                        "effect",
                        "input",
                        "targetIntrinsic",
                        "obstructionArms",
                        "obstructionFailures",
                    ],
                )?;
                &["binding", "effect", "input"]
            }
        };
        for key in keys {
            if get(node, key)? != get(instruction, key)? {
                return Err(());
            }
        }
    }
    Ok(())
}

fn unique_profile<'a>(adapter: &'a Value, core: &str) -> Check<&'a Value> {
    let mut selected = None;
    for (_, profile) in members(get(adapter, "operationProfiles")?)? {
        if string(get(profile, "core")?)? == core && selected.replace(profile).is_some() {
            return Err(());
        }
    }
    selected.ok_or(())
}

fn resolve(name: &str, alias: &str, coordinate: &str) -> Check<String> {
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
        .ok_or(())?;
    Ok(format!("{coordinate}.{member}"))
}

/// The lawpack alias the source declares in its import preamble, with the
/// lawpack's exact digest.
fn alias(source: &str, coordinate: &str, identity: &Value, module: &str) -> Check<String> {
    let Value::Bytes(identity) = identity else {
        return Err(());
    };
    let expected = format!("\"sha256:{}\"", lower_hex(identity));
    let mut lines = source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());
    if lines.next() != Some(format!("package {module};").as_str()) {
        return Err(());
    }
    let mut result = None;
    for line in lines {
        if !line.starts_with("use ") {
            break;
        }
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        let ["use", "lawpack", name, "digest", value, "as", alias] = tokens.as_slice() else {
            return Err(());
        };
        let alias = alias.strip_suffix(';').ok_or(())?;
        if *name != coordinate
            || *value != expected
            || !identifier(alias)
            || result.replace(alias.to_owned()).is_some()
        {
            return Err(());
        }
    }
    result.ok_or(())
}

/// Every Core type under the lawpack's namespace renders to exactly the
/// definition the pinned exports declare for it.
fn imported_types(module: &Value, exports: &Value, lawpack: &str) -> Check<()> {
    let mut definitions = BTreeMap::new();
    let mut remaining = MAX_NODES;
    for item in list(get(exports, "types")?)? {
        spend(&mut remaining)?;
        exact(item, &["coordinate", "definition"])?;
        let name = nonempty(get(item, "coordinate")?)?;
        let definition = nonempty(get(item, "definition")?)?;
        if definitions.insert(name, definition).is_some() {
            return Err(());
        }
    }
    let prefix = format!("{lawpack}.");
    for (name, value) in members(get(module, "types")?)? {
        spend(&mut remaining)?;
        let name = string(name)?;
        if !name.starts_with(&prefix) {
            continue;
        }
        let mut expected = *definitions.get(name).ok_or(())?;
        let mut seen = BTreeSet::new();
        while let Some(next) = definitions.get(expected) {
            spend(&mut remaining)?;
            if seen.len() >= MAX_DEPTH || !seen.insert(expected) {
                return Err(());
            }
            expected = next;
        }
        if render(name, value, &mut remaining)? != expected {
            return Err(());
        }
    }
    Ok(())
}

fn render(name: &str, value: &Value, remaining: &mut usize) -> Check<String> {
    spend(remaining)?;
    let kind = string(get(value, "kind")?)?;
    match kind {
        "Bool" | "Unit" | "I8" | "I16" | "I32" | "I64" | "U8" | "U16" | "U32" | "U64" => {
            exact(value, &["kind"])?;
            Ok(kind.to_owned())
        }
        "Bytes" => {
            let max = number(get(value, "max")?)?;
            if let Some(min) = map_field(value, "min") {
                exact(value, &["kind", "min", "max"])?;
                let min = number(min)?;
                if min > max {
                    return Err(());
                }
                Ok(if min == max {
                    format!("Bytes<exact={max}>")
                } else {
                    format!("Bytes<min={min},max={max}>")
                })
            } else {
                exact(value, &["kind", "max"])?;
                Ok(format!("Bytes<max={max}>"))
            }
        }
        "String" => {
            exact(value, &["kind", "max", "canonical"])?;
            let max = number(get(value, "max")?)?;
            let canonical = string(get(value, "canonical")?)?;
            if !["raw-utf8", "unicode-scalar-nfc"].contains(&canonical) {
                return Err(());
            }
            Ok(format!("String<max={max},canonical={canonical}>"))
        }
        "Record" => {
            exact(value, &["kind", "fields"])?;
            let mut fields = BTreeMap::new();
            for (key, ty) in members(get(value, "fields")?)? {
                let key = string(key)?;
                if !identifier(key) || fields.insert(key, reference(ty, remaining)?).is_some() {
                    return Err(());
                }
            }
            let fields = fields
                .into_iter()
                .map(|(name, ty)| format!("{name}:{ty}"))
                .collect::<Vec<_>>()
                .join(",");
            Ok(format!("Record<{fields}>"))
        }
        "Nominal" => {
            exact(value, &["kind", "contract", "representation"])?;
            equal_text(value, "contract", name)?;
            Ok(format!(
                "Nominal<{}>",
                reference(get(value, "representation")?, remaining)?
            ))
        }
        "Option" | "CapabilityRef" => {
            exact(value, &["kind", "item"])?;
            Ok(format!(
                "{kind}<{}>",
                reference(get(value, "item")?, remaining)?
            ))
        }
        "List" => {
            exact(value, &["kind", "item", "max"])?;
            let item = reference(get(value, "item")?, remaining)?;
            Ok(format!("List<{item},max={}>", number(get(value, "max")?)?))
        }
        "Map" => {
            exact(value, &["kind", "key", "value", "max"])?;
            let key = reference(get(value, "key")?, remaining)?;
            let item = reference(get(value, "value")?, remaining)?;
            Ok(format!(
                "Map<{key},{item},max={}>",
                number(get(value, "max")?)?
            ))
        }
        "ExternalActionRequest" => {
            exact(value, &["kind", "settlement"])?;
            Ok(format!(
                "edict.external-action.request/v1<{}>",
                reference(get(value, "settlement")?, remaining)?
            ))
        }
        _ => Err(()),
    }
}

fn reference<'a>(value: &'a Value, remaining: &mut usize) -> Check<&'a str> {
    spend(remaining)?;
    let value = string(value)?;
    if value.is_empty() || value.len() > 1024 {
        return Err(());
    }
    let mut depth = 0usize;
    for byte in value.bytes() {
        match byte {
            b'<' => {
                depth += 1;
                if depth > MAX_DEPTH {
                    return Err(());
                }
            }
            b'>' => depth = depth.checked_sub(1).ok_or(())?,
            _ => {}
        }
    }
    if depth != 0 {
        return Err(());
    }
    Ok(value)
}

fn spend(remaining: &mut usize) -> Check<()> {
    *remaining = remaining.checked_sub(1).ok_or(())?;
    Ok(())
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .enumerate()
            .all(|(i, c)| c == b'_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit()))
}

fn lower_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 15)]));
    }
    out
}

fn identity(domain: &str, value: &Value) -> Check<Value> {
    digest(domain, value)
        .map(|bytes| Value::Bytes(bytes.to_vec()))
        .map_err(|_| ())
}

fn pinned(reference: &Value, expected: Value) -> Check<()> {
    exact(reference, &["id", "digest"])?;
    nonempty(get(reference, "id")?)?;
    if get(reference, "digest")? != &Value::Array(vec![text("sha256"), expected]) {
        return Err(());
    }
    // A pin is canonical bytes, never a looser spelling of the same digest.
    encode(reference).map(|_| ()).map_err(|_| ())
}

fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}
fn map_field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    let Value::Map(fields) = value else {
        return None;
    };
    fields
        .iter()
        .find_map(|(k, v)| matches!(k, Value::Text(name) if name == key).then_some(v))
}
fn get<'a>(value: &'a Value, key: &str) -> Check<&'a Value> {
    map_field(value, key).ok_or(())
}
fn members(value: &Value) -> Check<&[(Value, Value)]> {
    match value {
        Value::Map(fields) => Ok(fields),
        _ => Err(()),
    }
}
fn list(value: &Value) -> Check<&[Value]> {
    match value {
        Value::Array(items) => Ok(items),
        _ => Err(()),
    }
}
fn string(value: &Value) -> Check<&str> {
    match value {
        Value::Text(text) => Ok(text),
        _ => Err(()),
    }
}
fn nonempty(value: &Value) -> Check<&str> {
    string(value).and_then(|text| if text.is_empty() { Err(()) } else { Ok(text) })
}
fn bytes(value: &Value) -> Check<&[u8]> {
    match value {
        Value::Bytes(bytes) => Ok(bytes),
        _ => Err(()),
    }
}
fn number(value: &Value) -> Check<u64> {
    match value {
        Value::Integer(n) => u64::try_from(*n).map_err(|_| ()),
        _ => Err(()),
    }
}
fn boolean(value: &Value) -> Check<bool> {
    match value {
        Value::Bool(value) => Ok(*value),
        _ => Err(()),
    }
}
fn equal_text(value: &Value, key: &str, expected: &str) -> Check<()> {
    if get(value, key)? == &text(expected) {
        Ok(())
    } else {
        Err(())
    }
}
fn exact(value: &Value, keys: &[&str]) -> Check<()> {
    let fields = members(value)?;
    if fields.len() != keys.len()
        || fields
            .iter()
            .any(|(k, _)| !matches!(k, Value::Text(key) if keys.contains(&key.as_str())))
    {
        return Err(());
    }
    Ok(())
}
