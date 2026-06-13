// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Revelation posture for retained causal artifacts (Three-Tier Thinking
//! Room, AIΩN Paper VII §6.3; tracked by echo#538).
//!
//! Every retained shell-family artifact carries an explicit causal/revelation
//! posture instead of implicit shared visibility:
//!
//! - [`CausalPosture::Scratch`] — local, weakly retained, disposable.
//! - [`CausalPosture::AuthorOnly`] — durable and replayable, sealed to the
//!   creating authority until explicitly admitted.
//! - [`CausalPosture::Shared`] — scoped, collaboratively admitted visibility.
//!
//! Posture is load-bearing, not cosmetic. Two laws are enforced here:
//!
//! 1. **Admission is explicit and witnessed.** Shared admission only widens
//!    through [`PromotionIntent`], which binds authority, scope, intent, and
//!    witness; silent widening and any narrowing are obstructions, never
//!    no-ops.
//! 2. **Least-revealed-member invariant.** A composite artifact (for
//!    example a braid shell over member strands) cannot reveal more than
//!    its least-revealed member unless a witnessed redaction/promotion
//!    transform exists; [`shell_posture_obstruction`] is the single
//!    admission check for that rule.
//!
//! This module is the E0-lite core required by design packet 0026 before
//! any θ_braid shell lands; the full strand-creation posture system
//! remains echo#538.

use crate::ident::Hash;
use crate::playback::SessionId;
use crate::strand::StrandId;

macro_rules! hash_id {
    ($name:ident) => {
        #[doc = concat!("Opaque hash-backed identifier for `", stringify!($name), "`.")]
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(Hash);

        impl $name {
            /// Construct this identifier from raw canonical bytes.
            #[must_use]
            pub const fn from_bytes(bytes: Hash) -> Self {
                Self(bytes)
            }

            /// Returns this identifier's raw canonical bytes.
            #[must_use]
            pub const fn as_bytes(&self) -> &Hash {
                &self.0
            }
        }
    };
}

/// Causal/revelation posture for one retained causal artifact.
///
/// Ordering is revelation breadth: `Scratch < AuthorOnly < Shared`. The
/// type intentionally has no global default; named constructors and migration
/// paths are the only places that may choose posture policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CausalPosture {
    /// Local, weakly retained, disposable working tier.
    Scratch,
    /// Durable and replayable, sealed to the creating principal.
    AuthorOnly,
    /// Scoped, collaboratively admitted visibility.
    Shared,
}

/// Compatibility alias for E0-lite callers. New code uses [`CausalPosture`].
#[deprecated(note = "Use CausalPosture")]
pub type RevelationPosture = CausalPosture;

impl CausalPosture {
    /// Stable wire tag for canonical serialization and digest domains.
    #[must_use]
    pub fn canonical_tag(self) -> u8 {
        match self {
            Self::Scratch => 0x01,
            Self::AuthorOnly => 0x02,
            Self::Shared => 0x03,
        }
    }
}

hash_id!(OriginId);
hash_id!(ActorId);
hash_id!(AuthorityDomainId);
hash_id!(AuthorityCapabilityDigest);
hash_id!(AdmissionScopeId);
hash_id!(RetentionContractId);
hash_id!(IntentId);
hash_id!(ImportedArtifactId);
hash_id!(KeyId);
hash_id!(KeyProofId);
hash_id!(DelegationProofId);
hash_id!(ImportGrantId);
hash_id!(ProjectionSpecId);
hash_id!(AdmissionId);

/// Witnessed record that one artifact's posture lawfully widened.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PosturePromotion {
    /// Posture before the promotion act.
    pub from: CausalPosture,
    /// Posture after the promotion act.
    pub to: CausalPosture,
    /// Witness digest binding the explicit promotion act.
    pub witness: Hash,
}

/// Globally comparable authority-domain reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AuthorityDomainRef {
    /// Origin where this authority domain was minted.
    pub origin_id: OriginId,
    /// Authority domain local to `origin_id`.
    pub domain_id: AuthorityDomainId,
}

impl AuthorityDomainRef {
    /// Builds a canonical authority-domain reference.
    #[must_use]
    pub const fn new(origin_id: OriginId, domain_id: AuthorityDomainId) -> Self {
        Self {
            origin_id,
            domain_id,
        }
    }
}

/// Binding proof shape for an authority domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityBinding {
    /// Local policy-only authority tied to an origin.
    LocalUnbound {
        /// Local origin creating the authority.
        origin: OriginId,
    },
    /// Local authority tied to a key id.
    LocalKeyed {
        /// Key identifier.
        key_id: KeyId,
    },
    /// Delegated authority from another domain.
    Delegated {
        /// Delegating authority.
        from: AuthorityDomainRef,
        /// Delegation proof identifier.
        proof: DelegationProofId,
    },
    /// Imported authority not yet resolved locally.
    ImportedUnresolved {
        /// Remote origin carrying the authority.
        remote_origin: OriginId,
        /// Remote authority reference.
        remote_authority: AuthorityDomainRef,
    },
}

/// Honest strength of an author-only seal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SealStrength {
    /// Metadata/policy only.
    Advisory,
    /// Enforced by local runtime process boundaries.
    LocalProcess,
    /// Enforced by local storage permissions or keychain.
    LocalStorage,
    /// Enforced by cryptographic wrapping/signing.
    Cryptographic,
}

/// Capability evidence usable before full identity exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityProof {
    /// Current session presents authority.
    LocalSessionAuthority(SessionId),
    /// Local authority-domain proof.
    LocalAuthorityDomain(AuthorityDomainRef),
    /// Digest of a witnessed delegated authority/capability claim.
    AuthorityCapabilityDigest(AuthorityCapabilityDigest),
    /// Delegation proof.
    DelegationProof(DelegationProofId),
    /// Explicit import/adoption grant.
    ImportGrant(ImportGrantId),
}

impl CapabilityProof {
    /// Rejects generic causal witnesses as authority proof.
    ///
    /// # Errors
    ///
    /// Always returns [`PostureObstruction::WitnessIsNotAuthorityCapability`].
    pub const fn from_causal_witness(_witness: WitnessDigest) -> Result<Self, PostureObstruction> {
        Err(PostureObstruction::WitnessIsNotAuthorityCapability)
    }
}

/// Authority context attached to retained causal work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CausalAuthority {
    /// Origin where this work was created.
    pub origin_id: OriginId,
    /// Concrete actor that performed creation.
    pub actor_id: ActorId,
    /// Authority domain controlling revelation/admission.
    pub author_domain: AuthorityDomainRef,
    /// Binding strength for `author_domain`.
    pub binding: AuthorityBinding,
    /// Enforcement quality of the seal.
    pub seal_strength: SealStrength,
}

