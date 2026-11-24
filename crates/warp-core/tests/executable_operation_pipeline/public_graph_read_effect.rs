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
    EdgeId, EdgeKey, EdgeRecord, EffectPackageHost, EngineBuilder, GraphStore, InboxPolicy, NodeId,
    NodeKey, NodeRecord, PlaybackMode, PortalInit, TickCommitStatus, TypeId, WarpId, WarpOp,
    WarpTickPatchV1, WorldlineId, WorldlineRuntime, WorldlineState, WriterHead, WriterHeadKey,
};

const PACKAGE: &str =
    include_str!("../fixtures/edict-effect-graph-read/link-executable-operation-package.hex");
/// `echo:operation-package:v1` identity of the retained package bytes.
const PACKAGE_PIN: &str = "31c092bec1c0b3a061232e01ab0fa2943469f22324f5e240f2d30d6fb1dbe4c1";
/// The same source compiled by Amblicit 65dc5362, retained for exact
/// admission (echo#1016), and the shared-read minimum, (steps, bytes), that
/// Amblicit 80b52f95 reports for it with Echo's `/v3` templates and that
/// `effect_programs_are_admitted_at_exactly_the_compiler_minimum` in
/// `echo-edict-provider-lowerer` pins.
const V2_ADMISSION_PACKAGE: &str =
    include_str!("../fixtures/edict-effect-v2-admission/link-executable-operation-package.hex");
const V2_ADMISSION_MINIMUM: (u64, u64) = (244, 8_240);
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

fn host(state: WorldlineState) -> (EffectPackageHost, WriterHeadKey) {
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
    (EffectPackageHost::new(runtime, engine).expect("host"), head)
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
    host: EffectPackageHost,
    output: Result<Vec<u8>, EffectFrameError>,
    /// The steps and bytes the frame charged.
    charged: (u64, u64),
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
    run_package(bytes, identity, shape, source)
}

/// Admits, installs and evaluates `bytes` under the exact policy for
/// `identity`, as `run` does for the retained package.
fn run_package(bytes: Vec<u8>, identity: [u8; 32], shape: &Shape, source: NodeId) -> Run {
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
    let (output, charged, completed) = {
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
        let charged = (frame.consumed().steps(), frame.allocated_bytes());
        let completed = if output.is_ok() {
            Some(
                frame
                    .into_effect_evaluation(1 << 20)
                    .expect("completed evidence"),
            )
        } else {
            None
        };
        (output, charged, completed)
    };
    Run {
        host,
        output,
        charged,
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

fn decoded(value: &Value) -> Value {
    let Value::Bytes(bytes) = value else {
        panic!("embedded canonical bytes")
    };
    echo_edict_canonical::decode_canonical_cbor_v1(bytes).expect("canonical")
}

fn entry_mut<'v>(value: &'v mut Value, key: &str) -> &'v mut Value {
    let Value::Map(fields) = value else {
        panic!("map around {key}")
    };
    &mut fields
        .iter_mut()
        .find(|(name, _)| *name == publication::text(key))
        .unwrap_or_else(|| panic!("missing {key}"))
        .1
}

fn identity_of(domain: &str, value: &Value) -> Value {
    Value::Bytes(
        echo_edict_canonical::digest_canonical_value_bytes_v1(domain, value)
            .expect("digest")
            .to_vec(),
    )
}

/// The retained package with its Core requiring the shared-read schedule
/// (echo#1016), every pin of the Core and Target IR recomputed, and its own
/// new package identity. SYNTHETIC: a repinned copy of the public output.
fn shared_schedule_package() -> (Vec<u8>, [u8; 32]) {
    shared_schedule_package_of(&retained().0, None)
}

