// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Exact signatures of the typed outgoing-edge read and the guided closure read.
//!
//! Both are generic graph observations. Nothing here names an application
//! structure: the guide is whatever type the authored selector shares with the
//! closure it guides, and its bytes are never interpreted.

use super::types::{ReadType, Scope};
use super::{array, field, invalid, map_field, text, ProviderRefusalV1, Value};

pub(super) const EDGE_READ: &str = "echo.dpo@1.outgoing-edge-read";
pub(super) const CLOSURE_READ: &str = "echo.dpo@1.closure-read";

/// Each edge-read failure and the authority class its authored arm must carry.
pub(super) const EDGE_FAILURES: [(&str, &str); 3] = [
    ("echo.edge-source-missing/v1", "resourceFault"),
    ("echo.edge-bound-exceeded/v1", "resourceFault"),
    ("echo.edge-scan-bound-exceeded/v1", "resourceFault"),
];

/// Each closure failure and the authority class its authored arm must carry.
///
/// Resource limits, a selector that asked for too little and a store that
/// breaks the declared invariants stay three distinguishable classes.
pub(super) const CLOSURE_FAILURES: [(&str, &str); 7] = [
    ("echo.closure-start-missing/v1", "resourceFault"),
    ("echo.closure-node-bound-exceeded/v1", "resourceFault"),
    ("echo.closure-depth-bound-exceeded/v1", "resourceFault"),
    ("echo.closure-scan-bound-exceeded/v1", "resourceFault"),
    ("echo.closure-work-bound-exceeded/v1", "resourceFault"),
    ("echo.closure-selector-insufficient/v1", "domainMappable"),
    ("echo.closure-storage-malformed/v1", "integrityFault"),
];

const MAX_EDGE_TYPES: u64 = 16;
const MAX_RESULT_EDGES: u64 = 65_536;
const MAX_RESULT_NODES: u64 = 4096;

const ID: ReadType = ReadType::Bytes(32, 32);
const WORD: ReadType = ReadType::Unsigned(u64::MAX);

fn record<'a, const N: usize>(
    ty: &'a ReadType,
    names: [&str; N],
) -> Result<[&'a ReadType; N], ProviderRefusalV1> {
    let ReadType::Record(declared) = ty.representation() else {
        return Err(invalid());
    };
    if declared.len() != N {
        return Err(invalid());
    }
    let mut found = [ty; N];
    for (slot, name) in found.iter_mut().zip(names) {
        *slot = declared.get(name).ok_or_else(invalid)?;
    }
    Ok(found)
}

fn list(ty: &ReadType, ceiling: u64) -> Result<(&ReadType, u64), ProviderRefusalV1> {
    let ReadType::List(Some(item), max) = ty.representation() else {
        return Err(invalid());
    };
    if !(1..=ceiling).contains(max) {
        return Err(invalid());
    }
    Ok((item, *max))
}

fn exactly(ty: &ReadType, expected: &ReadType) -> Result<(), ProviderRefusalV1> {
    if ty.representation() == expected {
        Ok(())
    } else {
        Err(invalid())
    }
}

/// An edge read takes `{ warpId, nodeId, edgeType }` and returns a bounded
/// list of `{ edgeId, toId }`.
pub(super) fn edge_read(input: &ReadType, output: &ReadType) -> Result<(), ProviderRefusalV1> {
    for id in record(input, ["warpId", "nodeId", "edgeType"])? {
        exactly(id, &ID)?;
    }
    let (row, _) = list(output, MAX_RESULT_EDGES)?;
    for id in record(row, ["edgeId", "toId"])? {
        exactly(id, &ID)?;
    }
    Ok(())
}

/// The node-row and guide types one closure read shares with its selector.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct ClosureTypes<'a> {
    pub node: &'a ReadType,
    pub guide: &'a ReadType,
    /// Worst-case read attempts: every node it may materialize, and one typed
    /// range per declared edge type for every occurrence it may record.
    pub attempts: u64,
}