impl CausalAuthority {
    /// Builds a validated authority context.
    ///
    /// # Errors
    ///
    /// Returns an authority-coherence obstruction when the work origin,
    /// author-domain origin, or binding origin/domain disagree.
    pub fn new(
        origin_id: OriginId,
        actor_id: ActorId,
        author_domain: AuthorityDomainRef,
        binding: AuthorityBinding,
        seal_strength: SealStrength,
    ) -> Result<Self, PostureObstruction> {
        validate_authority_coherence(origin_id, author_domain, binding)?;
        Ok(Self {
            origin_id,
            actor_id,
            author_domain,
            binding,
            seal_strength,
        })
    }

    fn validate(&self) -> Result<(), PostureObstruction> {
        validate_authority_coherence(self.origin_id, self.author_domain, self.binding)
    }
}

/// Records how posture was assigned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostureDerivation {
    /// Explicit admission or construction intent.
    ExplicitIntent,
    /// Inherited from a session default.
    SessionDefault,
    /// Debugger constructor default.
    DebuggerDefault,
    /// Counterfactual constructor default.
    CounterfactualDefault,
    /// Legacy durable evidence assumed shared for compatibility.
    LegacyDurableAssumedShared,
    /// Legacy ephemeral registry assumed scratch.
    LegacyEphemeralAssumedScratch,
    /// Imported manifest supplied the posture.
    ImportedManifest,
}

/// Posture, authority, scope, derivation, and retention contract bundle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetentionPosture {
    /// Effective causal posture.
    pub causal_posture: CausalPosture,
    /// How posture was assigned.
    pub posture_derivation: PostureDerivation,
    /// Causal authority for revelation/admission.
    pub authority: CausalAuthority,
    /// Retention contract Lambda.
    pub retention_contract: RetentionContractId,
    /// Shared-admission scope, present only for `Shared`.
    pub admission_scope: Option<AdmissionScopeId>,
}

impl RetentionPosture {
    /// Builds a validated retention posture bundle.
    ///
    /// # Errors
    ///
    /// Returns an obstruction when the posture/scope pair, derivation, or
    /// authority context is incoherent.
    pub fn new(
        causal_posture: CausalPosture,
        posture_derivation: PostureDerivation,
        authority: CausalAuthority,
        retention_contract: RetentionContractId,
        admission_scope: Option<AdmissionScopeId>,
    ) -> Result<Self, PostureObstruction> {
        validate_admission_scope(causal_posture, admission_scope)?;
        validate_posture_derivation(causal_posture, posture_derivation)?;
        authority.validate()?;
        Ok(Self {
            causal_posture,
            posture_derivation,
            authority,
            retention_contract,
            admission_scope,
        })
    }
}

/// Session context posture and authority defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionContext {
    /// Session identifier.
    pub session_id: SessionId,
    /// Local/remote origin.
    pub origin_id: OriginId,
    /// Concrete actor performing operations.
    pub actor_id: ActorId,
    /// Authority domain work is created under.
    pub author_domain: AuthorityDomainRef,
    /// Authority binding proof shape.
    pub authority_binding: AuthorityBinding,
    /// Seal strength for author-only work.
    pub seal_strength: SealStrength,
    /// Default posture for newly created work.
    pub default_posture: CausalPosture,
    /// Default admission scope. Present only when default posture is shared.
    pub default_admission_scope: Option<AdmissionScopeId>,
    /// Retention contract Lambda.
    pub retention_contract: RetentionContractId,
}

impl SessionContext {
    /// Builds a validated session context.
    ///
    /// # Errors
    ///
    /// Returns an admission-scope obstruction when the session default posture
    /// and default admission scope do not agree.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        session_id: SessionId,
        origin_id: OriginId,
        actor_id: ActorId,
        author_domain: AuthorityDomainRef,
        authority_binding: AuthorityBinding,
        seal_strength: SealStrength,
        default_posture: CausalPosture,
        default_admission_scope: Option<AdmissionScopeId>,
        retention_contract: RetentionContractId,
    ) -> Result<Self, PostureObstruction> {
        validate_admission_scope(default_posture, default_admission_scope)?;
        validate_authority_coherence(origin_id, author_domain, authority_binding)?;
        Ok(Self {
            session_id,
            origin_id,
            actor_id,
            author_domain,
            authority_binding,
            seal_strength,
            default_posture,
            default_admission_scope,
            retention_contract,
        })
    }
}

/// Obstruction raised when a posture act is unlawful.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostureObstruction {
    /// Posture may only widen; narrowing is never a promotion.
    NarrowingRefused {
        /// Posture the artifact currently holds.
        from: CausalPosture,
        /// Narrower posture that was unlawfully requested.
        requested: CausalPosture,
    },
    /// Promotion to the same posture is a no-op dressed as an act.
    AlreadyAtPosture {
        /// Posture the artifact already holds.
        posture: CausalPosture,
    },
    /// A composite shell may not reveal more than its least-revealed member.
    ExceedsLeastRevealedMember {
        /// Posture requested for the composite shell.
        shell: CausalPosture,
        /// Least-revealed posture among the shell's members.
        least_revealed_member: CausalPosture,
    },
    /// A witness digest must never be a 32-byte shrug.
    EmptyWitness,
    /// Shared posture requires an explicit admission scope.
    MissingAdmissionScope {
        /// Posture that requires the missing scope.
        posture: CausalPosture,
    },
    /// Non-shared posture must not carry an admission scope.
    UnexpectedAdmissionScope {
        /// Posture that unlawfully carried a scope.
        posture: CausalPosture,
    },
    /// Durable materialization has exactly one posture pair:
    /// `Scratch -> AuthorOnly`.
    InvalidMaterializationTransition {
        /// Requested source posture.
        from: CausalPosture,
        /// Requested target posture.
        to: CausalPosture,
    },
    /// Shared admission must target `Shared`.
    PromotionRequiresSharedTarget {
        /// Requested target posture.
        to: CausalPosture,
    },
    /// Shared admission requires [`PromotionIntent`], not raw posture widening.
    SharedAdmissionRequiresIntent,
    /// Authority proof names a different authority than the operation.
    AuthorityProofMismatch {
        /// Authority named by the operation.
        authorized_by: AuthorityDomainRef,
    },
    /// Authority-domain origin does not match the work origin.
    AuthorityOriginMismatch {
        /// Work origin.
        origin_id: OriginId,
        /// Origin carried by the authority domain.
        authority_origin: OriginId,
    },
    /// Binding origin does not match the work origin.
    AuthorityBindingOriginMismatch {
        /// Work origin.
        origin_id: OriginId,
        /// Origin carried by the binding.
        binding_origin: OriginId,
    },
    /// Binding authority domain does not match the work authority domain.
    AuthorityBindingDomainMismatch {
        /// Authority domain carried by the work.
        author_domain: AuthorityDomainRef,
    },
    /// Posture derivation cannot truthfully explain the effective posture.
    PostureDerivationMismatch {
        /// Effective posture.
        posture: CausalPosture,
        /// Claimed derivation.
        derivation: PostureDerivation,
    },
    /// Legacy shared compatibility proof is not authority for new admission.
    LegacyAuthorityCannotAuthorizeNewAdmission,
    /// Generic causal witness digests are not authority-capability proofs.
    WitnessIsNotAuthorityCapability,
}

