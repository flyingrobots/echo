// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Echo's pure identity intrinsics against the frozen `ContentAddressV1` vectors.
//!
//! Expected identities are the frozen files in
//! `tests/fixtures/content-address-v1/`, which were derived from independently
//! assembled preimages; no expectation here is computed by the production
//! identity module. Expected costs are the contract numbers stated in
//! `docs/architecture/authored-operation-effects.md`, restated below.
use super::{
    evaluate::{expression, Meter},
    functions::{check_roots, decode},
    model::MAX_PROGRAM_NODES,
    syntax::Parser,
    EvaluationError as Error, EvaluationLimits,
};
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::BTreeMap;

const CONTENT_ADDRESS: &str = "echo.dpo@1.contentAddress";
const CONTENT_EDGE_ID: &str = "echo.dpo@1.contentEdgeId";
const SLOT_EDGE_ID: &str = "echo.dpo@1.slotEdgeId";

const NODE_ZERO_EDGES: &str =
    include_str!("../../tests/fixtures/content-address-v1/node-zero-edges.hex");
const NODE_ONE_EDGE: &str =
    include_str!("../../tests/fixtures/content-address-v1/node-one-edge.hex");
const NODE_TWO_EDGES: &str =
    include_str!("../../tests/fixtures/content-address-v1/node-two-edges.hex");
const CONTENT_EDGE: &str = include_str!("../../tests/fixtures/content-address-v1/content-edge.hex");
const SLOT_EDGE: &str = include_str!("../../tests/fixtures/content-address-v1/slot-edge.hex");

// The fixture README's inputs: 32 repetitions of the named byte.
const WARP: [u8; 32] = [3; 32];
const NODE_TYPE: [u8; 32] = [7; 32];
const ATOM_TYPE: [u8; 32] = [2; 32];
const EDGE_A: [u8; 32] = [6; 32];
const EDGE_B: [u8; 32] = [8; 32];
const TO_LOW: [u8; 32] = [4; 32];
const TO_HIGH: [u8; 32] = [5; 32];
const ATOM: &[u8] = b"new";

// Documented cost contract: steps and storage one call adds beyond evaluating
// its arguments.
const NODE_STEPS: u64 = 71;
const NODE_STEPS_PER_EDGE: u64 = 67;
const CONTENT_EDGE_STEPS: u64 = 70;
const SLOT_EDGE_STEPS: u64 = 69;
const ID_STORAGE: u64 = 96;
const STORAGE_PER_EDGE: u64 = 64;

fn map(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        fields
            .into_iter()
            .map(|(k, v)| (Value::Text(k.into()), v))
            .collect(),
    )
}
fn text(value: &str) -> Value {
    Value::Text(value.into())
}
fn bytes(value: &[u8]) -> Value {
    map([
        ("kind", text("const")),
        (
            "value",
            map([
                ("kind", text("bytes")),
                ("value", Value::Bytes(value.to_vec())),
            ]),
        ),
    ])
}
fn integer(value: u64) -> Value {
    map([
        ("kind", text("const")),
        (
            "value",
            map([
                ("kind", text("int")),
                ("width", text("U64")),
                ("value", Value::Integer(i128::from(value))),
            ]),
        ),
    ])
}
fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    map([("kind", text("record")), ("fields", map(fields))])
}
fn edge(edge_type: &[u8], to: &[u8]) -> Value {
    record([("edgeType", bytes(edge_type)), ("to", bytes(to))])
}
fn list(values: Vec<Value>) -> Value {
    map([("kind", text("list")), ("values", Value::Array(values))])
}
fn call(callee: &str, types: &[&str], args: Vec<Value>) -> Value {
    map([
        ("kind", text("call")),
        ("callee", text(callee)),
        (
            "typeArgs",
            Value::Array(types.iter().map(|ty| text(ty)).collect()),
        ),
        ("args", Value::Array(args)),
    ])
}
fn node_arguments(atom: &[u8], edges: Vec<Value>) -> Vec<Value> {
    vec![
        bytes(&WARP),
        bytes(&NODE_TYPE),
        bytes(&ATOM_TYPE),
        bytes(atom),
        list(edges),
    ]
}
fn node(atom: &[u8], edges: Vec<Value>) -> Value {
    call(CONTENT_ADDRESS, &[], node_arguments(atom, edges))
}
fn content_edge_arguments() -> Vec<Value> {
    vec![
        bytes(&WARP),
        bytes(&TO_LOW),
        bytes(&EDGE_A),
        bytes(&TO_HIGH),
    ]
}
fn slot_edge_arguments() -> Vec<Value> {
    vec![bytes(&WARP), bytes(&TO_LOW), bytes(&EDGE_A)]
}
fn frozen(hex: &str) -> Value {
    let digits = hex
        .trim()
        .bytes()
        .map(|digit| match digit {
            b'0'..=b'9' => digit - b'0',
            b'a'..=b'f' => digit - b'a' + 10,
            _ => u8::MAX,
        })
        .collect::<Vec<_>>();
    assert!(
        digits.len() == 64 && digits.iter().all(|digit| *digit < 16),
        "a frozen vector is 64 lowercase hex digits"
    );
    Value::Bytes(
        digits
            .chunks_exact(2)
            .map(|pair| pair[0] << 4 | pair[1])
            .collect(),
    )
}
fn limits(steps: u64, allocated: u64) -> EvaluationLimits {
    EvaluationLimits {
        max_package_bytes: 1024,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        max_steps: steps,
        max_allocated_bytes: allocated,
    }
}
const AMPLE: (u64, u64) = (1 << 22, 1 << 23);

