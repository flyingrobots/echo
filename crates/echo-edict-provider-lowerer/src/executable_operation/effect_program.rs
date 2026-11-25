// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Lowerer-owned judgment of a compiler-produced authored effect program.
//!
//! The lowerer establishes, without consulting the runtime, that the program
//! is bound to its own artifacts, that each effect is authorized by its pinned
//! adapter and conforms to its intrinsic's runtime ABI, that every failure arm
//! names a declared obstruction through the version-scoped failure table, that
//! Core and Target IR agree node for node, and that the body is typed, proven
//! and within its declared budget.
use super::source_functions::{judge_effect_intent, EffectSite};
use crate::refusal_reason::{ProgramRefusal, RefusalReason as R, Trail};
use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, digest_canonical_value_bytes_v1 as digest,
    encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value,
};
use std::collections::{BTreeMap, BTreeSet};

type Check<T> = Result<T, ()>;

const PROFILE_GRAPH_EFFECTS: &str = "continuum.profile.graph-effects/v1";
/// The ordered Target IR contract: a flat order of instruction ids.
const ORDERED_TARGET_IR: &str = "echo.span-ir/v2";
/// The structured Target IR contract: an order tree with `for` and `branch`.
const STRUCTURED_TARGET_IR: &str = "echo.span-ir/v3";
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

const fn resource(catalog: &'static str, runtime: &'static str) -> FailureName {
    FailureName {
        catalog,
        runtime,
        authority: "resourceFault",
    }
}

/// The typed outgoing-edge read's failures (echo#982): the bounded-read
/// route's runtime tags, each a resource limit.
const OUTGOING_EDGE_READ_V1: [FailureName; 3] = [
    resource("edgeSourceMissing", "echo.edge-source-missing/v1"),
    resource("edgeBoundExceeded", "echo.edge-bound-exceeded/v1"),
    resource("edgeScanBoundExceeded", "echo.edge-scan-bound-exceeded/v1"),
];

/// The guided closure read's failures: resource limits, an insufficient
/// selector and malformed storage stay three authority classes.
const CLOSURE_READ_V1: [FailureName; 7] = [
    resource("closureStartMissing", "echo.closure-start-missing/v1"),
    resource(
        "closureNodeBoundExceeded",
        "echo.closure-node-bound-exceeded/v1",
    ),
    resource(
        "closureDepthBoundExceeded",
        "echo.closure-depth-bound-exceeded/v1",
    ),
    resource(
        "closureScanBoundExceeded",
        "echo.closure-scan-bound-exceeded/v1",
    ),
    resource(
        "closureWorkBoundExceeded",
        "echo.closure-work-bound-exceeded/v1",
    ),
    FailureName {
        catalog: "closureSelectorInsufficient",
        runtime: "echo.closure-selector-insufficient/v1",
        authority: "domainMappable",
    },
    FailureName {
        catalog: "closureStorageMalformed",
        runtime: "echo.closure-storage-malformed/v1",
        authority: "integrityFault",
    },
];

/// The failure contract of each runtime intrinsic, keyed by its operation
/// below either effect namespace. An intrinsic outside this table is refused.
fn failures(intrinsic: &str) -> Option<&'static [FailureName]> {
    match super::intrinsic_names::operation(intrinsic)? {
        "node-atom-read" => Some(&NODE_ATOM_READ_V1),
        "outgoing-edge-read" => Some(&OUTGOING_EDGE_READ_V1),
        "closure-read" => Some(&CLOSURE_READ_V1),
        "ensure-node" | "create-node" | "replace-atom" | "ensure-edge" | "replace-edge" => {
            Some(&[])
        }
        _ => None,
    }
}

