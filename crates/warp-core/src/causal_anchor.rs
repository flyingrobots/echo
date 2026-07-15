// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Canonical causal-anchor claim contract.
//!
//! An application request binds a subject, causal-frontier digest, claimed
//! authority/evidence roots, optional projection roots, and purpose into a
//! deterministic claim. Claim construction does not confer admission. Only the
//! trusted Echo path may attach a receipt and construct an admitted fact.

use std::collections::BTreeSet;

use blake3::Hasher;
use thiserror::Error;

use crate::ident::Hash;

const CAUSAL_ANCHOR_CLAIM_DIGEST_DOMAIN: &[u8] = b"echo:causal-anchor:claim:v1\0";
const CAUSAL_ANCHOR_ADMISSION_RECEIPT_ID_DOMAIN: &[u8] =
    b"echo:causal-anchor:admission-receipt-id:v1\0";
const CAUSAL_ANCHOR_FACT_DIGEST_DOMAIN: &[u8] = b"echo:causal-anchor:fact-digest:v1\0";
const CAUSAL_ANCHOR_ID_DOMAIN: &[u8] = b"echo:causal-anchor:id:v1\0";
const CAUSAL_ANCHOR_SUPPORT_POLICY_DIGEST_DOMAIN: &[u8] = b"echo:causal-anchor:support-policy:v1\0";
const CAUSAL_ANCHOR_SUPPORT_GRANT_DIGEST_DOMAIN: &[u8] = b"echo:causal-anchor:support-grant:v1\0";
const CAUSAL_ANCHOR_CLAIM_PAYLOAD_MAGIC: &[u8; 8] = b"EACLM001";
const CAUSAL_ANCHOR_FACT_PAYLOAD_MAGIC: &[u8; 8] = b"EAFCT001";
const CAUSAL_ANCHOR_RECEIPT_PAYLOAD_MAGIC: &[u8; 8] = b"EARCP001";

/// Current causal anchor schema version.
pub const CAUSAL_ANCHOR_SCHEMA_VERSION: u32 = 1;

/// Subject named by a causal anchor.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CausalAnchorSubject {
    /// Application namespace owning the subject.
    pub app_id: String,
    /// Application-scoped subject kind.
    pub subject_kind: String,
    /// Application-scoped subject id.
    pub subject_id: String,
}

impl CausalAnchorSubject {
    /// Builds a causal anchor subject.
    #[must_use]
    pub fn new(
        app_id: impl Into<String>,
        subject_kind: impl Into<String>,
        subject_id: impl Into<String>,
    ) -> Self {
        Self {
            app_id: app_id.into(),
            subject_kind: subject_kind.into(),
            subject_id: subject_id.into(),
        }
    }
}

/// Opaque caller-provided reference to a causal frontier.
///
/// Possession of this value is not evidence that the frontier was admitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CausalFrontierRef {
    /// Caller-provided digest of the frontier being referenced.
    pub frontier_digest: Hash,
}

impl CausalFrontierRef {
    /// Builds a causal frontier reference from its digest.
    #[must_use]
    pub const fn from_digest(frontier_digest: Hash) -> Self {
        Self { frontier_digest }
    }
}

/// Claimed purpose encoded into a causal-anchor request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CausalAnchorPurpose {
    /// Recovery basis.
    Recovery,
    /// Retention boundary.
    Retention,
    /// Export basis.
    Export,
    /// User-visible save basis.
    UserSave,
    /// Automatic save basis.
    Autosave,
    /// Debug or diagnostic basis.
    Debug,
    /// Cache-warming basis.
    CacheWarm,
}

impl CausalAnchorPurpose {
    fn tag(self) -> u8 {
        match self {
            Self::Recovery => 1,
            Self::Retention => 2,
            Self::Export => 3,
            Self::UserSave => 4,
            Self::Autosave => 5,
            Self::Debug => 6,
            Self::CacheWarm => 7,
        }
    }

    fn from_tag(tag: u8) -> Result<Self, CausalAnchorError> {
        match tag {
            1 => Ok(Self::Recovery),
            2 => Ok(Self::Retention),
            3 => Ok(Self::Export),
            4 => Ok(Self::UserSave),
            5 => Ok(Self::Autosave),
            6 => Ok(Self::Debug),
            7 => Ok(Self::CacheWarm),
            code => Err(CausalAnchorError::UnknownEnumCode {
                enum_name: "CausalAnchorPurpose",
                code,
            }),
        }
    }
}

/// Role for a CAS object named by a causal anchor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CausalAnchorCasRole {
    /// Derived materialized projection.
    Materialization,
    /// Manifest that describes retained material.
    Manifest,
    /// Derived acceleration index.
    Index,
}

impl CausalAnchorCasRole {
    fn tag(self) -> u8 {
        match self {
            Self::Materialization => 1,
            Self::Manifest => 2,
            Self::Index => 3,
        }
    }

    fn from_tag(tag: u8) -> Result<Self, CausalAnchorError> {
        match tag {
            1 => Ok(Self::Materialization),
            2 => Ok(Self::Manifest),
            3 => Ok(Self::Index),
            code => Err(CausalAnchorError::UnknownEnumCode {
                enum_name: "CausalAnchorCasRole",
                code,
            }),
        }
    }
}

/// Role for an Echo graph fact named by a causal anchor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CausalAnchorGraphRole {
    /// Authoritative graph fact root.
    Authority,
    /// Evidence graph fact root.
    Evidence,
    /// Rebuildable graph index fact.
    Index,
}

