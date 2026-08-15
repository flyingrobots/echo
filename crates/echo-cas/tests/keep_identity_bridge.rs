// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Same-source Echo and pinned Keep identity-vector witness.
//!
//! The raw Echo vectors were generated independently with `b3sum` 1.8.5. The
//! Keep vectors and preimage are pinned to Keep commit
//! `3bf7b9179db41e90620e6d1875c2d40222a2330b`.

use std::num::TryFromIntError;

use echo_cas::blob_hash;

const KEEP_BLOB_DATA_MAGIC: &[u8; 16] = b"KEEP:BLOB:DATA\0\0";
const KEEP_BLOB_IDENTITY_VERSION: [u8; 2] = 1_u16.to_be_bytes();
const KEEP_BLOB_HASH_ALGORITHM: [u8; 1] = [1];

#[test]
fn same_exact_sources_reproduce_echo_and_pinned_keep_identity_vectors(
) -> Result<(), TryFromIntError> {
    assert_identity_pair(
        &[],
        "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262",
        "c0074a279c09f9d019dc10e4c821f79f1450cfb8541ab4627132ab9f3c75e33f",
    )?;
    assert_identity_pair(
        b"Keep exact bytes.\n",
        "3d4f69a42f4cde6629970fe61d548f60d475a0f9ac26a24be5fab204b6eb030a",
        "af75d70e4993121254ac71f16c5edd02410a36f94d795e4d6064ed3122b7967d",
    )?;

    let byte_ramp = (u8::MIN..=u8::MAX).collect::<Vec<_>>();
    assert_identity_pair(
        &byte_ramp,
        "4a495ba42461748eca8fdad618f976aa726cc2903de9fcb40735a786ac1c196b",
        "e782f90f48483f6a8520c9b05eca57ace1647374dd9456b9e41aadccacd10f12",
    )?;
    Ok(())
}

fn assert_identity_pair(
    bytes: &[u8],
    expected_echo: &str,
    expected_keep: &str,
) -> Result<(), TryFromIntError> {
    let echo = blob_hash(bytes);
    let keep = keep_blob_digest_v1(bytes)?;
    assert_eq!(
        blake3::Hash::from_bytes(*echo.as_bytes()).to_hex().as_str(),
        expected_echo
    );
    assert_eq!(keep.to_hex().as_str(), expected_keep);
    assert_ne!(echo.as_bytes(), keep.as_bytes());
    Ok(())
}

fn keep_blob_digest_v1(bytes: &[u8]) -> Result<blake3::Hash, TryFromIntError> {
    let logical_length = u64::try_from(bytes.len())?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(KEEP_BLOB_DATA_MAGIC);
    hasher.update(&KEEP_BLOB_IDENTITY_VERSION);
    hasher.update(&KEEP_BLOB_HASH_ALGORITHM);
    hasher.update(bytes);
    hasher.update(&logical_length.to_be_bytes());
    Ok(hasher.finalize())
}
