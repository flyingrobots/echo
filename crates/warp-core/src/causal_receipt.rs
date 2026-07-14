// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Stable causal coordinates for admitted tick-receipt events.

use crate::{GlobalTick, Hash, WorldlineId, WorldlineTick};

const CAUSAL_TICK_RECEIPT_REF_DOMAIN: &[u8] = b"echo:causal-tick-receipt-ref:v1\0";

/// Canonical byte length of a retained [`CausalTickReceiptRef`].
pub const CAUSAL_TICK_RECEIPT_REF_LEN: usize = 5 * core::mem::size_of::<Hash>() + 2 * 8;

/// Exact causal coordinate of one admitted scheduler tick receipt.
///
/// [`crate::TickReceipt::digest`] commits to decision content and can repeat.
/// This reference binds that content to one admitted worldline transition.
/// Possessing a reference is not proof that the transition exists; consumers
/// must resolve every field against retained provenance and receipt-correlation
/// evidence before granting authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CausalTickReceiptRef {
    /// Worldline whose frontier advanced.
    pub worldline_id: WorldlineId,
    /// Worldline frontier tick after the admitted transition.
    pub worldline_tick_after: WorldlineTick,
    /// Runtime-global logical cycle that committed the transition.
    pub commit_global_tick: GlobalTick,
    /// Commit hash of the admitted transition.
    pub commit_hash: Hash,
    /// Witnessed submission decided by the transition.
    pub submission_id: Hash,
    /// Admission ticket bound to runtime ingress.
    pub ticket_digest: Hash,
    /// Transaction-independent tick-receipt content commitment.
    pub receipt_content_digest: Hash,
}

impl CausalTickReceiptRef {
    /// Returns a domain-separated identity digest for this exact coordinate.
    #[must_use]
    pub fn identity_digest(self) -> Hash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(CAUSAL_TICK_RECEIPT_REF_DOMAIN);
        hasher.update(&self.to_canonical_bytes());
        hasher.finalize().into()
    }

    /// Encodes this coordinate as canonical fixed-width bytes.
    #[must_use]
    pub fn to_canonical_bytes(self) -> [u8; CAUSAL_TICK_RECEIPT_REF_LEN] {
        let mut bytes = [0; CAUSAL_TICK_RECEIPT_REF_LEN];
        let mut offset = 0;
        bytes[offset..offset + core::mem::size_of::<Hash>()]
            .copy_from_slice(self.worldline_id.as_bytes());
        offset += core::mem::size_of::<Hash>();
        bytes[offset..offset + 8]
            .copy_from_slice(&self.worldline_tick_after.as_u64().to_le_bytes());
        offset += 8;
        bytes[offset..offset + 8].copy_from_slice(&self.commit_global_tick.as_u64().to_le_bytes());
        offset += 8;
        for field in [
            self.commit_hash,
            self.submission_id,
            self.ticket_digest,
            self.receipt_content_digest,
        ] {
            bytes[offset..offset + field.len()].copy_from_slice(&field);
            offset += field.len();
        }
        bytes
    }

    /// Decodes one canonical fixed-width coordinate.
    #[must_use]
    pub fn from_canonical_bytes(bytes: [u8; CAUSAL_TICK_RECEIPT_REF_LEN]) -> Self {
        let mut offset = 0;
        let mut worldline_bytes = [0; core::mem::size_of::<Hash>()];
        worldline_bytes.copy_from_slice(&bytes[offset..offset + core::mem::size_of::<Hash>()]);
        let worldline_id = WorldlineId::from_bytes(worldline_bytes);
        offset += core::mem::size_of::<Hash>();
        let mut tick_bytes = [0; 8];
        tick_bytes.copy_from_slice(&bytes[offset..offset + 8]);
        let worldline_tick_after = WorldlineTick::from_raw(u64::from_le_bytes(tick_bytes));
        offset += 8;
        tick_bytes.copy_from_slice(&bytes[offset..offset + 8]);
        let commit_global_tick = GlobalTick::from_raw(u64::from_le_bytes(tick_bytes));
        offset += 8;
        let mut read_hash = || {
            let mut value = [0; core::mem::size_of::<Hash>()];
            value.copy_from_slice(&bytes[offset..offset + core::mem::size_of::<Hash>()]);
            offset += core::mem::size_of::<Hash>();
            value
        };
        let commit_hash = read_hash();
        let submission_id = read_hash();
        let ticket_digest = read_hash();
        let receipt_content_digest = read_hash();
        Self {
            worldline_id,
            worldline_tick_after,
            commit_global_tick,
            commit_hash,
            submission_id,
            ticket_digest,
            receipt_content_digest,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(tick: u64) -> CausalTickReceiptRef {
        CausalTickReceiptRef {
            worldline_id: WorldlineId::from_bytes([1; 32]),
            worldline_tick_after: WorldlineTick::from_raw(tick),
            commit_global_tick: GlobalTick::from_raw(tick + 10),
            commit_hash: [2; 32],
            submission_id: [3; 32],
            ticket_digest: [4; 32],
            receipt_content_digest: [5; 32],
        }
    }

    #[test]
    fn canonical_bytes_round_trip_exact_coordinate() {
        let reference = reference(7);
        assert_eq!(
            CausalTickReceiptRef::from_canonical_bytes(reference.to_canonical_bytes()),
            reference
        );
    }

    #[test]
    fn identity_distinguishes_equal_receipt_content_at_different_ticks() {
        assert_ne!(
            reference(7).identity_digest(),
            reference(8).identity_digest()
        );
    }
}
