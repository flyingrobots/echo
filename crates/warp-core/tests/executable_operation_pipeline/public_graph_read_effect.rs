// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! An authored effect program that reads a node's outgoing edges through
//! `echo.dpo@2.outgoing-edge-read` and writes an edge to what it read
//! (echo#982), published by the public compiler, admitted, installed, run
//! and derived by the generic runtime.
//!
//! The package is unchanged public compiler output retained under
//! `fixtures/edict-effect-graph-read/`; its identity is recorded here
//! independently of the bytes. The program reads the `E` edges of the
//! requested source, links `fromId` to the target of the first one it read
//! (or to the source itself when it read none) with `ensure-edge`, and
//! returns what it read. Oracles: the authored program's own law, written by
//! hand, over graphs built here; the bounded-read route's public edge package
//! over a plain view of the same state; and the runtime owner's exact
//! admission policy.
use super::current_effect_publication_fixture as publication;
use echo_edict_canonical::CanonicalValueV1 as Value;
use warp_core::edict_effects::{
    EffectFrameError, EffectMutation, EffectReadImageError, EffectReadSetView, TypedNodeRead,
};
use warp_core::edict_pure::EvaluationLimits;
use warp_core::edict_read::{GraphReadAperture, ReadLimits, ReadView};
use warp_core::{
    make_head_id, AttachmentKey, EchoOperationAdmissionErrorKind, EchoOperationAdmissionPolicy,
    EchoOperationApplicationBasisV1, EchoOperationBudgetV1, EchoOperationInvocationAdmissionPolicy,
    EdgeId, EdgeKey, EdgeRecord, EngineBuilder, GraphStore, InboxPolicy, NodeId, NodeKey,
    NodeRecord, PlaybackMode, PortalInit, TickCommitStatus, TrustedRuntimeHost, TypeId, WarpId,
    WarpOp, WarpTickPatchV1, WorldlineId, WorldlineRuntime, WorldlineState, WriterHead,
    WriterHeadKey,
};

const PACKAGE: &str =
    include_str!("../fixtures/edict-effect-graph-read/link-executable-operation-package.hex");
/// `echo:operation-package:v1` identity of the retained package bytes.
const PACKAGE_PIN: &str = "886fe5b50dacc0091068ba3a8fc1bb73a001afac1bce97463a4d6c298b15cf8b";
const OPERATION: &str = "consumer.links.program@1.link";
const AUTHORITY_LABEL: &[u8] = b"consumer.links.authority/v1";
/// The node, portal-owner and edge types the program's schema and law name.
const NODE_TYPE: TypeId = TypeId([0x6b; 32]);
const PORTAL_TYPE: TypeId = TypeId([0x6c; 32]);
const EDGE_TYPE: TypeId = TypeId([0x6d; 32]);
/// The bounded-read route's public edge package (echo#926) and its pin.
const PLAIN_EDGE_PACKAGE: &str =
    include_str!("../fixtures/edict-graph-read/edge-executable-operation-package.hex");
const PLAIN_EDGE_PIN: &str = "1f2ef3ce45822995465e9250f8a98083e23dbf2c1cd682f5372826099580b926";

const ROOT_WARP: WarpId = WarpId([0x61; 32]);
const CHILD_WARP: WarpId = WarpId([0x67; 32]);
const ROOT: NodeId = NodeId([0x62; 32]);
const SOURCE: NodeId = NodeId([0x63; 32]);
const TARGET: NodeId = NodeId([0x64; 32]);
const FROM: NodeId = NodeId([0x65; 32]);
const OTHER_TARGET: NodeId = NodeId([0x68; 32]);
const READ_EDGE: EdgeId = EdgeId([0x66; 32]);
/// The link's identity: `FROM` is a mutable node, so the identity law names
/// its `E` slot in `warp`, whatever target the program reads
/// (`content_address_v1::slot_edge_id`).
fn new_edge(warp: WarpId) -> EdgeId {
    warp_core::content_address_v1::slot_edge_id(warp, FROM, EDGE_TYPE)
}

fn retained() -> (Vec<u8>, [u8; 32]) {
    let bytes = hex::decode(PACKAGE.trim()).expect("retained hex");
    let identity = publication::hash(b"echo:operation-package:v1\0", &bytes);
    assert_eq!(
        hex::encode(identity),
        PACKAGE_PIN,
        "retained package identity"
    );
    (bytes, identity)
}

