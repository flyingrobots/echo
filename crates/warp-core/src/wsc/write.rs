// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! WSC file writer for serializing graph snapshots.
//!
//! This module provides deterministic serialization of WARP graph state into
//! the WSC (Write-Streaming Columnar) format. The output is byte-exact
//! regardless of the order in which data was inserted into the `GraphStore`.

use std::io::{self, Write};

use bytemuck::Pod;

use crate::ident::Hash;

use super::types::{AttRow, EdgeRow, NodeRow, OutEdgeRef, Range, WarpDirEntry, WscHeader};

/// Input data for writing a single WARP instance to a WSC file.
///
/// This struct owns all the data needed to serialize one WARP instance.
/// Use [`build_one_warp_input`](super::build::build_one_warp_input) to
/// construct this from a `GraphStore`.
#[derive(Debug, Clone)]
pub struct OneWarpInput {
    /// WARP instance identifier.
    pub warp_id: Hash,
    /// Root node identifier.
    pub root_node_id: Hash,

    /// Node rows (sorted by `NodeId`).
    pub nodes: Vec<NodeRow>,
    /// Edge rows (sorted by `EdgeId`).
    pub edges: Vec<EdgeRow>,

    /// Outbound edge index (one Range per node, parallel to `nodes`).
    pub out_index: Vec<Range>,
    /// Outbound edge references (concatenated sublists).
    pub out_edges: Vec<OutEdgeRef>,

    /// Node attachment index (one Range per node, parallel to `nodes`).
    pub node_atts_index: Vec<Range>,
    /// Node attachment rows (concatenated sublists).
    pub node_atts: Vec<AttRow>,

    /// Edge attachment index (one Range per edge, parallel to `edges`).
    pub edge_atts_index: Vec<Range>,
    /// Edge attachment rows (concatenated sublists).
    pub edge_atts: Vec<AttRow>,

    /// Blob data for atom payloads (8-byte aligned).
    pub blobs: Vec<u8>,
}

