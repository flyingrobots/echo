// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Runtime-owner patch derivation from actual completed compiled work.
use super::execution::{host, limits};
use super::mutations::fixture;
use super::*;
use warp_core::{
    AttachmentKey, EchoOperationAdmissionPolicy, EchoOperationApplicationBasisV1,
    EchoOperationBudgetV1, EchoOperationInvocationAdmissionPolicy, NodeId, NodeKey, SlotId, WarpOp,
};
fn publication() -> V {
    publication_for("create-node")
}
fn publication_for(name: &str) -> V {
    let (mut publication, _) = fixture(name, "continuum.profile.graph-effects/v1");
    let V::Bytes(raw) = get(&publication, "program") else {
        panic!("program")
    };
    let mut program = echo_edict_canonical::decode_canonical_cbor_v1(raw).expect("program");
    let graph = map([
        (
            "node_types",
            V::Map(vec![
                (
                    id(1),
                    map([
                        ("addressing", V::Integer(1)),
                        ("atom_types", V::Array(vec![id(2)])),
                    ]),
                ),
                (
                    id(7),
                    map([
                        ("addressing", V::Integer(0)),
                        ("atom_types", V::Array(vec![id(2)])),
                    ]),
                ),
            ]),
        ),
        (
            "edge_types",
            V::Map(vec![(
                id(6),
                V::Array(vec![
                    V::Array(vec![id(1), id(1)]),
                    V::Array(vec![id(1), id(7)]),
                    V::Array(vec![id(7), id(1)]),
                    V::Array(vec![id(7), id(7)]),
                ]),
            )]),
        ),
    ]);
    set(&mut program, "graph_schema", V::Bytes(encoded(&graph)));
    let mut permissions = law();
    set(
        &mut permissions,
        "read_node_types",
        V::Array(vec![id(1), id(7)]),
    );
    set(
        &mut permissions,
        "write_node_types",
        V::Array(vec![id(1), id(7)]),
    );
    set(&mut permissions, "write_edge_types", V::Array(vec![id(6)]));
    set(&mut permissions, "max_writes", V::Integer(8));
    set(
        &mut program,
        "footprint_law",
        V::Bytes(encoded(&permissions)),
    );
    set(&mut publication, "program", V::Bytes(encoded(&program)));
    set(
        &mut publication,
        "footprint_contract_identity",
        footprint(&permissions),
    );
    set(
        &mut publication,
        "budget_ceiling",
        map([
            ("steps", V::Integer(10000)),
            ("read_bytes", V::Integer(4096)),
            ("write_bytes", V::Integer(4096)),
        ]),
    );
    publication
}
fn budget() -> EchoOperationBudgetV1 {
    EchoOperationBudgetV1::new(10000, 4096, 4096)
}
fn evaluate(
    host: &mut warp_core::TrustedRuntimeHost,
    head: warp_core::WriterHeadKey,
    publication: &V,
    input: &[u8],
) -> (
    warp_core::InstalledEchoOperation,
    warp_core::AdmittedEchoOperationInvocation,
    warp_core::edict_effects::CompletedEffectEvaluation,
) {
    let raw = encoded(publication);
    let package = prepare(publication).expect("publication");
    let policy = EchoOperationAdmissionPolicy::exact(
        hash(b"echo:operation-package:v1\0", &raw),
        package.operation_coordinate(),
        [8; 32],
        budget(),
    );
    let admitted_package = host
        .admit_echo_operation_package(&policy, raw)
        .expect("package policy");
    let installed = host
        .install_admitted_echo_operation_package(admitted_package)
        .expect("install");
    let app = EchoOperationApplicationBasisV1::new(
        hash(
            b"echo:operation-profile:v1\0",
            b"echo.operation.basis.edict-effect/v1",
        ),
        hash(b"echo:operation-edict-effect-basis:v1\0", input),
    );
    let basis = host
        .echo_operation_evaluation_basis_v1(head, app)
        .expect("parent");
    let invocation = map([
        ("schema", text("echo.operation-invocation/v1")),
        ("kind", text("edict-effect-program/v1")),
        (
            "package_id",
            V::Bytes(installed.package_identity().to_vec()),
        ),
        (
            "operation_coordinate",
            text(installed.operation_coordinate()),
        ),
        ("authority_grant_identity", id(9)),
        (
            "delegated_budget",
            get(publication, "budget_ceiling").clone(),
        ),
        ("application_input_bytes", V::Bytes(input.to_vec())),
        (
            "evaluation_basis",
            map([
                (
                    "worldline_id",
                    V::Bytes(head.worldline_id.as_bytes().to_vec()),
                ),
                ("head_id", V::Bytes(head.head_id.as_bytes().to_vec())),
                (
                    "worldline_tick",
                    V::Integer(i128::from(basis.worldline_tick().as_u64())),
                ),
                ("commit_global_tick", V::Null),
                ("state_root", V::Bytes(basis.state_root().to_vec())),
                ("commit_id", V::Bytes(basis.commit_id().to_vec())),
                (
                    "application_basis_schema_identity",
                    V::Bytes(app.schema_identity().to_vec()),
                ),
                (
                    "application_basis_value_identity",
                    V::Bytes(app.value_identity().to_vec()),
                ),
            ]),
        ),
    ]);
    let admitted = host
        .admit_echo_operation_invocation(
            &EchoOperationInvocationAdmissionPolicy::new([8; 32], [9; 32], budget()),
            &encoded(&invocation),
        )
        .expect("invocation");
    let program = installed.program().as_effect_program().expect("effect");
    let mut frame = host
        .effect_evaluation_frame_v1(basis, &[], 0, budget(), limits())
        .expect("frame")
        .with_footprint_law(program.footprint_law().clone())
        .expect("law");
    frame
        .evaluate_effect(program, input, &[])
        .expect("compiled mutation");
    let completed = frame.into_effect_evaluation(4096).expect("origin");
    (installed, admitted, completed)
}
#[test]
fn runtime_owner_derives_create_node_patch_without_applying_it() {
    let (mut host, head, root) = host();
    let publication = publication();
    let input = encoded(&map([
        ("warpId", V::Bytes(root.warp_id.0.to_vec())),
        ("nodeId", id(17)),
        ("nodeType", id(1)),
        ("atomType", id(2)),
        ("atomBytes", V::Bytes(b"new".to_vec())),
    ]));
    let (installed, admitted, completed) = evaluate(&mut host, head, &publication, &input);
    let basis = admitted.invocation().evaluation_basis();
    let app = basis.application_basis();
    let derived = host
        .derive_echo_operation_effects(&admitted, completed)
        .expect("derived patch");
    let node = NodeKey {
        warp_id: root.warp_id,
        local_id: NodeId([17; 32]),
    };
    assert_eq!(derived.patch().ops().len(), 2);
    assert!(derived.patch().ops().iter().any(|op|matches!(op,WarpOp::UpsertNode {node:target,record} if *target==node && record.ty==TypeId([1;32]))));
    assert!(derived.patch().ops().iter().any(
        |op| matches!(op,WarpOp::SetAttachment {key,..} if *key==AttachmentKey::node_alpha(node))
    ));
    assert_eq!(derived.patch().rule_pack_id(), installed.identity());
    assert!(derived.patch().out_slots().contains(&SlotId::Node(node)));
    assert!(derived
        .patch()
        .out_slots()
        .contains(&SlotId::Attachment(AttachmentKey::node_alpha(node))));
    assert_eq!(derived.consumed().write_bytes(), 67);
    assert_eq!(derived.consumed().read_bytes(), 192);
    assert_eq!(
        host.echo_operation_evaluation_basis_v1(head, app)
            .expect("unchanged"),
        basis
    );
}
fn atom_hash(ty: u8, bytes: &[u8]) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(b"echo.bridge.attachment-value.v1\0");
    h.update(&[0]);
    h.update(&[ty; 32]);
    h.update(&(bytes.len() as u64).to_le_bytes());
    h.update(bytes);
    h.finalize().into()
}
fn leaf_id() -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(b"echo.bridge.node-id.v1\0");
    h.update(&[3; 32]);
    h.update(&[7; 32]);
    h.update(&atom_hash(2, b"new"));
    h.update(&0u64.to_le_bytes());
    h.finalize().into()
}
fn edge_id(from: [u8; 32], to: Option<[u8; 32]>) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(if to.is_some() {
        b"echo.bridge.edge-id.v1\0"
    } else {
        b"echo.bridge.slot-edge-id.v1\0"
    });
    h.update(&[3; 32]);
    h.update(&from);
    h.update(&[6; 32]);
    if let Some(to) = to {
        h.update(&to);
    }
    h.finalize().into()
}
fn make_host(
    edge: bool,
    leaf: bool,
    alias: bool,
) -> (
    warp_core::TrustedRuntimeHost,
    warp_core::WriterHeadKey,
    NodeKey,
) {
    use warp_core::{
        AtomPayload, AttachmentValue, EdgeId, EdgeRecord, GraphStore, NodeRecord, WarpId,
    };
    let root = NodeKey {
        warp_id: WarpId([3; 32]),
        local_id: NodeId([4; 32]),
    };
    let mut store = GraphStore::new(root.warp_id);
    for n in [4, 5] {
        store.insert_node(
            NodeId([n; 32]),
            NodeRecord {
                ty: TypeId([1; 32]),
            },
        );
    }
    store.set_node_attachment(
        root.local_id,
        Some(AttachmentValue::Atom(AtomPayload {
            type_id: TypeId([2; 32]),
            bytes: b"before".to_vec().into(),
        })),
    );
    if leaf {
        store.insert_node(
            NodeId(leaf_id()),
            NodeRecord {
                ty: TypeId([7; 32]),
            },
        );
        store.set_node_attachment(
            NodeId(leaf_id()),
            Some(AttachmentValue::Atom(AtomPayload {
                type_id: TypeId([2; 32]),
                bytes: b"new".to_vec().into(),
            })),
        );
    }
    if edge {
        store.insert_edge(
            root.local_id,
            EdgeRecord {
                id: EdgeId(edge_id([4; 32], None)),
                from: root.local_id,
                to: root.local_id,
                ty: TypeId([6; 32]),
            },
        );
    }
    if alias {
        store.insert_edge(
            root.local_id,
            EdgeRecord {
                id: EdgeId([99; 32]),
                from: root.local_id,
                to: NodeId([5; 32]),
                ty: TypeId([6; 32]),
            },
        );
    }
    super::execution::host_with_store(store, root)
}
fn input(name: &str) -> V {
    let mut fields = vec![("warpId", id(3))];
    match name {
        "ensure-node" => fields.extend([
            ("nodeId", V::Bytes(leaf_id().to_vec())),
            ("nodeType", id(7)),
            ("atomType", id(2)),
            ("atomBytes", V::Bytes(b"new".to_vec())),
        ]),
        "create-node" => fields.extend([
            ("nodeId", id(17)),
            ("nodeType", id(1)),
            ("atomType", id(2)),
            ("atomBytes", V::Bytes(b"new".to_vec())),
        ]),
        "replace-atom" => fields.extend([
            ("nodeId", id(4)),
            (
                "expectedAtomDigest",
                V::Bytes(atom_hash(2, b"before").to_vec()),
            ),
            ("atomType", id(2)),
            ("atomBytes", V::Bytes(b"new".to_vec())),
        ]),
        "ensure-edge" => fields.extend([
            ("edgeId", V::Bytes(edge_id([4; 32], None).to_vec())),
            ("fromId", id(4)),
            ("toId", id(5)),
            ("edgeType", id(6)),
        ]),
        "replace-edge" => fields.extend([
            ("edgeId", V::Bytes(edge_id([4; 32], None).to_vec())),
            ("expectedToId", id(4)),
            ("toId", id(5)),
        ]),
        _ => panic!("kind"),
    }
    map(fields)
}
fn derive_case(
    name: &str,
    body: V,
    publication: V,
    edge: bool,
    leaf: bool,
    alias: bool,
) -> Result<
    warp_core::edict_effects::DerivedEffectPatch,
    warp_core::edict_effects::EffectDerivationError,