fn authority() -> [u8; 32] {
    publication::hash(b"echo:operation-profile:v1\0", AUTHORITY_LABEL)
}

fn budget() -> EchoOperationBudgetV1 {
    EchoOperationBudgetV1::new(16_384, 4096, 4096)
}

fn limits() -> EvaluationLimits {
    EvaluationLimits {
        max_package_bytes: 1_048_576,
        max_input_bytes: 65_536,
        max_steps: 16_384,
        max_allocated_bytes: 1_048_576,
        max_output_bytes: 65_536,
    }
}

/// Where the read happens: the warp holding the source, the bucket it scans
/// (each edge from the source to `targets`, in order), and the other nodes.
struct Shape {
    /// Hold the graph in a child warp reached through the root's alpha portal.
    child_warp: bool,
    targets: Vec<NodeId>,
}

impl Shape {
    fn root(targets: Vec<NodeId>) -> Self {
        Self {
            child_warp: false,
            targets,
        }
    }
    fn warp(&self) -> WarpId {
        if self.child_warp {
            CHILD_WARP
        } else {
            ROOT_WARP
        }
    }
    fn key(&self, node: NodeId) -> NodeKey {
        NodeKey {
            warp_id: self.warp(),
            local_id: node,
        }
    }
}

fn node(store: &mut GraphStore, id: NodeId) {
    store.insert_node(id, NodeRecord { ty: NODE_TYPE });
}

/// The `index`th edge of the source's bucket; the first is `READ_EDGE`, and
/// identifiers ascend in bucket order.
fn bucket_edge(index: usize) -> EdgeId {
    let mut id = READ_EDGE.0;
    id[0] = id[0]
        .checked_add(u8::try_from(index).expect("small bucket"))
        .expect("small bucket");
    EdgeId(id)
}

/// The source's bucket: one `E` edge to each target, `READ_EDGE` first.
fn bucket(store: &mut GraphStore, targets: &[NodeId]) {
    for (index, to) in targets.iter().enumerate() {
        store.insert_edge(
            SOURCE,
            EdgeRecord {
                id: bucket_edge(index),
                from: SOURCE,
                to: *to,
                ty: EDGE_TYPE,
            },
        );
    }
}

/// The source, both possible targets and the node to link from, with the
/// source's bucket, in the root warp or in a child warp the root's alpha
/// portal opens.
fn state(shape: &Shape) -> WorldlineState {
    let root = NodeKey {
        warp_id: ROOT_WARP,
        local_id: ROOT,
    };
    let mut store = GraphStore::new(ROOT_WARP);
    store.insert_node(ROOT, NodeRecord { ty: PORTAL_TYPE });
    if !shape.child_warp {
        for id in [SOURCE, TARGET, OTHER_TARGET, FROM] {
            node(&mut store, id);
        }
        bucket(&mut store, &shape.targets);
        return WorldlineState::from_root_store(store, ROOT).expect("root state");
    }
    let mut warp = WorldlineState::from_root_store(store, ROOT)
        .expect("root state")
        .warp_state()
        .clone();
    WarpTickPatchV1::new(
        0,
        [0; 32],
        TickCommitStatus::Committed,
        vec![],
        vec![],
        vec![WarpOp::OpenPortal {
            key: AttachmentKey::node_alpha(root),
            child_warp: CHILD_WARP,
            child_root: SOURCE,
            init: PortalInit::Empty {
                root_record: NodeRecord { ty: NODE_TYPE },
            },
        }],
    )
    .apply_to_state(&mut warp)
    .expect("portal");
    let child = warp.store_mut(&CHILD_WARP).expect("child store");
    for id in [TARGET, OTHER_TARGET, FROM] {
        node(child, id);
    }
    bucket(child, &shape.targets);
    WorldlineState::new(warp, root).expect("portal state")
}

