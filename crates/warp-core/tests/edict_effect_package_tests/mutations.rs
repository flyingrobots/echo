// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Synthetic compiled mutation emitters; not public compiler/verifier evidence.
use super::execution::{host, limits};
use super::*;
use warp_core::edict_effects::EffectMutation;
use warp_core::{
    AtomPayload, EchoOperationApplicationBasisV1, EchoOperationBudgetV1, EdgeId, EdgeKey, NodeId,
    NodeKey, WarpId,
};
const NAMES: [&str; 5] = [
    "ensure-node",
    "create-node",
    "replace-atom",
    "ensure-edge",
    "replace-edge",
];
/// The provider effect kind each mutation intrinsic declares.
pub(super) fn effect_kind(name: &str) -> &'static str {
    match name {
        "create-node" => "create",
        "ensure-node" | "ensure-edge" => "ensure",
        "replace-atom" | "replace-edge" => "replace",
        _ => panic!("fixture kind"),
    }
}
fn layout(name: &str) -> Vec<(&'static str, &'static str)> {
    match name {
        "ensure-node" | "create-node" => vec![
            ("warpId", "Bytes<exact=32>"),
            ("nodeId", "Bytes<exact=32>"),
            ("nodeType", "Bytes<exact=32>"),
            ("atomType", "Bytes<exact=32>"),
            ("atomBytes", "Bytes<max=1024>"),
        ],
        "replace-atom" => vec![
            ("warpId", "Bytes<exact=32>"),
            ("nodeId", "Bytes<exact=32>"),
            ("expectedAtomDigest", "Bytes<exact=32>"),
            ("atomType", "Bytes<exact=32>"),
            ("atomBytes", "Bytes<max=1024>"),
        ],
        "ensure-edge" => vec![
            ("warpId", "Bytes<exact=32>"),
            ("edgeId", "Bytes<exact=32>"),
            ("fromId", "Bytes<exact=32>"),
            ("toId", "Bytes<exact=32>"),
            ("edgeType", "Bytes<exact=32>"),
        ],
        "replace-edge" => vec![
            ("warpId", "Bytes<exact=32>"),
            ("edgeId", "Bytes<exact=32>"),
            ("expectedToId", "Bytes<exact=32>"),
            ("toId", "Bytes<exact=32>"),
        ],
        _ => panic!("fixture kind"),
    }
}
/// One application's view of a runtime-ABI call: its own type names, the
/// catalog coordinates the provider pins, and the declared representations.
pub(super) struct Shape {
    pub(super) input_name: &'static str,
    pub(super) output_name: &'static str,
    pub(super) catalog_argument: String,
    pub(super) catalog_return: String,
    pub(super) fields: Vec<(&'static str, &'static str)>,
    pub(super) output: V,
    pub(super) effect_kind: String,
}
impl Shape {
    /// The conforming shape: the catalog names Echo's ABI coordinates and the
    /// application declares its own type names over the ABI representation.
    pub(super) fn conforming(name: &str) -> Self {
        Self {
            input_name: "MutationInput",
            output_name: "MutationUnit",
            catalog_argument: format!("echo.dpo@1.{name}.input"),
            catalog_return: format!("echo.dpo@1.{name}.result"),
            fields: layout(name),
            output: map([("kind", text("Record")), ("fields", map([]))]),
            effect_kind: effect_kind(name).to_owned(),
        }
    }
}
pub(super) fn fixture(name: &str, profile: &str) -> (V, Vec<u8>) {
    fixture_with(name, profile, &Shape::conforming(name))
}
pub(super) fn fixture_with(name: &str, profile: &str, shape: &Shape) -> (V, Vec<u8>) {
    let artifacts = repinned_lowering_artifacts();
    let decode = |key| {
        let V::Bytes(raw) = get(&artifacts, key) else {
            panic!("artifact")
        };
        echo_edict_canonical::decode_canonical_cbor_v1(raw).expect("artifact")
    };
    let (_, _, mut exports) = compiler_fixture(&artifacts);
    let V::Array(effects) = value_field_mut(&mut exports, "effects") else {
        panic!("effects")
    };
    let effect = &mut effects[0];
    set(effect, "inputType", text(shape.input_name));
    set(effect, "outputType", text(shape.output_name));
    set(effect, "effectFailures", map([]));
    let mut adapter = decode("adapter_artifact");
    let implementation = value_field_mut(
        value_field_mut(&mut adapter, "effectImplementations"),
        "jedit.text@1.readFact",
    );
    set(
        implementation,
        "targetIntrinsic",
        text(&format!("echo.dpo@1.{name}")),
    );
    set(implementation, "writeClass", text("write"));
    set(implementation, "failureMappings", map([]));
    let V::Map(profiles) = value_field_mut(&mut adapter, "operationProfiles") else {
        panic!("profiles")
    };
    set(&mut profiles[0].1, "core", text(profile));
    let mut catalog = catalog_fixture();
    let intrinsic = get(get(&catalog, "intrinsics"), "echo.dpo@1.node-atom-read").clone();
    let mut intrinsic = intrinsic;
    set(
        &mut intrinsic,
        "argumentTypes",
        V::Array(vec![text(&shape.catalog_argument)]),
    );
    set(&mut intrinsic, "returnType", text(&shape.catalog_return));
    set(&mut intrinsic, "effectKind", text(&shape.effect_kind));
    set(&mut intrinsic, "writeClass", text("write"));
    set(&mut intrinsic, "effectFailures", map([]));
    set(
        &mut catalog,
        "intrinsics",
        V::Map(vec![(text(&format!("echo.dpo@1.{name}")), intrinsic)]),
    );
    let lowering = lowering_fixture_inputs(Some(catalog), Some(exports.clone()), Some(adapter));
    let (mut core, mut target, _) = compiler_fixture(&lowering);
    let fields = shape.fields.clone();
    let input_type = map([
        ("kind", text("Record")),
        (
            "fields",
            V::Map(fields.iter().map(|(k, t)| (text(k), text(t))).collect()),
        ),
    ]);
    add(
        value_field_mut(&mut core, "types"),
        shape.input_name,
        input_type,
    );
    add(
        value_field_mut(&mut core, "types"),
        shape.output_name,
        shape.output.clone(),
    );
    let arg = map([
        ("id", text("arg.0")),
        ("type", text(shape.input_name)),
        ("alphaName", text("$arg0")),
    ]);
    let local = map([
        ("id", text("local.0")),
        ("type", text(shape.output_name)),
        ("alphaName", text("$local0")),
    ]);
    let input = map([("kind", text("local")), ("ref", arg.clone())]);
    let result = map([("kind", text("local")), ("ref", local.clone())]);
    let node = map([
        ("kind", text("effect")),
        ("binding", local.clone()),
        ("effect", text("text.readFact")),
        ("input", input.clone()),
        ("obstructionMap", map([])),
    ]);
    let intent = value_field_mut(value_field_mut(&mut core, "intents"), "replaceRange");
    set(intent, "input", text(shape.input_name));
    set(intent, "output", text(shape.output_name));
    set(intent, "requiredOperationProfile", text(profile));
    if let V::Map(fields) = intent {
        fields.retain(|(k, _)| k != &text("basis"));
    }
    set(
        intent,
        "body",
        map([
            ("locals", V::Array(vec![arg, local.clone()])),
            ("nodes", V::Array(vec![node])),
            ("result", result.clone()),
        ]),
    );
    let selected = selected_target_mut(&mut target);
    set(selected, "operationProfile", text(profile));
    if let V::Map(fields) = selected {
        fields.retain(|(k, _)| k != &text("basis"));
    }
    set(
        selected,
        "steps",
        V::Array(vec![map([
            ("id", text("effect.0")),
            ("binding", local),
            ("effect", text("text.readFact")),
            ("input", input),
            ("targetIntrinsic", text(&format!("echo.dpo@1.{name}"))),
            ("obstructionArms", map([])),
            ("obstructionFailures", V::Array(vec![])),
        ])]),
    );
    set(selected, "requirements", V::Array(vec![]));
    set(selected, "result", result);
    set(selected, "executionOrder", V::Array(vec![text("effect.0")]));
    set(
        value_field_mut(
            value_field_mut(&mut target, "semanticClosure"),
            "sourceCore",
        ),
        "digest",
        V::Array(vec![
            text("sha256"),
            artifact_identity("edict.core.module/v1", &core),
        ]),
    );
    let publication = publication_from_parts(&core, &target, &exports, &lowering);
    let input = V::Map(
        fields
            .iter()
            .enumerate()
            .map(|(i, (name, _))| {
                (
                    text(name),
                    if *name == "atomBytes" {
                        V::Bytes(b"payload".to_vec())
                    } else {
                        id(u8::try_from(i).expect("field index") + 10)
                    },
                )
            })
            .collect(),
    );
    (publication, encoded(&input))
}
#[test]
fn all_five_compiled_mutation_emitters_prepare_and_run_without_writing_the_frontier() {
    for name in NAMES {
        let (publication, input) = fixture(name, "continuum.profile.graph-effects/v1");
        let package = prepare(&publication).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let program = package.program().as_effect_program().expect("program");
        let (host, head, _) = host();
        let basis = host
            .echo_operation_evaluation_basis_v1(
                head,
                EchoOperationApplicationBasisV1::new([6; 32], [7; 32]),
            )
            .expect("basis");
        let mut frame = host
            .effect_evaluation_frame_v1(
                basis,
                &[],
                0,
                EchoOperationBudgetV1::new(10000, 4096, 4096),
                limits(),
            )
            .expect("frame")
            .with_footprint_law(program.footprint_law().clone())
            .expect("law");
        assert_eq!(
            frame.evaluate_effect(program, &input, &[]).expect(name),
            encoded(&map([]))
        );
        let node = NodeKey {
            warp_id: WarpId([10; 32]),
            local_id: NodeId([11; 32]),
        };
        let edge = EdgeKey {
            warp_id: WarpId([10; 32]),
            local_id: EdgeId([11; 32]),
        };
        let atom = AtomPayload {
            type_id: TypeId([13; 32]),
            bytes: b"payload".to_vec().into(),
        };
        let expected = match name {
            "ensure-node" => EffectMutation::EnsureNode {
                node,
                ty: TypeId([12; 32]),
                atom,
            },
            "create-node" => EffectMutation::CreateNode {
                node,
                ty: TypeId([12; 32]),
                atom,
            },
            "replace-atom" => EffectMutation::ReplaceAtom {
                node,
                expected: [12; 32],
                atom,
            },
            "ensure-edge" => EffectMutation::EnsureEdge {
                edge,
                from: NodeId([12; 32]),
                to: NodeId([13; 32]),
                ty: TypeId([14; 32]),
            },
            "replace-edge" => EffectMutation::ReplaceEdge {
                edge,
                expected_to: NodeId([12; 32]),
                to: NodeId([13; 32]),
            },
            _ => panic!("kind"),
        };
        assert_eq!(frame.buffered_mutations(), &[expected]);
        assert!(frame.readings().is_empty());
        assert_eq!(
            frame.consumed().write_bytes(),
            if name.ends_with("edge") { 128 } else { 71 }
        );
        assert_eq!(frame.evaluation_basis(), Some(basis));
    }
}
fn parts(publication: &V) -> (V, V, V, V) {
    let embedded = |value: &V| {
        let V::Bytes(raw) = value else {
            panic!("bytes")
        };
        echo_edict_canonical::decode_canonical_cbor_v1(raw).expect("canonical")
    };
    let program = embedded(get(publication, "program"));
    let bundle = embedded(get(&program, "compiled_program"));
    (
        embedded(get(&bundle, "core_artifact")),
        embedded(get(&bundle, "target_ir_artifact")),
        embedded(get(&bundle, "lawpack_exports_artifact")),
        get(&bundle, "lowering_artifacts").clone(),
    )
}
#[test]
fn mutation_emitters_cannot_be_authorized_by_a_read_only_profile() {
    for name in NAMES {
        assert!(
            prepare(&fixture(name, "continuum.profile.read-only/v1").0).is_err(),
            "{name}"
        );
    }
}
#[test]
fn mutation_emitter_abi_rejects_redefined_address_widths_under_fresh_pins() {
    for name in NAMES {
        let (publication, _) = fixture(name, "continuum.profile.graph-effects/v1");
        let (mut core, mut target, exports, lowering) = parts(&publication);
        let fields = value_field_mut(
            value_field_mut(value_field_mut(&mut core, "types"), "MutationInput"),
            "fields",
        );
        set(fields, "warpId", text("Bytes<exact=16>"));
        repin_core(&core, &mut target);
        assert!(
            prepare(&publication_from_parts(&core, &target, &exports, &lowering)).is_err(),
            "{name}"
        );
    }
}
#[test]
fn failure_after_emission_does_not_publish_a_partial_intent_buffer() {
    use warp_core::edict_effects::EffectFrameError;
    use warp_core::edict_pure::EvaluationError;
    use warp_core::edict_read::ReadError;
    for name in NAMES {
        let (mut publication, input) = fixture(name, "continuum.profile.graph-effects/v1");
        let V::Bytes(raw) = get(&publication, "program") else {
            panic!("program")
        };
        let mut value = echo_edict_canonical::decode_canonical_cbor_v1(raw).expect("program");
        set(&mut value, "max_result_bytes", V::Integer(0));
        set(&mut publication, "program", V::Bytes(encoded(&value)));
        let package = prepare(&publication).expect("zero output ceiling");
        let program = package.program().as_effect_program().expect("program");
        let (host, head, _) = host();
        let basis = host
            .echo_operation_evaluation_basis_v1(
                head,
                EchoOperationApplicationBasisV1::new([6; 32], [7; 32]),
            )
            .expect("basis");
        let mut frame = host
            .effect_evaluation_frame_v1(
                basis,
                &[],
                0,
                EchoOperationBudgetV1::new(10000, 4096, 4096),
                limits(),
            )
            .expect("frame")
            .with_footprint_law(program.footprint_law().clone())
            .expect("law");
        assert_eq!(
            frame.evaluate_effect(program, &input, &[]),
            Err(EffectFrameError::BoundedRead(ReadError::Evaluation(
                EvaluationError::OutputBudgetExceeded
            )))
        );
        assert!(frame.buffered_mutations().is_empty());
        assert_eq!(
            frame.consumed().write_bytes(),
            if name.ends_with("edge") { 128 } else { 71 },
            "a later failure must not refund an admitted emission"
        );
        assert_eq!(
            frame.evaluate_effect(program, &input, &[]),
            Err(EffectFrameError::Terminated)
        );
    }
}
#[test]
fn mutation_emission_refuses_a_write_budget_one_byte_short_before_buffering() {
    for name in NAMES {
        let (publication, input) = fixture(name, "continuum.profile.graph-effects/v1");
        let package = prepare(&publication).expect("program");
        let program = package.program().as_effect_program().expect("effect");
        let (host, head, _) = host();
        let basis = host
            .echo_operation_evaluation_basis_v1(
                head,
                EchoOperationApplicationBasisV1::new([6; 32], [7; 32]),
            )
            .expect("basis");
        let write = if name.ends_with("edge") { 127 } else { 70 };
        let mut frame = host
            .effect_evaluation_frame_v1(
                basis,
                &[],
                0,
                EchoOperationBudgetV1::new(10000, 4096, write),
                limits(),
            )
            .expect("frame")
            .with_footprint_law(program.footprint_law().clone())
            .expect("law");
        assert!(
            frame.evaluate_effect(program, &input, &[]).is_err(),
            "{name} ignored its emission write budget"
        );
        assert!(frame.buffered_mutations().is_empty());
        assert_eq!(
            frame.consumed().write_bytes(),
            0,
            "failed write charge must remain atomic"
        );
    }
}
fn repeat_emission(publication: &V, count: usize) -> V {
    let (mut core, mut target, exports, lowering) = parts(publication);
    let body = core_body_mut(&mut core);
    let V::Array(original_nodes) = get(body, "nodes") else {
        panic!("nodes")
    };
    let node_template = original_nodes[0].clone();
    let V::Array(original_locals) = get(body, "locals") else {
        panic!("locals")
    };
    let mut locals = vec![original_locals[0].clone()];
    let V::Array(original_steps) = get(selected_target_mut(&mut target), "steps") else {
        panic!("steps")
    };
    let step_template = original_steps[0].clone();
    let mut nodes = Vec::new();
    let mut steps = Vec::new();
    let mut order = Vec::new();
    for index in 0..count {
        let local = map([
            ("id", text(&format!("local.{index}"))),
            ("type", text("MutationUnit")),
            ("alphaName", text(&format!("$local{index}"))),
        ]);
        let mut node = node_template.clone();
        set(&mut node, "binding", local.clone());
        nodes.push(node);
        let id = text(&format!("effect.{index}"));
        let mut step = step_template.clone();
        set(&mut step, "id", id.clone());
        set(&mut step, "binding", local.clone());
        steps.push(step);
        order.push(id);
        locals.push(local);
    }
    let result = map([
        ("kind", text("local")),
        ("ref", locals.last().expect("local").clone()),
    ]);
    set(body, "locals", V::Array(locals));
    set(body, "nodes", V::Array(nodes));
    set(body, "result", result.clone());
    let selected = selected_target_mut(&mut target);
    set(selected, "steps", V::Array(steps));
    set(selected, "executionOrder", V::Array(order));
    set(selected, "result", result);
    repin_core(&core, &mut target);
    publication_from_parts(&core, &target, &exports, &lowering)
}
#[test]
fn emitted_effect_count_is_bounded_even_when_all_intents_share_one_target() {
    for count in [1024, 1025] {
        let (publication, input) = fixture("ensure-edge", "continuum.profile.graph-effects/v1");
        let package =
            prepare(&repeat_emission(&publication, count)).expect("bounded repeated source");
        let program = package.program().as_effect_program().expect("effect");
        let (host, head, _) = host();
        let basis = host
            .echo_operation_evaluation_basis_v1(
                head,
                EchoOperationApplicationBasisV1::new([6; 32], [7; 32]),
            )
            .expect("basis");
        let mut allocation = limits();
        allocation.max_package_bytes = 4 * 1024 * 1024;
        allocation.max_steps = 1_048_576;
        allocation.max_allocated_bytes = 16 * 1024 * 1024;
        let mut frame = host
            .effect_evaluation_frame_v1(
                basis,
                &[],
                0,
                EchoOperationBudgetV1::new(1_048_576, 4096, 16 * 1024 * 1024),
                allocation,
            )
            .expect("frame")
            .with_footprint_law(program.footprint_law().clone())
            .expect("law");
        let result = frame.evaluate_effect(program, &input, &[]);
        if count == 1024 {
            assert!(result.is_ok());
            assert_eq!(frame.buffered_mutations().len(), 1024);
        } else {
            assert!(
                result.is_err(),
                "the 1025th effect bypassed the fixed emission ceiling"
            );
            assert!(frame.buffered_mutations().is_empty());
        }
        assert_eq!(
            frame.consumed().write_bytes(),
            131_072,
            "the successful prefix must remain charged, including on refusal"
        );
    }
}
#[test]
fn row_reservation_failure_does_not_commit_its_emission_write_charge() {
    use warp_core::edict_effects::EffectFrameError;
    use warp_core::edict_pure::EvaluationError;
    use warp_core::edict_read::ReadError;
    for name in NAMES {
        let (publication, input) = fixture(name, "continuum.profile.graph-effects/v1");
        let package = prepare(&publication).expect("program");
        let program = package.program().as_effect_program().expect("effect");
        let (host, head, _) = host();
        let basis = host
            .echo_operation_evaluation_basis_v1(
                head,
                EchoOperationApplicationBasisV1::new([6; 32], [7; 32]),
            )
            .expect("basis");
        let mut control = host
            .effect_evaluation_frame_v1(
                basis,
                &[],
                0,
                EchoOperationBudgetV1::new(10000, 4096, 4096),
                limits(),
            )
            .expect("frame")
            .with_footprint_law(program.footprint_law().clone())
            .expect("law");
        control
            .evaluate_effect(program, &input, &[])
            .expect("control");
        // After the row, this empty-record program reserves three 64-byte
        // value cells: the unit binding, local result copy and encoding scratch.
        // Remove that tail and one extra byte to miss the row by exactly one.
        let mut ceiling = limits();
        ceiling.max_allocated_bytes = control
            .allocated_bytes()
            .checked_sub(193)
            .expect("row and output storage");
        let mut frame = host
            .effect_evaluation_frame_v1(
                basis,
                &[],
                0,
                EchoOperationBudgetV1::new(10000, 4096, 4096),
                ceiling,
            )
            .expect("frame")
            .with_footprint_law(program.footprint_law().clone())
            .expect("law");
        assert_eq!(
            frame.evaluate_effect(program, &input, &[]),
            Err(EffectFrameError::BoundedRead(ReadError::Evaluation(
                EvaluationError::AllocationBudgetExceeded
            )))
        );
        assert_eq!(
            frame.consumed().write_bytes(),
            0,
            "{name} committed its failed row's write reservation"
        );
        assert!(frame.buffered_mutations().is_empty());
    }
}
