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
    let (publication, _) = fixture(name, "continuum.profile.graph-effects/v1");
    with_graph_law(publication)
}
/// The graph schema and footprint law every derivation case runs under.
fn with_graph_law(mut publication: V) -> V {
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
    evaluate_with_aperture(host, head, publication, input, &[])
}
fn evaluate_with_aperture(
    host: &mut warp_core::TrustedRuntimeHost,
    head: warp_core::WriterHeadKey,
    publication: &V,
    input: &[u8],
    aperture: &[warp_core::edict_effects::TypedNodeRead],
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
        .evaluate_effect(program, input, aperture)
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

fn portal_host(
    depth: u8,
    beta: bool,
) -> (
    warp_core::TrustedRuntimeHost,
    warp_core::WriterHeadKey,
    Vec<NodeKey>,
    NodeKey,
) {
    use warp_core::{
        EdgeId, EdgeKey, EdgeRecord, PortalInit, TickCommitStatus, WarpId, WarpTickPatchV1,
    };
    let (host, head, root) = make_host(false, false, false);
    let mut state = host
        .runtime()
        .worldlines()
        .get(&head.worldline_id)
        .expect("worldline")
        .state()
        .warp_state()
        .clone();
    let mut owner = root;
    let mut owners = Vec::new();
    for index in 0..depth {
        owners.push(owner);
        let child = NodeKey {
            warp_id: WarpId([8 + index; 32]),
            local_id: NodeId([4; 32]),
        };
        let key = if beta {
            let edge = EdgeKey {
                warp_id: owner.warp_id,
                local_id: EdgeId([6; 32]),
            };
            state
                .store_mut(&owner.warp_id)
                .expect("parent store")
                .insert_edge(
                    owner.local_id,
                    EdgeRecord {
                        id: edge.local_id,
                        from: owner.local_id,
                        to: owner.local_id,
                        ty: TypeId([6; 32]),
                    },
                );
            AttachmentKey::edge_beta(edge)
        } else {
            AttachmentKey::node_alpha(owner)
        };
        WarpTickPatchV1::new(
            0,
            [0; 32],
            TickCommitStatus::Committed,
            vec![],
            vec![],
            vec![WarpOp::OpenPortal {
                key,
                child_warp: child.warp_id,
                child_root: child.local_id,
                init: PortalInit::Empty {
                    root_record: warp_core::NodeRecord {
                        ty: TypeId([1; 32]),
                    },
                },
            }],
        )
        .apply_to_state(&mut state)
        .expect("portal");
        owner = child;
    }
    let (host, head, _) = super::execution::host_with_state(
        warp_core::WorldlineState::new(state, root).expect("portal state"),
    );
    (host, head, owners, owner)
}

// Omitting portal resolutions would lose ancestors from the derived footprint.
#[test]
fn descendant_mutation_records_every_portal_dependency_and_charge() {
    for depth in [1, 2] {
        let (mut host, head, owners, target) = portal_host(depth, false);
        let mut body = input("create-node");
        set(&mut body, "warpId", V::Bytes(target.warp_id.0.to_vec()));
        let publication =
            with_law_field(publication(), "portal_owner_types", V::Array(vec![id(1)]));
        let (_, admitted, completed) = evaluate(&mut host, head, &publication, &encoded(&body));
        let basis = admitted.invocation().evaluation_basis();
        let derived = host
            .derive_echo_operation_effects(&admitted, completed)
            .expect("descendant mutation");
        assert_eq!(
            derived.consumed().read_bytes(),
            192 + u64::from(depth) * 128
        );
        assert_eq!(derived.consumed().write_bytes(), 67);
        for owner in owners {
            assert!(derived.patch().in_slots().contains(&SlotId::Node(owner)));
            assert!(derived
                .patch()
                .in_slots()
                .contains(&SlotId::Attachment(AttachmentKey::node_alpha(owner))));
            assert!(derived.footprint().n_read.iter().any(|key| *key == owner));
            assert!(derived
                .footprint()
                .a_read
                .iter()
                .any(|key| *key == AttachmentKey::node_alpha(owner)));
        }
        assert!(derived.patch().ops().iter().any(|op|matches!(op,WarpOp::UpsertNode {node,..} if node.warp_id==target.warp_id && node.local_id==NodeId([17;32]))));
        assert_eq!(
            host.echo_operation_evaluation_basis_v1(head, basis.application_basis())
                .expect("unchanged"),
            basis
        );
    }
}

// Authorizing ordinary node writes must not implicitly authorize portal traversal.
#[test]
fn descendant_mutation_refuses_unpermitted_portal_owners_and_beta_portals() {
    use warp_core::edict_effects::EffectDerivationKind as K;
    for beta in [false, true] {
        let (mut host, head, _, target) = portal_host(1, beta);
        let mut body = input("create-node");
        set(&mut body, "warpId", V::Bytes(target.warp_id.0.to_vec()));
        let (_, admitted, completed) = evaluate(&mut host, head, &publication(), &encoded(&body));
        let error = host
            .derive_echo_operation_effects(&admitted, completed)
            .expect_err("portal not authorized");
        assert_eq!(error.kind(), K::FootprintLawViolation);
        assert_eq!(error.consumed().write_bytes(), 67);
        assert_eq!(error.consumed().read_bytes(), if beta { 0 } else { 64 });
    }
}

// A reachable-state root does not bind detached records at an uncommitted parent.
// Removing the completed-origin owner check would import a different store's reads.
#[test]
fn completed_origin_from_another_owner_refuses_matching_parent_coordinates() {
    let (mut host, head, _) = make_host(false, false, false);
    let input = encoded(&input("create-node"));
    let (_, admitted, _) = evaluate(&mut host, head, &publication(), &input);
    let (mut other, other_head, _) = make_host(false, true, false);
    let (_, _, completed) = evaluate(&mut other, other_head, &publication(), &input);
    assert_eq!(
        completed.evaluation_basis(),
        admitted.invocation().evaluation_basis()
    );
    let result = host.derive_echo_operation_effects(&admitted, completed);
    assert!(
        matches!(result,Err(ref error) if error.kind()==warp_core::edict_effects::EffectDerivationKind::AuthorityMismatch),
        "foreign completed work crossed the runtime owner"
    );
}

// Program identity is independent of identical inputs and identical runtime parents.
#[test]
fn completed_work_cannot_be_substituted_from_another_program() {
    let (mut host, head, _) = make_host(false, false, false);
    let input = encoded(&input("create-node"));
    let (installed, admitted, _) = evaluate(&mut host, head, &publication(), &input);
    let alternate = prepare(&publication_for("ensure-node")).expect("alternate program");
    let program = alternate.program().as_effect_program().expect("effect");
    let basis = admitted.invocation().evaluation_basis();
    let mut frame = host
        .effect_evaluation_frame_v1(basis, &[], 0, budget(), limits())
        .expect("frame")
        .with_footprint_law(program.footprint_law().clone())
        .expect("law");
    frame
        .evaluate_effect(program, &input, &[])
        .expect("alternate private execution");
    let completed = frame.into_effect_evaluation(4096).expect("completion");
    assert_ne!(completed.program_identity(), installed.program_identity());
    let error = host
        .derive_echo_operation_effects(&admitted, completed)
        .expect_err("wrong program");
    assert_eq!(
        error.kind(),
        warp_core::edict_effects::EffectDerivationKind::OriginMismatch
    );
}

// Derivation must preserve the actual read footprint even without mutations.
#[test]
fn read_only_completion_derives_empty_patch_without_recharging_witnessed_reads() {
    let (mut host, head, node) = host();
    let mut publication = super::execution::publication();
    set(
        &mut publication,
        "budget_ceiling",
        get(&self::publication(), "budget_ceiling").clone(),
    );
    let input = super::execution::input(node, b"before");
    let (_, admitted, completed) = evaluate_with_aperture(
        &mut host,
        head,
        &publication,
        &input,
        &[warp_core::edict_effects::TypedNodeRead {
            node,
            absent_node_type: TypeId([1; 32]),
        }],
    );
    let consumed = completed.read_set_image().consumed();
    assert_eq!(consumed.read_bytes(), 134);
    let derived = host
        .derive_echo_operation_effects(&admitted, completed)
        .expect("read-only patch");
    assert_eq!(derived.consumed(), consumed);
    assert_eq!(
        derived.completed().output(),
        encoded(&V::Bytes(b"before".to_vec()))
    );
    assert!(derived.patch().ops().is_empty());
    assert!(derived.patch().out_slots().is_empty());
    assert_eq!(derived.patch().in_slots().len(), 2);
    assert!(derived.patch().in_slots().contains(&SlotId::Node(node)));
    assert!(derived
        .patch()
        .in_slots()
        .contains(&SlotId::Attachment(AttachmentKey::node_alpha(node))));
}

// Matching program identity alone cannot authorize another input's bound parent.
#[test]
fn completed_work_cannot_be_substituted_from_another_application_basis() {
    let (mut host, head, _) = host();
    let input = encoded(&input("create-node"));
    let (installed, admitted, _) = evaluate(&mut host, head, &publication(), &input);
    let other = encoded(&input_for_other_basis());
    let basis = host
        .echo_operation_evaluation_basis_v1(
            head,
            EchoOperationApplicationBasisV1::new(
                hash(
                    b"echo:operation-profile:v1\0",
                    b"echo.operation.basis.edict-effect/v1",
                ),
                hash(b"echo:operation-edict-effect-basis:v1\0", &other),
            ),
        )
        .expect("other basis");
    let program = installed.program().as_effect_program().expect("effect");
    let mut frame = host
        .effect_evaluation_frame_v1(basis, &[], 0, budget(), limits())
        .expect("frame")
        .with_footprint_law(program.footprint_law().clone())
        .expect("law");
    frame
        .evaluate_effect(program, &other, &[])
        .expect("other input");
    let completed = frame.into_effect_evaluation(4096).expect("completion");
    assert_eq!(completed.program_identity(), installed.program_identity());
    assert_ne!(
        completed.evaluation_basis(),
        admitted.invocation().evaluation_basis()
    );
    let error = host
        .derive_echo_operation_effects(&admitted, completed)
        .expect_err("wrong application basis");
    assert_eq!(
        error.kind(),
        warp_core::edict_effects::EffectDerivationKind::OriginMismatch
    );
}
fn input_for_other_basis() -> V {
    let mut body = input("create-node");
    set(&mut body, "nodeId", id(18));
    body
}

// Reconstruction through owned parts must retain both admission and origin owners.
#[test]
fn host_reconfiguration_preserves_its_completed_evaluation_authority() {
    let (mut host, head, _) = host();
    let (_, admitted, completed) = evaluate(
        &mut host,
        head,
        &publication(),
        &encoded(&input("create-node")),
    );
    let host = warp_core::TrustedRuntimeHost::from_parts(host.into_parts());
    let derived = host
        .derive_echo_operation_effects(&admitted, completed)
        .expect("same runtime owner after parts round trip");
    assert_eq!(derived.patch().ops().len(), 2);
}

// Each case violates two phases; returning whichever check happens to run first
// would make the public refusal taxonomy unstable after refactoring.
#[test]
fn competing_mutation_failures_follow_the_validation_phase_order() {
    use warp_core::edict_effects::EffectDerivationKind as K;
    for (name, input_field, input_value, law_field, want, edge) in [
        (
            "ensure-node",
            "nodeType",
            id(99),
            None,
            K::SchemaViolation,
            false,
        ),
        (
            "ensure-node",
            "nodeId",
            id(17),
            Some("write_node_types"),
            K::IdentityLawViolation,
            false,
        ),
        (
            "replace-atom",
            "nodeId",
            id(99),
            None,
            K::NodeMissing,
            false,
        ),
        (
            "create-node",
            "nodeId",
            id(4),
            Some("write_node_types"),
            K::FootprintLawViolation,
            false,
        ),
        (
            "ensure-edge",
            "edgeId",
            id(88),
            None,
            K::IdentityLawViolation,
            false,
        ),
        (
            "replace-edge",
            "expectedToId",
            id(99),
            Some("write_edge_types"),
            K::FootprintLawViolation,
            true,
        ),
    ] {
        let mut body = input(name);
        if name == "ensure-node" {
            set(&mut body, "nodeId", id(17));
        }
        if name == "replace-atom" {
            set(&mut body, "atomType", id(99));
        }
        if name == "ensure-edge" {
            set(&mut body, "toId", id(99));
        }
        set(&mut body, input_field, input_value);
        let mut publication = publication_for(name);
        if let Some(field) = law_field {
            publication = with_law_field(publication, field, V::Array(vec![]));
        }
        let error = derive_case(name, body, publication, edge, false, false)
            .expect_err("multiple invalid claims");
        assert_eq!(error.kind(), want, "{name}: {input_field}");
    }
}
// Identities admission accepted before ContentAddressV1 moved into its own
// module, captured from the pre-extraction runtime. Moving the law must not
// change an identity Echo has already admitted.
const ADMITTED_LEAF_ID: &str = "127c0d47351ee88620bec24603a1b8949610d792f06f913227308ee5c7e47f90";
const ADMITTED_SLOT_EDGE_ID: &str =
    "bd913d145bb0da61dc572cf56a5a4abd442bf295b7020c32aa1de85a3dd70aa2";
fn hex_id(hex: &str) -> V {
    V::Bytes(
        (0..32)
            .map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).expect("hex"))
            .collect(),
    )
}
#[test]
fn identities_admitted_before_the_law_moved_are_still_admitted() {
    let mut body = input("ensure-node");
    set(&mut body, "nodeId", hex_id(ADMITTED_LEAF_ID));
    derive_case(
        "ensure-node",
        body,
        publication_for("ensure-node"),
        false,
        false,
        false,
    )
    .expect("pre-extraction content node id");
    let mut body = input("ensure-edge");
    set(&mut body, "edgeId", hex_id(ADMITTED_SLOT_EDGE_ID));
    derive_case(
        "ensure-edge",
        body,
        publication_for("ensure-edge"),
        false,
        false,
        false,
    )
    .expect("pre-extraction slot edge id");
}
// An id is bound to the exact atom and edge set it was computed from.
// The surrounding package stays valid so refusal comes from the identity law.
#[test]
fn a_node_id_is_refused_when_the_submitted_atom_or_edge_set_differs() {
    use warp_core::edict_effects::EffectDerivationKind as K;
    for (field, value) in [
        ("atomBytes", V::Bytes(b"newer".to_vec())),
        ("atomBytes", V::Bytes(b"ne".to_vec())),
        ("atomBytes", V::Bytes(b"nex".to_vec())),
    ] {
        let mut body = input("ensure-node");
        set(&mut body, field, value);
        let error = derive_case(
            "ensure-node",
            body,
            publication_for("ensure-node"),
            false,
            false,
            false,
        )
        .expect_err("different atom");
        assert_eq!(error.kind(), K::IdentityLawViolation);
    }
    // Computed with one outgoing edge, submitted with none.
    let with_edge = warp_core::content_address_v1::content_node_id(
        warp_core::WarpId([3; 32]),
        TypeId([7; 32]),
        TypeId([2; 32]),
        b"new",
        &[warp_core::content_address_v1::OutgoingEdge {
            edge_type: TypeId([6; 32]),
            to: NodeId([5; 32]),
        }],
    )
    .expect("one edge");
    let mut body = input("ensure-node");
    set(&mut body, "nodeId", V::Bytes(with_edge.0.to_vec()));
    let error = derive_case(
        "ensure-node",
        body,
        publication_for("ensure-node"),
        false,
        false,
        false,
    )
    .expect_err("different edge set");
    assert_eq!(error.kind(), K::IdentityLawViolation);
}
// A content node together with an outgoing edge in one proposal. Every id
// below is assembled from its preimage here, never through the production law.
fn content_leaf_with_edge(edge_type: [u8; 32], to: [u8; 32]) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(b"echo.bridge.node-id.v1\0");
    h.update(&[3; 32]);
    h.update(&[7; 32]);
    h.update(&atom_hash(2, b"new"));
    h.update(&1u64.to_le_bytes());
    h.update(&edge_type);
    h.update(&to);
    h.finalize().into()
}
fn content_edge(from: [u8; 32], to: [u8; 32]) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(b"echo.bridge.edge-id.v1\0");
    h.update(&[3; 32]);
    h.update(&from);
    h.update(&[6; 32]);
    h.update(&to);
    h.finalize().into()
}
/// The admitted ids of the leaf with one edge to node 5, and of that edge.
const ADMITTED_LEAF_WITH_EDGE_ID: &str =
    "322afec5e32a48d3ab373c0b7efbf614bda7cf391d01b726a72c64c753fc5e10";
