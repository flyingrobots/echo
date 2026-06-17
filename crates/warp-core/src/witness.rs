// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Witness receipt boundary and deterministic simulator fixtures.

use blake3::Hasher;
use thiserror::Error;

use crate::ident::Hash;

const WITNESS_RECEIPT_DOMAIN: &[u8] = b"echo.witness.receipt.v1\0";

/// Witness receipt family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WitnessKind {
    /// E1 local self-witness: deterministic integrity evidence only.
    SelfWitness,
    /// Independent signature-backed witness receipt.
    SignedWitness,
    /// Threshold-backed witness receipt.
    ThresholdWitness,
    /// Runtime attestation receipt.
    RuntimeAttestation,
    /// Replay trace receipt emitted by a replay witness backend.
    ReplayTraceReceipt,
    /// ZK verifier receipt.
    ZkVerifierReceipt,
    /// Vector-opening verifier receipt.
    VectorOpeningReceipt,
}

impl WitnessKind {
    /// Stable wire tag for canonical witness receipt hashing.
    #[must_use]
    pub const fn canonical_tag(self) -> u8 {
        match self {
            Self::SelfWitness => 0x01,
            Self::SignedWitness => 0x02,
            Self::ThresholdWitness => 0x03,
            Self::RuntimeAttestation => 0x04,
            Self::ReplayTraceReceipt => 0x05,
            Self::ZkVerifierReceipt => 0x06,
            Self::VectorOpeningReceipt => 0x07,
        }
    }
}

/// Attestation strength claimed by a witness receipt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WitnessAttestation {
    /// Integrity-only evidence. This is not independent attestation.
    IntegrityOnly,
    /// A backend claims independent attestation.
    IndependentAttestation,
}

impl WitnessAttestation {
    const fn canonical_tag(self) -> u8 {
        match self {
            Self::IntegrityOnly => 0x01,
            Self::IndependentAttestation => 0x02,
        }
    }
}

/// Compatibility rule governing witness receipt identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WitnessCompatibilityRule {
    /// Public stable v1 identity. Changes require migration handling.
    StableV1,
    /// E1 scaffolding identity. May change with an explicit compatibility note.
    E1Scaffold,
    /// Identity change requiring a named migration.
    RequiresMigration {
        /// Source compatibility version.
        from: u32,
        /// Target compatibility version.
        to: u32,
    },
}

impl WitnessCompatibilityRule {
    fn hash_into(self, hasher: &mut Hasher) {
        match self {
            Self::StableV1 => {
                hasher.update(&[0x01]);
            }
            Self::E1Scaffold => {
                hasher.update(&[0x02]);
            }
            Self::RequiresMigration { from, to } => {
                hasher.update(&[0x03]);
                hasher.update(&from.to_le_bytes());
                hasher.update(&to.to_le_bytes());
            }
        }
    }
}

/// Receipt returned by a witness backend.
///
/// A receipt names the subject being witnessed, the evidence material digest,
/// the witness family, the attestation claim, and the compatibility rule that
/// governs the receipt identity. Self-witness receipts are integrity-only
/// scaffolding unless an external backend returns a stronger receipt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WitnessReceipt {
    /// Witness family.
    kind: WitnessKind,
    /// Digest of the subject being witnessed.
    subject_digest: Hash,
    /// Digest of the witness evidence material.
    evidence_digest: Hash,
    /// Compatibility rule governing receipt identity.
    compatibility: WitnessCompatibilityRule,
    /// Attestation strength claimed by this receipt.
    attestation: WitnessAttestation,
}

impl WitnessReceipt {
    /// Creates a witness receipt with an explicit compatibility rule.
    ///
    /// # Errors
    ///
    /// Returns [`WitnessError::UnsupportedCompatibility`] or
    /// [`WitnessError::UnsupportedAttestation`] when a self-witness receipt tries
    /// to claim more than E1 integrity-only scaffolding.
    pub fn new(
        kind: WitnessKind,
        subject_digest: Hash,
        evidence_digest: Hash,
        compatibility: WitnessCompatibilityRule,
        attestation: WitnessAttestation,
    ) -> Result<Self, WitnessError> {
        if kind == WitnessKind::SelfWitness {
            if compatibility != WitnessCompatibilityRule::E1Scaffold {
                return Err(WitnessError::UnsupportedCompatibility {
                    kind,
                    compatibility,
                });
            }
            if attestation != WitnessAttestation::IntegrityOnly {
                return Err(WitnessError::UnsupportedAttestation { kind, attestation });
            }
        }
        Ok(Self {
            kind,
            subject_digest,
            evidence_digest,
            compatibility,
            attestation,
        })
    }