> {
    let (mut host, head, _) = make_host(edge, leaf, alias);
    let (_, admitted, completed) = evaluate(&mut host, head, &publication, &encoded(&body));
    let before = admitted.invocation().evaluation_basis();
    let result = host.derive_echo_operation_effects(&admitted, completed);
    assert_eq!(
        host.echo_operation_evaluation_basis_v1(head, before.application_basis())
            .expect(name),
        before
    );
    result
}
#[test]
fn all_five_mutations_derive_their_exact_operation_shapes() {
    for name in [
        "ensure-node",
        "create-node",
        "replace-atom",
        "ensure-edge",
        "replace-edge",
    ] {
        let derived = derive_case(
            name,
            input(name),
            publication_for(name),
            name == "replace-edge",
            false,
            false,
        )
        .expect(name);
        let ops = derived.patch().ops();
        assert_eq!(ops.len(), if name.ends_with("node") { 2 } else { 1 });
        assert_eq!(
            derived.consumed().write_bytes(),
            if name.ends_with("edge") { 128 } else { 67 }
        );
        match name {
            "ensure-node"=>assert!(ops.iter().any(|op|matches!(op,WarpOp::UpsertNode {node,record} if node.local_id.0==leaf_id() && record.ty==TypeId([7;32])))),
            "create-node"=>assert!(ops.iter().any(|op|matches!(op,WarpOp::UpsertNode {node,record} if node.local_id.0==[17;32] && record.ty==TypeId([1;32])))),
            "replace-atom"=>assert!(matches!(&ops[0],WarpOp::SetAttachment {key,value:Some(warp_core::AttachmentValue::Atom(atom))} if key==&AttachmentKey::node_alpha(NodeKey {warp_id:warp_core::WarpId([3;32]),local_id:NodeId([4;32])}) && atom.type_id==TypeId([2;32]) && atom.bytes.as_ref()==b"new")),
            "ensure-edge"|"replace-edge"=>assert!(matches!(&ops[0],WarpOp::UpsertEdge {warp_id,record} if warp_id.0==[3;32] && record.id.0==edge_id([4;32],None) && record.from.0==[4;32] && record.to.0==[5;32] && record.ty.0==[6;32])),
            _=>panic!("kind"),
        }
        derived
            .patch()
            .validate_digest()
            .expect("canonical patch digest");
    }
}
#[test]
fn identical_ensures_derive_no_writes_but_keep_emission_charges() {
    for name in ["ensure-node", "ensure-edge"] {
        let mut body = input(name);
        if name == "ensure-edge" {
            set(&mut body, "toId", id(4));
        }
        let derived = derive_case(
            name,
            body,
            publication_for(name),
            name == "ensure-edge",
            name == "ensure-node",
            false,
        )
        .expect("identical Ensure");
        assert!(derived.patch().ops().is_empty());
        assert!(derived.patch().out_slots().is_empty());
        assert_eq!(
            derived.consumed().write_bytes(),
            if name == "ensure-node" { 67 } else { 128 }
        );
    }
}
#[test]
fn mutation_comparisons_missing_endpoints_and_aliases_refuse_before_patch_derivation() {
    use warp_core::edict_effects::EffectDerivationKind as K;
    for (name, field, value, edge, want) in [
        (
            "replace-atom",
            "expectedAtomDigest",
            id(99),
            false,
            K::PreconditionMismatch,
        ),
        (
            "replace-edge",
            "expectedToId",
            id(99),
            true,
            K::PreconditionMismatch,
        ),
        ("ensure-edge", "toId", id(99), false, K::DanglingReference),
        (
            "create-node",
            "nodeId",
            id(4),
            false,
            K::PreconditionMismatch,
        ),
        (
            "ensure-node",
            "nodeId",
            id(17),
            false,
            K::IdentityLawViolation,
        ),
    ] {
        let mut body = input(name);
        set(&mut body, field, value);
        let error = derive_case(name, body, publication_for(name), edge, false, false)
            .expect_err("must refuse");
        assert_eq!(error.kind(), want, "{name}");
    }
    let error = derive_case(
        "ensure-edge",
        input("ensure-edge"),
        publication_for("ensure-edge"),
        false,
        false,
        true,
    )
    .expect_err("alias");
    assert_eq!(error.kind(), K::IdentityLawViolation);
}

