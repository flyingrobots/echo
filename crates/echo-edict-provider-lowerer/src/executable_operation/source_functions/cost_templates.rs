// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Conformance of Echo's packaged `edict.target-profile.intrinsics/v3` effect
//! cost templates (amblicit#341, grammar of amblicit#345; authored in
//! `echo-wesley-gen`'s `provider_effect_costs`) to the lowerer's own
//! per-effect charge.
//!
//! Under the shared-read schedule a template replaces the compiler's generic
//! charge of an effect step, so each template states exactly what the lowerer
//! charges for that step beyond the step's input expression: the input's
//! validations plus the intrinsic's own work (`abi` or `graph_charge`). The
//! one empty obstruction payload, (1 step, 64 bytes) when a step maps any
//! obstruction arm, is charged by both sides outside the template.
//!
//! The evaluator below follows amblicit#345's evaluation over the lowerer's
//! own types; the provider value shape `copy`/`validations` is the lowerer's
//! `shape`, which has the same table as Amblicit's (checked by hand against
//! `effect_costs.rs` at 6e217bb9: Bool/Word/Bytes/Option/List/Record).
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
use super::effect_intent::{abi, graph_charge};
use super::{shape, Cost, Fact, Ty};
use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, encode_canonical_cbor_v1 as encode,
    CanonicalValueV1 as Value,
};
use std::collections::BTreeMap;

/// The packaged `echo.dpo@2` catalog, which provider generation writes from
/// `echo-wesley-gen`'s `provider_effect_costs`: the templates evaluated here
/// are exactly the published ones.
const CATALOG: &[u8] = include_bytes!(
    "../../../../../schemas/edict-provider/package/v1/generated/resources/resource.target-intrinsics-3.cbor"
);
const CATALOG_API: &str = "edict.target-profile.intrinsics/v3";

fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}

/// The published `effectCostRule` of one `echo.dpo@2` effect operation.
fn effect_cost_rule(operation: &str) -> Option<Value> {
    let catalog = decode(CATALOG).expect("packaged catalog");
    let intrinsic = field(
        field(&catalog, "intrinsics")?,
        &format!("echo.dpo@2.{operation}"),
    )?;
    field(intrinsic, "effectCostRule").cloned()
}

// ---- evaluation ------------------------------------------------------------

fn field<'v>(value: &'v Value, name: &str) -> Option<&'v Value> {
    let Value::Map(entries) = value else {
        return None;
    };
    entries
        .iter()
        .find_map(|(key, inner)| (*key == text(name)).then_some(inner))
}
fn single(value: &Value) -> (&str, &Value) {
    let Value::Map(entries) = value else {
        panic!("single-key map")
    };
    let [(Value::Text(key), inner)] = entries.as_slice() else {
        panic!("single-key map")
    };
    (key, inner)
}
fn names(value: Option<&Value>) -> Vec<&str> {
    match value {
        None => Vec::new(),
        Some(Value::Array(names)) => names
            .iter()
            .map(|name| match name {
                Value::Text(name) => name.as_str(),
                _ => panic!("name"),
            })
            .collect(),
        Some(_) => panic!("names"),
    }
}
fn integer(value: &Value) -> u64 {
    match value {
        Value::Integer(value) => u64::try_from(*value).expect("u64"),
        _ => panic!("integer"),
    }
}

/// What one template evaluation reads.
struct Site<'t> {
    input: &'t Ty,
    output: &'t Ty,
    max_edge_scans: u64,
    selector: Cost,
}

