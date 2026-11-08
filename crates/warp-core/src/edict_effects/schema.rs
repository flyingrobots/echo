// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Executable graph-schema preparation, separate from Action admission.
use crate::edict_pure::{
    values::{array, bytes, exact_fields, field, map, number},
    EvaluationError,
};
use crate::TypeId;
use echo_edict_canonical::{decode_canonical_cbor_v1, CanonicalValueV1 as Value};
use std::collections::{BTreeMap, BTreeSet};

/// Addressing classification declared by a node type's executable schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectNodeAddressing {
    /// Immutable content identity, enforced later by effect validation.
    Content,
    /// Program-chosen identity for a mutable record.
    Mutable,
}
/// Invalid canonical graph-schema shape or set encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EffectGraphSchemaError {
    /// The input is not an exact canonical graph schema.
    #[error("invalid canonical effect graph schema")]
    InvalidSchema,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct NodeType {
    addressing: EffectNodeAddressing,
    atoms: BTreeSet<TypeId>,
}
/// Decoded node/atom and edge-endpoint relations, without admission authority.
///
/// Preparation bounds input to 16 MiB and uses the canonical decoder's combined
/// node/depth ceilings. It precedes evaluation metering and is not an arena,
/// graph-state validation or authorization to read or mutate graph records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectGraphSchema {
    nodes: BTreeMap<TypeId, NodeType>,
    edges: BTreeMap<TypeId, BTreeSet<(TypeId, TypeId)>>,
}
impl EffectGraphSchema {
    /// Decodes the executable schema's canonical maps and ordered set arrays.
    /// # Errors
    /// Refuses noncanonical bytes, malformed fields, identifiers and sets.
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, EffectGraphSchemaError> {
        if u64::try_from(bytes.len()).map_err(|_| EffectGraphSchemaError::InvalidSchema)?
            > super::EFFECT_READ_STORAGE_CEILING_V1
        {
            return Err(EffectGraphSchemaError::InvalidSchema);
        }
        let value =
            decode_canonical_cbor_v1(bytes).map_err(|_| EffectGraphSchemaError::InvalidSchema)?;
        Self::parse(&value).map_err(|_| EffectGraphSchemaError::InvalidSchema)
    }
    /// Addressing classification for a declared node type, if present.
    #[must_use]
    pub fn node_addressing(&self, node_type: TypeId) -> Option<EffectNodeAddressing> {
        self.nodes.get(&node_type).map(|node| node.addressing)
    }
    /// Whether a node type declares this atom type.
    #[must_use]
    pub fn allows_atom(&self, node_type: TypeId, atom_type: TypeId) -> bool {
        self.nodes
            .get(&node_type)
            .is_some_and(|node| node.atoms.contains(&atom_type))
    }
    /// Whether an edge type declares this source/target node-type pair.
    ///
    /// This is relation membership only; it does not resolve endpoint records.
    #[must_use]
    pub fn allows_edge(&self, edge_type: TypeId, source_type: TypeId, target_type: TypeId) -> bool {
        self.edges
            .get(&edge_type)
            .is_some_and(|pairs| pairs.contains(&(source_type, target_type)))
    }
    #[cfg(feature = "native_rule_bootstrap")]
    pub(super) fn has_edge_type(&self, ty: TypeId) -> bool {
        self.edges.contains_key(&ty)
    }
    fn parse(value: &Value) -> Result<Self, EvaluationError> {
        exact_fields(value, &["node_types", "edge_types"])?;
        let mut nodes = BTreeMap::new();
        let mut previous = None;
        for (key, value) in map(field(value, "node_types")?)? {
            let ty = id(key)?;
            ascending(&mut previous, ty)?;
            exact_fields(value, &["addressing", "atom_types"])?;
            let addressing = match number(field(value, "addressing")?)? {
                0 => EffectNodeAddressing::Content,
                1 => EffectNodeAddressing::Mutable,
                _ => return Err(EvaluationError::InvalidArtifact),
            };
            let mut atoms = BTreeSet::new();
            let mut previous_atom = None;
            for atom in array(field(value, "atom_types")?)? {
                let atom = id(atom)?;
                ascending(&mut previous_atom, atom)?;
                atoms.insert(atom);
            }
            nodes.insert(ty, NodeType { addressing, atoms });
        }
        let mut edges = BTreeMap::new();
        let mut previous = None;
        for (key, value) in map(field(value, "edge_types")?)? {
            let ty = id(key)?;
            ascending(&mut previous, ty)?;
            let mut pairs = BTreeSet::new();
            let mut previous_pair = None;
            for pair in array(value)? {
                let [source, target] = array(pair)? else {
                    return Err(EvaluationError::InvalidArtifact);
                };
                let pair = (id(source)?, id(target)?);
                ascending(&mut previous_pair, pair)?;
                pairs.insert(pair);
            }
            edges.insert(ty, pairs);
        }
        Ok(Self { nodes, edges })
    }
}
fn id(value: &Value) -> Result<TypeId, EvaluationError> {
    bytes(value)?
        .try_into()
        .map(TypeId)
        .map_err(|_| EvaluationError::InvalidArtifact)
}
fn ascending<T: Copy + Ord>(previous: &mut Option<T>, value: T) -> Result<(), EvaluationError> {
    if previous.is_some_and(|previous| previous >= value) {
        return Err(EvaluationError::InvalidArtifact);
    }
    *previous = Some(value);
    Ok(())
}