/// The graph-read declaration of an authored-effect configuration, the
/// record the bounded-read configuration also uses (echo#982).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GraphReads<'a> {
    /// Bucket edges one evaluation may inspect, matching or not.
    pub(crate) max_edge_scans: u64,
    /// Distinct nodes one closure read may materialize.
    pub(crate) max_closure_nodes: u64,
    /// Deepest occurrence one closure read may select.
    pub(crate) max_closure_depth: u64,
    /// Module-local name of the source helper deciding each closure child.
    pub(crate) selector: Option<&'a str>,
}

/// Parses one `graphReads` record: exactly its three ceilings, within the
/// bounded-read bounds, and an optional nonempty selector name.
pub(crate) fn graph_reads(declared: &Value) -> Check<GraphReads<'_>> {
    let selector = map_field(declared, "closureSelector")
        .map(string)
        .transpose()?;
    let mut names = vec!["maxEdgeScans", "maxClosureNodes", "maxClosureDepth"];
    if selector.is_some() {
        names.push("closureSelector");
    }
    exact(declared, &names)?;
    let graph = GraphReads {
        max_edge_scans: number(get(declared, "maxEdgeScans")?)?,
        max_closure_nodes: number(get(declared, "maxClosureNodes")?)?,
        max_closure_depth: number(get(declared, "maxClosureDepth")?)?,
        selector,
    };
    if !(1..=65_536).contains(&graph.max_edge_scans)
        || graph.max_closure_nodes > 4096
        || graph.max_closure_depth > 256
        || selector.is_some_and(|name| name.is_empty() || name.len() > 1024)
    {
        return Err(());
    }
    Ok(graph)
}

/// The effect kind, which is also the write class, of each intrinsic.
fn effect_kind(intrinsic: &str) -> Option<&'static str> {
    Some(match super::intrinsic_names::operation(intrinsic)? {
        "node-atom-read" | "outgoing-edge-read" | "closure-read" => "read",
        "create-node" => "create",
        "ensure-node" | "ensure-edge" => "ensure",
        "replace-atom" | "replace-edge" => "replace",
        _ => return None,
    })
}

/// Judges compiler-produced effect-program bytes under the configuration's
/// graph-read declaration, which is present exactly when a step reads the
/// graph.
///
/// # Errors
/// Names the check that refused and the artifact path it was reading.
pub(crate) fn judge(
    compiled_program: &[u8],
    graph: Option<GraphReads<'_>>,
) -> Result<(), ProgramRefusal> {
    let mut trail = Trail::new();
    judge_on(compiled_program, graph, &mut trail).map_err(|()| trail.refusal())
}

fn judge_on(
    compiled_program: &[u8],
    graph: Option<GraphReads<'_>>,
    trail: &mut Trail,
) -> Check<()> {
    trail.at(R::BundleEnvelope, &[]);
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
    let structured = artifacts(&module, &target, &exports, trail)?;
    trail.at(R::IntentSelection, &["core", "intents", intent_name]);
    let intent = get(get(&module, "intents")?, intent_name)?;
    trail.at(R::IntentSelection, &["target", "intents", intent_name]);
    let lowered = get(get(&target, "intents")?, intent_name)?;
    trail.at(R::IntentBinding, &["core", "intents", intent_name]);
    binding(intent, lowered, trail)?;
    trail.at(
        R::CorrespondenceEnvelope,
        &["target", "intents", intent_name],
    );
    let pairs = correspondence(intent, lowered, structured, trail)?;
    let (sites, identity) = relation(
        &module,
        (intent_name, intent),
        &target,
        &pairs,
        &exports,
        get(&bundle, "lowering_artifacts")?,
        trail,
    )?;
    // A graph read is a declared capability, as on the bounded-read route:
    // the declaration is the one the program uses, and a closure names its
    // selector.
    trail.at(R::GraphReadDeclaration, &["configuration", "graphReads"]);
    let reads = sites
        .values()
        .filter(|site| site.graph_read().is_some())
        .count();
    let closures = sites
        .values()
        .filter(|site| site.graph_read() == Some("closure-read"))
        .count();
    if graph.is_some() != (reads > 0)
        || (closures > 0 && graph.and_then(|graph| graph.selector).is_none())
    {
        return Err(());
    }
    trail.at(R::TypeTable, &["core", "types"]);
    judge_effect_intent(
        &module,
        &exports,
        (intent_name, intent),
        &sites,
        graph,
        identity,
        trail,
    )
}

