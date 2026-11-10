// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Verifier-owned judgments for typed outgoing-edge and guided closure reads.
//!
//! This module owns the graph-read declaration of a lowering configuration and
//! the exact signatures the two graph intrinsics may have. It is written
//! independently of the lowerer: each expected shape is a table of field
//! demands matched against the schema Core resolves.

use super::super::{exact, get, number, text, Check, Value};
use super::types::{Schema, Symbols};

pub(super) const EDGE_INTRINSIC: &str = "echo.dpo@1.outgoing-edge-read";
pub(super) const CLOSURE_INTRINSIC: &str = "echo.dpo@1.closure-read";

/// Runtime failures of an edge read, each with its required authority class.
pub(super) const EDGE_OUTCOMES: [(&str, &str); 3] = [
    ("echo.edge-bound-exceeded/v1", "resourceFault"),
    ("echo.edge-scan-bound-exceeded/v1", "resourceFault"),
    ("echo.edge-source-missing/v1", "resourceFault"),
];

/// Runtime failures of a closure read, each with its required authority class.
/// A resource limit, an insufficient selector and malformed storage differ.
pub(super) const CLOSURE_OUTCOMES: [(&str, &str); 7] = [
    ("echo.closure-depth-bound-exceeded/v1", "resourceFault"),
    ("echo.closure-node-bound-exceeded/v1", "resourceFault"),
    ("echo.closure-scan-bound-exceeded/v1", "resourceFault"),
    ("echo.closure-selector-insufficient/v1", "domainMappable"),
    ("echo.closure-start-missing/v1", "resourceFault"),
    ("echo.closure-storage-malformed/v1", "integrityFault"),
    ("echo.closure-work-bound-exceeded/v1", "resourceFault"),
];

const EDGE_SCAN_CEILING: u64 = 65_536;
const CLOSURE_NODE_CEILING: u64 = 4096;
const CLOSURE_DEPTH_CEILING: u64 = 256;
const EDGE_TYPE_CEILING: u64 = 16;
const ROW_CEILING: u64 = 65_536;

/// The graph capability one configuration declares.
pub(super) struct Declaration {
    /// Module-local selector helper, present when closures may be read.
    pub selector: Option<String>,
}

/// Reads the configuration's exact fields; `None` is the node-only form.
pub(super) fn declaration(configuration: &Value) -> Check<Option<Declaration>> {
    let Ok(graph) = get(configuration, "graphReads") else {
        exact(
            configuration,
            &["apiVersion", "programKind", "maxReads", "maxReadBytes"],
        )?;
        return Ok(None);
    };
    exact(
        configuration,
        &[
            "apiVersion",
            "programKind",
            "maxReads",
            "maxReadBytes",
            "graphReads",
        ],
    )?;
    let selector = if get(graph, "closureSelector").is_ok() {
        exact(
            graph,
            &[
                "maxEdgeScans",
                "maxClosureNodes",
                "maxClosureDepth",
                "closureSelector",
            ],
        )?;
        Some(text(graph, "closureSelector")?.to_owned())
    } else {
        exact(
            graph,
            &["maxEdgeScans", "maxClosureNodes", "maxClosureDepth"],
        )?;
        None
    };
    let scans = number(get(graph, "maxEdgeScans")?)?;
    if scans == 0
        || scans > EDGE_SCAN_CEILING
        || number(get(graph, "maxClosureNodes")?)? > CLOSURE_NODE_CEILING
        || number(get(graph, "maxClosureDepth")?)? > CLOSURE_DEPTH_CEILING
    {
        return Err(());
    }
    Ok(Some(Declaration { selector }))
}

/// What one field of an expected row must be.
enum Demand<'a> {
    /// A 32-byte identifier, possibly behind a nominal name.
    Identifier,
    /// The full unsigned word.
    Word,
    /// A Boolean.
    Flag,
    /// Zero to 32 bytes: an atom type, empty when the node has no atom.
    AtomType,
    /// Bounded atom bytes no larger than the configured read-byte ceiling.
    Atom(u64),
    /// Exactly this already-resolved schema, nominal identity included.
    Same(&'a Schema),
    /// Any schema; the caller inspects it.
    Open,
}

fn identifier() -> Schema {
    Schema::Bytes {
        lower: 32,
        upper: 32,
    }
}

fn meets(schema: &Schema, demand: &Demand<'_>) -> bool {
    let plain = schema.representation();
    match demand {
        Demand::Identifier => plain == &identifier(),
        Demand::Word => plain == &Schema::Word(u64::MAX),
        Demand::Flag => plain == &Schema::Boolean,
        Demand::AtomType => {
            plain
                == &Schema::Bytes {
                    lower: 0,
                    upper: 32,
                }
        }
        Demand::Atom(ceiling) => {
            matches!(plain, Schema::Bytes { lower: 0, upper } if *upper > 0 && upper <= ceiling)
        }
        Demand::Same(expected) => &schema == expected,
        Demand::Open => true,
    }
}