const ADMITTED_CONTENT_EDGE_ID: &str =
    "3ff5f345ec93b3eeca43553ab0f02c33c68926cf88e692ef53fa9973ddb04147";
fn node_and_edge(node_id: [u8; 32], edge_id: [u8; 32], to: [u8; 32]) -> V {
    let mut node = input("ensure-node");
    set(&mut node, "nodeId", V::Bytes(node_id.to_vec()));
    let edge = map([
        ("warpId", id(3)),
        ("edgeId", V::Bytes(edge_id.to_vec())),
        ("fromId", V::Bytes(node_id.to_vec())),
        ("toId", V::Bytes(to.to_vec())),
        ("edgeType", id(6)),
    ]);
    map([("first", node), ("second", edge)])
}
fn derive_pair(
    body: V,
) -> Result<
    warp_core::edict_effects::DerivedEffectPatch,
    warp_core::edict_effects::EffectDerivationError,
> {
    derive_case(
        "ensure-node+ensure-edge",
        body,
        with_graph_law(super::mutations::fixture_pair(
            "ensure-node",
            "ensure-edge",
            "continuum.profile.graph-effects/v1",
        )),
        false,
        false,
        false,
    )
}
#[test]
fn a_content_node_and_its_content_edge_are_admitted_together() {
    let leaf = content_leaf_with_edge([6; 32], [5; 32]);
    let edge = content_edge(leaf, [5; 32]);
    assert_eq!(hex_id(ADMITTED_LEAF_WITH_EDGE_ID), V::Bytes(leaf.to_vec()));
    assert_eq!(hex_id(ADMITTED_CONTENT_EDGE_ID), V::Bytes(edge.to_vec()));
    let derived = derive_pair(node_and_edge(leaf, edge, [5; 32])).expect("content node and edge");
    let ops = derived.patch().ops();
    assert!(ops
        .iter()
        .any(|op| matches!(op, WarpOp::UpsertNode { node, record }
        if node.local_id.0 == leaf && record.ty == TypeId([7; 32]))));
    assert!(ops
        .iter()
        .any(|op| matches!(op, WarpOp::UpsertEdge { warp_id, record }
        if warp_id.0 == [3; 32] && record.id.0 == edge && record.from.0 == leaf
            && record.to.0 == [5; 32] && record.ty.0 == [6; 32])));
}
#[test]
fn a_content_edge_id_computed_for_another_target_is_refused() {
    use warp_core::edict_effects::EffectDerivationKind as K;
    let leaf = content_leaf_with_edge([6; 32], [5; 32]);
    let error = derive_pair(node_and_edge(leaf, content_edge(leaf, [4; 32]), [5; 32]))
        .expect_err("edge id for node 4, submitted toward node 5");
    assert_eq!(error.kind(), K::IdentityLawViolation);
    // The slot form of the same edge is wrong for a content source too.
    let mut h = blake3::Hasher::new();
    h.update(b"echo.bridge.slot-edge-id.v1\0");
    h.update(&[3; 32]);
    h.update(&leaf);
    h.update(&[6; 32]);
    let slot: [u8; 32] = h.finalize().into();
    let error = derive_pair(node_and_edge(leaf, slot, [5; 32])).expect_err("slot id from content");
    assert_eq!(error.kind(), K::IdentityLawViolation);
}
#[test]
fn a_node_id_that_omits_an_edge_the_proposal_adds_is_refused() {
    use warp_core::edict_effects::EffectDerivationKind as K;
    let without_edge = leaf_id();
    let error = derive_pair(node_and_edge(
        without_edge,
        content_edge(without_edge, [5; 32]),
        [5; 32],
    ))
    .expect_err("node id computed without the proposal's edge");
    assert_eq!(error.kind(), K::IdentityLawViolation);
}