/// Witness digest with a quality bar: zero and empty-input digests refused.
///
/// The witnessed-act law is enforced by the type system: any API that takes
/// a `WitnessDigest` cannot be handed a shrug, because the shrug never
/// constructs. Shared by posture promotion and the braid shell family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WitnessDigest(Hash);

impl WitnessDigest {
    /// Wraps a witness digest, refusing shrug values.
    ///
    /// # Errors
    ///
    /// Returns [`PostureObstruction::EmptyWitness`] for the all-zero digest
    /// and the digest of empty input.
    pub fn new(hash: Hash) -> Result<Self, PostureObstruction> {
        if hash == [0; 32] || hash == crate::blake3_empty() {
            return Err(PostureObstruction::EmptyWitness);
        }
        Ok(Self(hash))
    }

    /// Returns the underlying digest.
    #[must_use]
    pub fn as_hash(&self) -> &Hash {
        &self.0
    }
}

/// Authority-resolution proof for admission/materialization operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityResolutionProof {
    /// Local authority-domain proof.
    LocalAuthorityDomain(AuthorityDomainRef),
    /// Local capability presentation.
    LocalCapability(CapabilityProof),
    /// Key proof.
    KeyProof(KeyProofId),
    /// Delegation proof.
    DelegationProof(DelegationProofId),
    /// Explicit import/adoption grant.
    ImportGrant(ImportGrantId),
    /// Compatibility-only explanation for migrated legacy shared visibility.
    LegacySharedAuthority,
}

impl AuthorityResolutionProof {
    fn authorizes_new_admission(
        self,
        authorized_by: AuthorityDomainRef,
    ) -> Result<(), PostureObstruction> {
        match self {
            Self::LegacySharedAuthority => {
                Err(PostureObstruction::LegacyAuthorityCannotAuthorizeNewAdmission)
            }
            Self::LocalAuthorityDomain(proof_authority)
            | Self::LocalCapability(CapabilityProof::LocalAuthorityDomain(proof_authority))
                if proof_authority != authorized_by =>
            {
                Err(PostureObstruction::AuthorityProofMismatch { authorized_by })
            }
            Self::LocalAuthorityDomain(_)
            | Self::LocalCapability(_)
            | Self::KeyProof(_)
            | Self::DelegationProof(_)
            | Self::ImportGrant(_) => Ok(()),
        }
    }
}

/// Basis for durable scratch materialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterializationBasis {
    /// User/tool explicitly saved the scratch object.
    ExplicitSave,
}

/// Receipt for a durable `Scratch -> AuthorOnly` materialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterializationReceipt {
    /// Source strand being materialized.
    source: StrandId,
    /// Source posture.
    from: CausalPosture,
    /// Target posture.
    to: CausalPosture,
    /// Concrete actor performing the save.
    actor: ActorId,
    /// Authority domain authorizing retention.
    authorized_by: AuthorityDomainRef,
    /// Authority proof.
    authority_proof: AuthorityResolutionProof,
    /// Retention contract Lambda.
    retention_contract: RetentionContractId,
    /// Materialization basis.
    basis: MaterializationBasis,
}

impl MaterializationReceipt {
    /// Returns the source strand being materialized.
    #[must_use]
    pub const fn source(&self) -> StrandId {
        self.source
    }

    /// Returns the source posture.
    #[must_use]
    pub const fn from(&self) -> CausalPosture {
        self.from
    }

    /// Returns the target posture.
    #[must_use]
    pub const fn to(&self) -> CausalPosture {
        self.to
    }

    /// Returns the actor that performed the save.
    #[must_use]
    pub const fn actor(&self) -> ActorId {
        self.actor
    }

    /// Returns the authority domain authorizing retention.
    #[must_use]
    pub const fn authorized_by(&self) -> AuthorityDomainRef {
        self.authorized_by
    }

    /// Returns the authority proof.
    #[must_use]
    pub const fn authority_proof(&self) -> AuthorityResolutionProof {
        self.authority_proof
    }

    /// Returns the retention contract.
    #[must_use]
    pub const fn retention_contract(&self) -> RetentionContractId {
        self.retention_contract
    }

    /// Returns the materialization basis.
    #[must_use]
    pub const fn basis(&self) -> MaterializationBasis {
        self.basis
    }

    /// Builds a validated materialization receipt.
    ///
    /// # Errors
    ///
    /// Returns [`PostureObstruction::InvalidMaterializationTransition`] unless
    /// the transition is exactly `Scratch -> AuthorOnly`, or
    /// [`PostureObstruction::LegacyAuthorityCannotAuthorizeNewAdmission`] when
    /// legacy shared compatibility proof is used as authority.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source: StrandId,
        from: CausalPosture,
        to: CausalPosture,
        actor: ActorId,
        authorized_by: AuthorityDomainRef,
        authority_proof: AuthorityResolutionProof,
        retention_contract: RetentionContractId,
        basis: MaterializationBasis,
    ) -> Result<Self, PostureObstruction> {
        if from != CausalPosture::Scratch || to != CausalPosture::AuthorOnly {
            return Err(PostureObstruction::InvalidMaterializationTransition { from, to });
        }
        authority_proof.authorizes_new_admission(authorized_by)?;
        Ok(Self {
            source,
            from,
            to,
            actor,
            authorized_by,
            authority_proof,
            retention_contract,
            basis,
        })
    }
}

/// Basis for promotion/admission to shared history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromotionBasis {
    /// Explicit admission intent.
    ExplicitAdmission,
}

/// Projection policy for shared admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionPolicy {
    /// Admit only the final result projection.
    FinalResultOnly,
    /// Admit result with redacted basis.
    ResultPlusRedactedBasis,
    /// Admit result with stubbed basis.
    ResultPlusStubbedBasis,
    /// Admit the full source chain.
    FullSourceChain,
    /// Custom projection spec.
    CustomProjection(ProjectionSpecId),
}

/// Source disclosure policy for shared admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceDisclosurePolicy {
    /// Reveal no source.
    RevealNone,
    /// Reveal a source stub.
    RevealStub,
    /// Reveal redacted source.
    RevealRedacted,
    /// Reveal full source.
    RevealFull,
    /// Reveal source only to authority holders.
    RevealByAuthorityOnly,
}

