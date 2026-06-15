// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Installed contract package registry boundary tests.
#![cfg(feature = "native_rule_bootstrap")]

use echo_registry_api::{
    ArgDef, ContractArtifactRejection, ContractArtifactVerificationPolicy, ObjectDef, OpDef,
    OpKind, RegistryInfo, RegistryProvider,
};
use warp_core::{
    make_node_id, make_type_id, AuthoredObserverPlan, ContractMutationHandler,
    ContractPackageIdentity, ContractQueryObserver, ContractQueryObserverResult, EngineBuilder,
    GraphStore, GraphView, InstalledContractPackage, InstalledContractPackageError, NodeId,
    NodeRecord, ObservationAt, ObservationCoordinate, ObservationError, ObservationFrame,
    ObservationPayload, ObservationProjection, ObservationRequest, ObservationService,
    ObserverPlanId, PatternGraph, ProvenanceService, RewriteRule, TickDelta, WorldlineId,
    WorldlineRuntime, WorldlineState,
};

const SCHEMA_SHA256_HEX: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const MUTATION_OP_ID: u32 = 1001;
const QUERY_OP_ID: u32 = 1002;
const SECOND_MUTATION_OP_ID: u32 = 1003;
const UNKNOWN_OP_ID: u32 = 9999;
const MUTATION_RULE_NAME: &str =
    "cmd/contract/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef/1001/increment";
const SECOND_MUTATION_RULE_NAME: &str =
    "cmd/contract/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef/1003/decrement";
const UNKNOWN_MUTATION_RULE_NAME: &str =
    "cmd/contract/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef/9999/unknown";
const ALT_INCREMENT_RULE_NAME: &str =
    "cmd/contract/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef/1001/incrementAlt";

static INCREMENT_ARGS: &[ArgDef] = &[ArgDef {
    name: "input",
    ty: "IncrementInput",
    required: true,
    list: false,
}];

static OPS: &[OpDef] = &[
    OpDef {
        kind: OpKind::Mutation,
        name: "increment",
        op_id: MUTATION_OP_ID,
        args: INCREMENT_ARGS,
        result_ty: "CounterValue",
        directives_json: "{}",
        footprint_certificate: None,
    },
    OpDef {
        kind: OpKind::Query,
        name: "counterValue",
        op_id: QUERY_OP_ID,
        args: &[],
        result_ty: "CounterValue",
        directives_json: "{}",
        footprint_certificate: None,
    },
    OpDef {
        kind: OpKind::Mutation,
        name: "decrement",
        op_id: SECOND_MUTATION_OP_ID,
        args: INCREMENT_ARGS,
        result_ty: "CounterValue",
        directives_json: "{}",
        footprint_certificate: None,
    },
];

struct StaticRegistry;

impl RegistryProvider for StaticRegistry {
    fn info(&self) -> RegistryInfo {
        RegistryInfo {
            echo_abi_version: 1,
            codec_id: "cbor-canon-v1",
            registry_version: 1,
            schema_sha256_hex: SCHEMA_SHA256_HEX,
            wesley_generator_version: "echo-wesley-gen/0.1.0",
            helper_api_version: 1,
        }
    }

    fn op_by_id(&self, op_id: u32) -> Option<&'static OpDef> {
        OPS.iter().find(|op| op.op_id == op_id)
    }

    fn all_ops(&self) -> &'static [OpDef] {
        OPS
    }

    fn all_enums(&self) -> &'static [echo_registry_api::EnumDef] {
        &[]
    }

    fn all_objects(&self) -> &'static [ObjectDef] {
        &[]
    }
}

fn engine() -> warp_core::Engine {
    let mut store = GraphStore::default();
    let root = make_node_id("root");
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id("world"),
        },
    );
    EngineBuilder::new(store, root).workers(1).build()
}

fn package_identity() -> ContractPackageIdentity<'static> {
    ContractPackageIdentity {
        package_name: "toy-counter",
        package_version: "0.1.0",
        artifact_hash_hex: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    }
}