// Removing the owner-token check would admit a foreign runtime's authority.
#[test]
fn derivation_rejects_another_runtime_owners_invocation() {
    let (mut producing, head, _) = make_host(false, false, false);
    let (_, admitted, completed) = evaluate(
        &mut producing,
        head,
        &publication(),
        &encoded(&input("create-node")),
    );
    let (other, _, _) = make_host(false, false, false);
    let error = other
        .derive_echo_operation_effects(&admitted, completed)
        .expect_err("foreign admission authority");
    assert_eq!(
        error.kind(),
        warp_core::edict_effects::EffectDerivationKind::AuthorityMismatch
    );
}

// Resetting the validation meter would lose the successful resolution prefix.
#[test]
fn validation_budget_refusal_retains_evaluation_and_resolution_costs() {
    let mut publication = publication();
    set(
        &mut publication,
        "budget_ceiling",
        map([
            ("steps", V::Integer(10000)),
            ("read_bytes", V::Integer(191)),
            ("write_bytes", V::Integer(4096)),
        ]),
    );
    let error = derive_case(
        "create-node",
        input("create-node"),
        publication,
        false,
        false,
        false,
    )
    .expect_err("three presence reads require 192 bytes");
    assert_eq!(
        error.kind(),
        warp_core::edict_effects::EffectDerivationKind::BudgetExceeded
    );
    assert_eq!(error.consumed().read_bytes(), 128);
    assert_eq!(error.consumed().write_bytes(), 67);
}

