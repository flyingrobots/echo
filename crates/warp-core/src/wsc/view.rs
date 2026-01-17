// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Zero-copy views over WSC file data.
//!
//! This module provides `WscFile` for opening WSC files and `WarpView` for
//! accessing individual WARP instances without copying data.
//!
//! # Example
//!
//! ```ignore
//! let file = WscFile::open("state.wsc")?;
//! let view = file.warp_view(0)?;
//!
//! for node in view.nodes() {
//!     println!("Node: {:?}", node.node_id);
//! }
//! ```

use std::path::Path;

use crate::ident::Hash;

use super::read::{read_bytes, read_slice, validate_header, ReadError};
use super::types::{AttRow, EdgeRow, NodeRow, OutEdgeRef, Range, WarpDirEntry, WscHeader};

/// A memory-mapped or in-memory WSC file.
///
/// This struct owns the file data and provides access to individual WARP
/// instances via [`warp_view`](Self::warp_view).
#[derive(Debug)]
pub struct WscFile {
    /// The raw file data (owned).
    data: Vec<u8>,
}

impl WscFile {
    /// Opens a WSC file from disk.
    ///
    /// The entire file is read into memory. For very large files, consider
    /// using memory-mapped IO instead (not yet implemented).
    ///
    /// # Errors
    ///
    /// Returns [`ReadError::Io`] if the file cannot be read.
    /// Returns [`ReadError::InvalidMagic`] if the file header is invalid.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ReadError> {
        let data = std::fs::read(path)?;
        Self::from_bytes(data)
    }

    /// Creates a `WscFile` from raw bytes.
    ///
    /// Validates the header but does not fully validate all sections.
    /// Use [`validate`](super::validate::validate_wsc) for full validation.
    ///
    /// # Errors
    ///
    /// Returns [`ReadError::FileTooSmall`] or [`ReadError::InvalidMagic`] if the header is invalid.
    pub fn from_bytes(data: Vec<u8>) -> Result<Self, ReadError> {
        // Validate header
        let _ = validate_header(&data)?;
        Ok(Self { data })
    }

    /// Returns the file header.
    pub fn header(&self) -> &WscHeader {
        // Use bytemuck for safe transmutation - we know data is valid from from_bytes
        bytemuck::from_bytes(&self.data[..std::mem::size_of::<WscHeader>()])
    }

    /// Returns the number of WARP instances in this file.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // WARP count won't exceed usize on any platform
    pub fn warp_count(&self) -> usize {
        self.header().warp_count() as usize
    }

    /// Returns the tick number this snapshot represents.
    #[must_use]
    pub fn tick(&self) -> u64 {
        self.header().tick()
    }

    /// Returns the schema hash.
    #[must_use]
    pub fn schema_hash(&self) -> &Hash {
        &self.header().schema_hash
    }

    /// Returns the raw file data.
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Returns a view over a specific WARP instance.
    ///
    /// # Arguments
    ///
    /// * `index` - Zero-based index of the WARP instance
    ///
    /// # Errors
    ///
    /// Returns an error if the index is out of bounds or the instance data
    /// is malformed.
    pub fn warp_view(&self, index: usize) -> Result<WarpView<'_>, ReadError> {
        let count = self.warp_count();
        if index >= count {
            return Err(ReadError::WarpIndexOutOfBounds { index, count });
        }

        // Use read_slice to safely get the directory entries
        let dir_entries = read_slice::<WarpDirEntry>(
            &self.data,
            self.header().warp_dir_off(),
            count as u64,
            "warp_directory",
        )?;

        // Get the specific entry (bounds already checked above)
        let entry = &dir_entries[index];

        WarpView::new(&self.data, entry)
    }
}

/// Zero-copy view over a single WARP instance within a WSC file.
///
/// Provides direct access to nodes, edges, and attachments without copying.
#[derive(Debug)]
pub struct WarpView<'a> {
    /// Reference to the full file data.
    #[allow(dead_code)]
    data: &'a [u8],
    /// The directory entry for this WARP.
    entry: &'a WarpDirEntry,
    /// Cached node slice.
    nodes: &'a [NodeRow],
    /// Cached edge slice.
    edges: &'a [EdgeRow],
    /// Cached `out_index` slice.
    out_index: &'a [Range],
    /// Cached `out_edges` slice.
    out_edges: &'a [OutEdgeRef],
    /// Cached `node_atts_index` slice.
    node_atts_index: &'a [Range],
    /// Cached `node_atts` slice.
    node_atts: &'a [AttRow],
    /// Cached `edge_atts_index` slice.
    edge_atts_index: &'a [Range],
    /// Cached `edge_atts` slice.
    edge_atts: &'a [AttRow],
    /// Cached blobs slice.
    blobs: &'a [u8],
}

