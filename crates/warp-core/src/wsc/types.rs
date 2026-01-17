// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! WSC row types for columnar graph representation.
//!
//! These structures define the fixed-size rows used in WSC (Write-Streaming
//! Columnar) snapshot files. All fields use little-endian byte order for
//! cross-platform determinism.
//!
//! # Layout Guarantees
//!
//! All row types are `#[repr(C)]` with explicit padding to ensure:
//! - Predictable byte layout across compilers
//! - Zero-copy mmap compatibility
//! - 8-byte alignment for SIMD-friendly access
//!
//! All types derive `bytemuck::Pod` and `bytemuck::Zeroable` for safe
//! transmutation without unsafe code.

use bytemuck::{Pod, Zeroable};

use crate::ident::Hash;

/// Fixed-size node row (64 bytes).
///
/// Contains the skeleton-plane identity of a node: its ID and type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct NodeRow {
    /// Node identifier (32 bytes).
    pub node_id: Hash,
    /// Type identifier for this node (32 bytes).
    pub node_type: Hash,
}

const _: () = assert!(std::mem::size_of::<NodeRow>() == 64);

/// Fixed-size edge row (128 bytes).
///
/// Contains the skeleton-plane identity of an edge: its ID, endpoints, and type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct EdgeRow {
    /// Edge identifier (32 bytes).
    pub edge_id: Hash,
    /// Source node identifier (32 bytes).
    pub from_node_id: Hash,
    /// Destination node identifier (32 bytes).
    pub to_node_id: Hash,
    /// Type identifier for this edge (32 bytes).
    pub edge_type: Hash,
}

const _: () = assert!(std::mem::size_of::<EdgeRow>() == 128);

/// Range descriptor for variable-length sublists (16 bytes).
///
/// Used by index tables (e.g., `out_index`) to describe a slice into
/// a corresponding data table.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Pod, Zeroable)]
pub struct Range {
    /// Starting offset in the target table (little-endian u64).
    pub start_le: u64,
    /// Number of elements in this range (little-endian u64).
    pub len_le: u64,
}

const _: () = assert!(std::mem::size_of::<Range>() == 16);

impl Range {
    /// Returns the start offset (converting from little-endian).
    #[must_use]
    pub fn start(&self) -> u64 {
        u64::from_le(self.start_le)
    }

    /// Returns the length (converting from little-endian).
    #[must_use]
    pub fn len(&self) -> u64 {
        u64::from_le(self.len_le)
    }

    /// Returns true if the range is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Outbound edge reference (40 bytes).
///
/// Links a node to its outgoing edges via the global edge table index.
/// Includes the edge ID for direct lookup without dereferencing.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct OutEdgeRef {
    /// Index into the global edge table (little-endian u64).
    pub edge_ix_le: u64,
    /// Edge identifier (32 bytes) for direct matching.
    pub edge_id: Hash,
}

const _: () = assert!(std::mem::size_of::<OutEdgeRef>() == 40);

impl OutEdgeRef {
    /// Returns the edge table index (converting from little-endian).
    #[must_use]
    pub fn edge_ix(&self) -> u64 {
        u64::from_le(self.edge_ix_le)
    }
}

/// Attachment row (56 bytes).
///
/// Describes a single attachment value. The `tag` field discriminates between:
/// - `1`: Atom payload (`type_or_warp` = `TypeId`, `blob_off`/`len` = payload location)
/// - `2`: Descend portal (`type_or_warp` = child `WarpId`, blob fields unused)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct AttRow {
    /// Discriminator tag: 1 = Atom, 2 = Descend.
    pub tag: u8,
    /// Reserved for future use (must be zero).
    pub reserved0: [u8; 7],
    /// For Atom: `TypeId`. For Descend: child `WarpId`.
    pub type_or_warp: Hash,
    /// Blob offset in the blob section (little-endian u64).
    pub blob_off_le: u64,
    /// Blob length in bytes (little-endian u64).
    pub blob_len_le: u64,
}

const _: () = assert!(std::mem::size_of::<AttRow>() == 56);

impl AttRow {
    /// Attachment tag for atom payloads.
    pub const TAG_ATOM: u8 = 1;
    /// Attachment tag for descend portals.
    pub const TAG_DESCEND: u8 = 2;

    /// Returns the blob offset (converting from little-endian).
    #[must_use]
    pub fn blob_off(&self) -> u64 {
        u64::from_le(self.blob_off_le)
    }

    /// Returns the blob length (converting from little-endian).
    #[must_use]
    pub fn blob_len(&self) -> u64 {
        u64::from_le(self.blob_len_le)
    }

