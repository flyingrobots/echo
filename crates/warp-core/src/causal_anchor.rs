// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Canonical causal-anchor claim contract.
//!
//! An application request binds a subject, causal-frontier digest, claimed
//! authority/evidence roots, optional projection roots, and purpose into a
//! deterministic claim. Claim construction does not confer admission. Only the
//! trusted Echo path may attach a receipt and construct an admitted fact.

use blake3::Hasher;
use thiserror::Error;

use crate::ident::Hash;

const CAUSAL_ANCHOR_CLAIM_DIGEST_DOMAIN: &[u8] = b"echo:causal-anchor:claim:v1\0";

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
    /// Subject field cannot be empty.
    #[error("causal anchor subject field `{field}` cannot be empty")]
    EmptySubjectField {
        /// Empty subject field name.
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
    roots.sort();
    if roots.windows(2).any(|window| window[0] == window[1]) {
        return Err(match root_set {
            CausalAnchorRootSet::Retained => CausalAnchorError::DuplicateRetainedRoot,
            CausalAnchorRootSet::Materialization => CausalAnchorError::DuplicateMaterializationRoot,
        });
    }
    Ok(roots)
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