impl<'a> WarpView<'a> {
    /// Creates a new view from file data and a directory entry.
    fn new(data: &'a [u8], entry: &'a WarpDirEntry) -> Result<Self, ReadError> {
        let nodes = read_slice::<NodeRow>(
            data,
            u64::from_le(entry.nodes_off_le),
            u64::from_le(entry.nodes_len_le),
            "nodes",
        )?;

        let edges = read_slice::<EdgeRow>(
            data,
            u64::from_le(entry.edges_off_le),
            u64::from_le(entry.edges_len_le),
            "edges",
        )?;

        let out_index = read_slice::<Range>(
            data,
            u64::from_le(entry.out_index_off_le),
            nodes.len() as u64, // One range per node
            "out_index",
        )?;

        let out_edges = read_slice::<OutEdgeRef>(
            data,
            u64::from_le(entry.out_edges_off_le),
            u64::from_le(entry.out_edges_len_le),
            "out_edges",
        )?;

        let node_atts_index = read_slice::<Range>(
            data,
            u64::from_le(entry.node_atts_index_off_le),
            nodes.len() as u64, // One range per node
            "node_atts_index",
        )?;

        let node_atts = read_slice::<AttRow>(
            data,
            u64::from_le(entry.node_atts_off_le),
            u64::from_le(entry.node_atts_len_le),
            "node_atts",
        )?;

        let edge_atts_index = read_slice::<Range>(
            data,
            u64::from_le(entry.edge_atts_index_off_le),
            edges.len() as u64, // One range per edge
            "edge_atts_index",
        )?;

        let edge_atts = read_slice::<AttRow>(
            data,
            u64::from_le(entry.edge_atts_off_le),
            u64::from_le(entry.edge_atts_len_le),
            "edge_atts",
        )?;

        let blobs = read_bytes(
            data,
            u64::from_le(entry.blobs_off_le),
            u64::from_le(entry.blobs_len_le),
            "blobs",
        )?;

        Ok(Self {
            data,
            entry,
            nodes,
            edges,
            out_index,
            out_edges,
            node_atts_index,
            node_atts,
            edge_atts_index,
            edge_atts,
            blobs,
        })
    }

    /// Returns the WARP instance identifier.
    #[must_use]
    pub fn warp_id(&self) -> &Hash {
        &self.entry.warp_id
    }

    /// Returns the root node identifier.
    #[must_use]
    pub fn root_node_id(&self) -> &Hash {
        &self.entry.root_node_id
    }

    /// Returns all nodes in this WARP instance.
    #[must_use]
    pub fn nodes(&self) -> &[NodeRow] {
        self.nodes
    }

    /// Returns all edges in this WARP instance.
    #[must_use]
    pub fn edges(&self) -> &[EdgeRow] {
        self.edges
    }

    /// Finds a node's index by its ID using binary search.
    ///
    /// Returns `Some(index)` if found, `None` otherwise.
    #[must_use]
    pub fn node_ix(&self, node_id: &Hash) -> Option<usize> {
        self.nodes.binary_search_by_key(node_id, |n| n.node_id).ok()
    }

    /// Finds an edge's index by its ID using binary search.
    ///
    /// Returns `Some(index)` if found, `None` otherwise.
    #[must_use]
    pub fn edge_ix(&self, edge_id: &Hash) -> Option<usize> {
        self.edges.binary_search_by_key(edge_id, |e| e.edge_id).ok()
    }

    /// Returns the outbound edges for a node by index.
    ///
    /// Returns an empty slice if the index is out of bounds.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // Safe: .get() bounds-checks
    pub fn out_edges_for_node(&self, node_ix: usize) -> &[OutEdgeRef] {
        self.out_index.get(node_ix).map_or(&[], |range| {
            let start = range.start() as usize;
            let len = range.len() as usize;
            self.out_edges.get(start..start + len).unwrap_or(&[])
        })
    }

    /// Returns the attachments for a node by index.
    ///
    /// Returns an empty slice if the index is out of bounds.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // Safe: .get() bounds-checks
    pub fn node_attachments(&self, node_ix: usize) -> &[AttRow] {
        self.node_atts_index.get(node_ix).map_or(&[], |range| {
            let start = range.start() as usize;
            let len = range.len() as usize;
            self.node_atts.get(start..start + len).unwrap_or(&[])
        })
    }

    /// Returns the attachments for an edge by index.
    ///
    /// Returns an empty slice if the index is out of bounds.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // Safe: .get() bounds-checks
    pub fn edge_attachments(&self, edge_ix: usize) -> &[AttRow] {
        self.edge_atts_index.get(edge_ix).map_or(&[], |range| {
            let start = range.start() as usize;
            let len = range.len() as usize;
            self.edge_atts.get(start..start + len).unwrap_or(&[])
        })
    }

    /// Returns the blob data for an attachment.
    ///
    /// Returns `None` if the attachment is a `Descend` or the blob reference
    /// is out of bounds.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // Safe: .get() bounds-checks
    pub fn blob_for_attachment(&self, att: &AttRow) -> Option<&[u8]> {
        if !att.is_atom() {
            return None;
        }

        let off = att.blob_off() as usize;
        let len = att.blob_len() as usize;

        self.blobs.get(off..off + len)
    }

