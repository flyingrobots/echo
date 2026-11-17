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
/// The ordered contract: one flat order of instruction ids.
const ORDERED_DOMAIN: &str = "echo.span-ir/v2";
/// The structured contract: an order tree with bounded loops and branches.
const STRUCTURED_DOMAIN: &str = "echo.span-ir/v3";
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

/// The failure contract of each runtime intrinsic, keyed by its operation
/// below either effect namespace. An intrinsic outside this table is refused.
fn failures(intrinsic: &str) -> Option<&'static [FailureName]> {
    match super::intrinsic_names::operation(intrinsic)? {
        "node-atom-read" => Some(&NODE_ATOM_READ_V1),
        "ensure-node" | "create-node" | "replace-atom" | "ensure-edge" | "replace-edge" => {
            Some(&[])
        }
        _ => None,
    }
}

/// The effect kind, which is also the write class, of each intrinsic.
fn effect_kind(intrinsic: &str) -> Option<&'static str> {
    Some(match super::intrinsic_names::operation(intrinsic)? {
        "node-atom-read" => "read",
        "create-node" => "create",
        "ensure-node" | "ensure-edge" => "ensure",
        "replace-atom" | "replace-edge" => "replace",
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
    let structured = artifacts(&module, &target, &exports)?;
    let intent = get(get(&module, "intents")?, intent_name)?;
    let lowered = get(get(&target, "intents")?, intent_name)?;
    binding(intent, lowered)?;
    let instructions = correspondence(intent, lowered, structured)?;
    let (sites, identity_admitted) = relation(
        &module,
        intent,
        &target,
        &instructions,
        &exports,
        get(&bundle, "lowering_artifacts")?,
    )?;
    judge_effect_intent(&module, &exports, intent, &sites, identity_admitted)?;
    Ok(Judged {
        core_identity: identity("edict.core.module/v1", &module)?,
        target_identity: identity("edict.target-ir.artifact/v1", &target)?,
        exports_identity: identity("edict.lawpack-exports/v1", &exports)?,
        operation: format!("{}.{intent_name}", string(get(&module, "coordinate")?)?),
        output: string(get(intent, "output")?)?.to_owned(),
    })
}

/// The three embedded artifacts and the Core pin the Target IR carries.
/// True when the Target IR declares the structured contract.
fn artifacts(module: &Value, target: &Value, exports: &Value) -> Check<bool> {
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
    // The declared domain and kind must name the same contract.
    let structured = if equal_text(target, "domain", ORDERED_DOMAIN).is_ok() {
        equal_text(target, "kind", "orderedTargetIrArtifact")?;
        false
    } else {
        equal_text(target, "domain", STRUCTURED_DOMAIN)?;
        equal_text(target, "kind", "structuredTargetIrArtifact")?;
        true
    };
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
    pinned(source, identity("edict.core.module/v1", module)?)?;
    Ok(structured)
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
    instructions: &[(&'a Value, &'a Value)],
    exports: &'a Value,
    lowering: &'a Value,
) -> Check<(BTreeMap<usize, EffectSite>, bool)> {
    let structured = equal_text(target_artifact, "domain", STRUCTURED_DOMAIN).is_ok();
    let mut effect_count = 0_usize;
    for (node, _) in instructions {
        if string(get(node, "kind")?)? == "effect" {
            effect_count += 1;
        }
    }
    if effect_count == 0 {
        // Without an effect step no profile is authenticated. Keep the
        // no-effect path on frozen @1; @2 needs authentication in the artifacts.
        let profile_coordinate = string(get(get(target_artifact, "targetProfile")?, "id")?)?;
        if structured || profile_coordinate != "echo.dpo@1" {
            return Err(());
        }
        return if lowering == &Value::Null {
            Ok((BTreeMap::new(), false))
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
    // Only Echo's two effect namespaces name runtime intrinsics; each pins
    // its own catalog coordinate, which is also the catalog's digest domain.
    let namespace = super::intrinsic_names::namespace(&profile_coordinate).ok_or(())?;
    // Only echo.dpo@2 admits structured control; echo.dpo@1 is frozen
    // (owner decision on flyingrobots/echo#947).
    if structured && !namespace.structured {
        return Err(());
    }
    let catalog_reference = get(&profile, "intrinsics")?;
    equal_text(catalog_reference, "id", namespace.catalog)?;
    pinned(catalog_reference, identity(namespace.catalog, &catalog)?)?;
    // Catalog v2 may also publish the named types its pure contracts
    // reference; the frozen v1 catalog has exactly two fields.
    if namespace.catalog_api == "edict.target-profile.intrinsics/v2"
        && get(&catalog, "types").is_ok()
    {
        exact(&catalog, &["apiVersion", "intrinsics", "types"])?;
        members(get(&catalog, "types")?)?;
    } else {
        exact(&catalog, &["apiVersion", "intrinsics"])?;
    }
    equal_text(&catalog, "apiVersion", namespace.catalog_api)?;
    let identity_admitted =
        identity_binding(namespace.coordinate, &catalog, module, profile_reference)?;
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
    owner_pinned(get(&lawpack, "exports")?, exports)?;
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
    owner_pinned(get(entry, "adapter")?, &adapter)?;
    let alias = alias(
        source,
        &lawpack_coordinate,
        &lawpack_identity,
        string(get(module, "coordinate")?)?,
        &target_line(profile_reference)?,
    )?;
    imported_types(module, exports, &lawpack_coordinate)?;
    let operation_profile = string(get(intent, "requiredOperationProfile")?)?;
    let authorized = list(get(
        unique_profile(&adapter, operation_profile)?,
        "semanticEffects",
    )?)?;
    let implementations = get(&adapter, "effectImplementations")?;
    // An effect site is keyed by its instruction's place in the pre-order
    // walk of the Core body; the body audit walks in the same order.
    let mut sites = BTreeMap::new();
    for (index, &(node, step)) in instructions.iter().enumerate() {
        if string(get(node, "kind")?)? != "effect" {
            continue;
        }
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
    if sites.len() != effect_count {
        return Err(());
    }
    Ok((sites, identity_admitted))
}

/// Echo's identity contract as this verifier states it: each coordinate's
/// operand ceilings, base steps and per-operand step coefficients.
fn identity_contract(name: &str) -> Option<(Vec<&'static str>, i128, Vec<i128>)> {
    const ID: &str = "Bytes<exact=32>";
    Some(match name {
        "echo.dpo@2.contentAddress" => (
            vec![
                ID,
                ID,
                ID,
                "Bytes<max=1048576>",
                "List<echo.dpo@2.OutgoingEdge,max=16>",
            ],
            71,
            vec![0, 0, 0, 1, 67],
        ),
        "echo.dpo@2.contentEdgeId" => (vec![ID; 4], 70, vec![0; 4]),
        "echo.dpo@2.slotEdgeId" => (vec![ID; 3], 69, vec![0; 3]),
        _ => return None,
    })
}

/// Binds identity calls to the pinned catalog. Admitted only when the
/// profile is `echo.dpo@2`, the catalog carries all three contracts exactly
/// with the `OutgoingEdge` record, and Core's one target import is the
/// selected profile. Any partial or altered contract is refused.
fn identity_binding(
    profile: &str,
    catalog: &Value,
    module: &Value,
    profile_reference: &Value,
) -> Check<bool> {
    let mut target_import = None;
    for entry in list(get(module, "imports")?)? {
        if string(get(entry, "kind")?)? == "target" {
            if target_import.is_some() || get(entry, "ref")? != profile_reference {
                return Err(());
            }
            target_import = Some(entry);
        }
    }
    let names = [
        "echo.dpo@2.contentAddress",
        "echo.dpo@2.contentEdgeId",
        "echo.dpo@2.slotEdgeId",
    ];
    let intrinsics = members(get(catalog, "intrinsics")?)?;
    let declared: Vec<&Value> = names
        .iter()
        .filter_map(|name| {
            intrinsics
                .iter()
                .find(|(key, _)| key == &text(name))
                .map(|(_, entry)| entry)
        })
        .collect();
    if declared.is_empty() {
        return Ok(false);
    }
    if profile != "echo.dpo@2" || declared.len() != names.len() {
        return Err(());
    }
    equal_text(
        get(catalog, "types")?,
        "echo.dpo@2.OutgoingEdge",
        "Record<edgeType:Bytes<exact=32>,to:Bytes<exact=32>>",
    )?;
    for (name, entry) in names.iter().zip(declared) {
        let (arguments, base, coefficients) = identity_contract(name).ok_or(())?;
        let fields = members(entry)?;
        if fields.len() != 10 {
            return Err(());
        }
        equal_text(entry, "intrinsicClass", "pure")?;
        equal_text(entry, "returnType", "Bytes<exact=32>")?;
        equal_text(entry, "writeClass", "none")?;
        equal_text(entry, "contractVersion", "echo.content-address/v1")?;
        nonempty(get(entry, "footprintTemplate")?)?;
        nonempty(get(entry, "costTemplate")?)?;
        if !list(get(entry, "typeParameters")?)?.is_empty()
            || get(entry, "guardSupport")? != &Value::Bool(false)
        {
            return Err(());
        }
        let declared_arguments = list(get(entry, "argumentTypes")?)?;
        if declared_arguments.len() != arguments.len()
            || declared_arguments
                .iter()
                .zip(&arguments)
                .any(|(value, want)| value != &text(want))
        {
            return Err(());
        }
        let rule = get(entry, "costRule")?;
        if members(rule)?.len() != 2 || get(rule, "base")? != &Value::Integer(base) {
            return Err(());
        }
        let declared_coefficients = list(get(rule, "perArgumentMax")?)?;
        if declared_coefficients.len() != coefficients.len()
            || declared_coefficients
                .iter()
                .zip(&coefficients)
                .any(|(value, want)| value != &Value::Integer(*want))
        {
            return Err(());
        }
    }
    Ok(target_import.is_some())
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
    let listed_failures = list(get(step, "obstructionFailures")?)?;
    let declared_failures = listed_failures
        .iter()
        .map(string)
        .collect::<Check<BTreeSet<_>>>()?;
    // Each failure is declared once, as the runtime requires.
    if declared_failures.len() != mappings.len() || listed_failures.len() != mappings.len() {
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
/// lowered execution order, with identical kinds and contents. A structured
/// order also nests one order per Core loop body or branch arm, under a
/// control entry that repeats the Core node's binder, iterable, bound or
/// predicate. Yields every Core instruction with its Target instruction in
/// pre-order.
fn correspondence<'a>(
    intent: &'a Value,
    lowered: &'a Value,
    structured: bool,
) -> Check<Vec<(&'a Value, &'a Value)>> {
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
    let mut controls = BTreeSet::new();
    let mut paired = Vec::new();
    let mut budget = MAX_NODES;
    pair_block(
        get(intent, "body")?,
        get(lowered, "executionOrder")?,
        structured,
        &mut Pairing {
            instructions: &mut instructions,
            controls: &mut controls,
            paired: &mut paired,
            budget: &mut budget,
        },
        0,
    )?;
    if !instructions.is_empty() {
        return Err(());
    }
    Ok(paired)
}

/// What one pairing pass consumes and produces.
struct Pairing<'p, 'a> {
    /// Target instructions not yet placed.
    instructions: &'p mut BTreeMap<&'a str, (&'static str, &'a Value)>,
    /// Control ids already used.
    controls: &'p mut BTreeSet<&'a str>,
    paired: &'p mut Vec<(&'a Value, &'a Value)>,
    budget: &'p mut usize,
}

fn pair_block<'a>(
    body: &'a Value,
    order: &'a Value,
    structured: bool,
    pairing: &mut Pairing<'_, 'a>,
    depth: usize,
) -> Check<()> {
    if depth > MAX_DEPTH {
        return Err(());
    }
    exact(body, &["locals", "nodes", "result"])?;
    let nodes = list(get(body, "nodes")?)?;
    let order = list(order)?;
    if nodes.len() != order.len() {
        return Err(());
    }
    for (node, entry) in nodes.iter().zip(order) {
        *pairing.budget = pairing.budget.checked_sub(1).ok_or(())?;
        match entry {
            Value::Text(id) => {
                let (kind, instruction) = pairing.instructions.remove(id.as_str()).ok_or(())?;
                same_instruction(node, kind, instruction)?;
                pairing.paired.push((node, instruction));
            }
            Value::Map(_) if structured => pair_control(node, entry, pairing, depth)?,
            _ => return Err(()),
        }
    }
    Ok(())
}

fn pair_control<'a>(
    node: &'a Value,
    entry: &'a Value,
    pairing: &mut Pairing<'_, 'a>,
    depth: usize,
) -> Check<()> {
    let id = nonempty(get(entry, "id")?)?;
    // Control ids are unique and never an instruction id.
    if pairing.instructions.contains_key(id)
        || pairing.paired.iter().any(|(_, instruction)| {
            map_field(instruction, "id") == Some(&Value::Text(id.to_owned()))
        })
        || !pairing.controls.insert(id)
    {
        return Err(());
    }
    let kind = string(get(node, "kind")?)?;
    equal_text(entry, "kind", kind)?;
    let blocks: &[(&str, &str)] = match kind {
        "for" => {
            exact(node, &["kind", "binder", "iter", "bound", "body"])?;
            exact(
                entry,
                &["kind", "id", "binder", "iterable", "bound", "body"],
            )?;
            if get(node, "binder")? != get(entry, "binder")?
                || get(node, "iter")? != get(entry, "iterable")?
                || get(node, "bound")? != get(entry, "bound")?
            {
                return Err(());
            }
            &[("body", "body")]
        }
        "branch" => {
            exact(node, &["kind", "predicate", "then", "else"])?;
            exact(entry, &["kind", "id", "predicate", "then", "else"])?;
            if get(node, "predicate")? != get(entry, "predicate")? {
                return Err(());
            }
            &[("then", "then"), ("else", "else")]
        }
        _ => return Err(()),
    };
    for (core_block, target_order) in blocks {
        let block = get(node, core_block)?;
        // Loop bodies and branch arms are statements: their result is null.
        let result = get(block, "result")?;
        exact(result, &["kind", "value"])?;
        equal_text(result, "kind", "const")?;
        exact(get(result, "value")?, &["kind"])?;
        equal_text(get(result, "value")?, "kind", "null")?;
        pair_block(block, get(entry, target_order)?, true, pairing, depth + 1)?;
    }
    Ok(())
}

/// A Core instruction and its Target instruction agree in kind and content.
fn same_instruction(node: &Value, kind: &str, instruction: &Value) -> Check<()> {
    {
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
/// The only `use target` declaration a source may carry: the selected
/// profile under its pinned digest, as `<id> "sha256:<hex>"`.
fn target_line(profile_reference: &Value) -> Check<String> {
    let [scheme, Value::Bytes(digest)] = list(get(profile_reference, "digest")?)? else {
        return Err(());
    };
    if scheme != &text("sha256") {
        return Err(());
    }
    Ok(format!(
        "{} \"sha256:{}\"",
        string(get(profile_reference, "id")?)?,
        lower_hex(digest)
    ))
}

fn alias(
    source: &str,
    coordinate: &str,
    identity: &Value,
    module: &str,
    target: &str,
) -> Check<String> {
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
    let mut targeted = false;
    for line in lines {
        if !line.starts_with("use ") {
            break;
        }
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        // One `use target` naming exactly the selected, pinned profile.
        if let ["use", "target", name, "digest", value, "as", alias] = tokens.as_slice() {
            let alias = alias.strip_suffix(';').ok_or(())?;
            if format!("{name} {value}") != target || !identifier(alias) || targeted {
                return Err(());
            }
            targeted = true;
            continue;
        }
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

/// A lawpack pin: the compiler frames the exports and adapter digests under
/// the coordinate each reference names, not under a fixed artifact domain.
fn owner_pinned(reference: &Value, value: &Value) -> Check<()> {
    let owner = string(get(reference, "id")?)?;
    pinned(reference, identity(owner, value)?)
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