fn verification_policy() -> ContractArtifactVerificationPolicy<'static> {
    ContractArtifactVerificationPolicy {
        echo_abi_version: 1,
        codec_id: "cbor-canon-v1",
        registry_version: 1,
        schema_sha256_hex: SCHEMA_SHA256_HEX,
        wesley_generator_version: "echo-wesley-gen/0.1.0",
        helper_api_version: 1,
        footprint_certificates: &[],
        require_mutation_footprint_certificates: false,
    }
}

fn mismatched_verification_policy() -> ContractArtifactVerificationPolicy<'static> {
    ContractArtifactVerificationPolicy {
        echo_abi_version: 1,
        codec_id: "cbor-canon-v1",
        registry_version: 1,
        schema_sha256_hex: "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        wesley_generator_version: "echo-wesley-gen/0.1.0",
        helper_api_version: 1,
        footprint_certificates: &[],
        require_mutation_footprint_certificates: false,
    }
}

fn rule_name_for_op_id(op_id: u32) -> &'static str {
    match op_id {
        MUTATION_OP_ID => MUTATION_RULE_NAME,
        SECOND_MUTATION_OP_ID => SECOND_MUTATION_RULE_NAME,
        UNKNOWN_OP_ID => UNKNOWN_MUTATION_RULE_NAME,
        _ => "cmd/contract/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef/4242/other",
    }
}

fn mutation_rule(name: &'static str) -> RewriteRule {
    fn matches(_: GraphView<'_>, _: &NodeId) -> bool {
        false
    }
    fn execute(_: GraphView<'_>, _: &NodeId, _: &mut TickDelta) {}
    fn footprint(_: GraphView<'_>, _: &NodeId) -> warp_core::Footprint {
        warp_core::Footprint::default()
    }

    RewriteRule {
        id: make_type_id(name).0,
        name,
        left: PatternGraph { nodes: vec![] },
        matcher: matches,
        executor: execute,
        compute_footprint: footprint,
        factor_mask: 0,
        conflict_policy: warp_core::ConflictPolicy::Abort,
        join_fn: None,
    }
}

fn mutation_handler(op_id: u32) -> ContractMutationHandler {
    ContractMutationHandler {
        op_id,
        rule: mutation_rule(rule_name_for_op_id(op_id)),
    }
}

fn mutation_handler_with_rule(op_id: u32, rule: RewriteRule) -> ContractMutationHandler {
    ContractMutationHandler { op_id, rule }
}

fn query_observer(query_id: u32) -> ContractQueryObserver {
    ContractQueryObserver::new(query_id, observer_plan(), |_context| {
        Ok(ContractQueryObserverResult::complete(b"value=42".to_vec()))
    })
}

fn observer_plan() -> AuthoredObserverPlan {
    AuthoredObserverPlan {
        plan_id: ObserverPlanId::from_bytes([1; 32]),
        artifact_hash: [2; 32],
        schema_hash: [3; 32],
        state_schema_hash: [4; 32],
        update_law_hash: [5; 32],
        emission_law_hash: [6; 32],
    }
}

fn package_with_ops(mutation_op_id: u32, query_op_id: u32) -> InstalledContractPackage<'static> {
    package_with_identity_ops(package_identity(), mutation_op_id, query_op_id)
}

fn package_with_identity_ops(
    identity: ContractPackageIdentity<'static>,
    mutation_op_id: u32,
    query_op_id: u32,
) -> InstalledContractPackage<'static> {
    package_with_handlers(
        identity,
        verification_policy(),
        vec![mutation_handler(mutation_op_id)],
        vec![query_observer(query_op_id)],
    )
}