fn parsed(wire: &Value) -> Result<super::model::Expr, Error> {
    let types = Value::Map(vec![]);
    let mut parser = Parser {
        types: &types,
        coordinate: "ids.example@1",
        remaining: MAX_PROGRAM_NODES,
    };
    parser.expr(wire, 0)
}
/// The value and the (steps, storage) one expression consumes under `limits`.
fn run(wire: &Value, limits: EvaluationLimits) -> Result<(Value, (u64, u64)), Error> {
    let expr = parsed(wire)?;
    check_roots(&BTreeMap::new(), [&expr], [])?;
    let mut meter = Meter::new(limits);
    let value = expression(&expr, &BTreeMap::new(), &BTreeMap::new(), &mut meter, 0)?;
    Ok((value, meter.usage()))
}
fn evaluated(wire: &Value) -> Result<Value, Error> {
    run(wire, limits(AMPLE.0, AMPLE.1)).map(|(value, _)| value)
}
/// What one call consumes beyond evaluating each of its arguments.
fn own_usage(callee: &str, arguments: Vec<Value>) -> Result<(u64, u64), Error> {
    let mut operands = (0, 0);
    for argument in &arguments {
        let (_, (steps, allocated)) = run(argument, limits(AMPLE.0, AMPLE.1))?;
        operands = (operands.0 + steps, operands.1 + allocated);
    }
    let (_, (steps, allocated)) = run(&call(callee, &[], arguments), limits(AMPLE.0, AMPLE.1))?;
    Ok((steps - operands.0, allocated - operands.1))
}

#[test]
fn content_address_reproduces_frozen_node_vectors_for_empty_and_ordered_edges() -> Result<(), Error>
{
    let a_high = || edge(&EDGE_A, &TO_HIGH);
    let b_low = || edge(&EDGE_B, &TO_LOW);
    for (name, edges, vector) in [
        ("no edges", vec![], NODE_ZERO_EDGES),
        ("one edge", vec![a_high()], NODE_ONE_EDGE),
        ("fixture order", vec![b_low(), a_high()], NODE_TWO_EDGES),
        ("canonical order", vec![a_high(), b_low()], NODE_TWO_EDGES),
    ] {
        assert_eq!(evaluated(&node(ATOM, edges))?, frozen(vector), "{name}");
    }
    Ok(())
}

#[test]
fn edge_id_calls_reproduce_their_frozen_vectors_and_stay_domain_separated() -> Result<(), Error> {
    let content = evaluated(&call(CONTENT_EDGE_ID, &[], content_edge_arguments()))?;
    let slot = evaluated(&call(SLOT_EDGE_ID, &[], slot_edge_arguments()))?;
    assert_eq!(content, frozen(CONTENT_EDGE));
    assert_eq!(slot, frozen(SLOT_EDGE));
    // Same warp, source and edge type: the two laws never name one edge alike.
    assert_ne!(content, slot);
    Ok(())
}

