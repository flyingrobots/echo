// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use super::*;
use keep::{LayoutEntryLimit, ReferenceStore, ReferenceStoreCapacity};
use std::io::Cursor;

#[test]
fn pinned_vectors_bind_distinct_laws_and_reconstructed_bytes()
-> Result<(), Box<dyn std::error::Error>> {
    // Keep golden-file-worldline/v1 vectors at the pinned revision.
    let cases = [
        (
            Vec::new(),
            "keep:blob:v1:blake3-256:0:c0074a279c09f9d019dc10e4c821f79f1450cfb8541ab4627132ab9f3c75e33f",
        ),
        (
            b"Keep exact bytes.\n".to_vec(),
            "keep:blob:v1:blake3-256:18:af75d70e4993121254ac71f16c5edd02410a36f94d795e4d6064ed3122b7967d",
        ),
        (
            (0..=255).collect::<Vec<u8>>(),
            "keep:blob:v1:blake3-256:256:e782f90f48483f6a8520c9b05eca57ace1647374dd9456b9e41aadccacd10f12",
        ),
        (
            (0..=255).cycle().take(1_048_576).collect::<Vec<u8>>(),
            "keep:blob:v1:blake3-256:1048576:25399c3df18ecd403c8cacf50a44409e005ca71452e4ad367bd14423c1f86e20",
        ),
    ];
    for (bytes, vector) in cases {
        let limit = bytes.len() as u64;
        let binding = IdentityBinding::from_source(&mut Cursor::new(&bytes), limit)?;
        assert_eq!(binding.echo, echo_cas::blob_hash(&bytes));
        assert_eq!(binding.keep, vector.parse()?);
        assert_eq!(binding.length, limit);
        // Keep digest domain and framing never become an Echo CAS coordinate.
        let mut independent = blake3::Hasher::new();
        independent.update(b"KEEP:BLOB:DATA\0\0");
        independent.update(&1_u16.to_be_bytes());
        independent.update(&[1]);
        independent.update(&bytes);
        independent.update(&limit.to_be_bytes());
        let expected = format!(
            "keep:blob:v1:blake3-256:{limit}:{}",
            independent.finalize().to_hex()
        );
        assert_eq!(binding.keep.to_string(), expected);
        assert_ne!(
            binding.echo.to_string(),
            independent.finalize().to_hex().to_string()
        );
        let mut store = ReferenceStore::new(ReferenceStoreCapacity::new(bytes.len()));
        let published = store
            .stage_expected(
                &mut Cursor::new(&bytes),
                binding.keep,
                LayoutEntryLimit::MAXIMUM,
            )?
            .commit(&mut store)?;
        let mut output = Vec::new();
        let receipt = store.reconstruct(published.target(), &mut output)?;
        assert_eq!(receipt.target(), binding.keep);
        assert_eq!(receipt.bytes_written().get(), binding.length);
        assert_eq!(output, bytes);
        binding.verify_source(&mut Cursor::new(&output), limit)?;
    }
    // Official raw-BLAKE3 empty vector anchors the Echo domain independently.
    assert_eq!(
        echo_cas::blob_hash(b"").to_string(),
        "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
    );
    Ok(())
}

#[test]
fn substitution_and_length_mismatch_refuse() -> Result<(), IdentityError> {
    let binding = IdentityBinding::from_source(&mut Cursor::new(b"abc"), 3)?;
    assert!(matches!(
        binding.verify_source(&mut Cursor::new(b"abd"), 3),
        Err(IdentityError::Mismatch)
    ));
    assert!(matches!(
        binding.verify_source(&mut Cursor::new(b"abcd"), 3),
        Err(IdentityError::Mismatch)
    ));
    for changed in [
        IdentityBinding {
            echo: echo_cas::blob_hash(b"other"),
            ..binding
        },
        IdentityBinding {
            keep: BlobId::hash_bytes(b"other")
                .map_err(|error| IdentityError::Accounting(Box::new(error)))?,
            ..binding
        },
        IdentityBinding {
            length: 4,
            ..binding
        },
    ] {
        assert!(matches!(
            changed.verify_source(&mut Cursor::new(b"abc"), 4),
            Err(IdentityError::Mismatch)
        ));
    }
    assert!(matches!(
        IdentityBinding::from_source(&mut Cursor::new(b"abcd"), 3),
        Err(IdentityError::ResourceLimit)
    ));
    Ok(())
}

struct AdversarialRead {
    phase: u8,
    failure: bool,
}
impl Read for AdversarialRead {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        self.phase += 1;
        match self.phase {
            1 => Err(io::ErrorKind::Interrupted.into()),
            2 => {
                out[0] = 120;
                Ok(1)
            }
            _ if self.failure => Err(io::ErrorKind::BrokenPipe.into()),
            _ => Ok(0),
        }
    }
}
#[test]
fn interrupted_short_source_retries_but_failed_source_returns_no_binding()
-> Result<(), IdentityError> {
    let binding = IdentityBinding::from_source(
        &mut AdversarialRead {
            phase: 0,
            failure: false,
        },
        1,
    )?;
    assert_eq!(binding.echo, echo_cas::blob_hash(b"x"));
    assert!(
        matches!(IdentityBinding::from_source(&mut AdversarialRead { phase: 0, failure: true }, 1), Err(IdentityError::Input(e)) if e.kind() == io::ErrorKind::BrokenPipe)
    );
    Ok(())
}

#[test]
fn complete_binding_budget_preflight_does_not_read_source() -> Result<(), IdentityError> {
    struct UnreadSource(bool);
    impl Read for UnreadSource {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            self.0 = true;
            Err(io::ErrorKind::BrokenPipe.into())
        }
    }
    let binding = IdentityBinding::from_source(&mut Cursor::new(b"abc"), 3)?;
    let mut source = UnreadSource(false);
    assert!(matches!(
        binding.verify_source(&mut source, 1),
        Err(IdentityError::ResourceLimit)
    ));
    assert!(!source.0);
    assert!(matches!(
        binding.verify_source(&mut Cursor::new(b""), 3),
        Err(IdentityError::Mismatch)
    ));
    Ok(())
}