/// `bytes` with its Core requiring the shared-read schedule and, when given,
/// a Core budget of (steps, bytes) in Core and its lowered intent alike,
/// repinned as `shared_schedule_package` describes. SYNTHETIC.
fn shared_schedule_package_of(bytes: &[u8], budget: Option<(u64, u64)>) -> (Vec<u8>, [u8; 32]) {
    let mut package = echo_edict_canonical::decode_canonical_cbor_v1(&bytes).expect("package");
    let mut program = decoded(publication::get(&package, "program"));
    let mut bundle = decoded(publication::get(&program, "compiled_program"));
    let mut core = decoded(publication::get(&bundle, "core_artifact"));
    let mut target = decoded(publication::get(&bundle, "target_ir_artifact"));
    let old_core = identity_of("edict.core.module/v1", &core);
    let Value::Array(capabilities) = entry_mut(&mut core, "requiredCoreCapabilities") else {
        panic!("capabilities")
    };
    capabilities.push(publication::text("core.cost.shared-reads/v1"));
    if let Some((steps, bytes_)) = budget {
        for (axis, value) in [("maxSteps", steps), ("maxAllocatedBytes", bytes_)] {
            let value = Value::Integer(i128::from(value));
            for artifact in [&mut core, &mut target] {
                let Value::Map(intents) = entry_mut(artifact, "intents") else {
                    panic!("intents")
                };
                publication::set(
                    entry_mut(&mut intents[0].1, "coreEvaluationBudget"),
                    axis,
                    value.clone(),
                );
            }
        }
    }
    let new_core = identity_of("edict.core.module/v1", &core);
    *entry_mut(
        entry_mut(entry_mut(&mut target, "semanticClosure"), "sourceCore"),
        "digest",
    ) = Value::Array(vec![publication::text("sha256"), new_core.clone()]);
    let new_target = identity_of("edict.target-ir.artifact/v1", &target);
    publication::set(
        &mut bundle,
        "core_artifact",
        Value::Bytes(publication::encoded(&core)),
    );
    publication::set(
        &mut bundle,
        "target_ir_artifact",
        Value::Bytes(publication::encoded(&target)),
    );
    publication::set(
        &mut program,
        "compiled_program",
        Value::Bytes(publication::encoded(&bundle)),
    );
    publication::set(&mut program, "target_ir_identity", new_target.clone());
    publication::set(
        &mut package,
        "program",
        Value::Bytes(publication::encoded(&program)),
    );
    let closure = entry_mut(&mut package, "semantic_closure");
    publication::set(closure, "core_identity", new_core.clone());
    publication::set(closure, "target_ir_identity", new_target);
    if *publication::get(closure, "canonical_meaning_identity") == old_core {
        publication::set(closure, "canonical_meaning_identity", new_core);
    }
    let bytes = publication::encoded(&package);
    let identity = publication::hash(b"echo:operation-package:v1\0", &bytes);
    (bytes, identity)
}

/// Requiring the shared-read schedule changes how the program is metered,
/// never what it reads, writes or returns. Oracle: the retained package run
/// by the copying executor over the same graphs (a differential).
#[test]
fn the_shared_schedule_reads_and_links_exactly_as_the_copying_schedule() {
    let shapes = [
        (Shape::root(vec![TARGET]), SOURCE),
        (Shape::root(vec![]), SOURCE),
        (
            Shape {
                child_warp: true,
                targets: vec![TARGET, OTHER_TARGET],
            },
            SOURCE,
        ),
        (Shape::root(vec![TARGET; 5]), SOURCE),
        (Shape::root(vec![TARGET; 9]), SOURCE),
        (Shape::root(vec![TARGET]), NodeId([0x70; 32])),
    ];
    for (index, (shape, source)) in shapes.iter().enumerate() {
        let copying = run(shape, *source);
        let (bytes, identity) = shared_schedule_package();
        let sharing = run_package(bytes, identity, shape, *source);
        assert_eq!(sharing.output, copying.output, "case {index}: output");
        assert_eq!(
            sharing
                .completed
                .as_ref()
                .map(|(_, completed)| completed.mutations().to_vec()),
            copying
                .completed
                .as_ref()
                .map(|(_, completed)| completed.mutations().to_vec()),
            "case {index}: mutations"
        );
    }
}

/// Given exactly the compiler's shared-read minimum as its Core budget, the
/// exact-admission package reads and links as the copying schedule does on
/// every graph above, and the runtime's charge stays at or below that static
/// bound on both axes (echo#1016). Oracles: the same package under the
/// copying schedule (a differential) and the compiler's minimum.
#[test]
fn the_shared_schedule_runs_within_the_compiler_minimum() {
    let bytes = hex::decode(V2_ADMISSION_PACKAGE.trim()).expect("retained hex");
    let identity = publication::hash(b"echo:operation-package:v1\0", &bytes);
    let (steps, bytes_) = V2_ADMISSION_MINIMUM;
    let shapes = [
        (Shape::root(vec![TARGET]), SOURCE),
        (Shape::root(vec![]), SOURCE),
        (
            Shape {
                child_warp: true,
                targets: vec![TARGET, OTHER_TARGET],
            },
            SOURCE,
        ),
        (Shape::root(vec![TARGET; 5]), SOURCE),
        (Shape::root(vec![TARGET; 9]), SOURCE),
    ];
    for (index, (shape, source)) in shapes.iter().enumerate() {
        let copying = run_package(bytes.clone(), identity, shape, *source);
        // Cases 3 and 4 overflow the read's bound and obstruct on both
        // schedules; the differential still requires the same obstruction.
        assert_eq!(
            copying.output.is_ok(),
            index < 3,
            "case {index}: copying run"
        );
        let (exact, exact_identity) = shared_schedule_package_of(&bytes, Some((steps, bytes_)));
        let sharing = run_package(exact, exact_identity, shape, *source);
        assert_eq!(sharing.output, copying.output, "case {index}: output");
        assert_eq!(
            sharing
                .completed
                .as_ref()
                .map(|(_, completed)| completed.mutations().to_vec()),
            copying
                .completed
                .as_ref()
                .map(|(_, completed)| completed.mutations().to_vec()),
            "case {index}: mutations"
        );
        let (charged_steps, charged_bytes) = sharing.charged;
        assert!(
            charged_steps <= steps && charged_bytes <= bytes_,
            "case {index}: runtime charge ({charged_steps}, {charged_bytes}) exceeds the static bound ({steps}, {bytes_})"
        );
    }
}