#[test]
fn each_wrong_identity_input_names_a_different_node() -> Result<(), Error> {
    let other = [9_u8; 32];
    let arguments = |index: usize, replacement: Value| {
        let mut arguments = node_arguments(ATOM, vec![edge(&EDGE_A, &TO_HIGH)]);
        arguments[index] = replacement;
        arguments
    };
    for (name, arguments) in [
        ("warp", arguments(0, bytes(&other))),
        ("node type", arguments(1, bytes(&other))),
        ("atom type", arguments(2, bytes(&other))),
        ("atom bytes", arguments(3, bytes(b"old"))),
        ("empty atom", arguments(3, bytes(b""))),
        (
            "edge type",
            arguments(4, list(vec![edge(&other, &TO_HIGH)])),
        ),
        (
            "edge target",
            arguments(4, list(vec![edge(&EDGE_A, &other)])),
        ),
    ] {
        let id = evaluated(&call(CONTENT_ADDRESS, &[], arguments))?;
        assert!(matches!(&id, Value::Bytes(id) if id.len() == 32), "{name}");
        assert_ne!(id, frozen(NODE_ONE_EDGE), "{name}");
    }
    Ok(())
}

#[test]
fn duplicate_outgoing_edges_refuse_instead_of_deduplicating() {
    let a_high = || edge(&EDGE_A, &TO_HIGH);
    let b_low = || edge(&EDGE_B, &TO_LOW);
    for (name, edges) in [
        ("adjacent", vec![a_high(), a_high()]),
        ("separated", vec![a_high(), b_low(), a_high()]),
        ("after sorting", vec![b_low(), a_high(), b_low()]),
    ] {
        assert_eq!(
            evaluated(&node(ATOM, edges)),
            Err(Error::DuplicateOutgoingEdge),
            "{name}"
        );
    }
    // Sharing only the edge type, or only the target, is two distinct edges.
    for edges in [
        vec![edge(&EDGE_A, &TO_HIGH), edge(&EDGE_A, &TO_LOW)],
        vec![edge(&EDGE_A, &TO_HIGH), edge(&EDGE_B, &TO_HIGH)],
    ] {
        assert!(matches!(evaluated(&node(ATOM, edges)), Ok(Value::Bytes(id)) if id.len() == 32));
    }
}

#[test]
fn operands_outside_the_contract_types_refuse_before_hashing() {
    let sixteen = || {
        (0..16_u8)
            .map(|i| edge(&EDGE_A, &[i; 32]))
            .collect::<Vec<_>>()
    };
    assert!(
        evaluated(&node(ATOM, sixteen())).is_ok(),
        "sixteen edges fit"
    );
    let mut seventeen = sixteen();
    seventeen.push(edge(&EDGE_B, &TO_LOW));
    let arguments = |index: usize, replacement: Value| {
        let mut arguments = node_arguments(ATOM, vec![]);
        arguments[index] = replacement;
        call(CONTENT_ADDRESS, &[], arguments)
    };
    for (name, wire) in [
        ("short warp", arguments(0, bytes(&[3; 31]))),
        ("long node type", arguments(1, bytes(&[7; 33]))),
        ("integer atom type", arguments(2, integer(2))),
        ("integer atom", arguments(3, integer(0))),
        ("seventeen edges", arguments(4, list(seventeen))),
        ("edges not a list", arguments(4, bytes(b""))),
        (
            "short edge type",
            arguments(4, list(vec![edge(&[6; 31], &TO_HIGH)])),
        ),
        (
            "edge without target",
            arguments(4, list(vec![record([("edgeType", bytes(&EDGE_A))])])),
        ),
        (
            "edge with an extra field",
            arguments(
                4,
                list(vec![record([
                    ("edgeType", bytes(&EDGE_A)),
                    ("to", bytes(&TO_HIGH)),
                    ("weight", integer(1)),
                ])]),
            ),
        ),
        (
            "short edge source",
            call(CONTENT_EDGE_ID, &[], {
                let mut arguments = content_edge_arguments();
                arguments[1] = bytes(&[4; 31]);
                arguments
            }),
        ),
        (
            "long slot edge type",
            call(SLOT_EDGE_ID, &[], {
                let mut arguments = slot_edge_arguments();
                arguments[2] = bytes(&[6; 33]);
                arguments
            }),
        ),
    ] {
        assert_eq!(evaluated(&wire), Err(Error::InvalidArtifact), "{name}");
    }
}