/// Explicit, witnessed admission intent for shared history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PromotionIntent {
    /// Intent identity.
    intent_id: IntentId,
    /// Concrete actor performing admission.
    actor: ActorId,
    /// Authority domain authorizing admission.
    authorized_by: AuthorityDomainRef,
    /// Authority proof.
    authority_proof: AuthorityResolutionProof,
    /// Source strand being admitted.
    source_strand: StrandId,
    /// Source posture.
    from: CausalPosture,
    /// Target posture. Must be `Shared`.
    to: CausalPosture,
    /// Target admission scope.
    admission_scope: AdmissionScopeId,
    /// Witness binding the admission act.
    witness: WitnessDigest,
    /// Promotion basis.
    basis: PromotionBasis,
    /// Projection policy.
    projection_policy: ProjectionPolicy,
    /// Source disclosure policy.
    source_disclosure: SourceDisclosurePolicy,
}

impl PromotionIntent {
    /// Returns the intent identity.
    #[must_use]
    pub const fn intent_id(&self) -> IntentId {
        self.intent_id
    }

    /// Returns the actor performing admission.
    #[must_use]
    pub const fn actor(&self) -> ActorId {
        self.actor
    }

    /// Returns the authority domain authorizing admission.
    #[must_use]
    pub const fn authorized_by(&self) -> AuthorityDomainRef {
        self.authorized_by
    }

    /// Returns the authority proof.
    #[must_use]
    pub const fn authority_proof(&self) -> AuthorityResolutionProof {
        self.authority_proof
    }

    /// Returns the source strand being admitted.
    #[must_use]
    pub const fn source_strand(&self) -> StrandId {
        self.source_strand
    }

    /// Returns the source posture.
    #[must_use]
    pub const fn from(&self) -> CausalPosture {
        self.from
    }

    /// Returns the target posture.
    #[must_use]
    pub const fn to(&self) -> CausalPosture {
        self.to
    }

    /// Returns the target admission scope.
    #[must_use]
    pub const fn admission_scope(&self) -> AdmissionScopeId {
        self.admission_scope
    }

    /// Returns the witness binding the admission act.
    #[must_use]
    pub const fn witness(&self) -> WitnessDigest {
        self.witness
    }

    /// Returns the promotion basis.
    #[must_use]
    pub const fn basis(&self) -> PromotionBasis {
        self.basis
    }

    /// Returns the projection policy.
    #[must_use]
    pub const fn projection_policy(&self) -> ProjectionPolicy {
        self.projection_policy
    }

    /// Returns the source disclosure policy.
    #[must_use]
    pub const fn source_disclosure(&self) -> SourceDisclosurePolicy {
        self.source_disclosure
    }

    /// Builds a validated promotion intent targeting shared history.
    ///
    /// # Errors
    ///
    /// Returns an obstruction when authority is legacy compatibility only or
    /// when the source posture is already `Shared`.
    #[allow(clippy::too_many_arguments)]
    pub fn admit_shared(
        intent_id: IntentId,
        actor: ActorId,
        authorized_by: AuthorityDomainRef,
        authority_proof: AuthorityResolutionProof,
        source_strand: StrandId,
        from: CausalPosture,
        admission_scope: AdmissionScopeId,
        witness: WitnessDigest,
        basis: PromotionBasis,
        projection_policy: ProjectionPolicy,
        source_disclosure: SourceDisclosurePolicy,
    ) -> Result<Self, PostureObstruction> {
        authority_proof.authorizes_new_admission(authorized_by)?;
        if from == CausalPosture::Shared {
            return Err(PostureObstruction::AlreadyAtPosture {
                posture: CausalPosture::Shared,
            });
        }
        Ok(Self {
            intent_id,
            actor,
            authorized_by,
            authority_proof,
            source_strand,
            from,
            to: CausalPosture::Shared,
            admission_scope,
            witness,
            basis,
            projection_policy,
            source_disclosure,
        })
    }
}

/// Source identity disclosed by a shared admission projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionSourceDisclosure {
    /// Source identity is withheld from the shared projection.
    Hidden,
    /// Source identity is disclosed as a stub.
    Stub {
        /// Sealed source strand.
        source_strand: StrandId,
    },
    /// Source identity is disclosed with redacted backing detail.
    Redacted {
        /// Sealed source strand.
        source_strand: StrandId,
    },
    /// Full source identity is disclosed.
    Full {
        /// Sealed source strand.
        source_strand: StrandId,
    },
    /// Source identity is available only to authority holders.
    AuthorityOnly {
        /// Sealed source strand.
        source_strand: StrandId,
    },
}

impl AdmissionSourceDisclosure {
    const fn from_policy(policy: SourceDisclosurePolicy, source_strand: StrandId) -> Self {
        match policy {
            SourceDisclosurePolicy::RevealNone => Self::Hidden,
            SourceDisclosurePolicy::RevealStub => Self::Stub { source_strand },
            SourceDisclosurePolicy::RevealRedacted => Self::Redacted { source_strand },
            SourceDisclosurePolicy::RevealFull => Self::Full { source_strand },
            SourceDisclosurePolicy::RevealByAuthorityOnly => Self::AuthorityOnly { source_strand },
        }
    }
}

/// Shared admission projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedAdmission {
    /// Admission identity.
    pub admission_id: AdmissionId,
    /// Source identity disclosure for this shared projection.
    pub source: AdmissionSourceDisclosure,
    /// Digest of the shared projection.
    pub projection_digest: Hash,
    /// Admission scope.
    pub admission_scope: AdmissionScopeId,
    /// Source disclosure policy.
    pub source_disclosure: SourceDisclosurePolicy,
}

impl SharedAdmission {
    /// Builds a shared admission from a validated promotion intent.
    #[must_use]
    pub const fn from_promotion(
        admission_id: AdmissionId,
        promotion: PromotionIntent,
        projection_digest: Hash,
    ) -> Self {
        Self {
            admission_id,
            source: AdmissionSourceDisclosure::from_policy(
                promotion.source_disclosure,
                promotion.source_strand,
            ),
            projection_digest,
            admission_scope: promotion.admission_scope,
            source_disclosure: promotion.source_disclosure,
        }
    }
}

/// Receipt proving imported source-shared material was admitted locally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportAdmissionReceipt {
    /// Intent that admitted the import locally.
    intent_id: IntentId,
    /// Imported artifact identity this receipt admits.
    imported_artifact_id: ImportedArtifactId,
    /// Authority domain authorizing local admission.
    authorized_by: AuthorityDomainRef,
    /// Authority proof.
    authority_proof: AuthorityResolutionProof,
    /// Local admission scope.
    admission_scope: AdmissionScopeId,
    /// Witness binding the import admission.
    witness: WitnessDigest,
}

impl ImportAdmissionReceipt {
    /// Returns the intent that admitted the import locally.
    #[must_use]
    pub const fn intent_id(&self) -> IntentId {
        self.intent_id
    }

    /// Returns the imported artifact identity this receipt admits.
    #[must_use]
    pub const fn imported_artifact_id(&self) -> ImportedArtifactId {
        self.imported_artifact_id
    }

    /// Returns the authority domain authorizing local admission.
    #[must_use]
    pub const fn authorized_by(&self) -> AuthorityDomainRef {
        self.authorized_by
    }