    /// Returns the raw blob section.
    #[must_use]
    pub fn blobs(&self) -> &[u8] {
        self.blobs
    }

    /// Returns a reference to the raw file data.
    #[must_use]
    pub fn raw_data(&self) -> &[u8] {
        self.data
    }

    /// Validates that all index ranges are within bounds of their data tables.
    ///
    /// This is called by [`validate_wsc`](super::validate::validate_wsc) to detect
    /// corrupted index tables that would otherwise be silently masked by the
    /// accessors returning empty slices.
    ///
    /// # Errors
    ///
    /// Returns [`ReadError::IndexRangeOutOfBounds`] if any range extends past
    /// its data table.
    pub fn validate_index_ranges(&self) -> Result<(), ReadError> {
        // Validate out_index ranges against out_edges
        for (i, range) in self.out_index.iter().enumerate() {
            let start = range.start();
            let end = start.saturating_add(range.len());
            if end > self.out_edges.len() as u64 {
                return Err(ReadError::IndexRangeOutOfBounds {
                    index_name: "out_index",
                    entry_index: i,
                    start,
                    end,
                    data_name: "out_edges",
                    data_len: self.out_edges.len(),
                });
            }
        }

        // Validate node_atts_index ranges against node_atts
        for (i, range) in self.node_atts_index.iter().enumerate() {
            let start = range.start();
            let end = start.saturating_add(range.len());
            if end > self.node_atts.len() as u64 {
                return Err(ReadError::IndexRangeOutOfBounds {
                    index_name: "node_atts_index",
                    entry_index: i,
                    start,
                    end,
                    data_name: "node_atts",
                    data_len: self.node_atts.len(),
                });
            }
        }

        // Validate edge_atts_index ranges against edge_atts
        for (i, range) in self.edge_atts_index.iter().enumerate() {
            let start = range.start();
            let end = start.saturating_add(range.len());
            if end > self.edge_atts.len() as u64 {
                return Err(ReadError::IndexRangeOutOfBounds {
                    index_name: "edge_atts_index",
                    entry_index: i,
                    start,
                    end,
                    data_name: "edge_atts",
                    data_len: self.edge_atts.len(),
                });
            }
        }

        Ok(())
    }
}

/// Reference to an attachment value with blob access.
#[derive(Debug, Clone, Copy)]
pub struct AttachmentRef<'a> {
    /// The attachment row.
    pub row: &'a AttRow,
    /// The blob data (if this is an atom attachment).
    pub blob: Option<&'a [u8]>,
}

impl<'a> AttachmentRef<'a> {
    /// Creates a new attachment reference.
    #[must_use]
    pub fn new(row: &'a AttRow, blob: Option<&'a [u8]>) -> Self {
        Self { row, blob }
    }

    /// Returns true if this is an atom attachment.
    #[must_use]
    pub fn is_atom(&self) -> bool {
        self.row.is_atom()
    }

    /// Returns true if this is a descend portal.
    #[must_use]
    pub fn is_descend(&self) -> bool {
        self.row.is_descend()
    }

    /// For atom attachments, returns the type ID.
    /// For descend attachments, returns the child WARP ID.
    #[must_use]
    pub fn type_or_warp_id(&self) -> &Hash {
        &self.row.type_or_warp
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::wsc::write::{write_wsc_one_warp, OneWarpInput};

    fn make_test_input() -> OneWarpInput {
        use crate::wsc::types::NodeRow;

        OneWarpInput {
            warp_id: [1u8; 32],
            root_node_id: [2u8; 32],
            nodes: vec![NodeRow {
                node_id: [2u8; 32],
                node_type: [3u8; 32],
            }],
            edges: vec![],
            out_index: vec![Range::default()],
            out_edges: vec![],
            node_atts_index: vec![Range::default()],
            node_atts: vec![],
            edge_atts_index: vec![],
            edge_atts: vec![],
            blobs: vec![],
        }
    }

    #[test]
    fn roundtrip_single_node() {
        let input = make_test_input();
        let bytes = write_wsc_one_warp(&input, [0u8; 32], 42).expect("write failed");

        let file = WscFile::from_bytes(bytes).expect("from_bytes failed");
        assert_eq!(file.warp_count(), 1);
        assert_eq!(file.tick(), 42);

        let view = file.warp_view(0).expect("warp_view failed");
        assert_eq!(view.warp_id(), &[1u8; 32]);
        assert_eq!(view.root_node_id(), &[2u8; 32]);
        assert_eq!(view.nodes().len(), 1);
        assert_eq!(view.nodes()[0].node_id, [2u8; 32]);
        assert_eq!(view.nodes()[0].node_type, [3u8; 32]);
    }

    #[test]
    fn warp_view_out_of_bounds() {
        let input = make_test_input();
        let bytes = write_wsc_one_warp(&input, [0u8; 32], 0).expect("write failed");
        let file = WscFile::from_bytes(bytes).expect("from_bytes failed");

        let err = file.warp_view(1).unwrap_err();
        assert!(matches!(err, ReadError::WarpIndexOutOfBounds { .. }));
    }
}