#[test]
fn calls_with_the_wrong_arity_or_type_arguments_do_not_parse() {
    let mut extra = content_edge_arguments();
    extra.push(bytes(&TO_HIGH));
    for (name, wire) in [
        (
            "node without edges",
            call(
                CONTENT_ADDRESS,
                &[],
                node_arguments(ATOM, vec![])[..4].to_vec(),
            ),
        ),
        (
            "node with a type argument",
            call(
                CONTENT_ADDRESS,
                &["Bytes<max=32>"],
                node_arguments(ATOM, vec![]),
            ),
        ),
        (
            "content edge without target",
            call(CONTENT_EDGE_ID, &[], slot_edge_arguments()),
        ),
        (
            "content edge with five ids",
            call(CONTENT_EDGE_ID, &[], extra),
        ),
        (
            "slot edge with a target",
            call(SLOT_EDGE_ID, &[], content_edge_arguments()),
        ),
        (
            "slot edge with a type argument",
            call(SLOT_EDGE_ID, &["Bytes<exact=32>"], slot_edge_arguments()),
        ),
    ] {
        assert_eq!(parsed(&wire), Err(Error::UnsupportedProgram), "{name}");
    }
}

#[test]
fn a_call_charges_its_documented_work_for_the_atom_and_each_edge() -> Result<(), Error> {
    let edges = |count: u8| {
        (0..count)
            .map(|i| edge(&EDGE_A, &[i; 32]))
            .collect::<Vec<_>>()
    };
    for (atom, count) in [(0_usize, 0_u8), (3, 0), (0, 1), (300, 2), (1, 16)] {
        let atom = vec![0x6e; atom];
        let arguments = node_arguments(&atom, edges(count));
        assert_eq!(
            own_usage(CONTENT_ADDRESS, arguments)?,
            (
                NODE_STEPS + atom.len() as u64 + NODE_STEPS_PER_EDGE * u64::from(count),
                ID_STORAGE + STORAGE_PER_EDGE * u64::from(count),
            ),
            "{} atom bytes, {count} edges",
            atom.len()
        );
    }
    // Edge order is not a cost input: sorted and unsorted inputs cost the same.
    assert_eq!(
        own_usage(
            CONTENT_ADDRESS,
            node_arguments(ATOM, vec![edge(&EDGE_B, &TO_LOW), edge(&EDGE_A, &TO_HIGH)])
        )?,
        own_usage(
            CONTENT_ADDRESS,
            node_arguments(ATOM, vec![edge(&EDGE_A, &TO_HIGH), edge(&EDGE_B, &TO_LOW)])
        )?,
    );
    assert_eq!(
        own_usage(CONTENT_EDGE_ID, content_edge_arguments())?,
        (CONTENT_EDGE_STEPS, ID_STORAGE)
    );
    assert_eq!(
        own_usage(SLOT_EDGE_ID, slot_edge_arguments())?,
        (SLOT_EDGE_STEPS, ID_STORAGE)
    );
    Ok(())
}

#[test]
fn one_unit_less_than_the_measured_budget_refuses_each_call() -> Result<(), Error> {
    for wire in [
        node(ATOM, vec![edge(&EDGE_B, &TO_LOW), edge(&EDGE_A, &TO_HIGH)]),
        call(CONTENT_EDGE_ID, &[], content_edge_arguments()),
        call(SLOT_EDGE_ID, &[], slot_edge_arguments()),
    ] {
        let (value, (steps, allocated)) = run(&wire, limits(AMPLE.0, AMPLE.1))?;
        assert_eq!(run(&wire, limits(steps, allocated))?.0, value, "exact");
        assert_eq!(
            run(&wire, limits(steps - 1, allocated)),
            Err(Error::StepBudgetExceeded)
        );
        assert_eq!(
            run(&wire, limits(steps, allocated - 1)),
            Err(Error::AllocationBudgetExceeded)
        );
    }
    // A duplicate edge set that cannot be paid for is a budget refusal first.
    let duplicate = node(ATOM, vec![edge(&EDGE_A, &TO_HIGH), edge(&EDGE_A, &TO_HIGH)]);
    let exact = run(
        &node(ATOM, vec![edge(&EDGE_A, &TO_HIGH), edge(&EDGE_A, &TO_LOW)]),
        limits(AMPLE.0, AMPLE.1),
    )?
    .1;
    // The refused call never pays for copying a result: 32 bytes and its cell.
    assert_eq!(
        run(&duplicate, limits(exact.0 - 33, exact.1)),
        Err(Error::DuplicateOutgoingEdge)
    );
    assert_eq!(
        run(&duplicate, limits(exact.0 - 34, exact.1)),
        Err(Error::StepBudgetExceeded)
    );
    Ok(())
}