fn package_with_handlers(
    identity: ContractPackageIdentity<'static>,
    verification_policy: ContractArtifactVerificationPolicy<'static>,
    mutation_handlers: Vec<ContractMutationHandler>,
    query_observers: Vec<ContractQueryObserver>,
) -> InstalledContractPackage<'static> {
    static REGISTRY: StaticRegistry = StaticRegistry;
    InstalledContractPackage {
        identity,
        registry: &REGISTRY,
        verification_policy,
        mutation_handlers,
        query_observers,
    }
}

fn runtime_with_worldline() -> Result<(WorldlineRuntime, ProvenanceService, WorldlineId), String> {
    let mut runtime = WorldlineRuntime::new();
    let worldline_id = WorldlineId::from_bytes([1; 32]);
    runtime
        .register_worldline(worldline_id, WorldlineState::empty())
        .map_err(|err| format!("worldline registration failed: {err:?}"))?;
    let state = runtime
        .worldlines()
        .get(&worldline_id)
        .ok_or("registered worldline missing")?
        .state()
        .clone();
    let mut provenance = ProvenanceService::new();
    provenance
        .register_worldline(worldline_id, &state)
        .map_err(|err| format!("provenance registration failed: {err:?}"))?;
    Ok((runtime, provenance, worldline_id))
}

fn query_request(worldline_id: WorldlineId, query_id: u32) -> Result<ObservationRequest, String> {
    ObservationRequest::builtin_one_shot(
        ObservationCoordinate {
            worldline_id,
            at: ObservationAt::Frontier,
        },
        ObservationFrame::QueryView,
        ObservationProjection::Query {
            query_id,
            vars_bytes: Vec::new(),
        },
    )
    .map_err(|err| format!("query request construction failed: {err:?}"))
}

#[test]
fn installed_contract_package_binds_supported_mutation_and_query() -> Result<(), String> {
    let mut engine = engine();
    let record = engine
        .register_contract_package(package_with_ops(MUTATION_OP_ID, QUERY_OP_ID))
        .map_err(|err| format!("supported package should install: {err:?}"))?;

    assert_eq!(record.package_name, "toy-counter");
    assert_eq!(record.package_version, "0.1.0");
    assert_eq!(record.registry_info.schema_sha256_hex, SCHEMA_SHA256_HEX);
    assert_eq!(record.mutation_op_ids, vec![MUTATION_OP_ID]);
    assert_eq!(record.query_op_ids, vec![QUERY_OP_ID]);
    assert_eq!(
        engine.installed_contract_mutation_package_id(MUTATION_OP_ID),
        Some(&record.package_id)
    );
    assert_eq!(
        engine.installed_contract_query_package_id(QUERY_OP_ID),
        Some(&record.package_id)
    );

    let (runtime, provenance, worldline_id) = runtime_with_worldline()?;
    let observed = ObservationService::observe(
        &runtime,
        &provenance,
        &engine,
        query_request(worldline_id, QUERY_OP_ID)?,
    )
    .map_err(|err| format!("installed query observer should serve supported query: {err:?}"))?;

    assert_eq!(
        observed.payload,
        ObservationPayload::QueryBytes(b"value=42".to_vec())
    );
    let contract = observed
        .reading
        .contract
        .as_ref()
        .ok_or("installed query reading must carry contract evidence")?;
    assert_eq!(contract.package_id, record.package_id);
    assert_eq!(contract.package_name, "toy-counter");
    assert_eq!(contract.package_version, "0.1.0");
    assert_eq!(
        contract.artifact_hash_hex,
        package_identity().artifact_hash_hex
    );
    assert_eq!(contract.schema_sha256_hex, SCHEMA_SHA256_HEX);
    assert_eq!(contract.echo_abi_version, 1);
    assert_eq!(contract.codec_id, "cbor-canon-v1");
    assert_eq!(contract.registry_version, 1);
    assert_eq!(contract.wesley_generator_version, "echo-wesley-gen/0.1.0");
    assert_eq!(contract.helper_api_version, 1);
    assert_eq!(contract.op_id, QUERY_OP_ID);
    assert_eq!(contract.op_kind, warp_core::ContractOperationKind::Query);
    Ok(())
}