    /// Creates an E1 self-witness receipt.
    ///
    /// The returned receipt claims only local integrity evidence, not
    /// independent attestation.
    #[must_use]
    pub const fn self_witness(subject_digest: Hash, evidence_digest: Hash) -> Self {
        Self {
            kind: WitnessKind::SelfWitness,
            subject_digest,
            evidence_digest,
            compatibility: WitnessCompatibilityRule::E1Scaffold,
            attestation: WitnessAttestation::IntegrityOnly,
        }
    }

    /// Returns the witness family.
    #[must_use]
    pub const fn kind(self) -> WitnessKind {
        self.kind
    }

    /// Returns the digest of the subject being witnessed.
    #[must_use]
    pub const fn subject_digest(self) -> Hash {
        self.subject_digest
    }

    /// Returns the digest of the witness evidence material.
    #[must_use]
    pub const fn evidence_digest(self) -> Hash {
        self.evidence_digest
    }

    /// Returns the compatibility rule governing receipt identity.
    #[must_use]
    pub const fn compatibility(self) -> WitnessCompatibilityRule {
        self.compatibility
    }

    /// Returns the attestation strength claimed by this receipt.
    #[must_use]
    pub const fn attestation(self) -> WitnessAttestation {
        self.attestation
    }

    /// Returns the canonical receipt digest.
    #[must_use]
    pub fn digest(&self) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(WITNESS_RECEIPT_DOMAIN);
        hasher.update(&[self.kind.canonical_tag()]);
        hasher.update(&self.subject_digest);
        hasher.update(&self.evidence_digest);
        self.compatibility.hash_into(&mut hasher);
        hasher.update(&[self.attestation.canonical_tag()]);
        hasher.finalize().into()
    }
}

/// Request submitted to a witness backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WitnessRequest {
    /// Requested witness family.
    pub kind: WitnessKind,
    /// Digest of the subject being witnessed.
    pub subject_digest: Hash,
    /// Digest of the witness evidence material.
    pub evidence_digest: Hash,
    /// Compatibility rule the caller expects the receipt to bind.
    pub compatibility: WitnessCompatibilityRule,
}

impl WitnessRequest {
    /// Creates a witness request with an explicit compatibility rule.
    #[must_use]
    pub const fn new(
        kind: WitnessKind,
        subject_digest: Hash,
        evidence_digest: Hash,
        compatibility: WitnessCompatibilityRule,
    ) -> Self {
        Self {
            kind,
            subject_digest,
            evidence_digest,
            compatibility,
        }
    }

    /// Creates an E1 self-witness request.
    #[must_use]
    pub const fn self_witness(subject_digest: Hash, evidence_digest: Hash) -> Self {
        Self::new(
            WitnessKind::SelfWitness,
            subject_digest,
            evidence_digest,
            WitnessCompatibilityRule::E1Scaffold,
        )
    }
}

/// Future witness backend rejection code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WitnessRejectionCode {
    /// A witness backend rejected the request without a narrower public reason.
    Rejected,
}