impl CausalAnchorGraphRole {
    fn tag(self) -> u8 {
        match self {
            Self::Authority => 1,
            Self::Evidence => 2,
            Self::Index => 3,
        }
    }

    fn from_tag(tag: u8) -> Result<Self, CausalAnchorError> {
        match tag {
            1 => Ok(Self::Authority),
            2 => Ok(Self::Evidence),
            3 => Ok(Self::Index),
            code => Err(CausalAnchorError::UnknownEnumCode {
                enum_name: "CausalAnchorGraphRole",
                code,
            }),
        }
    }
}

/// Role for an application subject root named by a causal anchor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CausalAnchorAppRootRole {
    /// Authoritative application-domain root.
    Authority,
    /// Evidence application-domain root.
    Evidence,
}

impl CausalAnchorAppRootRole {
    fn tag(self) -> u8 {
        match self {
            Self::Authority => 1,
            Self::Evidence => 2,
        }
    }

    fn from_tag(tag: u8) -> Result<Self, CausalAnchorError> {
        match tag {
            1 => Ok(Self::Authority),
            2 => Ok(Self::Evidence),
            code => Err(CausalAnchorError::UnknownEnumCode {
                enum_name: "CausalAnchorAppRootRole",
                code,
            }),
        }
    }
}

/// Root claimed as retained by or attached to a causal-anchor request.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CausalAnchorRoot {
    /// Content-addressed object root.
    CasObject {
        /// CAS object id.
        id: Hash,
        /// CAS role.
        role: CausalAnchorCasRole,
    },
    /// Echo graph fact root.
    GraphFact {
        /// Graph fact id.
        id: Hash,
        /// Graph fact role.
        role: CausalAnchorGraphRole,
    },
    /// Application-domain subject root.
    AppSubjectRoot {
        /// Application namespace owning the root.
        app_id: String,
        /// Application-scoped root kind.
        subject_kind: String,
        /// Application-scoped root id.
        id: String,
        /// Application root role.
        role: CausalAnchorAppRootRole,
    },
}

impl CausalAnchorRoot {
    /// Returns true when this root declares itself as authority.
    #[must_use]
    pub fn is_authority(&self) -> bool {
        match self {
            Self::CasObject { .. } => false,
            Self::GraphFact { role, .. } => *role == CausalAnchorGraphRole::Authority,
            Self::AppSubjectRoot { role, .. } => *role == CausalAnchorAppRootRole::Authority,
        }
    }
}

/// Root-set position authorized by a causal-anchor support grant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CausalAnchorSupportSet {
    /// The root may appear in the retained authority/evidence set.
    Retained,
    /// The root may appear in the derived materialization set.
    Materialization,
}

impl CausalAnchorSupportSet {
    const fn tag(self) -> u8 {
        match self {
            Self::Retained => 1,
            Self::Materialization => 2,
        }
    }
}

/// Exact host-owned support grant for one subject, root, and root set.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CausalAnchorRootSupportGrant {
    subject: CausalAnchorSubject,
    root: CausalAnchorRoot,
    support_set: CausalAnchorSupportSet,
}

impl CausalAnchorRootSupportGrant {
    /// Grants one subject permission to retain the exact named root.
    #[must_use]
    pub const fn retained(subject: CausalAnchorSubject, root: CausalAnchorRoot) -> Self {
        Self {
            subject,
            root,
            support_set: CausalAnchorSupportSet::Retained,
        }
    }

    /// Grants one subject permission to attach the exact materialization root.
    #[must_use]
    pub const fn materialization(subject: CausalAnchorSubject, root: CausalAnchorRoot) -> Self {
        Self {
            subject,
            root,
            support_set: CausalAnchorSupportSet::Materialization,
        }
    }

    /// Returns the subject authorized by this grant.
    #[must_use]
    pub const fn subject(&self) -> &CausalAnchorSubject {
        &self.subject
    }

    /// Returns the exact root authorized by this grant.
    #[must_use]
    pub const fn root(&self) -> &CausalAnchorRoot {
        &self.root
    }

    /// Returns the root-set position authorized by this grant.
    #[must_use]
    pub const fn support_set(&self) -> CausalAnchorSupportSet {
        self.support_set
    }

    /// Returns the canonical identity of this exact support coordinate.
    #[must_use]
    pub fn grant_digest(&self) -> Hash {
        compute_support_grant_digest(self)
    }
}

/// Canonical host-owned policy for generic causal-anchor root support.
///
/// Echo does not infer application-domain root meaning. A trusted host installs
/// exact grants after obtaining support from its graph, CAS, or application
/// authority adapters. Application-facing admission cannot install this policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CausalAnchorRootSupportPolicy {
    grants: BTreeSet<CausalAnchorRootSupportGrant>,
    policy_digest: Hash,
}

impl CausalAnchorRootSupportPolicy {
    /// Builds a canonical exact-grant policy.
    #[must_use]
    pub fn new(grants: impl IntoIterator<Item = CausalAnchorRootSupportGrant>) -> Self {
        let grants = grants.into_iter().collect::<BTreeSet<_>>();
        let policy_digest = compute_support_policy_digest(&grants);
        Self {
            grants,
            policy_digest,
        }
    }

    /// Returns the digest bound into admission receipt evidence.
    #[must_use]
    pub const fn policy_digest(&self) -> &Hash {
        &self.policy_digest
    }