/// The three embedded artifacts and the Core pin the Target IR carries.
/// Returns whether the Target IR is structured (`echo.span-ir/v3`) rather
/// than ordered (`echo.span-ir/v2`).
fn artifacts(module: &Value, target: &Value, exports: &Value, trail: &mut Trail) -> Check<bool> {
    trail.at(R::TargetIrEnvelope, &["target"]);
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
    let structured = match (
        string(get(target, "domain")?)?,
        string(get(target, "kind")?)?,
    ) {
        (ORDERED_TARGET_IR, "orderedTargetIrArtifact") => false,
        (STRUCTURED_TARGET_IR, "structuredTargetIrArtifact") => true,
        _ => return Err(()),
    };
    trail.at(R::ExportsEnvelope, &["exports"]);
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
    trail.at(R::CoreEnvelope, &["core"]);
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
    trail.at(R::CorePin, &["target", "semanticClosure", "sourceCore"]);
    equal_text(target, "sourceCoreCoordinate", coordinate)?;
    let source = get(get(target, "semanticClosure")?, "sourceCore")?;
    exact(source, &["id", "digest"])?;
    equal_text(source, "id", coordinate)?;
    pinned(source, identity("edict.core.module/v1", module)?)?;
    Ok(structured)
}

/// The selected intent and its lowered form agree on everything the runtime
/// executes besides the body's instructions.
fn binding(intent: &Value, lowered: &Value, trail: &mut Trail) -> Check<()> {
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
        trail.mark(R::OpticIntent);
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
    (intent_name, intent): (&str, &'a Value),
    target_artifact: &'a Value,
    pairs: &[(&'a Value, &'a Value)],
    exports: &'a Value,
    lowering: &'a Value,
    trail: &mut Trail,
) -> Check<(BTreeMap<usize, EffectSite>, bool)> {
    trail.at(R::NoEffectProfile, &["target", "targetProfile"]);
    let structured = string(get(target_artifact, "domain")?)? == STRUCTURED_TARGET_IR;
    let effects = pairs
        .iter()
        .filter(|(node, _)| map_field(node, "kind") == Some(&text("effect")))
        .count();
    if effects == 0 {
        // Without an effect step no profile is pinned. Keep the no-effect
        // path on frozen @1; @2 needs authentication in the lowering artifacts.
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
    trail.at(R::LoweringArtifacts, &["lowering_artifacts"]);
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
    trail.at(R::SourceClosure, &["lowering_artifacts", "source_artifact"]);
    let sources =
        super::source_modules::Sources::new(&source, string(get(module, "coordinate")?)?)?;
    trail.at(R::LoweringArtifacts, &["lowering_artifacts"]);
    let lawpack = embedded("lawpack_artifact")?;
    let adapter = embedded("adapter_artifact")?;
    let profile = embedded("target_profile_artifact")?;
    let catalog = embedded("intrinsics_artifact")?;
    trail.at(
        R::TargetProfile,
        &["lowering_artifacts", "target_profile_artifact"],
    );
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
    // `echo.dpo@1` is frozen; only `echo.dpo@2` carries structured control
    // (owner decision on flyingrobots/echo#947).
    if structured && !namespace.structured {
        return Err(());
    }
    trail.at(
        R::IntrinsicCatalog,
        &["lowering_artifacts", "intrinsics_artifact"],
    );
    let catalog_reference = get(&profile, "intrinsics")?;
    equal_text(catalog_reference, "id", namespace.catalog)?;
    pinned(catalog_reference, identity(namespace.catalog, &catalog)?)?;
    // Catalog v3 carries its template `parameters` and may also publish the
    // named types its pure contracts reference; the frozen v1 catalog has
    // exactly two fields.
    let templated = namespace.catalog_api == "edict.target-profile.intrinsics/v3";
    if templated {
        let typed = get(&catalog, "types").is_ok();
        let fields: &[&str] = if typed {
            &["apiVersion", "intrinsics", "parameters", "types"]
        } else {
            &["apiVersion", "intrinsics", "parameters"]
        };
        exact(&catalog, fields)?;
        members(get(&catalog, "parameters")?)?;
        if typed {
            members(get(&catalog, "types")?)?;
        }
    } else {
        exact(&catalog, &["apiVersion", "intrinsics"])?;
    }
    equal_text(&catalog, "apiVersion", namespace.catalog_api)?;
    trail.mark(R::IdentityContracts);
    let identity_enabled =
        identity_contracts(namespace.coordinate, &catalog, module, profile_reference)?;
    trail.mark(R::IntrinsicCatalog);
    let catalog = get(&catalog, "intrinsics")?;
    members(catalog)?;
    trail.at(R::Lawpack, &["lowering_artifacts", "lawpack_artifact"]);
    equal_text(&lawpack, "apiVersion", "edict.lawpack/v1")?;
    trail.at(R::Adapter, &["lowering_artifacts", "adapter_artifact"]);
    equal_text(&adapter, "apiVersion", "edict.lawpack-adapter/v1")?;
    equal_text(&adapter, "class", "declarative")?;
    trail.at(R::Lawpack, &["lowering_artifacts", "lawpack_artifact"]);
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
    trail.at(R::Adapter, &["lowering_artifacts", "adapter_artifact"]);
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
    trail.at(R::SourceAlias, &["lowering_artifacts", "source_artifact"]);
    let target = target_line(profile_reference)?;
    let preamble = |module, source| {
        preamble(
            source,
            &lawpack_coordinate,
            &lawpack_identity,
            module,
            &target,
        )
    };
    let (alias, root_pins) = preamble(string(get(module, "coordinate")?)?, sources.root())?;
    let alias = alias.ok_or(())?;
    trail.at(
        R::SourceModulePins,
        &["lowering_artifacts", "source_artifact", "modules"],
    );
    sources.require_pinned(root_pins, |module, source| {
        preamble(module, source).map(|(_, pins)| pins)
    })?;
    trail.at(R::ImportedTypes, &["core", "types"]);
    imported_types(module, exports, &lawpack_coordinate)?;
    trail.at(
        R::OperationProfile,
        &[
            "lowering_artifacts",
            "adapter_artifact",
            "operationProfiles",
        ],
    );
    let operation_profile = string(get(intent, "requiredOperationProfile")?)?;
    let authorized = list(get(
        unique_profile(&adapter, operation_profile)?,
        "semanticEffects",
    )?)?;
    let implementations = get(&adapter, "effectImplementations")?;
    // A site is keyed by its instruction's position in the pre-order walk of
    // the Core body, which the body judgment repeats.
    let mut sites = BTreeMap::new();
    for (index, &(node, step)) in pairs.iter().enumerate() {
        if string(get(node, "kind")?)? != "effect" {
            continue;
        }
        trail.at(
            R::EffectAuthorization,
            &[
                "target",
                "intents",
                intent_name,
                "steps",
                string(get(step, "id")?)?,
            ],
        );
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
        trail.mark(R::IntrinsicContract);
        contract(
            intrinsic_name,
            kind,
            intrinsic,
            effect,
            implementation,
            templated,
        )?;
        let output = string(get(effect, "outputType")?)?;
        equal_text(get(node, "binding")?, "type", output)?;
        trail.mark(R::FailureArms);
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
    trail.at(
        R::EffectAuthorization,
        &["target", "intents", intent_name, "steps"],
    );
    if sites.len() != effects {
        return Err(());
    }
    Ok((sites, identity_enabled))
}

/// The identity intrinsics' exact contract, independently restated: the
/// operand ceilings, `echo.content-address/v1` and the runtime's cost floor.
const IDENTITY_ID: &str = "Bytes<exact=32>";
const IDENTITY_EDGE_TYPE: (&str, &str) = (
    "echo.dpo@2.OutgoingEdge",
    "Record<edgeType:Bytes<exact=32>,to:Bytes<exact=32>>",
);
const IDENTITY_CONTRACTS: [(&str, &[&str], u64, &[u64]); 4] = [
    (
        "echo.dpo@2.contentAddress",
        &[
            IDENTITY_ID,
            IDENTITY_ID,
            IDENTITY_ID,
            "Bytes<max=1048576>",
            "List<echo.dpo@2.OutgoingEdge,max=16>",
        ],
        71,
        &[0, 0, 0, 1, 67],
    ),
    (
        "echo.dpo@2.contentEdgeId",
        &[IDENTITY_ID, IDENTITY_ID, IDENTITY_ID, IDENTITY_ID],
        70,
        &[0, 0, 0, 0],
    ),
    (
        "echo.dpo@2.slotEdgeId",
        &[IDENTITY_ID, IDENTITY_ID, IDENTITY_ID],
        69,
        &[0, 0, 0],
    ),
    (
        "echo.dpo@2.attachmentValueDigest",
        &[IDENTITY_ID, "Bytes<max=1048576>"],
        68,
        &[0, 1],
    ),
];

/// Whether identity calls are admitted: the pinned catalog under
/// `echo.dpo@2` carries exactly Echo's four identity contracts and their
/// edge type, and Core imports exactly the selected profile as its target.
/// A catalog carrying any of them otherwise, or under another profile, is
/// refused rather than silently ignored.
fn identity_contracts(
    profile: &str,
    catalog: &Value,
    module: &Value,
    profile_reference: &Value,
) -> Check<bool> {
    let intrinsics = get(catalog, "intrinsics")?;
    let present = IDENTITY_CONTRACTS
        .iter()
        .filter(|(name, ..)| map_field(intrinsics, name).is_some())
        .count();
    let mut targets = list(get(module, "imports")?)?
        .iter()
        .filter(|entry| map_field(entry, "kind") == Some(&text("target")));
    let target = targets.next();
    if targets.next().is_some() || target.is_some_and(|t| get(t, "ref") != Ok(profile_reference)) {
        return Err(());
    }
    if present == 0 {
        return Ok(false);
    }
    if profile != "echo.dpo@2" || present != IDENTITY_CONTRACTS.len() {
        return Err(());
    }
    let (edge_name, edge_definition) = IDENTITY_EDGE_TYPE;
    let types = get(catalog, "types")?;
    if get(types, edge_name)? != &text(edge_definition) {
        return Err(());
    }
    for (name, arguments, base, per_argument) in IDENTITY_CONTRACTS {
        let entry = get(intrinsics, name)?;
        exact(
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
        let texts = |values: &[&str]| Value::Array(values.iter().map(|v| text(v)).collect());
        let integers = |values: &[u64]| {
            Value::Array(
                values
                    .iter()
                    .map(|v| Value::Integer(i128::from(*v)))
                    .collect(),
            )
        };
        let rule = get(entry, "costRule")?;
        exact(rule, &["base", "perArgumentMax"])?;
        if get(entry, "intrinsicClass")? != &text("pure")
            || get(entry, "typeParameters")? != &Value::Array(vec![])
            || get(entry, "argumentTypes")? != &texts(arguments)
            || get(entry, "returnType")? != &text(IDENTITY_ID)
            || get(entry, "guardSupport")? != &Value::Bool(false)
            || get(entry, "writeClass")? != &text("none")
            || get(entry, "contractVersion")? != &text("echo.content-address/v1")
            || get(rule, "base")? != &Value::Integer(i128::from(base))
            || get(rule, "perArgumentMax")? != &integers(per_argument)
        {
            return Err(());
        }
        nonempty(get(entry, "footprintTemplate")?)?;
        nonempty(get(entry, "costTemplate")?)?;
    }
    Ok(target.is_some())
}

/// The catalog entry, exported effect and adapter implementation agree with
/// each other and with the intrinsic's runtime ABI and failure table.
fn contract(
    name: &str,
    kind: &str,
    intrinsic: &Value,
    effect: &Value,
    implementation: &Value,
    templated: bool,
) -> Check<()> {
    // A catalog v3 effect intrinsic carries its `effectCostRule`; its bytes
    // are fixed by the pinned catalog digest.
    let rule: &[&str] = if templated { &["effectCostRule"] } else { &[] };
    exact(
        intrinsic,
        &[
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
            ][..],
            rule,
        ]
        .concat(),
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
/// lowered execution order, with identical kinds and contents. Under the
/// structured contract a `for` or `branch` entry copies its Core loop or
/// branch and nests one order per Core block. Returns each Core instruction
/// with its Target instruction, in pre-order.
fn correspondence<'a>(
    intent: &'a Value,
    lowered: &'a Value,
    structured: bool,
    trail: &mut Trail,
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
    trail.mark(R::CorrespondenceOrder);
    trail.enter("executionOrder");
    let mut walk = OrderWalk {
        structured,
        identities: instructions.keys().copied().collect(),
        instructions,
        remaining: MAX_NODES,
        pairs: Vec::new(),
        trail,
    };
    walk.block(get(intent, "body")?, get(lowered, "executionOrder")?, 0)?;
    walk.trail.mark(R::CorrespondenceOrder);
    if !walk.instructions.is_empty() {
        return Err(());
    }
    Ok(walk.pairs)
}

/// One pass over a Core body and its Target order.
struct OrderWalk<'a, 't> {
    structured: bool,
    /// Every instruction and control id seen so far.
    identities: BTreeSet<&'a str>,
    /// Target instructions not yet placed, by id.
    instructions: BTreeMap<&'a str, (&'static str, &'a Value)>,
    remaining: usize,
    pairs: Vec<(&'a Value, &'a Value)>,
    trail: &'t mut Trail,
}
impl<'a> OrderWalk<'a, '_> {
    fn block(&mut self, body: &'a Value, order: &'a Value, depth: usize) -> Check<()> {
        self.trail.mark(R::CorrespondenceOrder);
        if depth > MAX_DEPTH {
            return Err(());
        }
        exact(body, &["locals", "nodes", "result"])?;
        let nodes = list(get(body, "nodes")?)?;
        let entries = list(order)?;
        if nodes.len() != entries.len() {
            return Err(());
        }
        for (position, (node, entry)) in nodes.iter().zip(entries).enumerate() {
            self.trail.enter(position.to_string());
            self.trail.mark(R::CorrespondenceOrder);
            spend(&mut self.remaining)?;
            if let Value::Text(id) = entry {
                let (kind, instruction) = self.instructions.remove(id.as_str()).ok_or(())?;
                self.trail.mark(R::CorrespondenceInstruction);
                instruction_correspondence(node, kind, instruction)?;
                self.pairs.push((node, instruction));
            } else if self.structured {
                self.control(node, entry, depth)?;
            } else {
                return Err(());
            }
            self.trail.leave();
        }
        Ok(())
    }
    /// A `for` or `branch` entry: a unique control id, the Core node's
    /// binder, iterable, bound or predicate exactly, and one nested order per
    /// Core block.
    fn control(&mut self, node: &'a Value, entry: &'a Value, depth: usize) -> Check<()> {
        self.trail.mark(R::CorrespondenceControl);
        let id = nonempty(get(entry, "id")?)?;
        if !self.identities.insert(id) {
            return Err(());
        }
        let kind = string(get(node, "kind")?)?;
        equal_text(entry, "kind", kind)?;
        match kind {
            "for" => {
                exact(node, &["kind", "binder", "iter", "bound", "body"])?;
                exact(
                    entry,
                    &["kind", "id", "binder", "iterable", "bound", "body"],
                )?;
                for (source, target) in [
                    ("binder", "binder"),
                    ("iter", "iterable"),
                    ("bound", "bound"),
                ] {
                    if get(node, source)? != get(entry, target)? {
                        return Err(());
                    }
                }
                self.child("body", get(node, "body")?, get(entry, "body")?, depth + 1)
            }
            "branch" => {
                // A value branch binds a local; only statement branches lower
                // to control entries.
                exact(node, &["kind", "predicate", "then", "else"])?;
                exact(entry, &["kind", "id", "predicate", "then", "else"])?;
                if get(node, "predicate")? != get(entry, "predicate")? {
                    return Err(());
                }
                self.child("then", get(node, "then")?, get(entry, "then")?, depth + 1)?;
                self.child("else", get(node, "else")?, get(entry, "else")?, depth + 1)
            }
            _ => Err(()),
        }
    }
    /// A loop body or branch arm is a statement block whose result is `null`.
    fn child(&mut self, arm: &str, body: &'a Value, order: &'a Value, depth: usize) -> Check<()> {
        self.trail.enter(arm);
        self.trail.mark(R::CorrespondenceStatementBlock);
        let result = get(body, "result")?;
        exact(result, &["kind", "value"])?;
        equal_text(result, "kind", "const")?;
        let value = get(result, "value")?;
        exact(value, &["kind"])?;
        equal_text(value, "kind", "null")?;
        self.block(body, order, depth)?;
        self.trail.leave();
        self.trail.mark(R::CorrespondenceControl);
        Ok(())
    }
}

/// One Core instruction and its Target instruction carry the same contents.
fn instruction_correspondence(node: &Value, kind: &str, instruction: &Value) -> Check<()> {
    // A pure binding with failure arms lowers an `attempt`.
    let kind = if kind == "let" && map_field(instruction, "failureArms").is_some() {
        "attempt"
    } else {
        kind
    };
    equal_text(node, "kind", kind)?;
    let keys: &[&str] = match kind {
        "attempt" => {
            exact(instruction, &["id", "binding", "value", "failureArms"])?;
            if get(node, "failureMap")? != get(instruction, "failureArms")? {
                return Err(());
            }
            &["binding", "value"]
        }
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

/// One module's declaration preamble: its `package` line, then only `use`
/// lines. A `use target` names exactly the selected, pinned profile, at most
/// once; a `use lawpack` names exactly the bound lawpack, at most once; a
/// `use module` pins a source module, which the closure check resolves.
/// Returns the lawpack alias, if declared, and the module pins.
fn preamble<'s>(
    source: &'s str,
    coordinate: &str,
    identity: &Value,
    module: &str,
    target: &str,
) -> Check<(Option<String>, Vec<super::source_modules::Pin<'s>>)> {
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
    let mut pins = Vec::new();
    for line in lines {
        if !line.starts_with("use ") {
            break;
        }
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        let [_, kind, name, "digest", value, "as", alias] = tokens.as_slice() else {
            return Err(());
        };
        let alias = alias.strip_suffix(';').ok_or(())?;
        if !identifier(alias) {
            return Err(());
        }
        match *kind {
            "target" => {
                if format!("{name} {value}") != target || targeted {
                    return Err(());
                }
                targeted = true;
            }
            "lawpack" => {
                if *name != coordinate
                    || *value != expected
                    || result.replace(alias.to_owned()).is_some()
                {
                    return Err(());
                }
            }
            "module" => {
                if pins.iter().any(|(pinned, _)| pinned == name) {
                    return Err(());
                }
                pins.push((*name, *value));
            }
            _ => return Err(()),
        }
    }
    Ok((result, pins))
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
