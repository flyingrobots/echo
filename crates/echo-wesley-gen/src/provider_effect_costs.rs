// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Echo's `edict.target-profile.intrinsics/v3` effect cost templates
//! (amblicit#341, grammar of amblicit#345), the only source of the
//! `effectCostRule` and `parameters` entries of the `echo.dpo@2` catalog.
//!
//! Under the shared-read schedule a template replaces the compiler's generic
//! charge of an effect step, so each template states exactly what Echo's
//! lowerer charges for that step beyond the step's input expression: the
//! input's validations plus the intrinsic's own work. The lowerer's
//! `cost_templates` conformance tests evaluate the generated catalog against
//! the lowerer's own per-effect charge. The one empty obstruction payload,
//! (1 step, 64 bytes) when a step maps any obstruction arm, is charged by both
//! sides outside the template.

use serde_json::{json, Value as JsonValue};

/// API version of the catalog that carries these templates.
pub const EFFECT_COST_CATALOG_API: &str = "edict.target-profile.intrinsics/v3";

fn c(value: u64) -> JsonValue {
    json!({ "const": value })
}
fn sum<const N: usize>(terms: [JsonValue; N]) -> JsonValue {
    json!({ "sum": terms.to_vec() })
}
fn product(left: JsonValue, right: JsonValue) -> JsonValue {
    json!({ "product": [left, right] })
}
fn measure(inner: JsonValue) -> JsonValue {
    json!({ "measure": inner })
}
/// `{ path: { side, fields, item, ? then } }`.
fn path(side: &str, fields: &[&str], item: u64, then: &[&str]) -> JsonValue {
    let mut entries = json!({ "side": side, "fields": fields, "item": item });
    if !then.is_empty() {
        entries["then"] = json!(then);
    }
    json!({ "path": entries })
}
fn input(fields: &[&str]) -> JsonValue {
    path("input", fields, 0, &[])
}
fn output(fields: &[&str]) -> JsonValue {
    path("output", fields, 0, &[])
}
fn item(list: &str, then: &[&str]) -> JsonValue {
    path("output", &[list], 1, then)
}
fn bytes_max(ty: JsonValue) -> JsonValue {
    measure(json!({ "bytesMax": ty }))
}
fn list_max(ty: JsonValue) -> JsonValue {
    measure(json!({ "listMax": ty }))
}
fn validations(ty: JsonValue) -> JsonValue {
    measure(json!({ "validations": ty }))
}
fn copy(ty: JsonValue, half: &str) -> JsonValue {
    measure(json!({ "copy": ty, "of": half }))
}
fn parameter(name: &str) -> JsonValue {
    measure(json!({ "parameter": name }))
}
fn helper(half: &str) -> JsonValue {
    measure(json!({ "helper": { "parameter": "selector" }, "of": half }))
}
fn rule(steps: JsonValue, bytes: JsonValue) -> JsonValue {
    json!({ "steps": steps, "bytes": bytes })
}

/// The catalog's top-level `parameters` map: where each template parameter
/// lives in the operation configuration.
#[must_use]
pub fn effect_cost_parameters() -> JsonValue {
    json!({
        "maxEdgeScans": { "path": ["graphReads", "maxEdgeScans"], "kind": "uint" },
        "selector": { "path": ["graphReads", "closureSelector"], "kind": "helper" },
    })
}

/// The `effectCostRule` of one `echo.dpo@2` effect operation, or `None` for
/// an operation Echo states no template for.
#[must_use]
pub fn effect_cost_rule(operation: &str) -> Option<JsonValue> {
    let checked = || validations(input(&[]));
    Some(match operation {
        // `abi`: max + 1 steps; the payload twice beside the read records.
        "node-atom-read" => rule(
            sum([checked(), bytes_max(output(&[])), c(1)]),
            sum([product(c(2), bytes_max(output(&[]))), c(448)]),
        ),
        // `abi`: one step and the unit value beside the emission reservation.
        "ensure-node" | "create-node" | "replace-atom" | "ensure-edge" | "replace-edge" => {
            rule(sum([checked(), c(1)]), c(64 + 280))
        }
        // `graph_charge`: every scan (128 bytes, recorded twice), the whole
        // result copied and validated, and one recorded range.
        "outgoing-edge-read" => rule(
            sum([
                checked(),
                parameter("maxEdgeScans"),
                copy(output(&[]), "steps"),
                validations(output(&[])),
                c(1),
            ]),
            sum([
                product(parameter("maxEdgeScans"), c(256)),
                copy(output(&[]), "bytes"),
                c(384),
            ]),
        ),
        "closure-read" => closure_rule(),
        _ => return None,
    })
}

/// `graph_charge` of a closure read with only the declared selector:
/// `read_cost`'s closure terms plus the recorded nodes and ranges.
fn closure_rule() -> JsonValue {
    let guide = || input(&["guide"]);
    let node = || item("nodes", &[]);
    let atom = || bytes_max(item("nodes", &["atomBytes"]));
    let nodes = || list_max(output(&["nodes"]));
    let edges = || list_max(output(&["edges"]));
    let occurrences = || list_max(output(&["occurrences"]));
    let ranges = || product(occurrences(), list_max(input(&["edgeTypes"])));
    let candidate = || {
        json!({ "record": {
            "guide": guide(),
            "depth": { "word": null },
            "slot": { "word": null },
            "parent": node(),
            "child": node(),
        }})
    };
    let steps = sum([
        validations(input(&[])),
        validations(output(&[])),
        parameter("maxEdgeScans"),
        validations(guide()),
        copy(guide(), "steps"),
        // Each node: its atom, its row copy, and two recorded steps.
        product(nodes(), sum([atom(), copy(node(), "steps"), c(2)])),
        product(
            occurrences(),
            sum([
                c(1),
                copy(guide(), "steps"),
                copy(item("occurrences", &[]), "steps"),
            ]),
        ),
        product(
            edges(),
            sum([
                copy(item("edges", &[]), "steps"),
                copy(guide(), "steps"),
                copy(node(), "steps"),
                copy(node(), "steps"),
                copy(candidate(), "steps"),
                validations(candidate()),
                c(2),
                helper("steps"),
                copy(guide(), "steps"),
            ]),
        ),
        ranges(),
    ]);
    let bytes = sum([
        product(parameter("maxEdgeScans"), c(256)),
        copy(guide(), "bytes"),
        // Each node: its atom charged, then copied in the row, beside the
        // row copy and the node's recorded observations.
        product(
            nodes(),
            sum([atom(), c(64), copy(node(), "bytes"), atom(), c(384)]),
        ),
        product(
            occurrences(),
            sum([
                copy(guide(), "bytes"),
                copy(item("occurrences", &[]), "bytes"),
            ]),
        ),
        product(
            edges(),
            sum([
                copy(item("edges", &[]), "bytes"),
                copy(guide(), "bytes"),
                copy(node(), "bytes"),
                copy(node(), "bytes"),
                copy(candidate(), "bytes"),
                helper("bytes"),
                copy(guide(), "bytes"),
            ]),
        ),
        c(256),
        product(ranges(), c(384)),
    ]);
    rule(steps, bytes)
}
