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
    /// `max_reads` is above the effect profile's 4,096.
    #[error("footprint law max_reads is above 4096")]
    MaxReads,
    /// `max_writes` is above the effect profile's 256.
    #[error("footprint law max_writes is above 256")]
    MaxWrites,
    /// `max_range_edges` is above the effect profile's 65,536.
    #[error("footprint law max_range_edges is above 65536")]
    MaxRangeEdges,
}

/// Host-selected typed footprint permissions, not an admitted package capability.
///
/// Decoding validates all nine fields. Private derivation enforces write permissions.
/// Package identity and installation must separately bind these exact bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectFootprintLaw {
    pub(super) nodes: BTreeSet<TypeId>,
    pub(super) atoms: BTreeSet<TypeId>,
    pub(super) portals: BTreeSet<TypeId>,
    pub(super) ranges: BTreeSet<TypeId>,
    pub(super) max_reads: u64,
    pub(super) max_range_edges: u64,
    pub(super) write_nodes: BTreeSet<TypeId>,
    pub(super) write_edges: BTreeSet<TypeId>,
    pub(super) max_writes: u64,
}

impl EffectFootprintLaw {
    /// Decodes canonical bytes without sorting or deduplicating set arrays.
    ///
    /// Setup is bounded to 16 MiB and precedes evaluation metering. Counts may
    /// be zero; ceilings are 4096 reads, 256 writes and 65536 bucket edges.
    ///
    /// # Errors
    /// Refuses noncanonical CBOR, missing/extra fields and malformed or
    /// unordered sets with [`EffectLawError::InvalidLaw`], and a count above
    /// its effect-profile ceiling with that ceiling's variant.
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, EffectLawError> {
        if u64::try_from(bytes.len()).map_err(|_| EffectLawError::InvalidLaw)?
            > super::EFFECT_READ_STORAGE_CEILING_V1
        {
            return Err(EffectLawError::InvalidLaw);
        }
        let value = decode_canonical_cbor_v1(bytes).map_err(|_| EffectLawError::InvalidLaw)?;
        Self::parse(&value)
    }

    fn parse(value: &Value) -> Result<Self, EffectLawError> {
        let shape = |_: EvaluationError| EffectLawError::InvalidLaw;
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
        )
        .map_err(shape)?;
        let count = |name: &str| number(field(value, name)?);
        let max_reads = count("max_reads").map_err(shape)?;
        let max_writes = count("max_writes").map_err(shape)?;
        let max_range_edges = count("max_range_edges").map_err(shape)?;
        if max_reads > 4096 {
            return Err(EffectLawError::MaxReads);
        }
        if max_writes > 256 {
            return Err(EffectLawError::MaxWrites);
        }
        if max_range_edges > 65536 {
            return Err(EffectLawError::MaxRangeEdges);
        }
        Ok(Self {
            nodes: ids(value, "read_node_types").map_err(shape)?,
            atoms: ids(value, "atom_types").map_err(shape)?,
            portals: ids(value, "portal_owner_types").map_err(shape)?,
            ranges: ids(value, "range_edge_types").map_err(shape)?,
            max_reads,
            max_range_edges,
            write_nodes: ids(value, "write_node_types").map_err(shape)?,
            write_edges: ids(value, "write_edge_types").map_err(shape)?,
            max_writes,
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