    /// Returns true if this is an atom attachment.
    #[must_use]
    pub fn is_atom(&self) -> bool {
        self.tag == Self::TAG_ATOM
    }

    /// Returns true if this is a descend portal.
    #[must_use]
    pub fn is_descend(&self) -> bool {
        self.tag == Self::TAG_DESCEND
    }
}

/// WSC file header (128 bytes).
///
/// Contains file-level metadata and section offsets.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct WscHeader {
    /// Magic bytes: `b"WSC\x00\x01\x00\x00\x00"` (version 1).
    pub magic: [u8; 8],
    /// Schema hash for compatibility validation.
    pub schema_hash: Hash,
    /// Tick number this snapshot represents.
    pub tick_le: u64,
    /// Number of WARP instances in this file.
    pub warp_count_le: u64,
    /// Offset to the WARP directory section.
    pub warp_dir_off_le: u64,
    /// Reserved for future header fields.
    pub reserved: [u8; 64],
}

const _: () = assert!(std::mem::size_of::<WscHeader>() == 128);

impl WscHeader {
    /// Magic bytes for WSC format version 1.
    pub const MAGIC_V1: [u8; 8] = *b"WSC\x00\x01\x00\x00\x00";

    /// Returns the tick number (converting from little-endian).
    #[must_use]
    pub fn tick(&self) -> u64 {
        u64::from_le(self.tick_le)
    }

    /// Returns the WARP instance count (converting from little-endian).
    #[must_use]
    pub fn warp_count(&self) -> u64 {
        u64::from_le(self.warp_count_le)
    }

    /// Returns the WARP directory offset (converting from little-endian).
    #[must_use]
    pub fn warp_dir_off(&self) -> u64 {
        u64::from_le(self.warp_dir_off_le)
    }
}

/// WARP directory entry (184 bytes).
///
/// Describes the location of one WARP instance's data within the file.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct WarpDirEntry {
    /// WARP instance identifier.
    pub warp_id: Hash,
    /// Root node identifier for this instance.
    pub root_node_id: Hash,

    /// Offset to the node table.
    pub nodes_off_le: u64,
    /// Number of nodes.
    pub nodes_len_le: u64,

    /// Offset to the edge table.
    pub edges_off_le: u64,
    /// Number of edges.
    pub edges_len_le: u64,

    /// Offset to the `out_index` table.
    pub out_index_off_le: u64,
    /// Offset to the `out_edges` table.
    pub out_edges_off_le: u64,
    /// Number of `out_edges` entries.
    pub out_edges_len_le: u64,

    /// Offset to the `node_atts_index` table.
    pub node_atts_index_off_le: u64,
    /// Offset to the `node_atts` table.
    pub node_atts_off_le: u64,
    /// Number of `node_atts` entries.
    pub node_atts_len_le: u64,

    /// Offset to the `edge_atts_index` table.
    pub edge_atts_index_off_le: u64,
    /// Offset to the `edge_atts` table.
    pub edge_atts_off_le: u64,
    /// Number of `edge_atts` entries.
    pub edge_atts_len_le: u64,

    /// Offset to the blob section.
    pub blobs_off_le: u64,
    /// Length of the blob section.
    pub blobs_len_le: u64,
}

const _: () = assert!(std::mem::size_of::<WarpDirEntry>() == 184);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_sizes_are_stable() {
        // These sizes are part of the format specification.
        assert_eq!(std::mem::size_of::<NodeRow>(), 64);
        assert_eq!(std::mem::size_of::<EdgeRow>(), 128);
        assert_eq!(std::mem::size_of::<Range>(), 16);
        assert_eq!(std::mem::size_of::<OutEdgeRef>(), 40);
        assert_eq!(std::mem::size_of::<AttRow>(), 56);
        assert_eq!(std::mem::size_of::<WscHeader>(), 128);
        assert_eq!(std::mem::size_of::<WarpDirEntry>(), 184);
    }

    #[test]
    fn range_accessors_work() {
        let r = Range {
            start_le: 100u64.to_le(),
            len_le: 42u64.to_le(),
        };
        assert_eq!(r.start(), 100);
        assert_eq!(r.len(), 42);
        assert!(!r.is_empty());
    }

    #[test]
    fn types_are_pod() {
        // These will fail to compile if the types aren't Pod
        fn assert_pod<T: Pod>() {}
        assert_pod::<NodeRow>();
        assert_pod::<EdgeRow>();
        assert_pod::<Range>();
        assert_pod::<OutEdgeRef>();
        assert_pod::<AttRow>();
        assert_pod::<WscHeader>();
        assert_pod::<WarpDirEntry>();
    }
}
