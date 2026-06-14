// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Proof envelopes and honesty assertions.

use crate::braid_shell::BraidCoordinate;
use crate::ident::Hash;
use crate::revelation::AuthorityDomainRef;

/// The type of cryptographic proof enclosed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProofKind {
    /// Zero-Knowledge Succinct Non-Interactive Argument of Knowledge.
    ZkSnark,
    /// Plain execution replay trace proof.
    ReplayTrace,
    /// Verkle/Merkle vector commitment opening.
    VectorOpening,
}

/// A cryptographic envelope encapsulating validation proof details.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProofEnvelope {
    /// The style/kind of proof.
    pub kind: ProofKind,
    /// Raw serialized proof bytes.
    pub proof_bytes: Vec<u8>,
    /// Salted commitment digest binding public inputs.
    pub public_inputs_hash: Hash,
}

impl ProofEnvelope {
    /// Validates the proof against the expected public inputs hash.
    ///
    /// # Errors
    ///
    /// Returns a validation error string if proof bytes are empty or public inputs mismatch.
    pub fn verify(&self, expected_public_inputs_hash: Hash) -> Result<(), String> {
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
