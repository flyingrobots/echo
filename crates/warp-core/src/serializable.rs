// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! UI-friendly serializable wrappers for ledger artifacts.
//!
//! These types can be serialized using echo-wasm-abi's deterministic CBOR encoder.

use crate::ident::NodeKey;
use crate::receipt::{TickReceipt, TickReceiptDisposition};
use crate::snapshot::Snapshot;
use crate::tick_patch::WarpTickPatchV1;
use crate::TxId;

/// A UI-friendly wrapper for a single tick's ledger entry.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SerializableTick {
    /// Snapshot metadata.
    pub snapshot: SerializableSnapshot,
    /// Receipt metadata.
    pub receipt: SerializableReceipt,
    /// The actual state patch delta.
    pub patch: WarpTickPatchV1,
}

/// UI-friendly snapshot metadata.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SerializableSnapshot {
    /// Root node of the snapshot.
    pub root: NodeKey,
    /// Raw commit hash.
    pub hash: [u8; 32],
    /// Hex-encoded commit hash.
    pub hash_hex: String,
    /// Transaction ID.
    pub tx: TxId,
}

/// UI-friendly receipt metadata.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SerializableReceipt {
    /// Transaction ID.
    pub tx: TxId,
    /// Individual candidate outcomes.
    pub entries: Vec<SerializableReceiptEntry>,
}

/// UI-friendly receipt entry.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SerializableReceiptEntry {
    /// Raw rule family ID.
    pub rule_id: [u8; 32],
    /// Short hex representation of the rule ID.
    pub rule_id_short: String,
    /// Scope node.
    pub scope: NodeKey,
    /// Acceptance/rejection disposition.
    pub disposition: TickReceiptDisposition,
}

impl SerializableTick {
    /// Constructs a serializable tick from its raw engine components.
    #[must_use]
    pub fn from_parts(snapshot: &Snapshot, receipt: &TickReceipt, patch: &WarpTickPatchV1) -> Self {
        Self {
            snapshot: SerializableSnapshot {
                root: snapshot.root,
                hash: snapshot.hash,
                hash_hex: hex::encode(snapshot.hash),
                tx: snapshot.tx,
            },
            receipt: SerializableReceipt {
                tx: receipt.tx(),
                entries: receipt
                    .entries()
                    .iter()
                    .map(|e| SerializableReceiptEntry {
                        rule_id: e.rule_id,
                        rule_id_short: hex::encode(&e.rule_id[0..8]),
                        scope: e.scope,
                        disposition: e.disposition,
                    })
                    .collect(),
            },
            patch: patch.clone(),
        }
    }
}