impl Site<'_> {
    fn evaluate(&self, rule: &Value) -> Cost {
        Cost {
            steps: self.expr(field(rule, "steps").expect("steps")),
            bytes: self.expr(field(rule, "bytes").expect("bytes")),
        }
    }

    fn expr(&self, value: &Value) -> u64 {
        match single(value) {
            ("const", inner) => integer(inner),
            ("sum", Value::Array(terms)) => {
                assert!((2..=16).contains(&terms.len()), "sum arity");
                terms.iter().map(|term| self.expr(term)).sum()
            }
            ("product", Value::Array(factors)) => {
                let [left, right] = factors.as_slice() else {
                    panic!("product arity")
                };
                self.expr(left) * self.expr(right)
            }
            ("measure", inner) => self.measure(inner),
            (kind, _) => panic!("expression {kind}"),
        }
    }

    fn measure(&self, value: &Value) -> u64 {
        if let Some(Value::Text(half)) = field(value, "of") {
            let pick = |cost: Cost| {
                if half == "steps" {
                    cost.steps
                } else {
                    cost.bytes
                }
            };
            if let Some(ty) = field(value, "copy") {
                return pick(shape(&self.ty(ty)).expect("shape").0);
            }
            return pick(self.selector);
        }
        match single(value) {
            ("bytesMax", ty) => match self.ty(ty).representation() {
                Ty::Bytes(_, max) => *max,
                _ => panic!("bytesMax of non-bytes"),
            },
            ("listMax", ty) => match self.ty(ty).representation() {
                Ty::List(_, max) => *max,
                _ => panic!("listMax of non-list"),
            },
            ("validations", ty) => shape(&self.ty(ty)).expect("shape").1,
            ("parameter", Value::Text(name)) if name == "maxEdgeScans" => self.max_edge_scans,
            (kind, _) => panic!("measure {kind}"),
        }
    }

    fn ty(&self, value: &Value) -> Ty {
        match single(value) {
            ("word", Value::Null) => Ty::Word(u64::MAX),
            ("record", Value::Map(entries)) => Ty::Record(
                entries
                    .iter()
                    .map(|(key, inner)| match key {
                        Value::Text(key) => (key.clone(), self.ty(inner)),
                        _ => panic!("record key"),
                    })
                    .collect(),
            ),
            ("path", path) => {
                let step = |ty: &Ty, name: &str| match ty.representation() {
                    Ty::Record(fields) => fields.get(name).expect("path field").clone(),
                    _ => panic!("path through non-record"),
                };
                let mut ty = match field(path, "side") {
                    Some(Value::Text(side)) if side == "input" => self.input.clone(),
                    Some(Value::Text(side)) if side == "output" => self.output.clone(),
                    _ => panic!("side"),
                };
                for name in names(field(path, "fields")) {
                    ty = step(&ty, name);
                }
                for _ in 0..integer(field(path, "item").expect("item")) {
                    ty = match ty.representation() {
                        Ty::List(Some(item), _) => (**item).clone(),
                        _ => panic!("item of non-list"),
                    };
                }
                for name in names(field(path, "then")) {
                    ty = step(&ty, name);
                }
                ty
            }
            (kind, _) => panic!("type {kind}"),
        }
    }
}

// ---- fixtures --------------------------------------------------------------

const ID: Ty = Ty::Bytes(32, 32);
const WORD: Ty = Ty::Word(u64::MAX);

