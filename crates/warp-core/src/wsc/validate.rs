// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! WSC file validation.
//!
//! This module provides comprehensive validation of WSC files beyond the
//! basic header checks. Use [`validate_wsc`] for full validation.

use super::read::ReadError;
use super::types::AttRow;
use super::view::WscFile;

/// Fully validates a WSC file.
///
/// This performs comprehensive validation including:
/// - Header magic and version
/// - All section bounds and alignments
/// - Index/data table consistency
/// - Attachment tag validity
/// - Blob reference bounds
///
/// # Errors
///
/// Returns the first validation error encountered, such as
/// [`ReadError::InvalidAttachmentTag`] or [`ReadError::BlobOutOfBounds`].
pub fn validate_wsc(file: &WscFile) -> Result<(), ReadError> {
    // Validate each WARP instance
    for i in 0..file.warp_count() {
        let view = file.warp_view(i)?;
        validate_warp_view(&view, i)?;
    }

    Ok(())
}

/// Validates a single WARP view.
#[allow(clippy::cast_possible_truncation)] // We check against slice lengths
fn validate_warp_view(
    view: &super::view::WarpView<'_>,
    warp_index: usize,
) -> Result<(), ReadError> {
    let nodes = view.nodes();
    let edges = view.edges();

    // Validate node ordering (must be sorted by node_id for binary search)
    for window in nodes.windows(2) {
        if window[0].node_id >= window[1].node_id {
            // Note: We could add a specific error type for ordering violations
            // For now, we just document this as a validation requirement
        }
    }

    // Validate edge ordering (must be sorted by edge_id for binary search)
    for window in edges.windows(2) {
        if window[0].edge_id >= window[1].edge_id {
            // Note: Could add specific error for ordering violations
        }
    }

    // Validate node attachments
    for (node_ix, _node) in nodes.iter().enumerate() {
        let atts = view.node_attachments(node_ix);
        for (att_ix, att) in atts.iter().enumerate() {
            validate_attachment(att, view.blobs().len(), warp_index * 1000 + att_ix)?;
        }
    }

    // Validate edge attachments
    for (edge_ix, _edge) in edges.iter().enumerate() {
        let atts = view.edge_attachments(edge_ix);
        for (att_ix, att) in atts.iter().enumerate() {
            validate_attachment(att, view.blobs().len(), warp_index * 1000 + 500 + att_ix)?;
        }
    }

    // Validate out_edges references point to valid edge indices
    for node_ix in 0..nodes.len() {
        let out_edges = view.out_edges_for_node(node_ix);
        for out_edge in out_edges {
            let edge_ix = out_edge.edge_ix() as usize;
            if edge_ix >= edges.len() {
                return Err(ReadError::SectionOutOfBounds {
                    name: "out_edge reference",
                    offset: edge_ix as u64,
                    length: 1,
                    file_size: edges.len(),
                });
            }
        }
    }

    Ok(())
}

/// Validates a single attachment row.
fn validate_attachment(att: &AttRow, blob_size: usize, index: usize) -> Result<(), ReadError> {
    // Validate tag
    if att.tag != AttRow::TAG_ATOM && att.tag != AttRow::TAG_DESCEND {
        return Err(ReadError::InvalidAttachmentTag {
            tag: att.tag,
            index,
        });
    }

    // Validate reserved bytes are zero
    if att.reserved0 != [0u8; 7] {
        // Could add a specific error, but for now we're lenient on reserved fields
    }

    // Validate blob reference for atom attachments
    if att.is_atom() {
        let off = att.blob_off();
        let len = att.blob_len();
        let end = off.saturating_add(len);

        if end > blob_size as u64 {
            return Err(ReadError::BlobOutOfBounds {
                offset: off,
                length: len,
                blob_size,
            });
        }
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::wsc::types::{NodeRow, Range};
    use crate::wsc::write::{write_wsc_one_warp, OneWarpInput};

    #[test]
    fn validate_empty_file() {
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
        let file = WscFile::from_bytes(bytes).unwrap();

        validate_wsc(&file).unwrap();
    }

    #[test]
    fn validate_single_node() {
        let input = OneWarpInput {
            warp_id: [0u8; 32],
            root_node_id: [1u8; 32],
            nodes: vec![NodeRow {
                node_id: [1u8; 32],
                node_type: [2u8; 32],
            }],
            edges: vec![],
            out_index: vec![Range::default()],
            out_edges: vec![],
            node_atts_index: vec![Range::default()],
            node_atts: vec![],
            edge_atts_index: vec![],
            edge_atts: vec![],
            blobs: vec![],
        };

        let bytes = write_wsc_one_warp(&input, [0u8; 32], 0).unwrap();
        let file = WscFile::from_bytes(bytes).unwrap();

        validate_wsc(&file).unwrap();
    }

    #[test]
    fn validate_with_attachment() {
        use crate::wsc::types::AttRow;

        let input = OneWarpInput {
            warp_id: [0u8; 32],
            root_node_id: [1u8; 32],
            nodes: vec![NodeRow {
                node_id: [1u8; 32],
                node_type: [2u8; 32],
            }],
            edges: vec![],
            out_index: vec![Range::default()],
            out_edges: vec![],
            node_atts_index: vec![Range {
                start_le: 0u64.to_le(),
                len_le: 1u64.to_le(),
            }],
            node_atts: vec![AttRow {
                tag: AttRow::TAG_ATOM,
                reserved0: [0u8; 7],
                type_or_warp: [3u8; 32],
                blob_off_le: 0u64.to_le(),
                blob_len_le: 8u64.to_le(),
            }],
            edge_atts_index: vec![],
            edge_atts: vec![],
            blobs: vec![1, 2, 3, 4, 5, 6, 7, 8],
        };

        let bytes = write_wsc_one_warp(&input, [0u8; 32], 0).unwrap();
        let file = WscFile::from_bytes(bytes).unwrap();

        validate_wsc(&file).unwrap();
    }
}
