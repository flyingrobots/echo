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
        validate_warp_view(&view)?;
    }

    Ok(())
}

/// Validates a single WARP view.
fn validate_warp_view(view: &super::view::WarpView<'_>) -> Result<(), ReadError> {
    // Validate index ranges first - this catches corrupted index tables
    // that would otherwise be silently masked by accessors returning empty slices
    view.validate_index_ranges()?;

    let nodes = view.nodes();
    let edges = view.edges();

    // Validate node ordering (must be sorted by node_id for binary search)
    for (i, window) in nodes.windows(2).enumerate() {
        if window[0].node_id >= window[1].node_id {
            return Err(ReadError::OrderingViolation {
                kind: "node",
                index: i + 1,
            });
        }
    }

    // Validate edge ordering (must be sorted by edge_id for binary search)
    for (i, window) in edges.windows(2).enumerate() {
        if window[0].edge_id >= window[1].edge_id {
            return Err(ReadError::OrderingViolation {
                kind: "edge",
                index: i + 1,
            });
        }
    }

    // Validate root_node_id exists in nodes array
    let root_id = *view.root_node_id();
    if !nodes.is_empty() {
        // Binary search for root node (nodes are sorted by node_id)
        let found = nodes.binary_search_by_key(&root_id, |n| n.node_id).is_ok();
        if !found {
            return Err(ReadError::MissingRoot { root: root_id });
        }
    } else if root_id != [0u8; 32] {
        // Empty nodes array but non-zero root - only valid if root is also zeroed
        return Err(ReadError::MissingRoot { root: root_id });
    }

    // Validate node attachments using a monotonically increasing index
    // to avoid collisions from the previous warp_index * 1000 scheme.
    let mut att_index = 0usize;
    for node_ix in 0..nodes.len() {
        for att in view.node_attachments(node_ix) {
            validate_attachment(att, view.blobs().len(), att_index)?;
            att_index += 1;
        }
    }

    // Validate edge attachments (continuing the monotonic index)
    for edge_ix in 0..edges.len() {
        for att in view.edge_attachments(edge_ix) {
            validate_attachment(att, view.blobs().len(), att_index)?;
            att_index += 1;
        }
    }

    // Validate out_edges references point to valid edge indices.
    // Important: Compare as u64 BEFORE casting to usize to avoid truncation
    // on 32-bit targets where usize is 32 bits.
    let edges_len_u64 = edges.len() as u64;
    for node_ix in 0..nodes.len() {
        let out_edges = view.out_edges_for_node(node_ix);
        for out_edge in out_edges {
            let edge_ix = out_edge.edge_ix();
            if edge_ix >= edges_len_u64 {
                return Err(ReadError::SectionOutOfBounds {
                    name: "out_edge reference",
                    offset: edge_ix,
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
        return Err(ReadError::NonZeroReservedBytes {
            field: "AttRow.reserved0",
            index,
        });
    }

    // Validate blob reference fields
    let off = att.blob_off();
    let len = att.blob_len();

    if att.is_atom() {
        let end = off.saturating_add(len);
        if end > blob_size as u64 {
            return Err(ReadError::BlobOutOfBounds {
                offset: off,
                length: len,
                blob_size,
            });
        }
    } else {
        // Non-ATOM attachments (DESCEND) must have zero blob fields
        if off != 0 || len != 0 {
            return Err(ReadError::NonAtomHasBlobFields {
                index,
                blob_off: off,
                blob_len: len,
            });
        }
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::wsc::types::{EdgeRow, NodeRow, Range};
    use crate::wsc::write::{write_wsc_one_warp, OneWarpInput};

    #[test]
    fn validate_empty_file() {
        // An empty file must have a zero root_node_id (no nodes = no valid root)
        let input = OneWarpInput {
            warp_id: [0u8; 32],
            root_node_id: [0u8; 32], // Zero root for empty nodes
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

    #[test]
    fn validate_rejects_out_of_bounds_node_atts_range() {
        // Create a file where node_atts_index points past node_atts
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
            // Range points to indices 0..5, but node_atts is empty
            node_atts_index: vec![Range {
                start_le: 0u64.to_le(),
                len_le: 5u64.to_le(),
            }],
            node_atts: vec![], // Empty! Range extends past this
            edge_atts_index: vec![],
            edge_atts: vec![],
            blobs: vec![],
        };

        let bytes = write_wsc_one_warp(&input, [0u8; 32], 0).unwrap();
        let file = WscFile::from_bytes(bytes).unwrap();

        let err = validate_wsc(&file).unwrap_err();
        assert!(
            matches!(
                err,
                ReadError::IndexRangeOutOfBounds {
                    index_name: "node_atts_index",
                    ..
                }
            ),
            "expected IndexRangeOutOfBounds for node_atts_index, got: {err:?}"
        );
    }

    #[test]
    fn validate_rejects_out_of_bounds_out_index_range() {
        // Create a file where out_index points past out_edges
        let input = OneWarpInput {
            warp_id: [0u8; 32],
            root_node_id: [1u8; 32],
            nodes: vec![NodeRow {
                node_id: [1u8; 32],
                node_type: [2u8; 32],
            }],
            edges: vec![],
            // Range points to indices 10..13, but out_edges is empty
            out_index: vec![Range {
                start_le: 10u64.to_le(),
                len_le: 3u64.to_le(),
            }],
            out_edges: vec![], // Empty! Range extends past this
            node_atts_index: vec![Range::default()],
            node_atts: vec![],
            edge_atts_index: vec![],
            edge_atts: vec![],
            blobs: vec![],
        };

        let bytes = write_wsc_one_warp(&input, [0u8; 32], 0).unwrap();
        let file = WscFile::from_bytes(bytes).unwrap();

        let err = validate_wsc(&file).unwrap_err();
        assert!(
            matches!(
                err,
                ReadError::IndexRangeOutOfBounds {
                    index_name: "out_index",
                    ..
                }
            ),
            "expected IndexRangeOutOfBounds for out_index, got: {err:?}"
        );
    }

    #[test]
    fn validate_rejects_unordered_nodes() {
        let input = OneWarpInput {
            warp_id: [0u8; 32],
            root_node_id: [1u8; 32],
            nodes: vec![
                NodeRow {
                    node_id: [2u8; 32],
                    node_type: [0u8; 32],
                },
                NodeRow {
                    node_id: [1u8; 32],
                    node_type: [0u8; 32],
                }, // Out of order!
            ],
            edges: vec![],
            out_index: vec![Range::default(), Range::default()],
            out_edges: vec![],
            node_atts_index: vec![Range::default(), Range::default()],
            node_atts: vec![],
            edge_atts_index: vec![],
            edge_atts: vec![],
            blobs: vec![],
        };
        let bytes = write_wsc_one_warp(&input, [0u8; 32], 0).unwrap();
        let file = WscFile::from_bytes(bytes).unwrap();
        let err = validate_wsc(&file).unwrap_err();
        assert!(
            matches!(err, ReadError::OrderingViolation { kind: "node", .. }),
            "expected OrderingViolation for node, got: {err:?}"
        );
    }

    #[test]
    fn validate_rejects_invalid_attachment_tag() {
        let input = OneWarpInput {
            warp_id: [0u8; 32],
            root_node_id: [1u8; 32],
            nodes: vec![NodeRow {
                node_id: [1u8; 32],
                node_type: [0u8; 32],
            }],
            edges: vec![],
            out_index: vec![Range::default()],
            out_edges: vec![],
            node_atts_index: vec![Range {
                start_le: 0u64.to_le(),
                len_le: 1u64.to_le(),
            }],
            node_atts: vec![AttRow {
                tag: 0xFF, // Invalid tag!
                reserved0: [0u8; 7],
                type_or_warp: [0u8; 32],
                blob_off_le: 0,
                blob_len_le: 0,
            }],
            edge_atts_index: vec![],
            edge_atts: vec![],
            blobs: vec![],
        };
        let bytes = write_wsc_one_warp(&input, [0u8; 32], 0).unwrap();
        let file = WscFile::from_bytes(bytes).unwrap();
        let err = validate_wsc(&file).unwrap_err();
        assert!(
            matches!(err, ReadError::InvalidAttachmentTag { .. }),
            "expected InvalidAttachmentTag, got: {err:?}"
        );
    }

    #[test]
    fn validate_rejects_nonzero_reserved_bytes() {
        let input = OneWarpInput {
            warp_id: [0u8; 32],
            root_node_id: [1u8; 32],
            nodes: vec![NodeRow {
                node_id: [1u8; 32],
                node_type: [0u8; 32],
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
                reserved0: [1, 2, 3, 4, 5, 6, 7], // Non-zero!
                type_or_warp: [0u8; 32],
                blob_off_le: 0,
                blob_len_le: 0,
            }],
            edge_atts_index: vec![],
            edge_atts: vec![],
            blobs: vec![],
        };
        let bytes = write_wsc_one_warp(&input, [0u8; 32], 0).unwrap();
        let file = WscFile::from_bytes(bytes).unwrap();
        let err = validate_wsc(&file).unwrap_err();
        assert!(
            matches!(err, ReadError::NonZeroReservedBytes { .. }),
            "expected NonZeroReservedBytes, got: {err:?}"
        );
    }

    #[test]
    fn validate_rejects_out_edges_with_large_u64_index() {
        // This test verifies that edge indices are validated as u64 values
        // before being cast to usize. On 32-bit targets, casting a large u64
        // to usize would truncate, potentially passing an invalid index through
        // the bounds check.
        //
        // For example: 0x1_0000_0001_u64 truncated to 32-bit usize becomes 1,
        // which could incorrectly pass a bounds check against a small edge array.
        use crate::wsc::types::OutEdgeRef;

        // Create a large u64 index that exceeds u32::MAX
        // This value truncated to u32 becomes 1, which would be in-bounds
        // for an edge array of size 2 if the check was done after truncation.
        let large_edge_ix: u64 = 0x1_0000_0001;

        let input = OneWarpInput {
            warp_id: [0u8; 32],
            root_node_id: [1u8; 32],
            nodes: vec![NodeRow {
                node_id: [1u8; 32],
                node_type: [0u8; 32],
            }],
            edges: vec![], // Empty edge table - any index is out of bounds
            out_index: vec![Range {
                start_le: 0u64.to_le(),
                len_le: 1u64.to_le(),
            }],
            out_edges: vec![OutEdgeRef {
                edge_ix_le: large_edge_ix.to_le(),
                edge_id: [0u8; 32],
            }],
            node_atts_index: vec![Range::default()],
            node_atts: vec![],
            edge_atts_index: vec![],
            edge_atts: vec![],
            blobs: vec![],
        };

        let bytes = write_wsc_one_warp(&input, [0u8; 32], 0).unwrap();
        let file = WscFile::from_bytes(bytes).unwrap();
        let err = validate_wsc(&file).unwrap_err();

        // Verify we get the right error type with the ORIGINAL u64 offset,
        // not a truncated value. This catches the truncation bug.
        assert!(
            matches!(
                err,
                ReadError::SectionOutOfBounds {
                    name: "out_edge reference",
                    offset,
                    length: 1,
                    ..
                } if offset == large_edge_ix
            ),
            "expected SectionOutOfBounds with offset {large_edge_ix:#x}, got: {err:?}"
        );
    }

    #[test]
    fn validate_rejects_unordered_edges() {
        let input = OneWarpInput {
            warp_id: [0u8; 32],
            root_node_id: [1u8; 32],
            nodes: vec![],
            edges: vec![
                EdgeRow {
                    edge_id: [2u8; 32],
                    from_node_id: [0u8; 32],
                    to_node_id: [0u8; 32],
                    edge_type: [0u8; 32],
                },
                EdgeRow {
                    edge_id: [1u8; 32], // Out of order!
                    from_node_id: [0u8; 32],
                    to_node_id: [0u8; 32],
                    edge_type: [0u8; 32],
                },
            ],
            out_index: vec![],
            out_edges: vec![],
            node_atts_index: vec![],
            node_atts: vec![],
            edge_atts_index: vec![Range::default(), Range::default()],
            edge_atts: vec![],
            blobs: vec![],
        };
        let bytes = write_wsc_one_warp(&input, [0u8; 32], 0).unwrap();
        let file = WscFile::from_bytes(bytes).unwrap();
        let err = validate_wsc(&file).unwrap_err();
        assert!(
            matches!(err, ReadError::OrderingViolation { kind: "edge", .. }),
            "expected OrderingViolation for edge, got: {err:?}"
        );
    }

    #[test]
    fn validate_rejects_blob_out_of_bounds() {
        let input = OneWarpInput {
            warp_id: [0u8; 32],
            root_node_id: [1u8; 32],
            nodes: vec![NodeRow {
                node_id: [1u8; 32],
                node_type: [0u8; 32],
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
                type_or_warp: [0u8; 32],
                blob_off_le: 0u64.to_le(),
                blob_len_le: 100u64.to_le(), // Claims 100 bytes but blobs is only 8!
            }],
            edge_atts_index: vec![],
            edge_atts: vec![],
            blobs: vec![1, 2, 3, 4, 5, 6, 7, 8], // Only 8 bytes
        };
        let bytes = write_wsc_one_warp(&input, [0u8; 32], 0).unwrap();
        let file = WscFile::from_bytes(bytes).unwrap();
        let err = validate_wsc(&file).unwrap_err();
        assert!(
            matches!(err, ReadError::BlobOutOfBounds { .. }),
            "expected BlobOutOfBounds, got: {err:?}"
        );
    }

    #[test]
    fn validate_rejects_descend_with_nonzero_blob_fields() {
        let input = OneWarpInput {
            warp_id: [0u8; 32],
            root_node_id: [1u8; 32],
            nodes: vec![NodeRow {
                node_id: [1u8; 32],
                node_type: [0u8; 32],
            }],
            edges: vec![],
            out_index: vec![Range::default()],
            out_edges: vec![],
            node_atts_index: vec![Range {
                start_le: 0u64.to_le(),
                len_le: 1u64.to_le(),
            }],
            node_atts: vec![AttRow {
                tag: AttRow::TAG_DESCEND,
                reserved0: [0u8; 7],
                type_or_warp: [0u8; 32],
                blob_off_le: 42u64.to_le(), // Non-zero! Invalid for DESCEND
                blob_len_le: 100u64.to_le(), // Non-zero! Invalid for DESCEND
            }],
            edge_atts_index: vec![],
            edge_atts: vec![],
            blobs: vec![],
        };
        let bytes = write_wsc_one_warp(&input, [0u8; 32], 0).unwrap();
        let file = WscFile::from_bytes(bytes).unwrap();
        let err = validate_wsc(&file).unwrap_err();
        assert!(
            matches!(err, ReadError::NonAtomHasBlobFields { .. }),
            "expected NonAtomHasBlobFields, got: {err:?}"
        );
    }

    #[test]
    fn validate_rejects_missing_root_node() {
        let input = OneWarpInput {
            warp_id: [0u8; 32],
            root_node_id: [99u8; 32], // Root ID that doesn't exist in nodes
            nodes: vec![NodeRow {
                node_id: [1u8; 32], // Different from root!
                node_type: [0u8; 32],
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
        let err = validate_wsc(&file).unwrap_err();
        assert!(
            matches!(err, ReadError::MissingRoot { .. }),
            "expected MissingRoot, got: {err:?}"
        );
    }

    #[test]
    fn validate_accepts_valid_root_node() {
        let input = OneWarpInput {
            warp_id: [0u8; 32],
            root_node_id: [1u8; 32], // Root exists in nodes
            nodes: vec![NodeRow {
                node_id: [1u8; 32], // Same as root
                node_type: [0u8; 32],
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
        validate_wsc(&file).unwrap(); // Should pass
    }
}