#[test]
fn installed_contract_package_rejects_unknown_mutation_before_registration() -> Result<(), String> {
    let mut engine = engine();

    let Err(err) = engine.register_contract_package(package_with_ops(UNKNOWN_OP_ID, QUERY_OP_ID))
    else {
        return Err("unknown mutation op id must be rejected before install".to_owned());
    };

    assert!(matches!(
        err,
        InstalledContractPackageError::UnknownMutationOperation {
            op_id: UNKNOWN_OP_ID
        }
    ));
    assert_eq!(
        engine.installed_contract_mutation_package_id(UNKNOWN_OP_ID),
        None
    );
    assert_eq!(
        engine.installed_contract_query_package_id(QUERY_OP_ID),
        None
    );
    Ok(())
}

#[test]
fn installed_contract_package_rejects_query_kind_mismatch_before_observer_install(
) -> Result<(), String> {
    let mut engine = engine();

    let Err(err) =
        engine.register_contract_package(package_with_ops(MUTATION_OP_ID, MUTATION_OP_ID))
    else {
        return Err("query observer for mutation op id must be rejected".to_owned());
    };

    assert!(matches!(
        err,
        InstalledContractPackageError::QueryOperationKindMismatch {
            op_id: MUTATION_OP_ID,
            actual: OpKind::Mutation,
        }
    ));
    assert_eq!(
        engine.installed_contract_mutation_package_id(MUTATION_OP_ID),
        None
    );

    let (runtime, provenance, worldline_id) = runtime_with_worldline()?;
    let Err(err) = ObservationService::observe(
        &runtime,
        &provenance,
        &engine,
        query_request(worldline_id, MUTATION_OP_ID)?,
    ) else {
        return Err("rejected observer must not be installed".to_owned());
    };

    assert!(matches!(
        err,
        ObservationError::UnsupportedQuery {
            query_id: MUTATION_OP_ID
        }
    ));
    Ok(())
}

#[test]
fn installed_contract_package_rejects_duplicate_mutation_op_id_before_registration(
) -> Result<(), String> {
    let mut engine = engine();
    let package = package_with_handlers(
        package_identity(),
        verification_policy(),
        vec![
            mutation_handler(MUTATION_OP_ID),
            mutation_handler_with_rule(MUTATION_OP_ID, mutation_rule(ALT_INCREMENT_RULE_NAME)),
        ],
        vec![query_observer(QUERY_OP_ID)],
    );

    let Err(err) = engine.register_contract_package(package) else {
        return Err("duplicate mutation op id must be rejected".to_owned());
    };

    assert!(matches!(
        err,
        InstalledContractPackageError::DuplicateMutationHandlerInPackage {
            op_id: MUTATION_OP_ID
        }
    ));
    assert_eq!(
        engine.installed_contract_mutation_package_id(MUTATION_OP_ID),
        None
    );
    assert_eq!(
        engine.installed_contract_query_package_id(QUERY_OP_ID),
        None
    );
    Ok(())
}

#[test]
fn installed_contract_package_rejects_duplicate_query_op_id_before_registration(
) -> Result<(), String> {
    let mut engine = engine();
    let package = package_with_handlers(
        package_identity(),
        verification_policy(),
        vec![mutation_handler(MUTATION_OP_ID)],
        vec![query_observer(QUERY_OP_ID), query_observer(QUERY_OP_ID)],
    );

    let Err(err) = engine.register_contract_package(package) else {
        return Err("duplicate query op id must be rejected".to_owned());
    };

    assert!(matches!(
        err,
        InstalledContractPackageError::DuplicateQueryObserverInPackage { op_id: QUERY_OP_ID }
    ));
    assert_eq!(
        engine.installed_contract_mutation_package_id(MUTATION_OP_ID),
        None
    );
    assert_eq!(
        engine.installed_contract_query_package_id(QUERY_OP_ID),
        None
    );
    Ok(())
}

