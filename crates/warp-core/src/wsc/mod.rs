// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! WSC (Write-Streaming Columnar) snapshot format.
//!
//! This module provides deterministic serialization and zero-copy deserialization
//! of WARP graph state. The format is designed for:
//!
//! - **Determinism**: Identical graph content produces identical bytes
//! - **Zero-copy access**: Memory-mapped files can be read without copying
//! - **Columnar layout**: Efficient for batch operations and SIMD
//! - **8-byte alignment**: All sections are aligned for efficient access
//!
//! # Overview
//!
//! A WSC file contains:
//! - A fixed-size header with file-level metadata
//! - A WARP directory listing all instances in the file
//! - Per-instance sections: nodes, edges, attachments, blobs
//!
//! # Usage
//!
//! ## Writing
//!
//! ```rust,no_run
//! use warp_core::{blake3_empty, make_node_id, make_type_id, GraphStore, NodeRecord};
//! use warp_core::wsc::{build_one_warp_input, write_wsc_one_warp};
//!
//! # fn main() -> Result<(), std::io::Error> {
//! let mut store = GraphStore::default();
//! let root_node_id = make_node_id("root");
//! store.insert_node(root_node_id, NodeRecord { ty: make_type_id("world") });
//!
//! let input = build_one_warp_input(&store, root_node_id);
//! let bytes = write_wsc_one_warp(&input, blake3_empty(), 0)?;
//! # let _bytes = bytes;
//! # Ok(())
//! # }
//! ```
//!
//! ## Reading
//!
//! ```rust,no_run
//! use warp_core::wsc::{validate_wsc, ReadError, WscFile};
//!
//! # fn main() -> Result<(), ReadError> {
//! let file = WscFile::open("state.wsc")?;
//! validate_wsc(&file)?;
//!
//! let view = file.warp_view(0)?;
//! for node in view.nodes() {
//!     println!("Node: {:?}", node.node_id);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Format Specification
//!
//! See the [`types`](crate::wsc::types) module for the binary layout of all row types.

pub mod build;
pub mod read;
pub mod store;
pub mod types;
pub mod validate;
pub mod view;
pub mod write;

// Re-exports for convenient access
pub use build::build_one_warp_input;
pub use read::ReadError;
pub use store::{
    accepted_submission_records_from_wsc_envelope, accepted_submission_records_from_wsc_store,
    accepted_submission_records_to_wsc_envelope, receipt_correlation_records_from_wsc_envelope,
    receipt_correlation_records_from_wsc_store, receipt_correlation_records_to_wsc_envelope,
    retention_records_from_wsc_envelope, retention_records_from_wsc_store,
    retention_records_to_wsc_envelope, topology_records_from_wsc_envelope,
    topology_records_from_wsc_store, topology_records_to_wsc_envelope,
    validate_wsc_causal_history_store, validate_wsc_ref_only_wal_export,
    validate_wsc_self_contained_wal_export, wsc_causal_history_export_profile,
    wsc_causal_history_export_profiles, wsc_ref_only_wal_export, wsc_self_contained_wal_export,
    InMemoryWscStore, WscCausalHistoryCasAuthority, WscCausalHistoryExportEvidence,
    WscCausalHistoryExportProfile, WscCausalHistoryExportProfileKind,
    WscCausalHistoryExportValidationMaterial, WscReceiptCorrelationRecords, WscRefOnlyWalExport,
    WscRefOnlyWalExportError, WscRefOnlyWalImport, WscRefOnlyWalImportError,
    WscRefOnlyWalLocatorPosture, WscRefOnlyWalMaterialDependency, WscRefOnlyWalSegmentDependency,
    WscRetentionRecords, WscSelfContainedWalExport, WscSelfContainedWalExportError,
    WscSelfContainedWalImport, WscSelfContainedWalImportError, WscSelfContainedWalSegmentMaterial,
    WscStoreEnvelope, WscStoreEnvelopeId, WscStoreObstruction, WscStoreObstructionKind,
    WscStorePort, WscStoreRecordKind, WscStoreSubject, WscStoreWriteReceipt, WscTopologyRecords,
    WSC_CAUSAL_HISTORY_EXPORT_PROFILE_VERSION,
};
pub use validate::validate_wsc;
pub use view::{AttachmentRef, WarpView, WscFile};
pub use write::{write_wsc_one_warp, OneWarpInput};
