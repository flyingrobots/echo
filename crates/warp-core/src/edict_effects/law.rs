// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Strict host-side decoding of the proposed footprint-law shape.
use crate::edict_pure::{
    values::{array, bytes, exact_fields, field, number},
    EvaluationError,
};
use crate::TypeId;
use echo_edict_canonical::{decode_canonical_cbor_v1, CanonicalValueV1 as Value};
use std::collections::BTreeSet;

/// Invalid canonical law shape, set ordering, type identifier or profile count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EffectLawError {
    /// The bounded input is not an exact canonical footprint-law value.
    #[error("invalid canonical footprint law")]
    InvalidLaw,
}

/// Host-selected typed footprint permissions, not an admitted package capability.
///
/// Decoding validates all nine fields, including currently unused write fields.
/// Package identity and installation must separately bind these exact bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectFootprintLaw {
    pub(super) nodes: BTreeSet<TypeId>,
    pub(super) atoms: BTreeSet<TypeId>,
    pub(super) portals: BTreeSet<TypeId>,
    pub(super) ranges: BTreeSet<TypeId>,
    pub(super) max_reads: u64,
    pub(super) max_range_edges: u64,
    _write_nodes: BTreeSet<TypeId>,
    _write_edges: BTreeSet<TypeId>,
    _max_writes: u64,
}

impl EffectFootprintLaw {
    /// Decodes canonical bytes without sorting or deduplicating set arrays.
    ///
    /// Setup is bounded to 16 MiB and precedes evaluation metering. Counts may
    /// be zero; ceilings are 4096 reads, 256 writes and 65536 bucket edges.
    ///
    /// # Errors
    /// Refuses noncanonical CBOR, missing/extra fields, malformed or unordered
    /// sets, and counts outside the effect-profile ceilings.
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, EffectLawError> {
        if u64::try_from(bytes.len()).map_err(|_| EffectLawError::InvalidLaw)?
            > super::EFFECT_READ_STORAGE_CEILING_V1
        {
            return Err(EffectLawError::InvalidLaw);
        }
        let value = decode_canonical_cbor_v1(bytes).map_err(|_| EffectLawError::InvalidLaw)?;
        Self::parse(&value).map_err(|_| EffectLawError::InvalidLaw)
    }

    fn parse(value: &Value) -> Result<Self, EvaluationError> {
        exact_fields(
            value,
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
        )?;
        let max_reads = number(field(value, "max_reads")?)?;
        let max_writes = number(field(value, "max_writes")?)?;
        let max_range_edges = number(field(value, "max_range_edges")?)?;
        if max_reads > 4096 || max_writes > 256 || max_range_edges > 65536 {
            return Err(EvaluationError::InvalidArtifact);
        }
        Ok(Self {
            nodes: ids(value, "read_node_types")?,
            atoms: ids(value, "atom_types")?,
            portals: ids(value, "portal_owner_types")?,
            ranges: ids(value, "range_edge_types")?,
            max_reads,
            max_range_edges,
            _write_nodes: ids(value, "write_node_types")?,
            _write_edges: ids(value, "write_edge_types")?,
            _max_writes: max_writes,
        })
    }
}

fn ids(value: &Value, name: &str) -> Result<BTreeSet<TypeId>, EvaluationError> {
    let mut result = BTreeSet::new();
    let mut previous = None;
    for value in array(field(value, name)?)? {
        let id = TypeId(
            bytes(value)?
                .try_into()
                .map_err(|_| EvaluationError::InvalidArtifact)?,
        );
        if previous.is_some_and(|previous| previous >= id) {
            return Err(EvaluationError::InvalidArtifact);
        }
        previous = Some(id);
        result.insert(id);
    }
    Ok(result)
}