fn with_law_field(mut publication: V, name: &str, value: V) -> V {
    let V::Bytes(raw) = get(&publication, "program") else {
        panic!("program")
    };
    let mut program = echo_edict_canonical::decode_canonical_cbor_v1(raw).expect("program");
    let V::Bytes(raw) = get(&program, "footprint_law") else {
        panic!("law")
    };
    let mut law = echo_edict_canonical::decode_canonical_cbor_v1(raw).expect("law");
    set(&mut law, name, value);
    set(&mut program, "footprint_law", V::Bytes(encoded(&law)));
    set(&mut publication, "program", V::Bytes(encoded(&program)));
    set(
        &mut publication,
        "footprint_contract_identity",
        footprint(&law),
    );
    publication
}

// Ignoring range writes would incorrectly fit an edge write into a one-slot law.
#[test]
fn resolved_write_limit_counts_attachment_and_range_dependencies() {
    for name in ["create-node", "ensure-edge", "replace-edge"] {
        let publication = with_law_field(publication_for(name), "max_writes", V::Integer(1));
        let error = derive_case(
            name,
            input(name),
            publication,
            name == "replace-edge",
            false,
            false,
        )
        .expect_err("each mutation resolves two write slots");
        assert_eq!(
            error.kind(),
            warp_core::edict_effects::EffectDerivationKind::FootprintLawViolation,
            "{name}"
        );
    }
}

// Returning early for an idempotent Ensure must not skip typed permissions.
#[test]
fn identical_ensure_still_requires_write_permission() {
    let publication = with_law_field(
        publication_for("ensure-node"),
        "write_node_types",
        V::Array(vec![]),
    );
    let error = derive_case(
        "ensure-node",
        input("ensure-node"),
        publication,
        false,
        true,
        false,
    )
    .expect_err("no-op Ensure lacks node write permission");
    assert_eq!(
        error.kind(),
        warp_core::edict_effects::EffectDerivationKind::FootprintLawViolation
    );
}
