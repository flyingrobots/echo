// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Bounded host export and allocation-free structural validation of read evidence.
mod encode;
mod rows;
mod validate;
use crate::EchoOperationEvaluationBasisV1;
pub use rows::{EffectEdgePairRow, EffectReadRow, EffectSlotRow};
pub use validate::EffectReadSetView;

const DOMAIN: &[u8] = b"echo.bridge.read-set.v1\0";

#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ImageWord([u8; 8]);
const _: () =
    assert!(std::mem::size_of::<ImageWord>() == 8 && std::mem::align_of::<ImageWord>() == 8);

/// Host export or read-image validation failure, never an Action decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EffectReadImageError {
    /// Export requires a runtime-selected parent and a bound typed read law.
    #[error("read image requires a typed runtime-bound frame")]
    MissingTypedParent,
    /// The complete image cannot fit the selected host export ceiling.
    #[error("read image exceeds the host export limit")]
    ExportLimitExceeded,
    /// Host storage could not be provisioned; no evidence may be published.
    #[error("host could not provision read image storage")]
    HostAllocationFailed,
    /// The image violates its framing, tags, fields, ordering or packed ranges.
    #[error("invalid canonical read-set image")]
    InvalidImage,
    /// Zero-copy row access requires an eight-byte-aligned input buffer.
    #[error("read-set image buffer is not eight-byte aligned")]
    UnalignedImage,
    /// A recorded slot/value/type combination is inconsistent.
    #[error("recorded typed observation is inconsistent")]
    InconsistentObservation,
}

/// Owned, aligned export of a runtime frame's completed typed observations.
///
/// The bytes are the read-set digest preimage after its domain: row count,
/// 192-byte rows, pair count, then 64-byte edge pairs. This carries the actual
/// runtime parent separately; neither the image nor a structural view is an
/// installation, policy-authentication or historical-decision capability.
#[derive(Debug)]
pub struct EffectReadSetImage {
    words: Vec<ImageWord>,
    basis: EchoOperationEvaluationBasisV1,
    consumed: crate::EchoOperationBudgetV1,
    allocated_bytes: u64,
}
impl EffectReadSetImage {
    /// Canonical read-set bytes in an eight-byte-aligned owned buffer.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.words)
    }
    /// Runtime-selected full parent retained by the consuming frame export.
    #[must_use]
    pub const fn evaluation_basis(&self) -> EchoOperationEvaluationBasisV1 {
        self.basis
    }
    /// Evaluation work and I/O preserved before the consuming host export.
    #[must_use]
    pub const fn consumed(&self) -> crate::EchoOperationBudgetV1 {
        self.consumed
    }
    /// Evaluation storage preserved separately from host export buffer storage.
    #[must_use]
    pub const fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes
    }
    /// BLAKE3 over the read-set domain and exact canonical image bytes.
    #[must_use]
    pub fn digest(&self) -> crate::Hash {
        digest(self.as_bytes())
    }
}
fn digest(bytes: &[u8]) -> crate::Hash {
    let mut hash = blake3::Hasher::new();
    hash.update(DOMAIN);
    hash.update(bytes);
    hash.finalize().into()
}
pub(super) use encode::export;