fn host(state: WorldlineState) -> (TrustedRuntimeHost, WriterHeadKey) {
    let store = state.store(&ROOT_WARP).expect("root store").clone();
    let head = WriterHeadKey {
        worldline_id: WorldlineId::from_bytes([0x6a; 32]),
        head_id: make_head_id("public-graph-read-effect"),
    };
    let mut runtime = WorldlineRuntime::new();
    runtime
        .register_worldline(head.worldline_id, state)
        .expect("worldline");
    runtime
        .register_writer_head(WriterHead::with_routing(
            head,
            PlaybackMode::Play,
            InboxPolicy::AcceptAll,
            None,
            true,
        ))
        .expect("head");
    let engine = EngineBuilder::new(store, ROOT).build();
    (
        TrustedRuntimeHost::new(runtime, engine).expect("host"),
        head,
    )
}

fn id(bytes: impl AsRef<[u8]>) -> Value {
    Value::Bytes(bytes.as_ref().to_vec())
}

/// The program's input: read `source`'s `E` edges and link `FROM` with
/// its `E` slot to whatever it reads first.
fn input(shape: &Shape, source: NodeId) -> Vec<u8> {
    publication::encoded(&publication::map([
        ("anchor", Value::Bytes(vec![0x5a; 32])),
        (
            "read",
            publication::map([
                ("edgeType", id(EDGE_TYPE.0)),
                ("nodeId", id(source.0)),
                ("warpId", id(shape.warp().0)),
            ]),
        ),
        ("edgeId", id(new_edge(shape.warp()).0)),
        ("fromId", id(FROM.0)),
    ]))
}

fn application_basis(input: &[u8]) -> EchoOperationApplicationBasisV1 {
    EchoOperationApplicationBasisV1::new(
        publication::hash(
            b"echo:operation-profile:v1\0",
            b"echo.operation.basis.edict-effect/v1",
        ),
        publication::hash(b"echo:operation-edict-effect-basis:v1\0", input),
    )
}

/// One evaluation of the installed public package: its result, and on
/// success the completed origin evidence and the admitted invocation.
struct Run {
    host: TrustedRuntimeHost,
    output: Result<Vec<u8>, EffectFrameError>,
    completed: Option<(
        warp_core::AdmittedEchoOperationInvocation,
        warp_core::edict_effects::CompletedEffectEvaluation,
    )>,
}

fn invocation(
    installed: &warp_core::InstalledEchoOperation,
    head: WriterHeadKey,
    basis: warp_core::EchoOperationEvaluationBasisV1,
    input: &[u8],
) -> Vec<u8> {
    let app = basis.application_basis();
    publication::encoded(&publication::map([
        ("schema", publication::text("echo.operation-invocation/v1")),
        ("kind", publication::text("edict-effect-program/v1")),
        ("package_id", id(installed.package_identity())),
        (
            "operation_coordinate",
            publication::text(installed.operation_coordinate()),
        ),
        ("authority_grant_identity", id([9; 32])),
        (
            "delegated_budget",
            publication::map([
                ("steps", Value::Integer(16_384)),
                ("read_bytes", Value::Integer(4096)),
                ("write_bytes", Value::Integer(4096)),
            ]),
        ),
        ("application_input_bytes", Value::Bytes(input.to_vec())),
        (
            "evaluation_basis",
            publication::map([
                ("worldline_id", id(head.worldline_id.as_bytes())),
                ("head_id", id(head.head_id.as_bytes())),
                (
                    "worldline_tick",
                    Value::Integer(i128::from(basis.worldline_tick().as_u64())),
                ),
                ("commit_global_tick", Value::Null),
                ("state_root", id(basis.state_root())),
                ("commit_id", id(basis.commit_id())),
                (
                    "application_basis_schema_identity",
                    id(app.schema_identity()),
                ),
                ("application_basis_value_identity", id(app.value_identity())),
            ]),
        ),
    ]))
}