    /// Returns the authority proof.
    #[must_use]
    pub const fn authority_proof(&self) -> AuthorityResolutionProof {
        self.authority_proof
    }

    /// Returns the local admission scope.
    #[must_use]
    pub const fn admission_scope(&self) -> AdmissionScopeId {
        self.admission_scope
    }

    /// Returns the witness binding the import admission.
    #[must_use]
    pub const fn witness(&self) -> WitnessDigest {
        self.witness
    }

    /// Builds a validated import admission receipt.
    ///
    /// # Errors
    ///
    /// Returns an obstruction when the authority proof cannot authorize
    /// `authorized_by`.
    pub fn new(
        intent_id: IntentId,
        imported_artifact_id: ImportedArtifactId,
        authorized_by: AuthorityDomainRef,
        authority_proof: AuthorityResolutionProof,
        admission_scope: AdmissionScopeId,
        witness: WitnessDigest,
    ) -> Result<Self, PostureObstruction> {
        authority_proof.authorizes_new_admission(authorized_by)?;
        Ok(Self {
            intent_id,
            imported_artifact_id,
            authorized_by,
            authority_proof,
            admission_scope,
            witness,
        })
    }
}

/// Quarantine or pending-admission namespace for imports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportQuarantineNamespace {
    /// Imported material with unresolved authority.
    ImportedUnresolvedLane,
    /// Foreign author-only material sealed pending authority resolution.
    ForeignAuthorOnlyQuarantine,
    /// Source-shared material readable elsewhere but not locally admitted.
    SourceSharedPendingAdmission,
}

/// Import disposition for source posture and local authority/scope state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportPostureDisposition {
    /// Scratch is not imported by default.
    ScratchNotImportedByDefault,
    /// Foreign author-only material remains sealed.
    ForeignAuthorOnlySealed {
        /// Quarantine namespace.
        namespace: ImportQuarantineNamespace,
    },
    /// Author-only material can be revealed under resolved authority.
    AuthorOnlyResolved,
    /// Source-shared material awaits local admission.
    SourceSharedPendingAdmission {
        /// Pending-admission namespace.
        namespace: ImportQuarantineNamespace,
    },
    /// Source-shared material has a local admission scope.
    LocallyAdmittedShared {
        /// Local admission scope.
        admission_scope: AdmissionScopeId,
    },
}

/// Classifies imported posture without laundering remote visibility into local truth.
#[must_use]
pub fn import_posture_disposition(
    source_posture: CausalPosture,
    authority_resolved: bool,
    imported_artifact_id: ImportedArtifactId,
    local_admission: Option<ImportAdmissionReceipt>,
) -> ImportPostureDisposition {
    match source_posture {
        CausalPosture::Scratch => ImportPostureDisposition::ScratchNotImportedByDefault,
        CausalPosture::AuthorOnly if authority_resolved => {
            ImportPostureDisposition::AuthorOnlyResolved
        }
        CausalPosture::AuthorOnly => ImportPostureDisposition::ForeignAuthorOnlySealed {
            namespace: ImportQuarantineNamespace::ForeignAuthorOnlyQuarantine,
        },
        CausalPosture::Shared => match local_admission {
            Some(receipt) if receipt.imported_artifact_id.0 == imported_artifact_id.0 => {
                ImportPostureDisposition::LocallyAdmittedShared {
                    admission_scope: receipt.admission_scope,
                }
            }
            Some(_) | None => ImportPostureDisposition::SourceSharedPendingAdmission {
                namespace: ImportQuarantineNamespace::SourceSharedPendingAdmission,
            },
        },
    }
}

/// Revelation-only operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevelationOperation {
    /// Read an object.
    Read,
    /// Replay an object.
    Replay,
    /// Inspect an object.
    Inspect,
    /// Debug-view an object.
    DebugView,
}

/// Posture effect of an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationPostureEffect {
    /// Operation leaves posture unchanged.
    Unchanged(CausalPosture),
}

/// Returns the posture effect of a revelation-only operation.
#[must_use]
pub const fn revelation_operation_effect(
    source: CausalPosture,
    _operation: RevelationOperation,
) -> OperationPostureEffect {
    OperationPostureEffect::Unchanged(source)
}

/// Returns the least-revealed posture among `members`.
///
/// An empty member set has no revelation to leak, so it imposes no bound;
/// this returns `None` and callers treat the shell posture as the only
/// constraint.
#[must_use]
pub fn least_revealed<I>(members: I) -> Option<CausalPosture>
where
    I: IntoIterator<Item = CausalPosture>,
{
    members.into_iter().min()
}

/// Checks the least-revealed-member invariant for a composite shell.
///
/// Returns the obstruction when `shell` would reveal more than the
/// least-revealed member; `None` means the posture is admissible.
#[must_use]
pub fn shell_posture_obstruction<I>(shell: CausalPosture, members: I) -> Option<PostureObstruction>
where
    I: IntoIterator<Item = CausalPosture>,
{
    let floor = least_revealed(members)?;
    if shell > floor {
        return Some(PostureObstruction::ExceedsLeastRevealedMember {
            shell,
            least_revealed_member: floor,
        });
    }
    None
}

/// Performs one explicit, witnessed posture promotion.
///
/// Promotion only widens posture. Narrowing and same-posture requests are
/// obstructions: a posture change must always be a real, witnessed act. The
/// witness arrives as a [`WitnessDigest`], so a shrug witness cannot reach
/// this function — the type system holds the door.
///
/// # Errors
///
/// Returns [`PostureObstruction::NarrowingRefused`] when `to` is narrower
/// than `from`, and [`PostureObstruction::AlreadyAtPosture`] when `to`
/// equals `from`.
#[cfg(test)]
fn promote_posture(
    from: CausalPosture,
    to: CausalPosture,
    witness: WitnessDigest,
) -> Result<PosturePromotion, PostureObstruction> {
    if to < from {
        return Err(PostureObstruction::NarrowingRefused {
            from,
            requested: to,
        });
    }
    if to == from {
        return Err(PostureObstruction::AlreadyAtPosture { posture: from });
    }
    if to == CausalPosture::Shared {
        return Err(PostureObstruction::SharedAdmissionRequiresIntent);
    }
    Ok(PosturePromotion {
        from,
        to,
        witness: *witness.as_hash(),
    })
}

fn validate_admission_scope(
    posture: CausalPosture,
    admission_scope: Option<AdmissionScopeId>,
) -> Result<(), PostureObstruction> {
    match (posture, admission_scope) {
        (CausalPosture::Shared, None) => Err(PostureObstruction::MissingAdmissionScope { posture }),
        (CausalPosture::Scratch | CausalPosture::AuthorOnly, Some(_)) => {
            Err(PostureObstruction::UnexpectedAdmissionScope { posture })
        }
        (CausalPosture::Scratch | CausalPosture::AuthorOnly, None)
        | (CausalPosture::Shared, Some(_)) => Ok(()),
    }
}