/// Writes a single-WARP WSC file to a byte vector.
///
/// # Arguments
///
/// * `input` - The WARP instance data to serialize
/// * `schema_hash` - Schema version hash for compatibility checking
/// * `tick` - Tick number this snapshot represents
///
/// # Errors
///
/// Returns an IO error if writing to the buffer fails (should not happen
/// for in-memory `Vec<u8>` writes).
///
/// # Panics
///
/// Panics if internal size calculations are inconsistent (indicates a bug
/// in the offset calculation logic, not user error).
///
/// # Determinism
///
/// The output is fully deterministic: given the same `input`, `schema_hash`,
/// and `tick`, the output bytes will be identical across runs.
#[allow(clippy::too_many_lines)] // Format writing is inherently verbose
pub fn write_wsc_one_warp(
    input: &OneWarpInput,
    schema_hash: Hash,
    tick: u64,
) -> io::Result<Vec<u8>> {
    // Pre-calculate sizes and offsets
    let header_size = std::mem::size_of::<WscHeader>();
    let dir_entry_size = std::mem::size_of::<WarpDirEntry>();
    let warp_dir_off = header_size;

    // Calculate section sizes
    let nodes_size = input.nodes.len() * std::mem::size_of::<NodeRow>();
    let edges_size = input.edges.len() * std::mem::size_of::<EdgeRow>();
    let out_index_size = input.out_index.len() * std::mem::size_of::<Range>();
    let out_edges_size = input.out_edges.len() * std::mem::size_of::<OutEdgeRef>();
    let node_atts_index_size = input.node_atts_index.len() * std::mem::size_of::<Range>();
    let node_atts_size = input.node_atts.len() * std::mem::size_of::<AttRow>();
    let edge_atts_index_size = input.edge_atts_index.len() * std::mem::size_of::<Range>();
    let edge_atts_size = input.edge_atts.len() * std::mem::size_of::<AttRow>();
    let blobs_size = input.blobs.len();

    // Calculate offsets (all sections are 8-byte aligned)
    let mut offset = warp_dir_off + dir_entry_size;
    offset = align8(offset);

    let nodes_off = offset;
    offset += nodes_size;
    offset = align8(offset);

    let edges_off = offset;
    offset += edges_size;
    offset = align8(offset);

    let out_index_off = offset;
    offset += out_index_size;
    offset = align8(offset);

    let out_edges_off = offset;
    offset += out_edges_size;
    offset = align8(offset);

    let node_atts_index_off = offset;
    offset += node_atts_index_size;
    offset = align8(offset);

    let node_atts_off = offset;
    offset += node_atts_size;
    offset = align8(offset);

    let edge_atts_index_off = offset;
    offset += edge_atts_index_size;
    offset = align8(offset);

    let edge_atts_off = offset;
    offset += edge_atts_size;
    offset = align8(offset);

    let blobs_off = offset;
    offset += blobs_size;

    let total_size = offset;

    // Allocate buffer
    let mut buf = Vec::with_capacity(total_size);

    // Write header
    let header = WscHeader {
        magic: WscHeader::MAGIC_V1,
        schema_hash,
        tick_le: tick.to_le(),
        warp_count_le: 1u64.to_le(),
        warp_dir_off_le: (warp_dir_off as u64).to_le(),
        reserved: [0u8; 64],
    };
    write_struct(&mut buf, &header)?;

    // Write WARP directory entry
    let dir_entry = WarpDirEntry {
        warp_id: input.warp_id,
        root_node_id: input.root_node_id,

        nodes_off_le: (nodes_off as u64).to_le(),
        nodes_len_le: (input.nodes.len() as u64).to_le(),

        edges_off_le: (edges_off as u64).to_le(),
        edges_len_le: (input.edges.len() as u64).to_le(),

        out_index_off_le: (out_index_off as u64).to_le(),
        out_edges_off_le: (out_edges_off as u64).to_le(),
        out_edges_len_le: (input.out_edges.len() as u64).to_le(),

        node_atts_index_off_le: (node_atts_index_off as u64).to_le(),
        node_atts_off_le: (node_atts_off as u64).to_le(),
        node_atts_len_le: (input.node_atts.len() as u64).to_le(),

        edge_atts_index_off_le: (edge_atts_index_off as u64).to_le(),
        edge_atts_off_le: (edge_atts_off as u64).to_le(),
        edge_atts_len_le: (input.edge_atts.len() as u64).to_le(),

        blobs_off_le: (blobs_off as u64).to_le(),
        blobs_len_le: (blobs_size as u64).to_le(),
    };
    write_struct(&mut buf, &dir_entry)?;

    // Pad to nodes section
    write_padding(&mut buf, 8);

    // Write nodes
    for node in &input.nodes {
        write_struct(&mut buf, node)?;
    }
    write_padding(&mut buf, 8);

    // Write edges
    for edge in &input.edges {
        write_struct(&mut buf, edge)?;
    }
    write_padding(&mut buf, 8);

    // Write out_index
    for range in &input.out_index {
        write_struct(&mut buf, range)?;
    }
    write_padding(&mut buf, 8);

    // Write out_edges
    for out_edge in &input.out_edges {
        write_struct(&mut buf, out_edge)?;
    }
    write_padding(&mut buf, 8);

    // Write node_atts_index
    for range in &input.node_atts_index {
        write_struct(&mut buf, range)?;
    }
    write_padding(&mut buf, 8);

    // Write node_atts
    for att in &input.node_atts {
        write_struct(&mut buf, att)?;
    }
    write_padding(&mut buf, 8);

    // Write edge_atts_index
    for range in &input.edge_atts_index {
        write_struct(&mut buf, range)?;
    }
    write_padding(&mut buf, 8);

    // Write edge_atts
    for att in &input.edge_atts {
        write_struct(&mut buf, att)?;
    }
    write_padding(&mut buf, 8);

    // Write blobs
    buf.write_all(&input.blobs)?;

    assert_eq!(
        buf.len(),
        total_size,
        "WSC buffer size mismatch: expected {} bytes, got {}",
        total_size,
        buf.len()
    );

    Ok(buf)
}

/// Aligns a value up to the next 8-byte boundary.
#[inline]
const fn align8(n: usize) -> usize {
    (n + 7) & !7
}

/// Writes padding zeros until the buffer is aligned to the given boundary.
fn write_padding(buf: &mut Vec<u8>, align: usize) {
    let current = buf.len();
    let target = current.next_multiple_of(align);
    buf.resize(target, 0);
}

/// Writes a `Pod` struct as raw bytes using bytemuck for safe transmutation.
fn write_struct<T: Pod>(buf: &mut Vec<u8>, value: &T) -> io::Result<()> {
    buf.write_all(bytemuck::bytes_of(value))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn empty_warp_produces_valid_file() {
        let input = OneWarpInput {
            warp_id: [0u8; 32],
            root_node_id: [1u8; 32],
            nodes: vec![],
            edges: vec![],
            out_index: vec![],
            out_edges: vec![],
            node_atts_index: vec![],
            node_atts: vec![],
            edge_atts_index: vec![],
            edge_atts: vec![],
            blobs: vec![],
        };

        let bytes = write_wsc_one_warp(&input, [0u8; 32], 0).unwrap();

        // Should at least have header + directory entry
        assert!(bytes.len() >= std::mem::size_of::<WscHeader>());

        // Check magic
        assert_eq!(&bytes[0..8], WscHeader::MAGIC_V1);
    }

    #[test]
    fn align8_works() {
        assert_eq!(align8(0), 0);
        assert_eq!(align8(1), 8);
        assert_eq!(align8(7), 8);
        assert_eq!(align8(8), 8);
        assert_eq!(align8(9), 16);
    }
}