/// Admits and installs the public package under the owner's exact policy,
/// then evaluates it for `source` over `shape`'s graph.
fn run(shape: &Shape, source: NodeId) -> Run {
    let (bytes, identity) = retained();
    let (mut host, head) = host(state(shape));
    let policy = EchoOperationAdmissionPolicy::exact(identity, OPERATION, authority(), budget());
    let admitted = host
        .admit_echo_operation_package(&policy, bytes)
        .expect("the exact policy admits the public package");
    let installed = host
        .install_admitted_echo_operation_package(admitted)
        .expect("installation");
    let program = installed
        .program()
        .as_effect_program()
        .expect("an effect program")
        .clone();
    let input = input(shape, source);
    let parent = host
        .echo_operation_evaluation_basis_v1(head, application_basis(&input))
        .expect("basis");
    let invocation = host
        .admit_echo_operation_invocation(
            &EchoOperationInvocationAdmissionPolicy::new(authority(), [9; 32], budget()),
            &invocation(&installed, head, parent, &input),
        )
        .expect("invocation");
    // The frame borrows the host, so it ends inside this block.
    let (output, completed) = {
        let mut frame = host
            .effect_evaluation_frame_v1(parent, &[EDGE_TYPE], 8, budget(), limits())
            .expect("frame")
            .with_footprint_law(program.footprint_law().clone())
            .expect("law");
        let output = frame
            .evaluate_effect(
                &program,
                &input,
                &[TypedNodeRead {
                    node: shape.key(source),
                    absent_node_type: NODE_TYPE,
                }],
            )
            .map(<[u8]>::to_vec);
        let completed = if output.is_ok() {
            Some(
                frame
                    .into_effect_evaluation(1 << 20)
                    .expect("completed evidence"),
            )
        } else {
            None
        };
        (output, completed)
    };
    Run {
        host,
        output,
        completed: completed.map(|completed| (invocation, completed)),
    }
}

/// What the program returns for the edges it read, written by hand.
fn edges(rows: &[(EdgeId, NodeId)]) -> Vec<u8> {
    publication::encoded(&Value::Array(
        rows.iter()
            .map(|(edge, to)| publication::map([("edgeId", id(edge.0)), ("toId", id(to.0))]))
            .collect(),
    ))
}

#[test]
fn the_public_graph_read_effect_links_to_the_target_of_the_edge_it_reads() {
    for to in [TARGET, OTHER_TARGET] {
        let shape = Shape::root(vec![to]);
        let run = run(&shape, SOURCE);
        assert_eq!(
            run.output.as_deref(),
            Ok(edges(&[(READ_EDGE, to)]).as_slice())
        );
        let (admitted, completed) = run.completed.expect("completed");
        assert_eq!(
            completed.mutations(),
            [EffectMutation::EnsureEdge {
                edge: EdgeKey {
                    warp_id: ROOT_WARP,
                    local_id: new_edge(ROOT_WARP),
                },
                from: FROM,
                to,
                ty: EDGE_TYPE,
            }]
        );
        let derived = run
            .host
            .derive_echo_operation_effects(&admitted, completed)
            .expect("the runtime owner derives the link");
        assert!(
            derived.patch().ops().iter().any(|op| matches!(
                op,
                WarpOp::UpsertEdge { warp_id, record }
                    if *warp_id == ROOT_WARP
                        && record.id == new_edge(ROOT_WARP)
                        && record.from == FROM
                        && record.to == to
                        && record.ty == EDGE_TYPE
            )),
            "the derived patch writes the link to {to:?}"
        );
    }
}

#[test]
fn an_empty_read_links_back_to_the_source() {
    let shape = Shape::root(vec![]);
    let run = run(&shape, SOURCE);
    assert_eq!(run.output.as_deref(), Ok(edges(&[]).as_slice()));
    let (_, completed) = run.completed.expect("completed");
    assert!(matches!(
        completed.mutations(),
        [EffectMutation::EnsureEdge { to, .. }] if *to == SOURCE
    ));
}

#[test]
fn the_read_image_witnesses_a_scanned_bucket_in_a_second_warp() {
    let shape = Shape {
        child_warp: true,
        targets: vec![TARGET, OTHER_TARGET],
    };
    let run = run(&shape, SOURCE);
    let (admitted, completed) = run.completed.expect("completed");
    let image = completed.read_set_image();
    let view = EffectReadSetView::parse(image.as_bytes()).expect("canonical image");
    let source = view
        .rows()
        .iter()
        .find(|row| {
            row.slot.class == 0 && row.slot.warp_id == CHILD_WARP.0 && row.slot.local_id == SOURCE.0
        })
        .expect("the source's node row");
    assert_eq!((source.bucket, source.bucket_len()), (1, 2));
    let state = run
        .host
        .runtime()
        .worldlines()
        .get(
            &admitted
                .invocation()
                .evaluation_basis()
                .writer_head()
                .worldline_id,
        )
        .expect("worldline")
        .state()
        .clone();
    assert_eq!(view.confirm_bucket_sizes(&state), Ok(()));
    // The same image against a state whose child-warp bucket holds one more
    // edge: the recorded size no longer describes it.
    let grown = self::state(&Shape {
        child_warp: true,
        targets: vec![TARGET, OTHER_TARGET, FROM],
    });
    assert_eq!(
        view.confirm_bucket_sizes(&grown),
        Err(EffectReadImageError::BucketSizeMismatch)
    );
    let derived = run
        .host
        .derive_echo_operation_effects(&admitted, completed)
        .expect("derivation confirms the bucket at the image's basis");
    assert!(derived.patch().ops().iter().any(
        |op| matches!(op, WarpOp::UpsertEdge { warp_id, record } if *warp_id == CHILD_WARP && record.to == TARGET)
    ));
}

