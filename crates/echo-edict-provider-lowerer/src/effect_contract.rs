// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! This module owns the provider's judgment of an authored effect package's
//! graph schema and footprint law: the shapes and effect-profile ceilings
//! Echo's runtime applies to them when it admits the package (echo#1023).
//!
//! `docs/architecture/authored-operation-effects.md` fixes both shapes and
//! the law's ceilings: at most 4,096 reads, 256 writes and 65,536 edges per
//! range read. A provider that accepted a law or schema the runtime refuses
//! would publish a package no runtime admits, so the provider applies the same
//! checks and names the failing one with the same stable reason code the
//! runtime's admission refusal carries. This file is kept byte-identical in
//! the lowerer and the verifier.
use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, encode_canonical_cbor_v1 as encode,
    CanonicalValueV1 as Value,
};

/// The most bytes a graph schema or footprint law may occupy (16 MiB).
const STORAGE_CEILING: usize = 16 * 1024 * 1024;
/// The effect-profile ceiling on `max_reads`.
pub const MAX_READS: u64 = 4096;
/// The effect-profile ceiling on `max_writes`.
pub const MAX_WRITES: u64 = 256;
/// The effect-profile ceiling on `max_range_edges`.
pub const MAX_RANGE_EDGES: u64 = 65_536;

/// The graph-schema or footprint-law check that refused, as a stable code.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ContractRefusal {
    /// The graph schema is not one canonical value of the exact schema shape.
    GraphSchema,
    /// The footprint law is not one canonical value of the exact law shape.
    FootprintLaw,
    /// The footprint law's `max_reads` is above 4,096.
    FootprintMaxReads,
    /// The footprint law's `max_writes` is above 256.
    FootprintMaxWrites,
    /// The footprint law's `max_range_edges` is above 65,536.
    FootprintMaxRangeEdges,
}

impl ContractRefusal {
    /// The stable machine-readable code, shared with runtime admission.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::GraphSchema => "effect-program.graph-schema",
            Self::FootprintLaw => "effect-program.footprint-law",
            Self::FootprintMaxReads => "effect-program.footprint-law.max-reads",
            Self::FootprintMaxWrites => "effect-program.footprint-law.max-writes",
            Self::FootprintMaxRangeEdges => "effect-program.footprint-law.max-range-edges",
        }
    }
}

impl std::fmt::Display for ContractRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

/// Judges a footprint law as the runtime admits it.
///
/// # Errors
/// [`ContractRefusal::FootprintLaw`] for a noncanonical value, a missing or
/// extra field, or a type set that is not strictly ascending 32-byte ids;
/// the matching ceiling refusal for a count above its effect-profile ceiling.
pub fn judge_footprint_law(bytes: &[u8]) -> Result<(), ContractRefusal> {
    const SHAPE: ContractRefusal = ContractRefusal::FootprintLaw;
    let law = canonical(bytes, SHAPE)?;
    exact_fields(
        &law,
        &[
            "read_node_types",
            "write_node_types",
            "atom_types",
            "range_edge_types",
            "write_edge_types",
            "portal_owner_types",
            "max_reads",
            "max_writes",
            "max_range_edges",
        ],
        SHAPE,
    )?;
    for (name, ceiling, refusal) in [
        ("max_reads", MAX_READS, ContractRefusal::FootprintMaxReads),
        (
            "max_writes",
            MAX_WRITES,
            ContractRefusal::FootprintMaxWrites,
        ),
        (
            "max_range_edges",
            MAX_RANGE_EDGES,
            ContractRefusal::FootprintMaxRangeEdges,
        ),
    ] {
        if unsigned(field(&law, name, SHAPE)?, SHAPE)? > ceiling {
            return Err(refusal);
        }
    }
    for name in [
        "read_node_types",
        "atom_types",
        "portal_owner_types",
        "range_edge_types",
        "write_node_types",
        "write_edge_types",
    ] {
        ascending_ids(field(&law, name, SHAPE)?, SHAPE)?;
    }
    Ok(())
}

