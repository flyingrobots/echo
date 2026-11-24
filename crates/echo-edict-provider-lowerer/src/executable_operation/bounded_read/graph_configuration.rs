// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The optional graph-read declaration of a bounded-read lowering configuration.
//!
//! This module owns the declared scan, node and depth ceilings and the name of
//! the closure selector. A configuration without the declaration is the
//! node-atom-only configuration and keeps its four exact fields.

use super::{
    invalid, map_field, require_exact_fields, required_u64, text, ProviderRefusalV1, Value, SUBJECT,
};

pub(super) const MAX_EDGE_SCANS: u64 = 65_536;
pub(super) const MAX_CLOSURE_NODES: u64 = 4096;
pub(super) const MAX_CLOSURE_DEPTH: u64 = 256;

/// Footprint contract of a package that declares no graph read.
pub(super) const NODE_FOOTPRINT: &str = "echo.operation.footprint.explicit-node-read-aperture/v1";
/// Footprint contract of a package whose reads discover graph material.
pub(super) const GRAPH_FOOTPRINT: &str = "echo.operation.footprint.typed-graph-read-aperture/v1";

/// Declared graph-read ceilings and the bound closure selector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct GraphReads<'a> {
    /// Bucket edges one evaluation may inspect, matching or not.
    pub max_edge_scans: u64,
    /// Distinct nodes one closure read may materialize.
    pub max_closure_nodes: u64,
    /// Deepest occurrence one closure read may select.
    pub max_closure_depth: u64,
    /// Module-local name of the source helper deciding each candidate child.
    pub selector: Option<&'a str>,
}

/// Checks the configuration's exact fields and returns its graph declaration.
pub(super) fn parse(configuration: &Value) -> Result<Option<GraphReads<'_>>, ProviderRefusalV1> {
    let base = ["apiVersion", "programKind", "maxReads", "maxReadBytes"];
    let Some(declared) = map_field(configuration, "graphReads") else {
        require_exact_fields(configuration, &base, SUBJECT)?;
        return Ok(None);
    };
    let mut fields = base.to_vec();
    fields.push("graphReads");
    require_exact_fields(configuration, &fields, SUBJECT)?;
    let mut names = vec!["maxEdgeScans", "maxClosureNodes", "maxClosureDepth"];
    let selector = if map_field(declared, "closureSelector").is_some() {
        names.push("closureSelector");
        Some(text(declared, "closureSelector")?)
    } else {
        None
    };
    require_exact_fields(declared, &names, SUBJECT)?;
    let graph = GraphReads {
        max_edge_scans: required_u64(declared, "maxEdgeScans", SUBJECT)?,
        max_closure_nodes: required_u64(declared, "maxClosureNodes", SUBJECT)?,
        max_closure_depth: required_u64(declared, "maxClosureDepth", SUBJECT)?,
        selector,
    };
    if !(1..=MAX_EDGE_SCANS).contains(&graph.max_edge_scans)
        || graph.max_closure_nodes > MAX_CLOSURE_NODES
        || graph.max_closure_depth > MAX_CLOSURE_DEPTH
    {
        return Err(invalid());
    }
    Ok(Some(graph))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(fields: &[(&str, Value)]) -> Value {
        Value::Map(
            fields
                .iter()
                .map(|(key, value)| (Value::Text((*key).to_owned()), value.clone()))
                .collect(),
        )
    }

    fn configuration(graph: Option<Value>) -> Value {
        let mut fields = vec![
            (
                "apiVersion",
                Value::Text("echo.operation-lowering-configuration/v1".to_owned()),
            ),
            (
                "programKind",
                Value::Text("compiler-produced-bounded-read/v1".to_owned()),
            ),
            ("maxReads", Value::Integer(4)),
            ("maxReadBytes", Value::Integer(64)),
        ];
        if let Some(graph) = graph {
            fields.push(("graphReads", graph));
        }
        map(&fields)
    }

    fn declared(scans: i128, nodes: i128, depth: i128) -> Vec<(&'static str, Value)> {
        vec![
            ("maxEdgeScans", Value::Integer(scans)),
            ("maxClosureNodes", Value::Integer(nodes)),
            ("maxClosureDepth", Value::Integer(depth)),
        ]
    }

    #[test]
    fn a_configuration_without_the_declaration_declares_no_graph_read() {
        assert_eq!(parse(&configuration(None)), Ok(None));
    }

    #[test]
    fn the_declaration_carries_its_ceilings_and_selector_name() {
        let mut fields = declared(64, 16, 8);
        fields.push(("closureSelector", Value::Text("select".to_owned())));
        let value = configuration(Some(map(&fields)));
        assert_eq!(
            parse(&value),
            Ok(Some(GraphReads {
                max_edge_scans: 64,
                max_closure_nodes: 16,
                max_closure_depth: 8,
                selector: Some("select"),
            }))
        );
    }

    #[test]
    fn each_ceiling_is_admitted_at_its_bound_and_refused_one_above() {
        for (scans, nodes, depth, admitted) in [
            (65_536, 4096, 256, true),
            (65_537, 4096, 256, false),
            (65_536, 4097, 256, false),
            (65_536, 4096, 257, false),
            (0, 0, 0, false),
            (1, 0, 0, true),
        ] {
            let value = configuration(Some(map(&declared(scans, nodes, depth))));
            assert_eq!(parse(&value).is_ok(), admitted, "{scans} {nodes} {depth}");
        }
    }

    #[test]
    fn unknown_missing_and_empty_declaration_fields_are_refused() {
        let mut extra = declared(1, 1, 1);
        extra.push(("maxEdges", Value::Integer(1)));
        let mut empty = declared(1, 1, 1);
        empty.push(("closureSelector", Value::Text(String::new())));
        let missing = declared(1, 1, 1).into_iter().skip(1).collect::<Vec<_>>();
        for fields in [extra, empty, missing] {
            assert!(parse(&configuration(Some(map(&fields)))).is_err());
        }
        let mut outer = configuration(Some(map(&declared(1, 1, 1))));
        if let Value::Map(entries) = &mut outer {
            entries.push((Value::Text("maxEdgeScans".to_owned()), Value::Integer(1)));
        }
        assert!(parse(&outer).is_err());
    }
}
