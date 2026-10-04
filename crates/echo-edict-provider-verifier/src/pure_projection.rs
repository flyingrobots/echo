// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Independently relate pure result projections to the authored result.

use echo_edict_canonical::CanonicalValueV1 as Value;

use crate::executable_operation::{
    MAX_RESULT_PROJECTION_NODES, MAX_RESULT_PROJECTION_PATH_SEGMENTS,
    MAX_RESULT_PROJECTION_TEXT_BYTES,
};
use crate::{array_field, as_map, as_text, map_field, text_field};

pub(super) fn preserves(
    core: &Value,
    target: &Value,
    intent_name: &str,
    projection: &Value,
) -> bool {
    let Some(body) = map_field(core, "body") else {
        return false;
    };
    let Some(target) = map_field(target, "intents").and_then(|v| map_field(v, intent_name)) else {
        return false;
    };
    let Some(budget) = map_field(core, "coreEvaluationBudget") else {
        return false;
    };
    if text_field(core, "output").is_none()
        || text_field(projection, "outputType") != text_field(core, "output")
        || map_field(budget, "maxOutputBytes").is_none()
        || map_field(projection, "maxOutputBytes") != map_field(budget, "maxOutputBytes")
    {
        return false;
    }
    let Some(locals) = array_field(body, "locals") else {
        return false;
    };
    let mut inputs = locals
        .iter()
        .filter(|local| text_field(local, "id") == Some("arg.0"));
    let Some(input) = inputs.next() else {
        return false;
    };
    if inputs.next().is_some() || text_field(input, "type") != text_field(core, "input") {
        return false;
    }
    let (Some(expression), Some(result)) = (
        map_field(projection, "expression"),
        map_field(body, "result"),
    ) else {
        return false;
    };
    let mut remaining = MAX_RESULT_PROJECTION_NODES;
    matches_result(expression, result, input, target, &mut remaining)
}

fn matches_result(
    projection: &Value,
    result: &Value,
    input: &Value,
    target: &Value,
    remaining: &mut usize,
) -> bool {
    let Some(next) = remaining.checked_sub(1) else {
        return false;
    };
    *remaining = next;
    match text_field(projection, "kind") {
        Some("record") => {
            let (Some(fields), Some(result_fields)) = (
                map_field(projection, "fields").and_then(as_map),
                map_field(result, "fields").and_then(as_map),
            ) else {
                return false;
            };
            text_field(result, "kind") == Some("record")
                && fields.len() == result_fields.len()
                && fields.iter().all(|(key, value)| {
                    as_text(key).is_some_and(|name| {
                        name.len() <= MAX_RESULT_PROJECTION_TEXT_BYTES
                            && result_fields.iter().find(|(k, _)| k == key).is_some_and(
                                |(_, expected)| {
                                    matches_result(value, expected, input, target, remaining)
                                },
                            )
                    })
                })
        }
        Some("source") => matches_source(projection, result, input, target),
        _ => false,
    }
}

fn matches_source(projection: &Value, result: &Value, input: &Value, target: &Value) -> bool {
    let (Some(path), Some(source)) = (
        array_field(projection, "path"),
        map_field(projection, "source"),
    ) else {
        return false;
    };
    if path.len() > MAX_RESULT_PROJECTION_PATH_SEGMENTS {
        return false;
    }
    let mut root = result;
    for segment in path.iter().rev() {
        let Some(name) = as_text(segment) else {
            return false;
        };
        if name.len() > MAX_RESULT_PROJECTION_TEXT_BYTES
            || text_field(root, "kind") != Some("field")
            || text_field(root, "field") != Some(name)
        {
            return false;
        }
        let Some(base) = map_field(root, "base") else {
            return false;
        };
        root = base;
    }
    if text_field(root, "kind") != Some("local") {
        return false;
    }
    match text_field(source, "kind") {
        Some("applicationInput") => map_field(root, "ref") == Some(input),
        Some("pureBinding") => {
            let Some(id) = text_field(source, "bindingId") else {
                return false;
            };
            array_field(target, "pureBindings").is_some_and(|bindings| {
                bindings.iter().any(|binding| {
                    text_field(binding, "id") == Some(id)
                        && map_field(binding, "binding")
                            .is_some_and(|local| map_field(root, "ref") == Some(local))
                })
            })
        }
        _ => false,
    }
}