/// A closure read takes `{ warpId, nodeId, edgeTypes, maxNodes, maxDepth,
/// guide }` and returns `{ nodes, edges, occurrences }` with fixed row shapes.
/// Occurrences carry exactly the guide type the read was started with.
pub(super) fn closure_read<'a>(
    input: &'a ReadType,
    output: &'a ReadType,
    max_read_bytes: u64,
) -> Result<ClosureTypes<'a>, ProviderRefusalV1> {
    let [warp, start, edge_types, max_nodes, max_depth, guide] = record(
        input,
        [
            "warpId",
            "nodeId",
            "edgeTypes",
            "maxNodes",
            "maxDepth",
            "guide",
        ],
    )?;
    exactly(warp, &ID)?;
    exactly(start, &ID)?;
    exactly(max_nodes, &WORD)?;
    exactly(max_depth, &WORD)?;
    let (edge_type, edge_type_count) = list(edge_types, MAX_EDGE_TYPES)?;
    exactly(edge_type, &ID)?;
    let [nodes, edges, occurrences] = record(output, ["nodes", "edges", "occurrences"])?;
    let (node, node_count) = list(nodes, MAX_RESULT_NODES)?;
    let [id, node_type, atom_type, atom] =
        record(node, ["nodeId", "nodeType", "atomType", "atomBytes"])?;
    exactly(id, &ID)?;
    exactly(node_type, &ID)?;
    exactly(atom_type, &ReadType::Bytes(0, 32))?;
    if !matches!(atom.representation(), ReadType::Bytes(0, max) if *max > 0 && *max <= max_read_bytes)
    {
        return Err(invalid());
    }
    let (edge, _) = list(edges, MAX_RESULT_EDGES)?;
    let [from, slot, edge_id, target] = record(edge, ["from", "slot", "edgeId", "node"])?;
    exactly(edge_id, &ID)?;
    let (occurrence, occurrence_count) = list(occurrences, MAX_RESULT_EDGES)?;
    let [index, via, depth, carried] = record(occurrence, ["node", "edge", "depth", "guide"])?;
    for word in [from, slot, target, index, via, depth] {
        exactly(word, &WORD)?;
    }
    if carried != guide {
        return Err(invalid());
    }
    let attempts = occurrence_count
        .checked_mul(edge_type_count)
        .and_then(|ranges| ranges.checked_add(node_count))
        .ok_or_else(invalid)?;
    Ok(ClosureTypes {
        node,
        guide,
        attempts,
    })
}

/// The selector is one source helper `{ guide, depth, slot, parent, child }
/// -> { expand, guide, wellFormed, sufficient }` over the closure's own node
/// and guide types.
pub(super) fn selector_conforms(
    candidate: &ReadType,
    decision: &ReadType,
    closure: &ClosureTypes<'_>,
) -> Result<(), ProviderRefusalV1> {
    let [guide, depth, slot, parent, child] =
        record(candidate, ["guide", "depth", "slot", "parent", "child"])?;
    let [expand, next, well_formed, sufficient] =
        record(decision, ["expand", "guide", "wellFormed", "sufficient"])?;
    exactly(depth, &WORD)?;
    exactly(slot, &WORD)?;
    for flag in [expand, well_formed, sufficient] {
        exactly(flag, &ReadType::Bool)?;
    }
    if guide != closure.guide
        || next != closure.guide
        || parent != closure.node
        || child != closure.node
    {
        return Err(invalid());
    }
    Ok(())
}

/// Resolves the declared selector among the module's own source helpers.
pub(super) fn selector(
    core: &Value,
    scope: &Scope<'_>,
    name: Option<&str>,
    closure: &ClosureTypes<'_>,
) -> Result<(), ProviderRefusalV1> {
    let function = map_field(core, "functions")
        .zip(name)
        .and_then(|(functions, name)| map_field(functions, name))
        .ok_or_else(invalid)?;
    let [parameter] = array(function, "params")? else {
        return Err(invalid());
    };
    let candidate = scope.ty(text(parameter, "type")?)?;
    let decision = scope.ty(text(function, "returnType")?)?;
    field(function, "body")?;
    selector_conforms(&candidate, &decision, closure)
}

#[cfg(test)]
mod tests;