    /// Validates every claimed root against an exact host-owned grant.
    ///
    /// # Errors
    ///
    /// Returns the first unsupported canonical root and its requested root set.
    pub fn validate_claim(
        &self,
        claim: &CausalAnchorClaim,
    ) -> Result<(), CausalAnchorSupportError> {
        for root in claim.retained_roots() {
            self.require_grant(claim.subject(), root, CausalAnchorSupportSet::Retained)?;
        }
        for root in claim.materialization_roots() {
            self.require_grant(
                claim.subject(),
                root,
                CausalAnchorSupportSet::Materialization,
            )?;
        }
        Ok(())
    }

    fn require_grant(
        &self,
        subject: &CausalAnchorSubject,
        root: &CausalAnchorRoot,
        support_set: CausalAnchorSupportSet,
    ) -> Result<(), CausalAnchorSupportError> {
        let grant = CausalAnchorRootSupportGrant {
            subject: subject.clone(),
            root: root.clone(),
            support_set,
        };
        if self.grants.contains(&grant) {
            Ok(())
        } else {
            Err(CausalAnchorSupportError::UnsupportedRoot {
                grant_digest: grant.grant_digest(),
                support_set,
            })
        }
    }
}

/// Generic causal-anchor support-policy refusal.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum CausalAnchorSupportError {
    /// No host support grant covers the exact subject, root, and root set.
    #[error("causal anchor root support grant {grant_digest:?} is unavailable in {support_set:?}")]
    UnsupportedRoot {
        /// Canonical digest of the exact subject, root, and root-set coordinate.
        grant_digest: Hash,
        /// Root set in which support was requested.
        support_set: CausalAnchorSupportSet,
    },
}

/// Application request for Echo to consider a causal-anchor claim for admission.
///
/// The request intentionally contains no admission receipt. Only Echo's trusted
/// admission path may derive that identity and turn the claim into an admitted
/// anchor fact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CausalAnchorAdmissionRequest {
    /// Schema version for the requested anchor claim.
    pub schema_version: u32,
    /// Application subject being anchored.
    pub subject: CausalAnchorSubject,
    /// Caller-provided causal-frontier reference.
    pub basis_frontier: CausalFrontierRef,
    /// Roots claimed as retained authority or evidence.
    pub retained_roots: Vec<CausalAnchorRoot>,
    /// Optional derived projection roots attached to the anchor.
    pub materialization_roots: Vec<CausalAnchorRoot>,
    /// Purpose of the anchor.
    pub purpose: CausalAnchorPurpose,
}

/// Canonical, shape-validated application claim awaiting Echo admission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CausalAnchorClaim {
    /// Schema version for this anchor claim.
    schema_version: u32,
    /// Application subject being anchored.
    subject: CausalAnchorSubject,
    /// Caller-provided causal-frontier reference.
    basis_frontier: CausalFrontierRef,
    /// Canonical retained root set.
    retained_roots: Vec<CausalAnchorRoot>,
    /// Canonical materialization root set.
    materialization_roots: Vec<CausalAnchorRoot>,
    /// Purpose of the anchor.
    purpose: CausalAnchorPurpose,
    /// Digest over the canonical claim fields, before any admission identity.
    claim_digest: Hash,
}

impl CausalAnchorClaim {
    /// Builds a canonical claim without conferring Echo admission.
    ///
    /// Root vectors are sorted and duplicate roots are rejected so the resulting
    /// digest represents root sets rather than caller iteration order. This
    /// function does not verify frontier admission, root existence, authority,
    /// retention, or publish anything through the WAL.
    pub fn from_admission_request(
        request: CausalAnchorAdmissionRequest,
    ) -> Result<Self, CausalAnchorError> {
        if request.schema_version != CAUSAL_ANCHOR_SCHEMA_VERSION {
            return Err(CausalAnchorError::UnsupportedSchemaVersion {
                expected: CAUSAL_ANCHOR_SCHEMA_VERSION,
                actual: request.schema_version,
            });
        }
        validate_subject(&request.subject)?;
        let retained_roots =
            canonicalize_roots(request.retained_roots, CausalAnchorRootSet::Retained)?;
        if retained_roots.is_empty() {
            return Err(CausalAnchorError::EmptyRetainedRoots);
        }
        let materialization_roots = canonicalize_roots(
            request.materialization_roots,
            CausalAnchorRootSet::Materialization,
        )?;
        if materialization_roots
            .iter()
            .any(CausalAnchorRoot::is_authority)
        {
            return Err(CausalAnchorError::AuthorityMaterializationRoot);
        }
        if retained_roots
            .iter()
            .any(|root| materialization_roots.binary_search(root).is_ok())
        {
            return Err(CausalAnchorError::RootAppearsInRetainedAndMaterialization);
        }
        let claim_digest = compute_claim_digest(
            request.schema_version,
            &request.subject,
            &request.basis_frontier,
            &retained_roots,
            &materialization_roots,
            request.purpose,
        );
        Ok(Self {
            schema_version: request.schema_version,
            subject: request.subject,
            basis_frontier: request.basis_frontier,
            retained_roots,
            materialization_roots,
            purpose: request.purpose,
            claim_digest,
        })
    }

    /// Returns the schema version used to canonicalize this claim.
    #[must_use]
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// Returns the application subject named by this claim.
    #[must_use]
    pub const fn subject(&self) -> &CausalAnchorSubject {
        &self.subject
    }