/// Matches a row against its complete demand table and returns the named field.
fn row<'s>(schema: &'s Schema, demands: &[(&str, Demand<'_>)]) -> Check<&'s Schema> {
    let Schema::Row(fields) = schema.representation() else {
        return Err(());
    };
    if fields.len() != demands.len() {
        return Err(());
    }
    for (name, demand) in demands {
        if !meets(fields.get(*name).ok_or(())?, demand) {
            return Err(());
        }
    }
    Ok(schema)
}

fn member<'s>(schema: &'s Schema, name: &str) -> Check<&'s Schema> {
    match schema.representation() {
        Schema::Row(fields) => fields.get(name).ok_or(()),
        _ => Err(()),
    }
}

fn elements(schema: &Schema, ceiling: u64) -> Check<&Schema> {
    match schema.representation() {
        Schema::Sequence {
            element: Some(element),
            maximum,
        } if *maximum >= 1 && *maximum <= ceiling => Ok(element),
        _ => Err(()),
    }
}

/// `{ warpId, nodeId, edgeType } -> List<{ edgeId, toId }>`.
pub(super) fn edge_signature(input: &Schema, output: &Schema) -> Check<()> {
    row(
        input,
        &[
            ("edgeType", Demand::Identifier),
            ("nodeId", Demand::Identifier),
            ("warpId", Demand::Identifier),
        ],
    )?;
    row(
        elements(output, ROW_CEILING)?,
        &[("edgeId", Demand::Identifier), ("toId", Demand::Identifier)],
    )?;
    Ok(())
}

/// The two schemas a closure shares with the selector that guides it.
pub(super) struct Guided {
    pub node: Schema,
    pub guide: Schema,
}

/// `{ warpId, nodeId, edgeTypes, maxNodes, maxDepth, guide } ->
/// { nodes, edges, occurrences }`, where every occurrence carries the guide
/// type the read started with.
pub(super) fn closure_signature(
    input: &Schema,
    output: &Schema,
    atom_ceiling: u64,
) -> Check<Guided> {
    row(
        input,
        &[
            ("edgeTypes", Demand::Open),
            ("guide", Demand::Open),
            ("maxDepth", Demand::Word),
            ("maxNodes", Demand::Word),
            ("nodeId", Demand::Identifier),
            ("warpId", Demand::Identifier),
        ],
    )?;
    if !meets(
        elements(member(input, "edgeTypes")?, EDGE_TYPE_CEILING)?,
        &Demand::Identifier,
    ) {
        return Err(());
    }
    let guide = member(input, "guide")?;
    row(
        output,
        &[
            ("edges", Demand::Open),
            ("nodes", Demand::Open),
            ("occurrences", Demand::Open),
        ],
    )?;
    let node = row(
        elements(member(output, "nodes")?, CLOSURE_NODE_CEILING)?,
        &[
            ("atomBytes", Demand::Atom(atom_ceiling)),
            ("atomType", Demand::AtomType),
            ("nodeId", Demand::Identifier),
            ("nodeType", Demand::Identifier),
        ],
    )?;
    row(
        elements(member(output, "edges")?, ROW_CEILING)?,
        &[
            ("edgeId", Demand::Identifier),
            ("from", Demand::Word),
            ("node", Demand::Word),
            ("slot", Demand::Word),
        ],
    )?;
    row(
        elements(member(output, "occurrences")?, ROW_CEILING)?,
        &[
            ("depth", Demand::Word),
            ("edge", Demand::Word),
            ("guide", Demand::Same(guide)),
            ("node", Demand::Word),
        ],
    )?;
    Ok(Guided {
        node: node.clone(),
        guide: guide.clone(),
    })
}

/// `{ guide, depth, slot, parent, child } -> { expand, guide, wellFormed,
/// sufficient }` over the closure's own node and guide schemas.
pub(super) fn selector_signature(
    candidate: &Schema,
    decision: &Schema,
    guided: &Guided,
) -> Check<()> {
    row(
        candidate,
        &[
            ("child", Demand::Same(&guided.node)),
            ("depth", Demand::Word),
            ("guide", Demand::Same(&guided.guide)),
            ("parent", Demand::Same(&guided.node)),
            ("slot", Demand::Word),
        ],
    )?;
    row(
        decision,
        &[
            ("expand", Demand::Flag),
            ("guide", Demand::Same(&guided.guide)),
            ("sufficient", Demand::Flag),
            ("wellFormed", Demand::Flag),
        ],
    )?;
    Ok(())
}

/// Finds the declared selector among the module's source helpers and checks
/// that it decides over this closure's types.
pub(super) fn selector(
    core: &Value,
    symbols: &mut Symbols<'_>,
    declared: Option<&Declaration>,
    guided: &Guided,
) -> Check<()> {
    let name = declared
        .and_then(|declared| declared.selector.as_deref())
        .ok_or(())?;
    let helper = get(get(core, "functions")?, name)?;
    let [parameter] = super::sequence(helper, "params")? else {
        return Err(());
    };
    get(helper, "body")?;
    let candidate = symbols.schema(text(parameter, "type")?, 0)?;
    let decision = symbols.schema(text(helper, "returnType")?, 0)?;
    selector_signature(&candidate, &decision, guided)
}

#[cfg(test)]
mod tests;
