// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Echo-admitted causal anchors.
//!
//! A causal anchor names a durable causal basis for a subject. It does not store
//! a materialized snapshot. It binds an application subject, a causal frontier,
//! retained authority/evidence roots, optional projection roots, an admission
//! receipt, and a purpose into deterministic Echo-owned evidence.

use blake3::Hasher;
use thiserror::Error;

use crate::ident::Hash;

const CAUSAL_ANCHOR_DIGEST_DOMAIN: &[u8] = b"echo:causal-anchor:digest:v1\0";
const CAUSAL_ANCHOR_ID_DOMAIN: &[u8] = b"echo:causal-anchor:id:v1\0";

/// Opaque Echo causal anchor identifier.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CausalAnchorId(Hash);

impl CausalAnchorId {
    /// Builds an anchor id from canonical bytes.
    #[must_use]
    pub const fn from_bytes(bytes: Hash) -> Self {
        Self(bytes)
    }

    /// Returns this anchor id's canonical bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &Hash {
        &self.0
    }
}

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

/// Opaque reference to an admitted causal frontier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CausalFrontierRef {
    /// Digest of the admitted frontier being anchored.
    pub frontier_digest: Hash,
}

impl CausalFrontierRef {
    /// Builds a causal frontier reference from its digest.
    #[must_use]
    pub const fn from_digest(frontier_digest: Hash) -> Self {
        Self { frontier_digest }
    }
}

/// Purpose under which Echo admitted a causal anchor.
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

/// Root retained by or attached to a causal anchor.
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

/// Request to construct an Echo causal anchor fact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CausalAnchorRequest {
    /// Application subject being anchored.
    pub subject: CausalAnchorSubject,
    /// Admitted causal frontier being anchored.
    pub basis_frontier: CausalFrontierRef,
    /// Authority/evidence roots retained by the anchor.
    pub retained_roots: Vec<CausalAnchorRoot>,
    /// Optional derived projection roots attached to the anchor.
    pub materialization_roots: Vec<CausalAnchorRoot>,
    /// Purpose of the anchor.
    pub purpose: CausalAnchorPurpose,
    /// Receipt digest that admitted the anchor request.
    pub admitted_by_receipt_id: Hash,
}

/// Echo causal anchor fact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CausalAnchorFact {
    /// Stable anchor id derived from the anchor digest.
    pub anchor_id: CausalAnchorId,
    /// Application subject being anchored.
    pub subject: CausalAnchorSubject,
    /// Admitted causal frontier being anchored.
    pub basis_frontier: CausalFrontierRef,
    /// Canonical retained root set.
    pub retained_roots: Vec<CausalAnchorRoot>,
    /// Canonical materialization root set.
    pub materialization_roots: Vec<CausalAnchorRoot>,
    /// Purpose of the anchor.
    pub purpose: CausalAnchorPurpose,
    /// Receipt digest that admitted the anchor request.
    pub admitted_by_receipt_id: Hash,
    /// Digest over subject, basis, roots, purpose, and admission receipt.
    pub anchor_digest: Hash,
}

impl CausalAnchorFact {
    /// Builds a validated, canonical causal anchor fact.
    ///
    /// Root vectors are sorted and duplicate roots are rejected so the resulting
    /// digest represents a set of retained/materialized roots rather than caller
    /// iteration order.
    pub fn from_request(request: CausalAnchorRequest) -> Result<Self, CausalAnchorError> {
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
        let anchor_digest = compute_anchor_digest(
            &request.subject,
            &request.basis_frontier,
            &retained_roots,
            &materialization_roots,
            request.purpose,
            &request.admitted_by_receipt_id,
        );
        let anchor_id = compute_anchor_id(&anchor_digest);
        Ok(Self {
            anchor_id,
            subject: request.subject,
            basis_frontier: request.basis_frontier,
            retained_roots,
            materialization_roots,
            purpose: request.purpose,
            admitted_by_receipt_id: request.admitted_by_receipt_id,
            anchor_digest,
        })
    }
}

/// Causal anchor validation error.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum CausalAnchorError {
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

fn compute_anchor_id(anchor_digest: &Hash) -> CausalAnchorId {
    let mut hasher = Hasher::new();
    hasher.update(CAUSAL_ANCHOR_ID_DOMAIN);
    hasher.update(anchor_digest);
    CausalAnchorId(hasher.finalize().into())
}

fn compute_anchor_digest(
    subject: &CausalAnchorSubject,
    basis_frontier: &CausalFrontierRef,
    retained_roots: &[CausalAnchorRoot],
    materialization_roots: &[CausalAnchorRoot],
    purpose: CausalAnchorPurpose,
    admitted_by_receipt_id: &Hash,
) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(CAUSAL_ANCHOR_DIGEST_DOMAIN);
    update_subject(&mut hasher, subject);
    hasher.update(&basis_frontier.frontier_digest);
    update_roots(&mut hasher, retained_roots);
    update_roots(&mut hasher, materialization_roots);
    hasher.update(&[purpose.tag()]);
    hasher.update(admitted_by_receipt_id);
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
