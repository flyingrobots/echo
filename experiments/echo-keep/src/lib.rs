// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Experimental identity conformance for the Echo–Keep physical-content boundary.
//!
//! Echo hashes raw bytes. Keep has a distinct, versioned identity law. A binding
//! authenticates one bounded source under both laws; it proves no storage presence,
//! retention, visibility, or durability. This crate has no persisted binding ABI.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

use echo_cas::BlobHash;
use keep::{BlobHasher, BlobId};
use std::io::{self, Read};

/// An opaque in-process binding under the version-1 identity bridge.
///
/// Keep coordinates stay private. This experimental type has no wire encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IdentityBinding {
    echo: BlobHash,
    keep: BlobId,
    length: u64,
}

/// Operational or verification failure; no complete identity claim is returned.
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    /// Input I/O failed; the original cause is retained.
    #[error("identity source I/O failed: {0}")]
    Input(#[from] io::Error),
    /// Source bytes exceeded the caller-selected limit.
    #[error("identity source exceeds its byte limit")]
    ResourceLimit,
    /// A reader returned an impossible byte count.
    #[error("identity source returned an invalid read count")]
    InvalidReadCount,
    /// Checked Keep length accounting failed.
    #[error("Keep identity accounting failed: {0}")]
    Accounting(#[from] keep::BlobHashError),
    /// Exact Echo identity, Keep identity, or length did not match the binding.
    #[error("reconstructed bytes do not match the identity binding")]
    Mismatch,
}

impl IdentityBinding {
    /// Computes both independent laws from one bounded, non-materializing stream.
    ///
    /// Interrupted reads are retried. EOF seals the binding. The limit applies
    /// to accepted source bytes, with one fixed 8 KiB read buffer.
    ///
    /// # Errors
    /// Returns an operational error or resource refusal without a binding.
    pub fn from_source(source: &mut dyn Read, byte_limit: u64) -> Result<Self, IdentityError> {
        let mut echo = blake3::Hasher::new();
        let mut keep = BlobHasher::new();
        let mut length = 0_u64;
        let mut buffer = [0_u8; 8192];
        loop {
            let count = match source.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => count,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error.into()),
            };
            let bytes = buffer.get(..count).ok_or(IdentityError::InvalidReadCount)?;
            let incoming = u64::try_from(count).map_err(|_| IdentityError::ResourceLimit)?;
            length = length
                .checked_add(incoming)
                .filter(|n| *n <= byte_limit)
                .ok_or(IdentityError::ResourceLimit)?;
            echo.update(bytes);
            keep.update(bytes)?;
        }
        Ok(Self {
            echo: BlobHash::from_bytes(*echo.finalize().as_bytes()),
            keep: keep.finish(),
            length,
        })
    }

    /// Returns the raw-content Echo identity.
    pub const fn echo_identity(&self) -> BlobHash {
        self.echo
    }

    /// Returns the exact authenticated length.
    pub const fn length(&self) -> u64 {
        self.length
    }

    /// Rechecks both laws and exact length on a bounded reconstructed source.
    ///
    /// # Errors
    /// Returns `Mismatch` for substituted identity or length, or a source error.
    pub fn verify_source(
        &self,
        source: &mut dyn Read,
        byte_limit: u64,
    ) -> Result<(), IdentityError> {
        if byte_limit < self.length {
            return Err(IdentityError::ResourceLimit);
        }
        let observed = match Self::from_source(source, byte_limit) {
            Err(IdentityError::ResourceLimit) => return Err(IdentityError::Mismatch),
            result => result?,
        };
        if observed == *self {
            Ok(())
        } else {
            Err(IdentityError::Mismatch)
        }
    }
}

#[cfg(test)]
mod tests;
