// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Shared backend-neutral complete-object witness, also used by Keep conformance.
use echo_cas::{
    blob_hash,
    physical_content::{
        ContentError, ContentTarget, MemoryContentDestination, PhysicalContentBackend,
        PhysicalContentView, StagedContent, TransactionalContentDestination, VerifiedContent,
    },
};
use std::io::{self, Cursor, Read};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Runs the complete-object contract against any admitted backend.
pub fn conformance(backend: &mut impl PhysicalContentBackend) -> TestResult {
    for bytes in [
        Vec::new(),
        b"exact bytes".to_vec(),
        (0..=255).cycle().take(262_145).collect(),
    ] {
        let target = ContentTarget {
            hash: blob_hash(&bytes),
            length: u64::try_from(bytes.len())?,
        };
        let mut destination = MemoryContentDestination::new(b"prior".to_vec(), bytes.len().max(5))?;
        assert!(matches!(
            backend.content_view().reconstruct(target, &mut destination),
            Err(ContentError::CapabilityUnavailable)
        ));
        let staged = StagedContent::read_expected(target, &mut Cursor::new(&bytes), bytes.len())?;
        // Staging has no observable backend effect.
        assert!(backend
            .content_view()
            .reconstruct(target, &mut destination)
            .is_err());
        let receipt = backend.publish_content(staged)?;
        assert_eq!(receipt.target(), target);
        assert!(!receipt.establishes_durability());
        assert!(!receipt.establishes_complete_view());
        let receipt = backend
            .content_view()
            .reconstruct(target, &mut destination)?;
        assert_eq!(receipt.target(), target);
        assert_eq!(destination.visible(), bytes);
        assert!(!receipt.establishes_durability());
        let wrong = ContentTarget {
            length: target.length + 1,
            ..target
        };
        let mut unchanged = MemoryContentDestination::new(b"prior".to_vec(), bytes.len() + 6)?;
        assert!(matches!(
            backend.content_view().reconstruct(wrong, &mut unchanged),
            Err(ContentError::Mismatch)
        ));
        assert_eq!(unchanged.visible(), b"prior");
        let mut failed = FailingDestination {
            visible: b"prior".to_vec(),
            capable: true,
            limit: bytes.len() + 6,
        };
        assert!(matches!(
            backend.content_view().reconstruct(target, &mut failed),
            Err(ContentError::Io(_))
        ));
        assert_eq!(failed.visible, b"prior");
        failed.capable = false;
        assert!(matches!(
            backend.content_view().reconstruct(target, &mut failed),
            Err(ContentError::CapabilityUnavailable)
        ));
        assert_eq!(failed.visible, b"prior");
        if !bytes.is_empty() {
            let mut limited = MemoryContentDestination::new(Vec::new(), bytes.len() - 1)?;
            assert!(matches!(
                backend.content_view().reconstruct(target, &mut limited),
                Err(ContentError::ResourceLimit)
            ));
            assert!(limited.visible().is_empty());
        }
    }
    let target = ContentTarget {
        hash: blob_hash(b"good"),
        length: 4,
    };
    assert!(matches!(
        StagedContent::read_expected(target, &mut Cursor::new(b"bad!"), 4),
        Err(ContentError::Mismatch)
    ));
    assert!(matches!(
        StagedContent::read_expected(target, &mut Cursor::new(b"good"), 3),
        Err(ContentError::ResourceLimit)
    ));
    assert!(
        matches!(StagedContent::read_expected(target, &mut FailedSource(false), 4), Err(ContentError::Io(e)) if e.kind() == io::ErrorKind::BrokenPipe)
    );
    assert!(backend
        .content_view()
        .reconstruct(target, &mut MemoryContentDestination::new(Vec::new(), 4)?)
        .is_err());
    Ok(())
}
struct FailedSource(bool);
impl Read for FailedSource {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if self.0 {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        self.0 = true;
        output[0] = 103;
        Ok(1)
    }
}
struct FailingDestination {
    visible: Vec<u8>,
    capable: bool,
    limit: usize,
}
impl TransactionalContentDestination for FailingDestination {
    fn supports_atomic_promotion(&self) -> bool {
        self.capable
    }
    fn byte_limit(&self) -> usize {
        self.limit
    }
    fn promote(&mut self, _: VerifiedContent) -> Result<(), ContentError> {
        Err(io::Error::other("injected promotion failure").into())
    }
}