#[test]
fn the_recorded_read_agrees_with_the_plain_bounded_read_of_the_same_state() {
    let shape = Shape::root(vec![TARGET, OTHER_TARGET]);
    let recorded = run(&shape, SOURCE).output.expect("recorded read");
    let package = hex::decode(PLAIN_EDGE_PACKAGE.trim()).expect("plain package hex");
    let pin = echo_edict_canonical::digest_canonical_value_bytes_v1(
        "echo.operation-package/v1",
        &echo_edict_canonical::decode_canonical_cbor_v1(&package).expect("canonical"),
    )
    .expect("pin");
    assert_eq!(hex::encode(pin), PLAIN_EDGE_PIN, "retained plain package");
    let frontier =
        warp_core::WorldlineFrontier::new(WorldlineId::from_bytes([0x6a; 32]), state(&shape));
    let aperture = [shape.key(SOURCE)];
    let view = ReadView::at(&frontier, &aperture)
        .expect("view")
        .with_graph_aperture(GraphReadAperture {
            edge_types: &[EDGE_TYPE],
            node_types: &[],
            max_edge_scans: 8,
        })
        .expect("graph aperture");
    let plain_input = publication::encoded(&publication::map([(
        "read",
        publication::map([
            ("edgeType", id(EDGE_TYPE.0)),
            ("nodeId", id(SOURCE.0)),
            ("warpId", id(ROOT_WARP.0)),
        ]),
    )]));
    let plain = warp_core::edict_read::evaluate(
        &package,
        pin,
        &plain_input,
        &view,
        ReadLimits {
            evaluation: limits(),
            max_reads: 16,
            max_read_bytes: 4096,
        },
    )
    .expect("plain read");
    assert_eq!(plain.output, recorded);
    assert_eq!(
        recorded,
        edges(&[(READ_EDGE, TARGET), (bucket_edge(1), OTHER_TARGET)])
    );
}

#[test]
fn each_graph_read_failure_maps_to_its_declared_obstruction() {
    let obstruction = |reason: &str| {
        Err(EffectFrameError::AuthoredObstruction {
            reason_kind: format!("fx.{reason}"),
            payload: publication::encoded(&publication::map([])),
        })
    };
    // A source absent from the state.
    let absent = run(&Shape::root(vec![TARGET]), NodeId([0x70; 32]));
    assert_eq!(absent.output, obstruction("SourceMissing"));
    // Five matching edges for a result list of at most four.
    let five = vec![TARGET; 5];
    assert_eq!(
        run(&Shape::root(five), SOURCE).output,
        obstruction("EdgeBoundExceeded")
    );
    // Nine bucket edges against a scan allowance of eight.
    let nine = vec![TARGET; 9];
    assert_eq!(
        run(&Shape::root(nine), SOURCE).output,
        obstruction("ScanBoundExceeded")
    );
}

#[test]
fn another_package_is_refused_under_the_public_graph_read_effect_policy() {
    let (_, identity) = retained();
    let (host, _) = host(state(&Shape::root(vec![TARGET])));
    let other = hex::decode(
        include_str!("../fixtures/edict-effect-step/read-executable-operation-package.hex").trim(),
    )
    .expect("read package");
    let policy = EchoOperationAdmissionPolicy::exact(identity, OPERATION, authority(), budget());
    assert_eq!(
        host.admit_echo_operation_package(&policy, other)
            .expect_err("refused at admission")
            .kind(),
        EchoOperationAdmissionErrorKind::PackageIdentityMismatch
    );
}