fn record<const N: usize>(fields: [(&str, Ty); N]) -> Ty {
    Ty::Record(fields.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
}
fn list(item: Ty, max: u64) -> Ty {
    Ty::List(Some(Box::new(item)), max)
}
fn unit() -> Ty {
    Ty::Record(BTreeMap::new())
}
fn ids(names: &[&str]) -> Vec<(String, Ty)> {
    names.iter().map(|name| ((*name).to_owned(), ID)).collect()
}

/// `(operation, input, output)` for every mutation, at two atom bounds.
fn mutation_fixtures() -> Vec<(&'static str, Ty, Ty)> {
    let mut fixtures = Vec::new();
    for atom in [1, 4096] {
        let mut node = ids(&["warpId", "nodeId", "nodeType", "atomType"]);
        node.push(("atomBytes".to_owned(), Ty::Bytes(0, atom)));
        let node = Ty::Record(node.into_iter().collect());
        fixtures.push(("ensure-node", node.clone(), unit()));
        fixtures.push(("create-node", node, unit()));
        let mut replace = ids(&["warpId", "nodeId", "expectedAtomDigest", "atomType"]);
        replace.push(("atomBytes".to_owned(), Ty::Bytes(0, atom)));
        fixtures.push((
            "replace-atom",
            Ty::Record(replace.into_iter().collect()),
            unit(),
        ));
    }
    fixtures.push((
        "ensure-edge",
        Ty::Record(
            ids(&["warpId", "edgeId", "fromId", "toId", "edgeType"])
                .into_iter()
                .collect(),
        ),
        unit(),
    ));
    fixtures.push((
        "replace-edge",
        Ty::Record(
            ids(&["warpId", "edgeId", "expectedToId", "toId"])
                .into_iter()
                .collect(),
        ),
        unit(),
    ));
    fixtures
}

fn closure_types(nodes: u64, atom: u64, edges: u64, occurrences: u64, edge_types: u64) -> (Ty, Ty) {
    // A nominal guide exercises paths through a wrapper.
    let guide = Ty::Nominal(
        "app.Guide".to_owned(),
        Box::new(record([("rank", WORD), ("tag", Ty::Bytes(0, 8))])),
    );
    let input = record([
        ("warpId", ID),
        ("nodeId", ID),
        ("edgeTypes", list(ID, edge_types)),
        ("maxNodes", WORD),
        ("maxDepth", WORD),
        ("guide", guide.clone()),
    ]);
    let node = record([
        ("nodeId", ID),
        ("nodeType", ID),
        ("atomType", Ty::Bytes(0, 32)),
        ("atomBytes", Ty::Bytes(0, atom)),
    ]);
    let edge = record([
        ("edgeId", ID),
        ("from", WORD),
        ("slot", WORD),
        ("node", WORD),
    ]);
    let occurrence = record([
        ("node", WORD),
        ("edge", WORD),
        ("depth", WORD),
        ("guide", guide),
    ]);
    let output = record([
        ("nodes", list(node, nodes)),
        ("edges", list(edge, edges)),
        ("occurrences", list(occurrence, occurrences)),
    ]);
    (input, output)
}

/// The lowerer's own charge of one effect step beyond its input expression,
/// as `effect_call` composes it.
fn lowerer_charge(
    operation: &str,
    input: &Ty,
    output: &Ty,
    scans: u64,
    helpers: &BTreeMap<String, Fact>,
) -> Cost {
    let intrinsic = format!("echo.dpo@2.{operation}");
    let work = match operation {
        "outgoing-edge-read" | "closure-read" => {
            graph_charge(operation, input, output, scans, helpers).expect("graph charge")
        }
        _ => abi(&intrinsic, input, output).expect("abi"),
    };
    work.plus(Cost {
        steps: shape(input).expect("shape").1,
        bytes: 0,
    })
    .expect("charge")
}

fn fact(steps: u64, bytes: u64) -> Fact {
    Fact {
        ty: Ty::Bool,
        cost: Cost { steps, bytes },
        depth: 0,
    }
}

fn agrees(operation: &str, input: &Ty, output: &Ty, scans: u64, selector: Cost) {
    let helpers = BTreeMap::from([(
        "app.select".to_owned(),
        fact(selector.steps, selector.bytes),
    )]);
    let expected = lowerer_charge(operation, input, output, scans, &helpers);
    let site = Site {
        input,
        output,
        max_edge_scans: scans,
        selector,
    };
    let actual = site.evaluate(&effect_cost_rule(operation).expect("template"));
    assert_eq!(
        (actual.steps, actual.bytes),
        (expected.steps, expected.bytes),
        "{operation}: template vs lowerer"
    );
}

// ---- tests -----------------------------------------------------------------

#[test]
fn every_mutation_template_equals_the_lowerer_charge() {
    for (operation, input, output) in mutation_fixtures() {
        agrees(operation, &input, &output, 0, Cost::default());
    }
}

#[test]
fn the_node_atom_read_template_equals_the_lowerer_charge() {
    let input = Ty::Record(ids(&["warpId", "nodeId", "typeId"]).into_iter().collect());
    for max in [1, 77, 16 * 1024 * 1024] {
        agrees(
            "node-atom-read",
            &input,
            &Ty::Bytes(0, max),
            0,
            Cost::default(),
        );
    }
}

#[test]
fn the_outgoing_edge_read_template_equals_the_lowerer_charge() {
    let input = Ty::Record(ids(&["warpId", "nodeId", "edgeType"]).into_iter().collect());
    let row = record([("edgeId", ID), ("toId", ID)]);
    for (rows, scans) in [(1, 0), (4, 16), (65_536, 1024)] {
        agrees(
            "outgoing-edge-read",
            &input,
            &list(row.clone(), rows),
            scans,
            Cost::default(),
        );
    }
}

#[test]
fn the_closure_read_template_equals_the_lowerer_charge_for_its_declared_selector() {
    for (nodes, atom, edges, occurrences, edge_types, scans) in [
        (1, 1, 1, 1, 1, 0),
        (8, 64, 16, 16, 2, 16),
        (4096, 4096, 65_536, 65_536, 16, 1024),
    ] {
        let (input, output) = closure_types(nodes, atom, edges, occurrences, edge_types);
        for selector in [
            Cost::default(),
            Cost {
                steps: 37,
                bytes: 4195,
            },
        ] {
            agrees("closure-read", &input, &output, scans, selector);
        }
    }
}

/// Under v2 the declared selector is charged, not the costliest helper; a
/// costlier unselected helper changes the copying charge, never the template.
#[test]
fn a_costlier_unselected_helper_does_not_reach_the_closure_template() {
    let (input, output) = closure_types(8, 64, 16, 16, 2);
    let selected = BTreeMap::from([("app.select".to_owned(), fact(10, 640))]);
    let mut all = selected.clone();
    all.insert("app.costly".to_owned(), fact(1000, 64_000));
    let v2 = graph_charge("closure-read", &input, &output, 16, &selected).unwrap();
    let v1 = graph_charge("closure-read", &input, &output, 16, &all).unwrap();
    // 16 edge rows each pay the helper once.
    assert_eq!(v1.steps - v2.steps, 16 * 990);
    assert_eq!(v1.bytes - v2.bytes, 16 * 63_360);
    agrees(
        "closure-read",
        &input,
        &output,
        16,
        Cost {
            steps: 10,
            bytes: 640,
        },
    );
}

/// amblicit#345 (6e217bb9) charges one empty obstruction payload of (1, 64)
/// outside the template when a step maps any arm; the lowerer's
/// `effect_call` charges `shape(record {})`'s copy for the same payload.
/// Templates therefore carry no obstruction term.
#[test]
fn the_obstruction_payload_is_amblicits_constant_and_no_template_carries_it() {
    let (copy, _, _, _) = shape(&unit()).unwrap();
    assert_eq!((copy.steps, copy.bytes), (1, 64));
    for operation in [
        "node-atom-read",
        "ensure-node",
        "create-node",
        "replace-atom",
        "ensure-edge",
        "replace-edge",
        "outgoing-edge-read",
        "closure-read",
    ] {
        let rule = encode(&effect_cost_rule(operation).unwrap()).unwrap();
        let rule = String::from_utf8_lossy(&rule);
        assert!(!rule.contains("obstruction"), "{operation}");
    }
}

#[test]
fn the_packaged_catalog_is_v3_with_a_template_on_every_effect() {
    let catalog = decode(CATALOG).unwrap();
    assert_eq!(field(&catalog, "apiVersion"), Some(&text(CATALOG_API)));
    let Some(Value::Map(intrinsics)) = field(&catalog, "intrinsics") else {
        panic!("intrinsics")
    };
    let effects = intrinsics
        .iter()
        .filter(|(_, entry)| field(entry, "intrinsicClass") == Some(&text("effect")))
        .inspect(|(_, entry)| assert!(field(entry, "effectCostRule").is_some()))
        .count();
    assert_eq!(effects, 8);
    assert!(field(&catalog, "parameters").is_some());
}