    /// Returns the caller-provided causal-frontier reference.
    #[must_use]
    pub const fn basis_frontier(&self) -> &CausalFrontierRef {
        &self.basis_frontier
    }

    /// Returns the canonical retained root set.
    #[must_use]
    pub fn retained_roots(&self) -> &[CausalAnchorRoot] {
        &self.retained_roots
    }

    /// Returns the canonical materialization root set.
    #[must_use]
    pub fn materialization_roots(&self) -> &[CausalAnchorRoot] {
        &self.materialization_roots
    }

    /// Returns the claimed anchor purpose.
    #[must_use]
    pub const fn purpose(&self) -> CausalAnchorPurpose {
        self.purpose
    }

    /// Returns the canonical digest of this claim.
    #[must_use]
    pub const fn claim_digest(&self) -> &Hash {
        &self.claim_digest
    }

    pub(crate) fn to_payload_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(CAUSAL_ANCHOR_CLAIM_PAYLOAD_MAGIC);
        out.extend_from_slice(&self.schema_version.to_le_bytes());
        push_string(&mut out, &self.subject.app_id);
        push_string(&mut out, &self.subject.subject_kind);
        push_string(&mut out, &self.subject.subject_id);
        out.extend_from_slice(&self.basis_frontier.frontier_digest);
        push_roots(&mut out, &self.retained_roots);
        push_roots(&mut out, &self.materialization_roots);
        out.push(self.purpose.tag());
        out.extend_from_slice(&self.claim_digest);
        out
    }

    pub(crate) fn from_payload_bytes(bytes: &[u8]) -> Result<Self, CausalAnchorError> {
        let mut cursor = CausalAnchorPayloadCursor::new(bytes);
        cursor.expect_magic(CAUSAL_ANCHOR_CLAIM_PAYLOAD_MAGIC, "claim")?;
        let schema_version = cursor.read_u32()?;
        let subject = CausalAnchorSubject::new(
            cursor.read_string("subject.app_id")?,
            cursor.read_string("subject.subject_kind")?,
            cursor.read_string("subject.subject_id")?,
        );
        let basis_frontier = CausalFrontierRef::from_digest(cursor.read_hash()?);
        let retained_roots = cursor.read_roots()?;
        let materialization_roots = cursor.read_roots()?;
        let purpose = CausalAnchorPurpose::from_tag(cursor.read_u8()?)?;
        let encoded_claim_digest = cursor.read_hash()?;
        cursor.finish()?;

        let claim = Self::from_admission_request(CausalAnchorAdmissionRequest {
            schema_version,
            subject,
            basis_frontier,
            retained_roots,
            materialization_roots,
            purpose,
        })?;
        if claim.claim_digest != encoded_claim_digest {
            return Err(CausalAnchorError::ClaimDigestMismatch);
        }
        if claim.to_payload_bytes() != bytes {
            return Err(CausalAnchorError::NonCanonicalPayload);
        }
        Ok(claim)
    }
}

/// Opaque identity of an Echo-admitted causal anchor.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CausalAnchorId(Hash);

impl CausalAnchorId {
    /// Returns the canonical anchor-id bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &Hash {
        &self.0
    }
}

/// Opaque identity of an Echo causal-anchor admission receipt.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CausalAnchorAdmissionReceiptId(Hash);

impl CausalAnchorAdmissionReceiptId {
    /// Returns the canonical admission-receipt-id bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &Hash {
        &self.0
    }
}

/// Echo-admitted causal-anchor fact.
///
/// Construction is restricted to Echo's trusted WAL admission path. Possessing
/// decoded value bytes alone is not proof that the fact was durably committed;
/// callers obtain authority through trusted admission or recovery results.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CausalAnchorFact {
    claim: CausalAnchorClaim,
    anchor_id: CausalAnchorId,
    admitted_by_receipt_id: CausalAnchorAdmissionReceiptId,
    anchor_digest: Hash,
}

impl CausalAnchorFact {
    /// Returns the canonical application claim Echo admitted.
    #[must_use]
    pub const fn claim(&self) -> &CausalAnchorClaim {
        &self.claim
    }

    /// Returns this admitted anchor's stable identity.
    #[must_use]
    pub const fn anchor_id(&self) -> &CausalAnchorId {
        &self.anchor_id
    }

    /// Returns the Echo receipt identity that admitted this fact.
    #[must_use]
    pub const fn admitted_by_receipt_id(&self) -> &CausalAnchorAdmissionReceiptId {
        &self.admitted_by_receipt_id
    }

    /// Returns the admitted fact digest.
    #[must_use]
    pub const fn anchor_digest(&self) -> &Hash {
        &self.anchor_digest
    }

    pub(crate) fn to_payload_bytes(&self) -> Vec<u8> {
        let claim_bytes = self.claim.to_payload_bytes();
        let mut out = Vec::new();
        out.extend_from_slice(CAUSAL_ANCHOR_FACT_PAYLOAD_MAGIC);
        push_bytes(&mut out, &claim_bytes);
        out.extend_from_slice(self.admitted_by_receipt_id.as_bytes());
        out.extend_from_slice(&self.anchor_digest);
        out.extend_from_slice(self.anchor_id.as_bytes());
        out
    }