/// Judges a graph schema as the runtime admits it.
///
/// # Errors
/// [`ContractRefusal::GraphSchema`] for a noncanonical value, a missing or
/// extra field, an addressing other than 0 or 1, or a type map or set that is
/// not strictly ascending 32-byte ids.
pub fn judge_graph_schema(bytes: &[u8]) -> Result<(), ContractRefusal> {
    const E: ContractRefusal = ContractRefusal::GraphSchema;
    let schema = canonical(bytes, E)?;
    exact_fields(&schema, &["node_types", "edge_types"], E)?;
    let Value::Map(nodes) = field(&schema, "node_types", E)? else {
        return Err(E);
    };
    let mut previous: Option<&[u8]> = None;
    for (node_type, node) in nodes {
        ascending(&mut previous, id(node_type, E)?, E)?;
        exact_fields(node, &["addressing", "atom_types"], E)?;
        if unsigned(field(node, "addressing", E)?, E)? > 1 {
            return Err(E);
        }
        ascending_ids(field(node, "atom_types", E)?, E)?;
    }
    let Value::Map(edges) = field(&schema, "edge_types", E)? else {
        return Err(E);
    };
    let mut previous: Option<&[u8]> = None;
    for (edge_type, pairs) in edges {
        ascending(&mut previous, id(edge_type, E)?, E)?;
        let Value::Array(pairs) = pairs else {
            return Err(E);
        };
        let mut previous_pair: Option<(&[u8], &[u8])> = None;
        for pair in pairs {
            let Value::Array(pair) = pair else {
                return Err(E);
            };
            let [source, target] = pair.as_slice() else {
                return Err(E);
            };
            let pair = (id(source, E)?, id(target, E)?);
            if previous_pair.is_some_and(|previous| previous >= pair) {
                return Err(E);
            }
            previous_pair = Some(pair);
        }
    }
    Ok(())
}

fn canonical(bytes: &[u8], refusal: ContractRefusal) -> Result<Value, ContractRefusal> {
    if bytes.len() > STORAGE_CEILING {
        return Err(refusal);
    }
    let value = decode(bytes).map_err(|_| refusal)?;
    if encode(&value).map_err(|_| refusal)? != bytes {
        return Err(refusal);
    }
    Ok(value)
}

fn field<'a>(
    value: &'a Value,
    name: &str,
    refusal: ContractRefusal,
) -> Result<&'a Value, ContractRefusal> {
    let Value::Map(fields) = value else {
        return Err(refusal);
    };
    fields
        .iter()
        .find_map(|(key, value)| matches!(key, Value::Text(key) if key == name).then_some(value))
        .ok_or(refusal)
}

fn exact_fields(
    value: &Value,
    names: &[&str],
    refusal: ContractRefusal,
) -> Result<(), ContractRefusal> {
    let Value::Map(fields) = value else {
        return Err(refusal);
    };
    if fields.len() != names.len()
        || names
            .iter()
            .any(|name| field(value, name, refusal).is_err())
    {
        return Err(refusal);
    }
    Ok(())
}

fn unsigned(value: &Value, refusal: ContractRefusal) -> Result<u64, ContractRefusal> {
    match value {
        Value::Integer(number) => u64::try_from(*number).map_err(|_| refusal),
        _ => Err(refusal),
    }
}

fn id(value: &Value, refusal: ContractRefusal) -> Result<&[u8], ContractRefusal> {
    match value {
        Value::Bytes(id) if id.len() == 32 => Ok(id),
        _ => Err(refusal),
    }
}

fn ascending<'a>(
    previous: &mut Option<&'a [u8]>,
    id: &'a [u8],
    refusal: ContractRefusal,
) -> Result<(), ContractRefusal> {
    if previous.is_some_and(|previous| previous >= id) {
        return Err(refusal);
    }
    *previous = Some(id);
    Ok(())
}

fn ascending_ids(value: &Value, refusal: ContractRefusal) -> Result<(), ContractRefusal> {
    let Value::Array(ids) = value else {
        return Err(refusal);
    };
    let mut previous = None;
    for value in ids {
        ascending(&mut previous, id(value, refusal)?, refusal)?;
    }
    Ok(())
}