#[test]
fn installed_contract_package_rejects_duplicate_rule_id_without_partial_install(
) -> Result<(), String> {
    let mut engine = engine();
    let first = mutation_rule(MUTATION_RULE_NAME);
    let mut second = mutation_rule(SECOND_MUTATION_RULE_NAME);
    second.id = first.id;
    let package = package_with_handlers(
        package_identity(),
        verification_policy(),
        vec![
            mutation_handler_with_rule(MUTATION_OP_ID, first),
            mutation_handler_with_rule(SECOND_MUTATION_OP_ID, second),
        ],
        vec![query_observer(QUERY_OP_ID)],
    );

    let Err(err) = engine.register_contract_package(package) else {
        return Err("duplicate rule id must be rejected".to_owned());
    };

    assert!(matches!(
        err,
        InstalledContractPackageError::DuplicateRuleId { .. }
    ));
    assert_eq!(
        engine.installed_contract_mutation_package_id(MUTATION_OP_ID),
        None
    );
    assert_eq!(
        engine.installed_contract_mutation_package_id(SECOND_MUTATION_OP_ID),
        None
    );
    assert_eq!(
        engine.installed_contract_query_package_id(QUERY_OP_ID),
        None
    );

    engine
        .register_contract_package(package_with_ops(MUTATION_OP_ID, QUERY_OP_ID))
        .map_err(|err| format!("failed install must not leave registered rule behind: {err:?}"))?;
    Ok(())
}

#[test]
fn installed_contract_package_rejects_rule_operation_mismatch_before_registration(
) -> Result<(), String> {
    let mut engine = engine();
    let package = package_with_handlers(
        package_identity(),
        verification_policy(),
        vec![mutation_handler_with_rule(
            MUTATION_OP_ID,
            mutation_rule(SECOND_MUTATION_RULE_NAME),
        )],
        vec![query_observer(QUERY_OP_ID)],
    );

    let Err(err) = engine.register_contract_package(package) else {
        return Err("mismatched mutation rule op id must be rejected".to_owned());
    };

    assert!(matches!(
        err,
        InstalledContractPackageError::MutationRuleOperationMismatch {
            declared_op_id: MUTATION_OP_ID,
            rule_op_id: Some(SECOND_MUTATION_OP_ID),
            rule_name: SECOND_MUTATION_RULE_NAME,
        }
    ));
    assert_eq!(
        engine.installed_contract_mutation_package_id(MUTATION_OP_ID),
        None
    );
    assert_eq!(
        engine.installed_contract_query_package_id(QUERY_OP_ID),
        None
    );
    Ok(())
}

#[test]
fn installed_contract_package_rejects_artifact_verification_without_registration(
) -> Result<(), String> {
    let mut engine = engine();
    let package = package_with_handlers(
        package_identity(),
        mismatched_verification_policy(),
        vec![mutation_handler(MUTATION_OP_ID)],
        vec![query_observer(QUERY_OP_ID)],
    );

    let Err(err) = engine.register_contract_package(package) else {
        return Err("artifact verification mismatch must be rejected".to_owned());
    };

    assert!(matches!(
        err,
        InstalledContractPackageError::ArtifactRejected(_)
    ));
    assert_eq!(
        engine.installed_contract_mutation_package_id(MUTATION_OP_ID),
        None
    );
    assert_eq!(
        engine.installed_contract_query_package_id(QUERY_OP_ID),
        None
    );
    Ok(())
}