    pub(crate) fn from_payload_bytes(bytes: &[u8]) -> Result<Self, CausalAnchorError> {
        let mut cursor = CausalAnchorPayloadCursor::new(bytes);
        cursor.expect_magic(CAUSAL_ANCHOR_FACT_PAYLOAD_MAGIC, "fact")?;
        let claim = CausalAnchorClaim::from_payload_bytes(&cursor.read_bytes()?)?;
        let admitted_by_receipt_id = CausalAnchorAdmissionReceiptId(cursor.read_hash()?);
        let anchor_digest = cursor.read_hash()?;
        let anchor_id = CausalAnchorId(cursor.read_hash()?);
        cursor.finish()?;

        let expected_anchor_digest =
            compute_anchor_digest(claim.claim_digest(), &admitted_by_receipt_id);
        if anchor_digest != expected_anchor_digest {
            return Err(CausalAnchorError::AnchorDigestMismatch);
        }
        if anchor_id != compute_anchor_id(&anchor_digest) {
            return Err(CausalAnchorError::AnchorIdMismatch);
        }
        Ok(Self {
            claim,
            anchor_id,
            admitted_by_receipt_id,
            anchor_digest,
        })
    }
}

/// Receipt evidence committed by Echo for one causal-anchor admission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CausalAnchorAdmissionReceipt {
    receipt_id: CausalAnchorAdmissionReceiptId,
    anchor_id: CausalAnchorId,
    claim_digest: Hash,
    basis_frontier: CausalFrontierRef,
    support_policy_digest: Hash,
    writer_epoch_id: Hash,
    wal_transaction_id: Hash,
    wal_first_lsn: u64,
}

impl CausalAnchorAdmissionReceipt {
    /// Returns this receipt's stable identity.
    #[must_use]
    pub const fn receipt_id(&self) -> &CausalAnchorAdmissionReceiptId {
        &self.receipt_id
    }

    /// Returns the admitted anchor identity.
    #[must_use]
    pub const fn anchor_id(&self) -> &CausalAnchorId {
        &self.anchor_id
    }

    /// Returns the digest of the exact application claim admitted by Echo.
    #[must_use]
    pub const fn claim_digest(&self) -> &Hash {
        &self.claim_digest
    }

    /// Returns the causal frontier named by the admitted claim.
    #[must_use]
    pub const fn basis_frontier(&self) -> &CausalFrontierRef {
        &self.basis_frontier
    }

    /// Returns the host-owned root-support policy applied at admission.
    #[must_use]
    pub const fn support_policy_digest(&self) -> &Hash {
        &self.support_policy_digest
    }

    /// Returns the WAL writer epoch that ordered this admission.
    #[must_use]
    pub const fn writer_epoch_id(&self) -> &Hash {
        &self.writer_epoch_id
    }

    /// Returns the WAL transaction identity that ordered this admission.
    #[must_use]
    pub const fn wal_transaction_id(&self) -> &Hash {
        &self.wal_transaction_id
    }

    /// Returns the first WAL LSN occupied by this admission transaction.
    #[must_use]
    pub const fn wal_first_lsn(&self) -> u64 {
        self.wal_first_lsn
    }

    pub(crate) fn to_payload_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(CAUSAL_ANCHOR_RECEIPT_PAYLOAD_MAGIC);
        out.extend_from_slice(self.receipt_id.as_bytes());
        out.extend_from_slice(self.anchor_id.as_bytes());
        out.extend_from_slice(&self.claim_digest);
        out.extend_from_slice(&self.basis_frontier.frontier_digest);
        out.extend_from_slice(&self.support_policy_digest);
        out.extend_from_slice(&self.writer_epoch_id);
        out.extend_from_slice(&self.wal_transaction_id);
        out.extend_from_slice(&self.wal_first_lsn.to_le_bytes());
        out
    }

    pub(crate) fn from_payload_bytes(bytes: &[u8]) -> Result<Self, CausalAnchorError> {
        let mut cursor = CausalAnchorPayloadCursor::new(bytes);
        cursor.expect_magic(CAUSAL_ANCHOR_RECEIPT_PAYLOAD_MAGIC, "admission receipt")?;
        let receipt_id = CausalAnchorAdmissionReceiptId(cursor.read_hash()?);
        let anchor_id = CausalAnchorId(cursor.read_hash()?);
        let claim_digest = cursor.read_hash()?;
        let basis_frontier = CausalFrontierRef::from_digest(cursor.read_hash()?);
        let support_policy_digest = cursor.read_hash()?;
        let writer_epoch_id = cursor.read_hash()?;
        let wal_transaction_id = cursor.read_hash()?;
        let wal_first_lsn = cursor.read_u64()?;
        cursor.finish()?;

        let expected_receipt_id = compute_admission_receipt_id(
            &claim_digest,
            &support_policy_digest,
            &writer_epoch_id,
            &wal_transaction_id,
            wal_first_lsn,
        );
        if receipt_id != expected_receipt_id {
            return Err(CausalAnchorError::AdmissionReceiptIdMismatch);
        }
        let anchor_digest = compute_anchor_digest(&claim_digest, &receipt_id);
        if anchor_id != compute_anchor_id(&anchor_digest) {
            return Err(CausalAnchorError::AnchorIdMismatch);
        }
        Ok(Self {
            receipt_id,
            anchor_id,
            claim_digest,
            basis_frontier,
            support_policy_digest,
            writer_epoch_id,
            wal_transaction_id,
            wal_first_lsn,
        })
    }
}