fn helpers(functions: Vec<(&'static str, Value)>, coordinate: &str) -> Result<(), Error> {
    let core = map([
        ("functions", map(functions)),
        ("intents", Value::Map(vec![])),
        ("types", Value::Map(vec![])),
    ]);
    let exports = map([
        ("pureFunctions", Value::Array(vec![])),
        ("effects", Value::Array(vec![])),
    ]);
    let types = Value::Map(vec![]);
    let mut parser = Parser {
        types: &types,
        coordinate,
        remaining: MAX_PROGRAM_NODES,
    };
    decode(&core, &exports, &mut parser).map(|_| ())
}
fn local(id: &str, ty: &str) -> Value {
    map([
        ("id", text(id)),
        ("alphaName", text(id)),
        ("type", text(ty)),
    ])
}
fn reference(local: &Value) -> Value {
    map([("kind", text("local")), ("ref", local.clone())])
}
/// A source helper `(atom) -> returns` whose body is `result`.
fn function(parameter: &str, returns: &str, result: Value) -> Value {
    map([
        ("params", Value::Array(vec![local("arg.0", parameter)])),
        ("returnType", text(returns)),
        (
            "body",
            map([
                ("locals", Value::Array(vec![])),
                ("bindings", Value::Array(vec![])),
                ("result", result),
            ]),
        ),
    ])
}

#[test]
fn source_helpers_may_call_identity_intrinsics_only_within_their_contract() {
    let id = "Bytes<exact=32>";
    let atom = |ty: &str| reference(&local("arg.0", ty));
    let node_of = |ty: &str| {
        let mut arguments = node_arguments(b"", vec![edge(&EDGE_A, &TO_HIGH)]);
        arguments[3] = atom(ty);
        call(CONTENT_ADDRESS, &[], arguments)
    };
    let cases = [
        ("an atom at the ceiling", "Bytes<max=1048576>", id, true),
        ("a narrower atom", "Bytes<max=64>", id, true),
        ("an atom past the ceiling", "Bytes<max=1048577>", id, false),
        (
            "a result wider than an id",
            "Bytes<max=64>",
            "Bytes<exact=33>",
            false,
        ),
    ];
    for (name, parameter, returns, accepted) in cases {
        let helper = function(parameter, returns, node_of(parameter));
        assert_eq!(
            helpers(vec![("node", helper)], "ids.example@1").is_ok(),
            accepted,
            "{name}"
        );
    }
    let wrong_operand = function("Bytes<max=64>", id, {
        let mut arguments = node_arguments(b"", vec![]);
        arguments[0] = atom("Bytes<max=64>");
        call(CONTENT_ADDRESS, &[], arguments)
    });
    assert!(helpers(vec![("node", wrong_operand)], "ids.example@1").is_err());
}

#[test]
fn a_module_cannot_define_a_helper_at_an_identity_intrinsic_coordinate() {
    let helper = || function("Bytes<exact=32>", "Bytes<exact=32>", atom_reference());
    for name in ["contentAddress", "contentEdgeId", "slotEdgeId"] {
        assert_eq!(
            helpers(vec![(name, helper())], "echo.dpo@1"),
            Err(Error::InvalidArtifact),
            "{name}"
        );
        assert_eq!(helpers(vec![(name, helper())], "ids.example@1"), Ok(()));
    }
}
fn atom_reference() -> Value {
    reference(&local("arg.0", "Bytes<exact=32>"))
}
