// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Proof envelopes and honesty assertions.

use blake3::Hasher;

use crate::braid_shell::BraidCoordinate;
use crate::ident::Hash;
use crate::revelation::AuthorityDomainRef;

const PROOF_ENVELOPE_DOMAIN: &[u8] = b"echo.proof.envelope.v1\0";

/// The kind of proof-shaped evidence enclosed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProofKind {
    /// Zero-Knowledge Succinct Non-Interactive Argument of Knowledge.
    ///
    /// Reserved until a verifier backend is wired.
    ZkSnark,
    /// Plain execution replay trace evidence.
    ReplayTrace,
    /// Verkle/Merkle vector commitment opening.
    ///
    /// Reserved until a verifier backend is wired.
    VectorOpening,
}

impl ProofKind {
    /// Stable wire tag for canonical hashing.
    #[must_use]
    pub fn canonical_tag(self) -> u8 {
        match self {
            Self::ZkSnark => 0x01,
            Self::ReplayTrace => 0x02,
            Self::VectorOpening => 0x03,
        }
    }

    const fn accepts_shape_only(self) -> bool {
        matches!(self, Self::ReplayTrace)
    }
}

/// A proof-shaped envelope whose current validation admits replay-trace
/// evidence by checking structure and public-input binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProofEnvelope {
    /// The style/kind of proof-shaped evidence.
    pub kind: ProofKind,
    /// Raw serialized proof/evidence bytes. These bytes are not cryptographically
    /// verified by [`Self::validate_shape`].
    pub proof_bytes: Vec<u8>,
    /// Salted commitment digest binding public inputs.
    pub public_inputs_hash: Hash,
}

impl ProofEnvelope {
    /// Validates the envelope shape and public inputs hash.
    ///
    /// # Errors
    ///
    /// Returns a validation error string if proof bytes are empty or public inputs mismatch.
    pub fn validate_shape(&self, expected_public_inputs_hash: Hash) -> Result<(), String> {
        if !self.kind.accepts_shape_only() {
            return Err(format!(
                "{:?} proof envelopes require a verifier backend before admission",
                self.kind
            ));
        }
        if self.proof_bytes.is_empty() {
            return Err("Proof payload is empty".to_string());
        }
        if self.public_inputs_hash != expected_public_inputs_hash {
            return Err(format!(
                "Public inputs mismatch: expected {:?}, got {:?}",
                expected_public_inputs_hash, self.public_inputs_hash
            ));
        }
        Ok(())
    }

    /// Returns the canonical digest of the envelope material.
    #[must_use]
    pub fn digest(&self) -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(PROOF_ENVELOPE_DOMAIN);
        hasher.update(&[self.kind.canonical_tag()]);
        hasher.update(&(self.proof_bytes.len() as u64).to_le_bytes());
        hasher.update(&self.proof_bytes);
        hasher.update(&self.public_inputs_hash);
        hasher.finalize().into()
    }
}

/// An assertion of honesty regarding a braid's causal execution path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObserverHonestyClaim {
    /// Braid coordinate whose history is claimed to be correct.
    pub coordinate: BraidCoordinate,
    /// Target shell digest certifying the settlement.
    pub shell_digest: Hash,
    /// Domain identifying the observer.
    pub observer_domain: AuthorityDomainRef,
}