#[cfg(any(
    test,
    all(feature = "native_rule_bootstrap", feature = "trusted_runtime")
))]
pub(crate) fn prepare_causal_anchor_admission(
    claim: CausalAnchorClaim,
    support_policy_digest: Hash,
    writer_epoch_id: Hash,
    wal_transaction_id: Hash,
    wal_first_lsn: u64,
) -> (CausalAnchorFact, CausalAnchorAdmissionReceipt) {
    let receipt_id = compute_admission_receipt_id(
        claim.claim_digest(),
        &support_policy_digest,
        &writer_epoch_id,
        &wal_transaction_id,
        wal_first_lsn,
    );
    let anchor_digest = compute_anchor_digest(claim.claim_digest(), &receipt_id);
    let anchor_id = compute_anchor_id(&anchor_digest);
    let receipt = CausalAnchorAdmissionReceipt {
        receipt_id,
        anchor_id,
        claim_digest: *claim.claim_digest(),
        basis_frontier: *claim.basis_frontier(),
        support_policy_digest,
        writer_epoch_id,
        wal_transaction_id,
        wal_first_lsn,
    };
    let fact = CausalAnchorFact {
        claim,
        anchor_id,
        admitted_by_receipt_id: receipt_id,
        anchor_digest,
    };
    (fact, receipt)
}

pub(crate) fn validate_causal_anchor_admission_evidence(
    fact: &CausalAnchorFact,
    receipt: &CausalAnchorAdmissionReceipt,
) -> Result<(), CausalAnchorError> {
    let receipt_matches = fact.admitted_by_receipt_id == receipt.receipt_id;
    let anchor_matches = fact.anchor_id == receipt.anchor_id;
    let claim_matches = fact.claim.claim_digest == receipt.claim_digest;
    let basis_matches = fact.claim.basis_frontier == receipt.basis_frontier;
    if !(receipt_matches && anchor_matches && claim_matches && basis_matches) {
        return Err(CausalAnchorError::AdmissionEvidenceMismatch);
    }
    Ok(())
}

/// Causal anchor validation error.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum CausalAnchorError {
    /// The request uses a schema this implementation cannot canonicalize.
    #[error("unsupported causal anchor schema version {actual}; expected {expected}")]
    UnsupportedSchemaVersion {
        /// Supported schema version.
        expected: u32,
        /// Requested schema version.
        actual: u32,
    },
    /// A causal-anchor payload ended before all required fields were decoded.
    #[error("causal anchor payload ended unexpectedly")]
    UnexpectedPayloadEnd,
    /// A causal-anchor payload contained bytes after its canonical final field.
    #[error("causal anchor payload contained trailing bytes")]
    TrailingPayloadBytes,
    /// A causal-anchor payload used the wrong versioned magic.
    #[error("invalid causal anchor {payload_kind} payload magic")]
    InvalidPayloadMagic {
        /// Payload family whose magic was invalid.
        payload_kind: &'static str,
    },
    /// A causal-anchor string field was not valid UTF-8.
    #[error("causal anchor field `{field}` is not valid UTF-8")]
    InvalidUtf8 {
        /// String field that failed decoding.
        field: &'static str,
    },
    /// A causal-anchor payload length cannot be represented by this runtime.
    #[error("causal anchor payload length exceeds the runtime address space")]
    PayloadLengthOverflow,
    /// A causal-anchor payload carried an unknown enum code.
    #[error("unknown causal anchor enum code {code} for {enum_name}")]
    UnknownEnumCode {
        /// Enum whose code was unknown.
        enum_name: &'static str,
        /// Unknown persisted code.
        code: u8,
    },
    /// A decoded payload did not use the one canonical byte representation.
    #[error("causal anchor payload is not canonically encoded")]
    NonCanonicalPayload,
    /// The encoded claim digest did not match the canonical claim fields.
    #[error("causal anchor claim digest mismatch")]
    ClaimDigestMismatch,
    /// The encoded admitted-fact digest did not match the claim and receipt.
    #[error("causal anchor fact digest mismatch")]
    AnchorDigestMismatch,
    /// The encoded anchor id did not match the admitted-fact digest.
    #[error("causal anchor id mismatch")]
    AnchorIdMismatch,
    /// The encoded admission receipt id did not match its WAL coordinate.
    #[error("causal anchor admission receipt id mismatch")]
    AdmissionReceiptIdMismatch,
    /// An anchor fact and receipt did not describe the same admission.
    #[error("causal anchor fact and admission receipt evidence mismatch")]
    AdmissionEvidenceMismatch,
    /// Subject field cannot be empty.
    #[error("causal anchor subject field `{field}` cannot be empty")]
    EmptySubjectField {
        /// Empty subject field name.
        field: &'static str,
    },
    /// An application root field cannot be empty.
    #[error("causal anchor {root_kind} root field `{field}` cannot be empty")]
    EmptyRootField {
        /// Root family containing the empty field.
        root_kind: &'static str,
        /// Empty application-root field.
        field: &'static str,
    },
    /// Anchor must retain at least one authority or evidence root.
    #[error("causal anchor must retain at least one root")]
    EmptyRetainedRoots,
    /// Retained root set includes a duplicate root.
    #[error("causal anchor retained roots must be unique")]
    DuplicateRetainedRoot,
    /// Materialization root set includes a duplicate root.
    #[error("causal anchor materialization roots must be unique")]
    DuplicateMaterializationRoot,
    /// Materialization roots cannot declare authority.
    #[error("causal anchor materialization roots cannot declare authority")]
    AuthorityMaterializationRoot,
    /// A root cannot be both retained authority/evidence and materialization.
    #[error("causal anchor roots must not appear in both retained and materialization sets")]
    RootAppearsInRetainedAndMaterialization,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CausalAnchorRootSet {
    Retained,
    Materialization,
}