fn validate_posture_derivation(
    posture: CausalPosture,
    derivation: PostureDerivation,
) -> Result<(), PostureObstruction> {
    let valid = match derivation {
        PostureDerivation::LegacyDurableAssumedShared => posture == CausalPosture::Shared,
        PostureDerivation::LegacyEphemeralAssumedScratch => posture == CausalPosture::Scratch,
        PostureDerivation::DebuggerDefault | PostureDerivation::CounterfactualDefault => {
            posture != CausalPosture::Shared
        }
        PostureDerivation::ExplicitIntent
        | PostureDerivation::SessionDefault
        | PostureDerivation::ImportedManifest => true,
    };
    if valid {
        Ok(())
    } else {
        Err(PostureObstruction::PostureDerivationMismatch {
            posture,
            derivation,
        })
    }
}

fn validate_authority_coherence(
    origin_id: OriginId,
    author_domain: AuthorityDomainRef,
    binding: AuthorityBinding,
) -> Result<(), PostureObstruction> {
    if author_domain.origin_id != origin_id {
        return Err(PostureObstruction::AuthorityOriginMismatch {
            origin_id,
            authority_origin: author_domain.origin_id,
        });
    }
    match binding {
        AuthorityBinding::LocalUnbound { origin } if origin != origin_id => {
            Err(PostureObstruction::AuthorityBindingOriginMismatch {
                origin_id,
                binding_origin: origin,
            })
        }
        AuthorityBinding::ImportedUnresolved { remote_origin, .. }
            if remote_origin != origin_id =>
        {
            Err(PostureObstruction::AuthorityBindingOriginMismatch {
                origin_id,
                binding_origin: remote_origin,
            })
        }
        AuthorityBinding::ImportedUnresolved {
            remote_authority, ..
        } if remote_authority != author_domain => {
            Err(PostureObstruction::AuthorityBindingDomainMismatch { author_domain })
        }
        AuthorityBinding::LocalUnbound { .. }
        | AuthorityBinding::LocalKeyed { .. }
        | AuthorityBinding::Delegated { .. }
        | AuthorityBinding::ImportedUnresolved { .. } => Ok(()),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn witness() -> WitnessDigest {
        WitnessDigest::new([0xA7; 32]).unwrap()
    }

    #[test]
    fn witness_digest_refuses_shrug_values() {
        assert_eq!(
            WitnessDigest::new([0; 32]),
            Err(PostureObstruction::EmptyWitness)
        );
        assert_eq!(
            WitnessDigest::new(crate::blake3_empty()),
            Err(PostureObstruction::EmptyWitness)
        );
        assert!(WitnessDigest::new([0x99; 32]).is_ok());
    }

    #[test]
    fn revelation_breadth_orders_scratch_below_author_only_below_shared() {
        assert!(CausalPosture::Scratch < CausalPosture::AuthorOnly);
        assert!(CausalPosture::AuthorOnly < CausalPosture::Shared);
    }

    #[test]
    fn canonical_tags_are_stable() {
        assert_eq!(CausalPosture::Scratch.canonical_tag(), 0x01);
        assert_eq!(CausalPosture::AuthorOnly.canonical_tag(), 0x02);
        assert_eq!(CausalPosture::Shared.canonical_tag(), 0x03);
    }

    #[test]
    fn least_revealed_finds_the_floor() {
        assert_eq!(
            least_revealed([
                CausalPosture::Shared,
                CausalPosture::Scratch,
                CausalPosture::AuthorOnly,
            ]),
            Some(CausalPosture::Scratch)
        );
        assert_eq!(least_revealed([]), None);
    }

    #[test]
    fn shell_cannot_reveal_more_than_least_revealed_member() {
        let obstruction = shell_posture_obstruction(
            CausalPosture::Shared,
            [CausalPosture::Shared, CausalPosture::AuthorOnly],
        );

        assert_eq!(
            obstruction,
            Some(PostureObstruction::ExceedsLeastRevealedMember {
                shell: CausalPosture::Shared,
                least_revealed_member: CausalPosture::AuthorOnly,
            })
        );
    }

    #[test]
    fn shell_at_or_below_member_floor_is_admissible() {
        assert_eq!(
            shell_posture_obstruction(
                CausalPosture::AuthorOnly,
                [CausalPosture::Shared, CausalPosture::AuthorOnly],
            ),
            None
        );
        assert_eq!(
            shell_posture_obstruction(CausalPosture::Scratch, [CausalPosture::AuthorOnly],),
            None
        );
    }

    #[test]
    fn empty_member_set_imposes_no_floor() {
        assert_eq!(shell_posture_obstruction(CausalPosture::Shared, []), None);
    }

    #[test]
    fn non_shared_promotion_widens_with_witness() {
        assert_eq!(
            promote_posture(CausalPosture::Scratch, CausalPosture::AuthorOnly, witness(),),
            Ok(PosturePromotion {
                from: CausalPosture::Scratch,
                to: CausalPosture::AuthorOnly,
                witness: *witness().as_hash(),
            })
        );
    }

    #[test]
    fn shared_promotion_requires_admission_intent() {
        assert_eq!(
            promote_posture(CausalPosture::AuthorOnly, CausalPosture::Shared, witness(),),
            Err(PostureObstruction::SharedAdmissionRequiresIntent)
        );
    }

    #[test]
    fn narrowing_is_refused_not_silently_applied() {
        assert_eq!(
            promote_posture(CausalPosture::Shared, CausalPosture::AuthorOnly, witness(),),
            Err(PostureObstruction::NarrowingRefused {
                from: CausalPosture::Shared,
                requested: CausalPosture::AuthorOnly,
            })
        );
    }

    #[test]
    fn promotion_to_same_posture_is_an_obstruction_not_a_noop() {
        assert_eq!(
            promote_posture(
                CausalPosture::AuthorOnly,
                CausalPosture::AuthorOnly,
                witness(),
            ),
            Err(PostureObstruction::AlreadyAtPosture {
                posture: CausalPosture::AuthorOnly,
            })
        );
    }

    #[test]
    fn causal_posture_has_no_global_default() {
        let source = include_str!("revelation.rs");
        let enum_start = source.find("pub enum CausalPosture").unwrap();
        let derive_line = source[..enum_start]
            .lines()
            .rev()
            .find(|line| line.contains("#[derive("))
            .unwrap();
        let impl_default = ["impl Default for ", "CausalPosture"].concat();

        assert!(!derive_line.contains("Default"));
        assert!(!source.contains(&impl_default));
    }

    #[test]
    fn authority_domain_ref_controls_cross_machine_equality() {
        let origin_a = OriginId::from_bytes([0xA1; 32]);
        let origin_b = OriginId::from_bytes([0xB1; 32]);
        let domain = AuthorityDomainId::from_bytes([0xD0; 32]);

        assert_eq!(
            AuthorityDomainRef::new(origin_a, domain),
            AuthorityDomainRef::new(origin_a, domain)
        );
        assert_ne!(
            AuthorityDomainRef::new(origin_a, domain),
            AuthorityDomainRef::new(origin_b, domain)
        );
    }

    #[test]
    fn new_shared_record_without_admission_scope_is_rejected() {
        let authority = fixture_authority();
        let retention_contract = RetentionContractId::from_bytes([0xA7; 32]);

        assert_eq!(
            RetentionPosture::new(
                CausalPosture::Shared,
                PostureDerivation::ExplicitIntent,
                authority,
                retention_contract,
                None,
            ),
            Err(PostureObstruction::MissingAdmissionScope {
                posture: CausalPosture::Shared,
            })
        );
        assert!(RetentionPosture::new(
            CausalPosture::AuthorOnly,
            PostureDerivation::SessionDefault,
            authority,
            retention_contract,
            None,
        )
        .is_ok());
    }

    #[test]
    fn legacy_derivation_must_match_effective_posture() {
        let authority = fixture_authority();
        let retention_contract = RetentionContractId::from_bytes([0xA7; 32]);

        assert_eq!(
            RetentionPosture::new(
                CausalPosture::AuthorOnly,
                PostureDerivation::LegacyDurableAssumedShared,
                authority,
                retention_contract,
                None,
            ),
            Err(PostureObstruction::PostureDerivationMismatch {
                posture: CausalPosture::AuthorOnly,
                derivation: PostureDerivation::LegacyDurableAssumedShared,
            })
        );
        assert_eq!(
            RetentionPosture::new(
                CausalPosture::Shared,
                PostureDerivation::LegacyEphemeralAssumedScratch,
                authority,
                retention_contract,
                Some(AdmissionScopeId::from_bytes([0x55; 32])),
            ),
            Err(PostureObstruction::PostureDerivationMismatch {
                posture: CausalPosture::Shared,
                derivation: PostureDerivation::LegacyEphemeralAssumedScratch,
            })
        );
    }

    #[test]
    fn causal_authority_rejects_incoherent_local_binding() {
        assert_eq!(
            CausalAuthority::new(
                OriginId::from_bytes([0xA1; 32]),
                ActorId::from_bytes([0xA2; 32]),
                AuthorityDomainRef::new(
                    OriginId::from_bytes([0xB1; 32]),
                    AuthorityDomainId::from_bytes([0xD0; 32]),
                ),
                AuthorityBinding::LocalUnbound {
                    origin: OriginId::from_bytes([0xA1; 32]),
                },
                SealStrength::Advisory,
            ),
            Err(PostureObstruction::AuthorityOriginMismatch {
                origin_id: OriginId::from_bytes([0xA1; 32]),
                authority_origin: OriginId::from_bytes([0xB1; 32]),
            })
        );
        assert_eq!(
            CausalAuthority::new(
                OriginId::from_bytes([0xA1; 32]),
                ActorId::from_bytes([0xA2; 32]),
                fixture_authority_ref(),
                AuthorityBinding::LocalUnbound {
                    origin: OriginId::from_bytes([0xB1; 32]),
                },
                SealStrength::Advisory,
            ),
            Err(PostureObstruction::AuthorityBindingOriginMismatch {
                origin_id: OriginId::from_bytes([0xA1; 32]),
                binding_origin: OriginId::from_bytes([0xB1; 32]),
            })
        );
    }

    #[test]
    fn replay_of_scratch_strand_does_not_materialize() {
        assert_eq!(
            revelation_operation_effect(CausalPosture::Scratch, RevelationOperation::Replay),
            OperationPostureEffect::Unchanged(CausalPosture::Scratch)
        );
        assert_eq!(
            revelation_operation_effect(CausalPosture::Scratch, RevelationOperation::Inspect),
            OperationPostureEffect::Unchanged(CausalPosture::Scratch)
        );
    }

    #[test]
    fn materialization_requires_explicit_receipt() {
        let receipt = MaterializationReceipt::new(
            crate::strand::make_strand_id("scratch"),
            CausalPosture::Scratch,
            CausalPosture::AuthorOnly,
            ActorId::from_bytes([0xA2; 32]),
            fixture_authority_ref(),
            AuthorityResolutionProof::LocalAuthorityDomain(fixture_authority_ref()),
            RetentionContractId::from_bytes([0xC0; 32]),
            MaterializationBasis::ExplicitSave,
        );

        assert!(receipt.is_ok());
        assert_eq!(
            MaterializationReceipt::new(
                crate::strand::make_strand_id("scratch"),
                CausalPosture::Scratch,
                CausalPosture::Shared,
                ActorId::from_bytes([0xA2; 32]),
                fixture_authority_ref(),
                AuthorityResolutionProof::LocalAuthorityDomain(fixture_authority_ref()),
                RetentionContractId::from_bytes([0xC0; 32]),
                MaterializationBasis::ExplicitSave,
            ),
            Err(PostureObstruction::InvalidMaterializationTransition {
                from: CausalPosture::Scratch,
                to: CausalPosture::Shared,
            })
        );
    }

    #[test]
    fn validated_posture_tokens_keep_invariant_fields_private() {
        let source = include_str!("revelation.rs");
        for type_name in [
            "MaterializationReceipt",
            "PromotionIntent",
            "ImportAdmissionReceipt",
        ] {
            let struct_start = source.find(&format!("pub struct {type_name}")).unwrap();
            let fields_start = source[struct_start..].find('{').unwrap() + struct_start;
            let impl_start = source[fields_start..]
                .find(&format!("impl {type_name}"))
                .unwrap()
                + fields_start;
            let field_block = &source[fields_start..impl_start];

            assert!(
                !field_block.contains("\n    pub "),
                "{type_name} exposes public fields that bypass validated constructors"
            );
        }
    }

    #[test]
    fn materialization_rejects_mismatched_authority_proof() {
        assert_eq!(
            MaterializationReceipt::new(
                crate::strand::make_strand_id("scratch"),
                CausalPosture::Scratch,
                CausalPosture::AuthorOnly,
                ActorId::from_bytes([0xA2; 32]),
                fixture_authority_ref(),
                AuthorityResolutionProof::LocalAuthorityDomain(AuthorityDomainRef::new(
                    OriginId::from_bytes([0xB1; 32]),
                    AuthorityDomainId::from_bytes([0xD0; 32]),
                )),
                RetentionContractId::from_bytes([0xC0; 32]),
                MaterializationBasis::ExplicitSave,
            ),
            Err(PostureObstruction::AuthorityProofMismatch {
                authorized_by: fixture_authority_ref(),
            })
        );
    }

    #[test]
    fn promotion_to_shared_requires_authority_scope_intent_and_witness() {
        let admission_scope = AdmissionScopeId::from_bytes([0x55; 32]);
        let promotion = PromotionIntent::admit_shared(
            IntentId::from_bytes([0x11; 32]),
            ActorId::from_bytes([0xA2; 32]),
            fixture_authority_ref(),
            AuthorityResolutionProof::LocalAuthorityDomain(fixture_authority_ref()),
            crate::strand::make_strand_id("author-only"),
            CausalPosture::AuthorOnly,
            admission_scope,
            witness(),
            PromotionBasis::ExplicitAdmission,
            ProjectionPolicy::FinalResultOnly,
            SourceDisclosurePolicy::RevealNone,
        );

        assert!(promotion.is_ok());
        assert_eq!(promotion.unwrap().admission_scope, admission_scope);
    }

    #[test]
    fn promotion_rejects_mismatched_authority_proof() {
        assert_eq!(
            PromotionIntent::admit_shared(
                IntentId::from_bytes([0x11; 32]),
                ActorId::from_bytes([0xA2; 32]),
                fixture_authority_ref(),
                AuthorityResolutionProof::LocalAuthorityDomain(AuthorityDomainRef::new(
                    OriginId::from_bytes([0xB1; 32]),
                    AuthorityDomainId::from_bytes([0xD0; 32]),
                )),
                crate::strand::make_strand_id("author-only"),
                CausalPosture::AuthorOnly,
                AdmissionScopeId::from_bytes([0x55; 32]),
                witness(),
                PromotionBasis::ExplicitAdmission,
                ProjectionPolicy::FinalResultOnly,
                SourceDisclosurePolicy::RevealNone,
            ),
            Err(PostureObstruction::AuthorityProofMismatch {
                authorized_by: fixture_authority_ref(),
            })
        );
    }

    #[test]
    fn imported_source_shared_is_not_local_admitted_without_admission_scope() {
        assert_eq!(
            import_posture_disposition(
                CausalPosture::Shared,
                false,
                ImportedArtifactId::from_bytes([0xA1; 32]),
                None,
            ),
            ImportPostureDisposition::SourceSharedPendingAdmission {
                namespace: ImportQuarantineNamespace::SourceSharedPendingAdmission,
            }
        );
    }

    #[test]
    fn imported_source_shared_requires_local_admission_receipt() {
        let import_a = ImportedArtifactId::from_bytes([0xA1; 32]);
        let import_b = ImportedArtifactId::from_bytes([0xB1; 32]);
        let receipt = ImportAdmissionReceipt::new(
            IntentId::from_bytes([0x11; 32]),
            import_a,
            fixture_authority_ref(),
            AuthorityResolutionProof::LocalAuthorityDomain(fixture_authority_ref()),
            AdmissionScopeId::from_bytes([0x55; 32]),
            witness(),
        )
        .unwrap();

        assert_eq!(
            import_posture_disposition(CausalPosture::Shared, false, import_a, None),
            ImportPostureDisposition::SourceSharedPendingAdmission {
                namespace: ImportQuarantineNamespace::SourceSharedPendingAdmission,
            }
        );
        assert_eq!(
            import_posture_disposition(CausalPosture::Shared, false, import_a, Some(receipt)),
            ImportPostureDisposition::LocallyAdmittedShared {
                admission_scope: AdmissionScopeId::from_bytes([0x55; 32]),
            }
        );
        assert_eq!(
            import_posture_disposition(CausalPosture::Shared, false, import_b, Some(receipt)),
            ImportPostureDisposition::SourceSharedPendingAdmission {
                namespace: ImportQuarantineNamespace::SourceSharedPendingAdmission,
            }
        );
    }

    #[test]
    fn shared_admission_reveals_projection_without_source_chain() {
        let source_strand = crate::strand::make_strand_id("sealed-source");
        let promotion = PromotionIntent::admit_shared(
            IntentId::from_bytes([0x11; 32]),
            ActorId::from_bytes([0xA2; 32]),
            fixture_authority_ref(),
            AuthorityResolutionProof::LocalAuthorityDomain(fixture_authority_ref()),
            source_strand,
            CausalPosture::AuthorOnly,
            AdmissionScopeId::from_bytes([0x55; 32]),
            witness(),
            PromotionBasis::ExplicitAdmission,
            ProjectionPolicy::FinalResultOnly,
            SourceDisclosurePolicy::RevealNone,
        )
        .unwrap();
        let admission = SharedAdmission::from_promotion(
            AdmissionId::from_bytes([0xAD; 32]),
            promotion,
            [0x44; 32],
        );

        assert_eq!(admission.source, AdmissionSourceDisclosure::Hidden);
        assert_eq!(admission.projection_digest, [0x44; 32]);
        assert_eq!(
            admission.source_disclosure,
            SourceDisclosurePolicy::RevealNone
        );
    }

    #[test]
    fn legacy_shared_authority_cannot_authorize_new_admission() {
        assert_eq!(
            PromotionIntent::admit_shared(
                IntentId::from_bytes([0x11; 32]),
                ActorId::from_bytes([0xA2; 32]),
                fixture_authority_ref(),
                AuthorityResolutionProof::LegacySharedAuthority,
                crate::strand::make_strand_id("legacy"),
                CausalPosture::AuthorOnly,
                AdmissionScopeId::from_bytes([0x55; 32]),
                witness(),
                PromotionBasis::ExplicitAdmission,
                ProjectionPolicy::FinalResultOnly,
                SourceDisclosurePolicy::RevealNone,
            ),
            Err(PostureObstruction::LegacyAuthorityCannotAuthorizeNewAdmission)
        );
    }

    #[test]
    fn generic_witness_digest_is_not_authority_capability() {
        assert_eq!(
            CapabilityProof::from_causal_witness(witness()),
            Err(PostureObstruction::WitnessIsNotAuthorityCapability)
        );
    }

    #[test]
    fn shared_session_default_requires_default_admission_scope() {
        assert_eq!(
            SessionContext::new(
                SessionId([0x51; 32]),
                OriginId::from_bytes([0xA1; 32]),
                ActorId::from_bytes([0xA2; 32]),
                fixture_authority_ref(),
                AuthorityBinding::LocalUnbound {
                    origin: OriginId::from_bytes([0xA1; 32]),
                },
                SealStrength::Advisory,
                CausalPosture::Shared,
                None,
                RetentionContractId::from_bytes([0xC0; 32]),
            ),
            Err(PostureObstruction::MissingAdmissionScope {
                posture: CausalPosture::Shared,
            })
        );
    }

    fn fixture_authority_ref() -> AuthorityDomainRef {
        AuthorityDomainRef::new(
            OriginId::from_bytes([0xA1; 32]),
            AuthorityDomainId::from_bytes([0xD0; 32]),
        )
    }

    fn fixture_authority() -> CausalAuthority {
        CausalAuthority::new(
            OriginId::from_bytes([0xA1; 32]),
            ActorId::from_bytes([0xA2; 32]),
            fixture_authority_ref(),
            AuthorityBinding::LocalUnbound {
                origin: OriginId::from_bytes([0xA1; 32]),
            },
            SealStrength::Advisory,
        )
        .unwrap()
    }
}