#[test]
fn installed_contract_package_rejects_helper_api_mismatch_without_registration(
) -> Result<(), String> {
    let mut engine = engine();
    let mut policy = verification_policy();
    policy.helper_api_version = 2;
    let package = package_with_handlers(
        package_identity(),
        policy,
        vec![mutation_handler(MUTATION_OP_ID)],
        vec![query_observer(QUERY_OP_ID)],
    );

    let Err(err) = engine.register_contract_package(package) else {
        return Err("helper API mismatch must be rejected".to_owned());
    };

    assert!(matches!(
        err,
        InstalledContractPackageError::ArtifactRejected(
            ContractArtifactRejection::HelperApiVersionMismatch {
                expected: 2,
                actual: 1,
            }
        )
    ));
    assert_eq!(
        engine.installed_contract_mutation_package_id(MUTATION_OP_ID),
        None
    );
    assert_eq!(
        engine.installed_contract_query_package_id(QUERY_OP_ID),
        None
    );
    Ok(())
}

#[test]
fn installed_contract_package_rejects_missing_join_fn_without_registration() -> Result<(), String> {
    let mut engine = engine();
    let mut rule = mutation_rule(MUTATION_RULE_NAME);
    rule.conflict_policy = warp_core::ConflictPolicy::Join;
    let package = package_with_handlers(
        package_identity(),
        verification_policy(),
        vec![mutation_handler_with_rule(MUTATION_OP_ID, rule)],
        vec![query_observer(QUERY_OP_ID)],
    );

    let Err(err) = engine.register_contract_package(package) else {
        return Err("join policy without join fn must be rejected".to_owned());
    };

    assert!(matches!(err, InstalledContractPackageError::MissingJoinFn));
    assert_eq!(
        engine.installed_contract_mutation_package_id(MUTATION_OP_ID),
        None
    );
    assert_eq!(
        engine.installed_contract_query_package_id(QUERY_OP_ID),
        None
    );
    Ok(())
}

#[test]
fn installed_contract_package_rejects_duplicate_package() -> Result<(), String> {
    let mut engine = engine();
    let record = engine
        .register_contract_package(package_with_ops(MUTATION_OP_ID, QUERY_OP_ID))
        .map_err(|err| format!("initial package install failed: {err:?}"))?;

    let Err(err) = engine.register_contract_package(package_with_ops(MUTATION_OP_ID, QUERY_OP_ID))
    else {
        return Err("duplicate package id must be rejected".to_owned());
    };

    assert!(matches!(
        err,
        InstalledContractPackageError::DuplicatePackage { package_id }
            if package_id == record.package_id
    ));
    Ok(())
}

#[test]
fn installed_contract_package_rejects_duplicate_installed_operation_ids() -> Result<(), String> {
    let mut engine = engine();
    engine
        .register_contract_package(package_with_ops(MUTATION_OP_ID, QUERY_OP_ID))
        .map_err(|err| format!("initial package install failed: {err:?}"))?;

    let mut identity = package_identity();
    identity.package_version = "0.2.0";
    let Err(err) = engine.register_contract_package(package_with_identity_ops(
        identity,
        MUTATION_OP_ID,
        QUERY_OP_ID,
    )) else {
        return Err("duplicate installed mutation op id must be rejected".to_owned());
    };

    assert!(matches!(
        err,
        InstalledContractPackageError::DuplicateInstalledMutationOperation {
            op_id: MUTATION_OP_ID
        }
    ));
    Ok(())
}

#[test]
fn installed_contract_package_rejects_duplicate_installed_query_id() -> Result<(), String> {
    let mut engine = engine();
    engine
        .register_contract_package(package_with_ops(MUTATION_OP_ID, QUERY_OP_ID))
        .map_err(|err| format!("initial package install failed: {err:?}"))?;

    let mut identity = package_identity();
    identity.package_version = "0.2.0";
    let Err(err) = engine.register_contract_package(package_with_identity_ops(
        identity,
        SECOND_MUTATION_OP_ID,
        QUERY_OP_ID,
    )) else {
        return Err("duplicate installed query op id must be rejected".to_owned());
    };

    assert!(matches!(
        err,
        InstalledContractPackageError::DuplicateInstalledQueryOperation { op_id: QUERY_OP_ID }
    ));
    assert_eq!(
        engine.installed_contract_mutation_package_id(SECOND_MUTATION_OP_ID),
        None
    );
    Ok(())
}