fn validate_subject(subject: &CausalAnchorSubject) -> Result<(), CausalAnchorError> {
    validate_non_empty("app_id", &subject.app_id)?;
    validate_non_empty("subject_kind", &subject.subject_kind)?;
    validate_non_empty("subject_id", &subject.subject_id)
}

fn validate_non_empty(field: &'static str, value: &str) -> Result<(), CausalAnchorError> {
    if value.is_empty() {
        return Err(CausalAnchorError::EmptySubjectField { field });
    }
    Ok(())
}

fn canonicalize_roots(
    mut roots: Vec<CausalAnchorRoot>,
    root_set: CausalAnchorRootSet,
) -> Result<Vec<CausalAnchorRoot>, CausalAnchorError> {
    for root in &roots {
        validate_root(root)?;
    }
    roots.sort();
    if roots.windows(2).any(|window| window[0] == window[1]) {
        return Err(match root_set {
            CausalAnchorRootSet::Retained => CausalAnchorError::DuplicateRetainedRoot,
            CausalAnchorRootSet::Materialization => CausalAnchorError::DuplicateMaterializationRoot,
        });
    }
    Ok(roots)
}

fn validate_root(root: &CausalAnchorRoot) -> Result<(), CausalAnchorError> {
    if let CausalAnchorRoot::AppSubjectRoot {
        app_id,
        subject_kind,
        id,
        ..
    } = root
    {
        validate_non_empty_root("app-subject", "app_id", app_id)?;
        validate_non_empty_root("app-subject", "subject_kind", subject_kind)?;
        validate_non_empty_root("app-subject", "id", id)?;
    }
    Ok(())
}

fn validate_non_empty_root(
    root_kind: &'static str,
    field: &'static str,
    value: &str,
) -> Result<(), CausalAnchorError> {
    if value.is_empty() {
        return Err(CausalAnchorError::EmptyRootField { root_kind, field });
    }
    Ok(())
}

fn compute_claim_digest(
    schema_version: u32,
    subject: &CausalAnchorSubject,
    basis_frontier: &CausalFrontierRef,
    retained_roots: &[CausalAnchorRoot],
    materialization_roots: &[CausalAnchorRoot],
    purpose: CausalAnchorPurpose,
) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(CAUSAL_ANCHOR_CLAIM_DIGEST_DOMAIN);
    hasher.update(&schema_version.to_le_bytes());
    update_subject(&mut hasher, subject);
    hasher.update(&basis_frontier.frontier_digest);
    update_roots(&mut hasher, retained_roots);
    update_roots(&mut hasher, materialization_roots);
    hasher.update(&[purpose.tag()]);
    hasher.finalize().into()
}

fn compute_support_policy_digest(grants: &BTreeSet<CausalAnchorRootSupportGrant>) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(CAUSAL_ANCHOR_SUPPORT_POLICY_DIGEST_DOMAIN);
    hasher.update(&(grants.len() as u64).to_le_bytes());
    for grant in grants {
        hasher.update(&[grant.support_set.tag()]);
        update_subject(&mut hasher, &grant.subject);
        update_root(&mut hasher, &grant.root);
    }
    hasher.finalize().into()
}

fn compute_support_grant_digest(grant: &CausalAnchorRootSupportGrant) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(CAUSAL_ANCHOR_SUPPORT_GRANT_DIGEST_DOMAIN);
    hasher.update(&[grant.support_set.tag()]);
    update_subject(&mut hasher, &grant.subject);
    update_root(&mut hasher, &grant.root);
    hasher.finalize().into()
}

fn compute_admission_receipt_id(
    claim_digest: &Hash,
    support_policy_digest: &Hash,
    writer_epoch_id: &Hash,
    wal_transaction_id: &Hash,
    wal_first_lsn: u64,
) -> CausalAnchorAdmissionReceiptId {
    let mut hasher = Hasher::new();
    hasher.update(CAUSAL_ANCHOR_ADMISSION_RECEIPT_ID_DOMAIN);
    hasher.update(claim_digest);
    hasher.update(support_policy_digest);
    hasher.update(writer_epoch_id);
    hasher.update(wal_transaction_id);
    hasher.update(&wal_first_lsn.to_le_bytes());
    CausalAnchorAdmissionReceiptId(hasher.finalize().into())
}

fn compute_anchor_digest(claim_digest: &Hash, receipt_id: &CausalAnchorAdmissionReceiptId) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(CAUSAL_ANCHOR_FACT_DIGEST_DOMAIN);
    hasher.update(claim_digest);
    hasher.update(receipt_id.as_bytes());
    hasher.finalize().into()
}

fn compute_anchor_id(anchor_digest: &Hash) -> CausalAnchorId {
    let mut hasher = Hasher::new();
    hasher.update(CAUSAL_ANCHOR_ID_DOMAIN);
    hasher.update(anchor_digest);
    CausalAnchorId(hasher.finalize().into())
}

fn update_roots(hasher: &mut Hasher, roots: &[CausalAnchorRoot]) {
    hasher.update(&(roots.len() as u64).to_le_bytes());
    for root in roots {
        update_root(hasher, root);
    }
}