/// Structured witness backend failure.
#[derive(Error, Clone, Copy, Debug, PartialEq, Eq)]
pub enum WitnessError {
    /// No backend is wired for this witness kind.
    #[error("{kind:?} witness receipts require a backend before admission")]
    UnsupportedBackend {
        /// Unsupported witness kind.
        kind: WitnessKind,
    },
    /// The requested compatibility rule is not valid for this witness kind.
    #[error("{kind:?} witness receipts do not support compatibility {compatibility:?}")]
    UnsupportedCompatibility {
        /// Witness kind that rejected the compatibility rule.
        kind: WitnessKind,
        /// Unsupported compatibility rule.
        compatibility: WitnessCompatibilityRule,
    },
    /// The requested attestation strength is not valid for this witness kind.
    #[error("{kind:?} witness receipts do not support attestation {attestation:?}")]
    UnsupportedAttestation {
        /// Witness kind that rejected the attestation strength.
        kind: WitnessKind,
        /// Unsupported attestation strength.
        attestation: WitnessAttestation,
    },
    /// A witness backend rejected the request.
    #[error("{kind:?} witness backend rejected request: {reason:?}")]
    BackendRejected {
        /// Witness kind rejected by the backend.
        kind: WitnessKind,
        /// Backend rejection code.
        reason: WitnessRejectionCode,
    },
}

/// Verifier-shaped boundary for witness receipt backends.
pub trait WitnessBackend {
    /// Verifies or witnesses the request and returns a typed receipt.
    ///
    /// # Errors
    ///
    /// Returns [`WitnessError`] when the backend is unsupported or rejects the
    /// request.
    fn verify(&self, request: &WitnessRequest) -> Result<WitnessReceipt, WitnessError>;
}

/// Deterministic witness simulator fixture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WitnessSimulatorFixture {
    /// Emits E1 self-witness integrity-only receipts.
    SelfWitness,
    /// Emits signed-witness fixture receipts.
    SignedWitnessFixture,
    /// Emits threshold-witness fixture receipts.
    ThresholdWitnessFixture,
    /// Rejects every request with a typed backend rejection.
    RejectedWitnessFixture,
    /// Reports every request as unsupported.
    UnsupportedWitnessFixture,
}

/// Deterministic witness backend simulator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WitnessBackendSimulator {
    fixture: WitnessSimulatorFixture,
}

impl WitnessBackendSimulator {
    /// Creates a deterministic witness backend simulator.
    #[must_use]
    pub const fn new(fixture: WitnessSimulatorFixture) -> Self {
        Self { fixture }
    }

    /// Returns the fixture behavior used by this simulator.
    #[must_use]
    pub const fn fixture(self) -> WitnessSimulatorFixture {
        self.fixture
    }
}

impl WitnessBackend for WitnessBackendSimulator {
    fn verify(&self, request: &WitnessRequest) -> Result<WitnessReceipt, WitnessError> {
        match self.fixture {
            WitnessSimulatorFixture::SelfWitness if request.kind == WitnessKind::SelfWitness => {
                if request.compatibility != WitnessCompatibilityRule::E1Scaffold {
                    return Err(WitnessError::UnsupportedCompatibility {
                        kind: request.kind,
                        compatibility: request.compatibility,
                    });
                }
                Ok(WitnessReceipt::self_witness(
                    request.subject_digest,
                    request.evidence_digest,
                ))
            }
            WitnessSimulatorFixture::SignedWitnessFixture
                if request.kind == WitnessKind::SignedWitness =>
            {
                Ok(WitnessReceipt::new(
                    request.kind,
                    request.subject_digest,
                    request.evidence_digest,
                    request.compatibility,
                    WitnessAttestation::IndependentAttestation,
                )?)
            }
            WitnessSimulatorFixture::ThresholdWitnessFixture
                if request.kind == WitnessKind::ThresholdWitness =>
            {
                Ok(WitnessReceipt::new(
                    request.kind,
                    request.subject_digest,
                    request.evidence_digest,
                    request.compatibility,
                    WitnessAttestation::IndependentAttestation,
                )?)
            }
            WitnessSimulatorFixture::RejectedWitnessFixture => Err(WitnessError::BackendRejected {
                kind: request.kind,
                reason: WitnessRejectionCode::Rejected,
            }),
            WitnessSimulatorFixture::UnsupportedWitnessFixture
            | WitnessSimulatorFixture::SelfWitness
            | WitnessSimulatorFixture::SignedWitnessFixture
            | WitnessSimulatorFixture::ThresholdWitnessFixture => {
                Err(WitnessError::UnsupportedBackend { kind: request.kind })
            }
        }
    }
}