fn update_root(hasher: &mut Hasher, root: &CausalAnchorRoot) {
    match root {
        CausalAnchorRoot::CasObject { id, role } => {
            hasher.update(&[1, role.tag()]);
            hasher.update(id);
        }
        CausalAnchorRoot::GraphFact { id, role } => {
            hasher.update(&[2, role.tag()]);
            hasher.update(id);
        }
        CausalAnchorRoot::AppSubjectRoot {
            app_id,
            subject_kind,
            id,
            role,
        } => {
            hasher.update(&[3, role.tag()]);
            update_string(hasher, app_id);
            update_string(hasher, subject_kind);
            update_string(hasher, id);
        }
    }
}

fn update_subject(hasher: &mut Hasher, subject: &CausalAnchorSubject) {
    update_string(hasher, &subject.app_id);
    update_string(hasher, &subject.subject_kind);
    update_string(hasher, &subject.subject_id);
}

fn update_string(hasher: &mut Hasher, value: &str) {
    let bytes = value.as_bytes();
    hasher.update(&(bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}

fn push_roots(out: &mut Vec<u8>, roots: &[CausalAnchorRoot]) {
    push_len(out, roots.len());
    for root in roots {
        match root {
            CausalAnchorRoot::CasObject { id, role } => {
                out.push(1);
                out.extend_from_slice(id);
                out.push(role.tag());
            }
            CausalAnchorRoot::GraphFact { id, role } => {
                out.push(2);
                out.extend_from_slice(id);
                out.push(role.tag());
            }
            CausalAnchorRoot::AppSubjectRoot {
                app_id,
                subject_kind,
                id,
                role,
            } => {
                out.push(3);
                push_string(out, app_id);
                push_string(out, subject_kind);
                push_string(out, id);
                out.push(role.tag());
            }
        }
    }
}

fn push_string(out: &mut Vec<u8>, value: &str) {
    push_bytes(out, value.as_bytes());
}

fn push_bytes(out: &mut Vec<u8>, value: &[u8]) {
    push_len(out, value.len());
    out.extend_from_slice(value);
}

fn push_len(out: &mut Vec<u8>, len: usize) {
    let encoded = match u64::try_from(len) {
        Ok(value) => value,
        Err(_) => u64::MAX,
    };
    out.extend_from_slice(&encoded.to_le_bytes());
}

struct CausalAnchorPayloadCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> CausalAnchorPayloadCursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn expect_magic(
        &mut self,
        expected: &[u8],
        payload_kind: &'static str,
    ) -> Result<(), CausalAnchorError> {
        if self.read_exact(expected.len())? != expected {
            return Err(CausalAnchorError::InvalidPayloadMagic { payload_kind });
        }
        Ok(())
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], CausalAnchorError> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(CausalAnchorError::UnexpectedPayloadEnd)?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or(CausalAnchorError::UnexpectedPayloadEnd)?;
        self.offset = end;
        Ok(bytes)
    }

    fn read_u8(&mut self) -> Result<u8, CausalAnchorError> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_u32(&mut self) -> Result<u32, CausalAnchorError> {
        let mut bytes = [0; 4];
        bytes.copy_from_slice(self.read_exact(4)?);
        Ok(u32::from_le_bytes(bytes))
    }

    fn read_u64(&mut self) -> Result<u64, CausalAnchorError> {
        let mut bytes = [0; 8];
        bytes.copy_from_slice(self.read_exact(8)?);
        Ok(u64::from_le_bytes(bytes))
    }

    fn read_hash(&mut self) -> Result<Hash, CausalAnchorError> {
        let mut hash = [0; 32];
        hash.copy_from_slice(self.read_exact(32)?);
        Ok(hash)
    }

    fn read_bytes(&mut self) -> Result<Vec<u8>, CausalAnchorError> {
        let len = usize::try_from(self.read_u64()?)
            .map_err(|_| CausalAnchorError::PayloadLengthOverflow)?;
        Ok(self.read_exact(len)?.to_vec())
    }

    fn read_string(&mut self, field: &'static str) -> Result<String, CausalAnchorError> {
        String::from_utf8(self.read_bytes()?).map_err(|_| CausalAnchorError::InvalidUtf8 { field })
    }

    fn read_roots(&mut self) -> Result<Vec<CausalAnchorRoot>, CausalAnchorError> {
        let count = usize::try_from(self.read_u64()?)
            .map_err(|_| CausalAnchorError::PayloadLengthOverflow)?;
        let mut roots = Vec::new();
        for _ in 0..count {
            roots.push(self.read_root()?);
        }
        Ok(roots)
    }

    fn read_root(&mut self) -> Result<CausalAnchorRoot, CausalAnchorError> {
        match self.read_u8()? {
            1 => Ok(CausalAnchorRoot::CasObject {
                id: self.read_hash()?,
                role: CausalAnchorCasRole::from_tag(self.read_u8()?)?,
            }),
            2 => Ok(CausalAnchorRoot::GraphFact {
                id: self.read_hash()?,
                role: CausalAnchorGraphRole::from_tag(self.read_u8()?)?,
            }),
            3 => Ok(CausalAnchorRoot::AppSubjectRoot {
                app_id: self.read_string("root.app_id")?,
                subject_kind: self.read_string("root.subject_kind")?,
                id: self.read_string("root.id")?,
                role: CausalAnchorAppRootRole::from_tag(self.read_u8()?)?,
            }),
            code => Err(CausalAnchorError::UnknownEnumCode {
                enum_name: "CausalAnchorRoot",
                code,
            }),
        }
    }

    fn finish(&self) -> Result<(), CausalAnchorError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(CausalAnchorError::TrailingPayloadBytes)
        }
    }
}
