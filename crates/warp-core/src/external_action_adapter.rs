// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Compiler-authored external requests and capability-rooted observation.
//!
//! This module is a trusted-host boundary. Edict contributes canonical request
//! data but receives no filesystem authority. The adapter accepts work only
//! through an [`ExternalActionClaimGrantV1`], which is constructible only after
//! Echo durably commits the corresponding request and claim.

use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Component, Path};

use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt, OpenOptionsSyncExt};
use cap_std::{
    ambient_authority,
    fs::{Dir, OpenOptions},
};
use echo_edict_canonical::{
    decode_canonical_cbor_v1, digest_canonical_value_v1, encode_canonical_cbor_v1,
    CanonicalValueErrorKind, CanonicalValueV1,
};
use thiserror::Error;

use crate::causal_wal::WalStorePort;
use crate::external_action::{
    admit_external_action_settlement, AdmittedExternalActionSettlementV1,
    ExternalActionAdapterBindingV1, ExternalActionAdapterIdV1, ExternalActionBudgetV1,
    ExternalActionClaimGrantV1, ExternalActionCoordinatorV1, ExternalActionOperationIdV1,
    ExternalActionProtocolErrorV1, ExternalActionRequestV1, ExternalActionSettlementCandidateV1,
    ExternalActionSettlementKindV1, ExternalActionTransactionContextV1,
};
use crate::{Hash, WorldlineId};

const CORE_DIGEST_DOMAIN: &str = "edict.core.module/v1";
const TARGET_IR_DIGEST_DOMAIN: &str = "edict.target-ir.artifact/v1";
const ECHO_TARGET_IR_DOMAIN: &str = "echo.span-ir/v1";
const ECHO_TARGET_PROFILE_COORDINATE: &str = "echo.dpo@1";
const EXTERNAL_REQUEST_OPERATION_PROFILE: &str = "continuum.profile.read-only/v1";
const RESOURCE_ID_DOMAIN: &[u8] = b"echo.external-action.resource-id/v1";
const TARGET_OPERATION_ID_DOMAIN: &[u8] = b"echo.external-action.target-operation-id/v1";
const INPUT_DIGEST_DOMAIN: &[u8] = b"echo.external-action.input/v1";
const OBSERVATION_INPUT_KIND: &str = "boundedWorkspaceObservationInput";
const OBSERVATION_SETTLEMENT_KIND: &str = "boundedWorkspaceObservationSettlement";
const OBSERVATION_BASIS_DOMAIN: &[u8] = b"echo.bounded-observation.basis/v1";
const OBSERVATION_SCHEMA_EVIDENCE_DOMAIN: &[u8] = b"echo.bounded-observation.schema-evidence/v1";
const OBSERVATION_REFUSAL_EVIDENCE_DOMAIN: &[u8] = b"echo.bounded-observation.refusal-evidence/v1";
const MAX_CORE_EVALUATION_STEPS_V1: u64 = 4_096;
const MAX_CORE_EVALUATION_ALLOCATED_BYTES_V1: u64 = 4 * 1_024 * 1_024;
const MAX_CORE_EVALUATION_OUTPUT_BYTES_V1: u64 = 1_024 * 1_024;

/// One canonical Edict request admitted from exact Core and Target IR bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmittedEdictExternalActionRequestV1 {
    request: ExternalActionRequestV1,
    canonical_operation_input: Vec<u8>,
    source_core_digest: String,
    target_ir_digest: String,
    operation_coordinate: String,
}

impl AdmittedEdictExternalActionRequestV1 {
    /// Returns the generic Echo request derived from compiler-owned data.
    #[must_use]
    pub const fn request(&self) -> ExternalActionRequestV1 {
        self.request
    }

    /// Returns the exact canonical operation input resolved from application input.
    #[must_use]
    pub fn canonical_operation_input(&self) -> &[u8] {
        &self.canonical_operation_input
    }

    /// Returns the independently recomputed reviewed Core digest.
    #[must_use]
    pub fn source_core_digest(&self) -> &str {
        &self.source_core_digest
    }

    /// Returns the independently recomputed reviewed Target IR digest.
    #[must_use]
    pub fn target_ir_digest(&self) -> &str {
        &self.target_ir_digest
    }

    /// Returns the compiler-declared operation coordinate for diagnostics.
    #[must_use]
    pub fn operation_coordinate(&self) -> &str {
        &self.operation_coordinate
    }
}

/// Independent compiler-artifact admission failures.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum EdictExternalActionAdmissionErrorV1 {
    /// The canonical decoder rejected one compiler artifact or runtime value.
    #[error("canonical Edict value was rejected: {0:?}")]
    Canonical(CanonicalValueErrorKind),
    /// The Target IR shape is not the exact supported request-only contract.
    #[error("Edict Target IR artifact shape is unsupported")]
    ArtifactShape,
    /// The requested intent is absent.
    #[error("Edict Target IR intent is absent")]
    MissingIntent,
    /// The intent does not contain exactly one external request.
    #[error("Edict Target IR intent must contain exactly one external request")]
    RequestCardinality,
    /// A request-only artifact also contains callable target steps.
    #[error("Edict external-request Target IR contains callable target steps")]
    CallableStepsPresent,
    /// The Target IR omitted its exact source/capability closure.
    #[error("Edict Target IR semantic closure is absent")]
    MissingSemanticClosure,
    /// The supplied Core bytes do not match the Target IR source-Core binding.
    #[error("Edict source Core digest does not match Target IR")]
    CoreDigestMismatch,
    /// The supplied Target IR request is not the independent projection of Core.
    #[error("Edict Target IR request is not derived from the supplied Core")]
    TargetDerivationMismatch,
    /// The request operation is absent or substituted in the capability closure.
    #[error("Edict external request is outside the exact capability closure")]
    CapabilityClosureMismatch,
    /// The supported runtime expression subset could not resolve a request field.
    #[error("Edict external request uses an unsupported runtime expression")]
    UnsupportedExpression,
    /// A runtime value has the wrong type or bounds.
    #[error("Edict external request runtime value is invalid")]
    InvalidRuntimeValue,
    /// Runtime expression evaluation exceeded the declared or host-owned budget.
    #[error("Edict external request expression evaluation exceeded its budget")]
    EvaluationBudgetExceeded,
    /// A reviewed digest does not use the required lowercase SHA-256 rendering.
    #[error("Edict external request contains an invalid reviewed digest")]
    InvalidDigest,
    /// The generic request protocol rejected the compiler-derived request.
    #[error(transparent)]
    Protocol(#[from] ExternalActionProtocolErrorV1),
}

/// Admits one request-only Edict Target IR artifact without invoking a provider.
#[allow(clippy::too_many_arguments)]
pub fn admit_edict_external_action_request_v1(
    worldline_id: WorldlineId,
    canonical_core_bytes: &[u8],
    canonical_target_ir_bytes: &[u8],
    intent_name: &str,
    canonical_application_input_bytes: &[u8],
) -> Result<AdmittedEdictExternalActionRequestV1, EdictExternalActionAdmissionErrorV1> {
    let core = decode_canonical_cbor_v1(canonical_core_bytes)
        .map_err(|error| EdictExternalActionAdmissionErrorV1::Canonical(error.kind()))?;
    let target_ir = decode_canonical_cbor_v1(canonical_target_ir_bytes)
        .map_err(|error| EdictExternalActionAdmissionErrorV1::Canonical(error.kind()))?;
    let application_input = decode_canonical_cbor_v1(canonical_application_input_bytes)
        .map_err(|error| EdictExternalActionAdmissionErrorV1::Canonical(error.kind()))?;

    let source_core_digest = digest_canonical_value_v1(CORE_DIGEST_DOMAIN, &core)
        .map_err(|error| EdictExternalActionAdmissionErrorV1::Canonical(error.kind()))?;
    let target_ir_digest = digest_canonical_value_v1(TARGET_IR_DIGEST_DOMAIN, &target_ir)
        .map_err(|error| EdictExternalActionAdmissionErrorV1::Canonical(error.kind()))?;

    let core_map = expect_map(&core)?;
    require_text_field(core_map, "apiVersion", "edict.core/v1")?;
    let core_coordinate = require_nonempty_text(core_map, "coordinate")?;

    let target_map = expect_map(&target_ir)?;
    require_text_field(target_map, "kind", "targetIrArtifact")?;
    require_text_field(target_map, "domain", ECHO_TARGET_IR_DOMAIN)?;
    let source_core_coordinate = require_nonempty_text(target_map, "sourceCoreCoordinate")?;
    let target_profile = parse_resource(require_field(target_map, "targetProfile")?)?;
    if target_profile.coordinate != ECHO_TARGET_PROFILE_COORDINATE {
        return Err(EdictExternalActionAdmissionErrorV1::ArtifactShape);
    }

    let closure_map = expect_map(
        require_field(target_map, "semanticClosure")
            .map_err(|_| EdictExternalActionAdmissionErrorV1::MissingSemanticClosure)?,
    )?;
    let closure_source = parse_resource(require_field(closure_map, "sourceCore")?)?;
    if source_core_coordinate != core_coordinate
        || closure_source.coordinate != core_coordinate
        || closure_source.review_digest() != source_core_digest
    {
        return Err(EdictExternalActionAdmissionErrorV1::CoreDigestMismatch);
    }
    let lawpacks = expect_array(
        require_field(closure_map, "lawpacks")
            .map_err(|_| EdictExternalActionAdmissionErrorV1::MissingSemanticClosure)?,
    )?
    .iter()
    .map(parse_resource)
    .collect::<Result<Vec<_>, _>>()?;
    let capabilities = expect_array(
        require_field(closure_map, "capabilities")
            .map_err(|_| EdictExternalActionAdmissionErrorV1::CapabilityClosureMismatch)?,
    )?
    .iter()
    .map(parse_resource)
    .collect::<Result<Vec<_>, _>>()?;

    let intents = expect_map(require_field(target_map, "intents")?)?;
    let intent = map_field(intents, intent_name)
        .ok_or(EdictExternalActionAdmissionErrorV1::MissingIntent)?;
    let intent_map = expect_map(intent)?;
    require_text_field(
        intent_map,
        "operationProfile",
        EXTERNAL_REQUEST_OPERATION_PROFILE,
    )?;
    if !expect_array(require_field(intent_map, "inputConstraints")?)?.is_empty()
        || !expect_array(require_field(intent_map, "requirements")?)?.is_empty()
    {
        return Err(EdictExternalActionAdmissionErrorV1::ArtifactShape);
    }
    if !expect_array(require_field(intent_map, "steps")?)?.is_empty() {
        return Err(EdictExternalActionAdmissionErrorV1::CallableStepsPresent);
    }
    let requests = expect_array(require_field(intent_map, "externalActionRequests")?)?;
    let [request_value] = requests else {
        return Err(EdictExternalActionAdmissionErrorV1::RequestCardinality);
    };
    let request_map = expect_map(request_value)?;
    require_nonempty_text(request_map, "id")?;
    require_text_field(request_map, "state", "awaitingSettlement")?;
    require_text_field(request_map, "settlementAdmission", "schemaRequired")?;
    let binding = parse_local_ref(require_field(request_map, "binding")?)?;
    let result = expect_map(require_field(intent_map, "result")?)?;
    require_text_field(result, "kind", "local")?;
    let result_reference = parse_local_ref(require_field(result, "ref")?)?;
    if result_reference != binding {
        return Err(EdictExternalActionAdmissionErrorV1::ArtifactShape);
    }
    if require_field(intent_map, "basis")? != require_field(request_map, "basis")? {
        return Err(EdictExternalActionAdmissionErrorV1::ArtifactShape);
    }
    let operation = parse_resource(require_field(request_map, "operation")?)?;
    if !capabilities
        .iter()
        .any(|capability| capability == &operation)
    {
        return Err(EdictExternalActionAdmissionErrorV1::CapabilityClosureMismatch);
    }
    verify_target_derivation(
        core_map,
        intent_name,
        intent_map,
        request_map,
        &lawpacks,
        &capabilities,
    )?;
    let input_schema = parse_resource(require_field(request_map, "inputSchema")?)?;
    let settlement_schema = parse_resource(require_field(request_map, "settlementSchema")?)?;
    let reconciliation_law = parse_resource(require_field(request_map, "reconciliationLaw")?)?;
    let input_type = require_nonempty_text(request_map, "inputType")?;
    let settlement_type = require_nonempty_text(request_map, "settlementType")?;

    let evaluation_budget =
        parse_evaluation_budget(require_field(intent_map, "coreEvaluationBudget")?)?;
    let mut evaluator = BoundedExpressionEvaluatorV1::new(&application_input, evaluation_budget)?;
    let operation_input_value = evaluator.evaluate_root(require_field(request_map, "input")?)?;
    let operation_input = expect_bytes(&operation_input_value)?.to_vec();
    if operation_input.len() > bytes_type_max(input_type)? {
        return Err(EdictExternalActionAdmissionErrorV1::InvalidRuntimeValue);
    }
    let authority_scope_digest =
        expect_hash(&evaluator.evaluate_root(require_field(request_map, "authorityScope")?)?)?;
    let basis_digest =
        expect_hash(&evaluator.evaluate_root(require_field(request_map, "basis")?)?)?;
    let budget_map = expect_map(require_field(request_map, "budget")?)?;
    let max_settlement_bytes =
        expect_u64(&evaluator.evaluate_root(require_field(budget_map, "maxSettlementBytes")?)?)?;
    let max_attempts =
        expect_u32(&evaluator.evaluate_root(require_field(budget_map, "maxAttempts")?)?)?;
    if max_settlement_bytes > u64::try_from(bytes_type_max(settlement_type)?).unwrap_or(u64::MAX) {
        return Err(EdictExternalActionAdmissionErrorV1::InvalidRuntimeValue);
    }
    if max_settlement_bytes < minimum_terminal_settlement_bytes_v1(basis_digest)? {
        return Err(EdictExternalActionAdmissionErrorV1::InvalidRuntimeValue);
    }

    let request = ExternalActionRequestV1::new(
        worldline_id,
        ExternalActionOperationIdV1::from_hash(target_operation_identity(
            &target_profile,
            &operation,
        )),
        input_schema.digest,
        settlement_schema.digest,
        authority_scope_digest,
        basis_digest,
        ExternalActionBudgetV1 {
            max_settlement_bytes,
            max_attempts,
        },
        input_identity(&operation_input),
        resource_identity(&reconciliation_law),
    )?;
    Ok(AdmittedEdictExternalActionRequestV1 {
        request,
        canonical_operation_input: operation_input,
        source_core_digest,
        target_ir_digest,
        operation_coordinate: operation.coordinate,
    })
}

/// Runtime-owner profile binding a validator and adapter to compiler identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundedWorkspaceObservationProfileV1 {
    /// Exact compiler-declared operation family.
    pub operation_id: ExternalActionOperationIdV1,
    /// Exact compiler-declared input schema.
    pub input_schema_digest: Hash,
    /// Exact compiler-declared settlement schema.
    pub settlement_schema_digest: Hash,
    /// Exact compiler-declared reconciliation law.
    pub reconciliation_law_digest: Hash,
    /// Maximum authority scope delegated by the runtime owner.
    pub authority_scope_digest: Hash,
    /// Runtime-owned adapter identity.
    pub adapter_id: ExternalActionAdapterIdV1,
}

/// Bounded, read-only host adapter rooted in one capability directory.
pub struct BoundedWorkspaceObservationAdapterV1 {
    root: Dir,
    permitted_paths: BTreeSet<String>,
    profile: BoundedWorkspaceObservationProfileV1,
}

/// Rootless settlement authority retained for post-claim reconciliation.
///
/// This handle carries no directory capability and cannot perform an
/// observation. It can only admit the bounded profile's schema-valid
/// `OutcomeUnknown` settlement for an exact durable claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundedWorkspaceObservationReconcilerV1 {
    profile: BoundedWorkspaceObservationProfileV1,
}

impl BoundedWorkspaceObservationAdapterV1 {
    /// Opens the configured root once and retains only directory-relative authority.
    pub fn open(
        root: &Path,
        permitted_paths: impl IntoIterator<Item = String>,
        profile: BoundedWorkspaceObservationProfileV1,
    ) -> Result<Self, BoundedWorkspaceObservationErrorV1> {
        validate_observation_profile(profile)?;
        let permitted_paths = permitted_paths.into_iter().collect::<BTreeSet<_>>();
        for path in &permitted_paths {
            validate_relative_path(path)?;
        }
        let root = Dir::open_ambient_dir(root, ambient_authority())
            .map_err(|_| BoundedWorkspaceObservationErrorV1::Io)?;
        Ok(Self {
            root,
            permitted_paths,
            profile,
        })
    }

    /// Returns the runtime registry binding for this attenuated adapter.
    #[must_use]
    pub const fn adapter_binding(&self) -> ExternalActionAdapterBindingV1 {
        ExternalActionAdapterBindingV1 {
            adapter_id: self.profile.adapter_id,
            operation_id: self.profile.operation_id,
            authority_scope_digest: self.profile.authority_scope_digest,
        }
    }

    /// Performs one bounded observation after request and claim durability.
    pub fn observe(
        &self,
        grant: &ExternalActionClaimGrantV1,
        admitted: &AdmittedEdictExternalActionRequestV1,
    ) -> Result<ExternalActionSettlementCandidateV1, BoundedWorkspaceObservationErrorV1> {
        self.validate_grant(grant, admitted)?;
        let paths = match decode_observation_input(admitted.canonical_operation_input()) {
            Ok(paths) => paths,
            Err(BoundedWorkspaceObservationErrorV1::Canonical) => {
                return self.refused_candidate(grant, "malformed-input", "");
            }
            Err(error) => return Err(error),
        };
        if paths.is_empty() {
            return self.refused_candidate(grant, "empty-path-set", "");
        }
        let mut unique_paths = BTreeSet::new();
        for path in &paths {
            if validate_relative_path(path).is_err() {
                return self.refused_candidate(grant, "invalid-path", path);
            }
            if !unique_paths.insert(path.as_str()) {
                return self.refused_candidate(grant, "duplicate-path", path);
            }
            if !self.permitted_paths.contains(path) {
                return self.refused_candidate(grant, "unauthorized-path", path);
            }
        }

        let mut observed = Vec::with_capacity(paths.len());
        let mut retained_input_bytes = 0_u64;
        for path in paths {
            let remaining = grant
                .request()
                .budget
                .max_settlement_bytes
                .saturating_sub(retained_input_bytes);
            match self.read_regular_file(&path, remaining) {
                Ok(bytes) => {
                    let byte_count = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
                    if byte_count > remaining {
                        return self.refused_candidate(grant, "settlement-budget-exceeded", &path);
                    }
                    retained_input_bytes = retained_input_bytes.saturating_add(byte_count);
                    observed.push(ObservedFileV1 { path, bytes });
                }
                Err(BoundedWorkspaceObservationErrorV1::SymlinkRefused) => {
                    return self.refused_candidate(grant, "symlink-refused", &path);
                }
                Err(BoundedWorkspaceObservationErrorV1::NotRegularFile) => {
                    return self.refused_candidate(grant, "not-regular-file", &path);
                }
                Err(BoundedWorkspaceObservationErrorV1::Io) => {
                    return self.failed_candidate(grant, "io-failure", &path);
                }
                Err(error) => return Err(error),
            }
        }
        observed.sort_by(|left, right| left.path.cmp(&right.path));
        let observed_basis = bounded_workspace_observation_basis_v1(
            observed
                .iter()
                .map(|file| (file.path.as_str(), file.bytes.as_slice())),
        );
        if observed_basis != grant.request().basis_digest {
            return self.refused_candidate_with_evidence(grant, "stale-basis", observed_basis);
        }
        let success = encode_observation_settlement(
            "succeeded",
            grant.request().basis_digest,
            observed_basis,
            &observed,
            None,
        )?;
        if u64::try_from(success.len()).unwrap_or(u64::MAX)
            > grant.request().budget.max_settlement_bytes
        {
            return self.refused_candidate(grant, "settlement-budget-exceeded", "");
        }
        self.candidate(
            grant,
            ExternalActionSettlementKindV1::Succeeded,
            success,
            observed_basis,
        )
    }

    /// Produces an explicit ambiguous settlement during reconciliation.
    pub fn outcome_unknown(
        &self,
        grant: &ExternalActionClaimGrantV1,
        admitted: &AdmittedEdictExternalActionRequestV1,
        external_evidence_digest: Hash,
    ) -> Result<ExternalActionSettlementCandidateV1, BoundedWorkspaceObservationErrorV1> {
        self.validate_grant(grant, admitted)?;
        if external_evidence_digest == [0; 32] {
            return Err(BoundedWorkspaceObservationErrorV1::SchemaAdmissionFailed);
        }
        let result = encode_observation_settlement(
            "outcomeUnknown",
            grant.request().basis_digest,
            external_evidence_digest,
            &[],
            Some("outcome-unknown"),
        )?;
        self.candidate(
            grant,
            ExternalActionSettlementKindV1::OutcomeUnknown,
            result,
            external_evidence_digest,
        )
    }

    /// Validates the operation-specific schema and durably admits the settlement.
    pub fn admit_settlement(
        &self,
        store: &mut impl WalStorePort,
        coordinator: &mut ExternalActionCoordinatorV1,
        context: ExternalActionTransactionContextV1,
        admitted: &AdmittedEdictExternalActionRequestV1,
        grant: ExternalActionClaimGrantV1,
        candidate: ExternalActionSettlementCandidateV1,
    ) -> Result<AdmittedExternalActionSettlementV1, BoundedWorkspaceObservationErrorV1> {
        self.validate_grant(&grant, admitted)?;
        self.validate_candidate(&grant, admitted, &candidate)?;
        Ok(admit_external_action_settlement(
            store,
            coordinator,
            context,
            grant,
            candidate,
        )?)
    }

    fn validate_grant(
        &self,
        grant: &ExternalActionClaimGrantV1,
        admitted: &AdmittedEdictExternalActionRequestV1,
    ) -> Result<(), BoundedWorkspaceObservationErrorV1> {
        validate_observation_grant(self.profile, grant, admitted)
    }

    fn read_regular_file(
        &self,
        path: &str,
        max_bytes: u64,
    ) -> Result<Vec<u8>, BoundedWorkspaceObservationErrorV1> {
        let components = Path::new(path).components().collect::<Vec<_>>();
        let Some((file_name, directory_components)) = components.split_last() else {
            return Err(BoundedWorkspaceObservationErrorV1::InvalidPath);
        };
        let Component::Normal(file_name) = file_name else {
            return Err(BoundedWorkspaceObservationErrorV1::InvalidPath);
        };
        let mut directory = self
            .root
            .try_clone()
            .map_err(|_| BoundedWorkspaceObservationErrorV1::Io)?;
        for component in directory_components {
            let Component::Normal(component) = component else {
                return Err(BoundedWorkspaceObservationErrorV1::InvalidPath);
            };
            let metadata = directory
                .symlink_metadata(component)
                .map_err(|_| BoundedWorkspaceObservationErrorV1::Io)?;
            if metadata.file_type().is_symlink() {
                return Err(BoundedWorkspaceObservationErrorV1::SymlinkRefused);
            }
            directory = directory
                .open_dir_nofollow(component)
                .map_err(|_| BoundedWorkspaceObservationErrorV1::Io)?;
        }
        let metadata = directory
            .symlink_metadata(file_name)
            .map_err(|_| BoundedWorkspaceObservationErrorV1::Io)?;
        if metadata.file_type().is_symlink() {
            return Err(BoundedWorkspaceObservationErrorV1::SymlinkRefused);
        }
        if !metadata.is_file() {
            return Err(BoundedWorkspaceObservationErrorV1::NotRegularFile);
        }
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No).nonblock(true);
        let mut file = directory
            .open_with(file_name, &options)
            .map_err(|_| BoundedWorkspaceObservationErrorV1::Io)?;
        let metadata = file
            .metadata()
            .map_err(|_| BoundedWorkspaceObservationErrorV1::Io)?;
        if !metadata.is_file() {
            return Err(BoundedWorkspaceObservationErrorV1::NotRegularFile);
        }
        let mut bytes = Vec::new();
        file.by_ref()
            .take(max_bytes.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|_| BoundedWorkspaceObservationErrorV1::Io)?;
        Ok(bytes)
    }

    fn refused_candidate(
        &self,
        grant: &ExternalActionClaimGrantV1,
        code: &str,
        detail: &str,
    ) -> Result<ExternalActionSettlementCandidateV1, BoundedWorkspaceObservationErrorV1> {
        self.refused_candidate_with_evidence(grant, code, refusal_evidence(code, detail))
    }

    fn refused_candidate_with_evidence(
        &self,
        grant: &ExternalActionClaimGrantV1,
        code: &str,
        evidence: Hash,
    ) -> Result<ExternalActionSettlementCandidateV1, BoundedWorkspaceObservationErrorV1> {
        let result = encode_observation_settlement(
            "rejected",
            grant.request().basis_digest,
            evidence,
            &[],
            Some(code),
        )?;
        if u64::try_from(result.len()).unwrap_or(u64::MAX)
            > grant.request().budget.max_settlement_bytes
        {
            return Err(BoundedWorkspaceObservationErrorV1::SettlementBudgetExceeded);
        }
        self.candidate(
            grant,
            ExternalActionSettlementKindV1::Rejected,
            result,
            evidence,
        )
    }

    fn failed_candidate(
        &self,
        grant: &ExternalActionClaimGrantV1,
        code: &str,
        detail: &str,
    ) -> Result<ExternalActionSettlementCandidateV1, BoundedWorkspaceObservationErrorV1> {
        let evidence = refusal_evidence(code, detail);
        let result = encode_observation_settlement(
            "failed",
            grant.request().basis_digest,
            evidence,
            &[],
            Some(code),
        )?;
        self.candidate(
            grant,
            ExternalActionSettlementKindV1::Failed,
            result,
            evidence,
        )
    }

    fn candidate(
        &self,
        grant: &ExternalActionClaimGrantV1,
        kind: ExternalActionSettlementKindV1,
        result: Vec<u8>,
        external_evidence_digest: Hash,
    ) -> Result<ExternalActionSettlementCandidateV1, BoundedWorkspaceObservationErrorV1> {
        if u64::try_from(result.len()).unwrap_or(u64::MAX)
            > grant.request().budget.max_settlement_bytes
        {
            return Err(BoundedWorkspaceObservationErrorV1::SettlementBudgetExceeded);
        }
        let schema_admission_evidence_digest =
            schema_admission_evidence(self.profile.settlement_schema_digest, &result);
        Ok(ExternalActionSettlementCandidateV1::new(
            grant.request().request_id(),
            grant.claim().attempt_id,
            self.profile.adapter_id,
            kind,
            self.profile.settlement_schema_digest,
            grant.request().basis_digest,
            result,
            schema_admission_evidence_digest,
            external_evidence_digest,
        ))
    }

    fn validate_candidate(
        &self,
        grant: &ExternalActionClaimGrantV1,
        admitted: &AdmittedEdictExternalActionRequestV1,
        candidate: &ExternalActionSettlementCandidateV1,
    ) -> Result<(), BoundedWorkspaceObservationErrorV1> {
        validate_observation_candidate(
            self.profile,
            Some(&self.permitted_paths),
            grant,
            admitted,
            candidate,
        )
    }
}

impl BoundedWorkspaceObservationReconcilerV1 {
    /// Constructs a reconciliation handle without acquiring filesystem authority.
    pub fn new(
        profile: BoundedWorkspaceObservationProfileV1,
    ) -> Result<Self, BoundedWorkspaceObservationErrorV1> {
        validate_observation_profile(profile)?;
        Ok(Self { profile })
    }

    /// Returns the registry binding for the exact retained adapter identity.
    #[must_use]
    pub const fn adapter_binding(&self) -> ExternalActionAdapterBindingV1 {
        ExternalActionAdapterBindingV1 {
            adapter_id: self.profile.adapter_id,
            operation_id: self.profile.operation_id,
            authority_scope_digest: self.profile.authority_scope_digest,
        }
    }

    /// Durably admits explicit uncertainty without reopening the external world.
    #[allow(clippy::too_many_arguments)]
    pub fn admit_outcome_unknown(
        &self,
        store: &mut impl WalStorePort,
        coordinator: &mut ExternalActionCoordinatorV1,
        context: ExternalActionTransactionContextV1,
        admitted: &AdmittedEdictExternalActionRequestV1,
        grant: ExternalActionClaimGrantV1,
        external_evidence_digest: Hash,
    ) -> Result<AdmittedExternalActionSettlementV1, BoundedWorkspaceObservationErrorV1> {
        validate_observation_grant(self.profile, &grant, admitted)?;
        if external_evidence_digest == [0; 32] {
            return Err(BoundedWorkspaceObservationErrorV1::SchemaAdmissionFailed);
        }
        let result = encode_observation_settlement(
            "outcomeUnknown",
            grant.request().basis_digest,
            external_evidence_digest,
            &[],
            Some("outcome-unknown"),
        )?;
        if u64::try_from(result.len()).unwrap_or(u64::MAX)
            > grant.request().budget.max_settlement_bytes
        {
            return Err(BoundedWorkspaceObservationErrorV1::SettlementBudgetExceeded);
        }
        let schema_admission_evidence_digest =
            schema_admission_evidence(self.profile.settlement_schema_digest, &result);
        let candidate = ExternalActionSettlementCandidateV1::new(
            grant.request().request_id(),
            grant.claim().attempt_id,
            self.profile.adapter_id,
            ExternalActionSettlementKindV1::OutcomeUnknown,
            self.profile.settlement_schema_digest,
            grant.request().basis_digest,
            result,
            schema_admission_evidence_digest,
            external_evidence_digest,
        );
        validate_observation_candidate(self.profile, None, &grant, admitted, &candidate)?;
        Ok(admit_external_action_settlement(
            store,
            coordinator,
            context,
            grant,
            candidate,
        )?)
    }
}

fn validate_observation_profile(
    profile: BoundedWorkspaceObservationProfileV1,
) -> Result<(), BoundedWorkspaceObservationErrorV1> {
    if profile.operation_id.as_hash() == [0; 32]
        || profile.input_schema_digest == [0; 32]
        || profile.settlement_schema_digest == [0; 32]
        || profile.reconciliation_law_digest == [0; 32]
        || profile.authority_scope_digest == [0; 32]
        || profile.adapter_id.as_hash() == [0; 32]
    {
        return Err(BoundedWorkspaceObservationErrorV1::ProfileMismatch);
    }
    Ok(())
}

fn validate_observation_grant(
    profile: BoundedWorkspaceObservationProfileV1,
    grant: &ExternalActionClaimGrantV1,
    admitted: &AdmittedEdictExternalActionRequestV1,
) -> Result<(), BoundedWorkspaceObservationErrorV1> {
    let request = grant.request();
    if request != admitted.request()
        || request.input_digest != input_identity(admitted.canonical_operation_input())
        || grant.claim().adapter_id != profile.adapter_id
    {
        return Err(BoundedWorkspaceObservationErrorV1::GrantMismatch);
    }
    if request.operation_id != profile.operation_id
        || request.input_schema_digest != profile.input_schema_digest
        || request.settlement_schema_digest != profile.settlement_schema_digest
        || request.reconciliation_law_digest != profile.reconciliation_law_digest
        || request.authority_scope_digest != profile.authority_scope_digest
    {
        return Err(BoundedWorkspaceObservationErrorV1::ProfileMismatch);
    }
    Ok(())
}

fn validate_observation_candidate(
    profile: BoundedWorkspaceObservationProfileV1,
    permitted_paths: Option<&BTreeSet<String>>,
    grant: &ExternalActionClaimGrantV1,
    admitted: &AdmittedEdictExternalActionRequestV1,
    candidate: &ExternalActionSettlementCandidateV1,
) -> Result<(), BoundedWorkspaceObservationErrorV1> {
    if candidate.request_id != grant.request().request_id()
        || candidate.attempt_id != grant.claim().attempt_id
        || candidate.adapter_id != profile.adapter_id
        || candidate.settlement_schema_digest != profile.settlement_schema_digest
        || candidate.basis_digest != grant.request().basis_digest
        || candidate.declared_result_digest
            != Hash::from(blake3::hash(&candidate.canonical_result_bytes))
    {
        return Err(BoundedWorkspaceObservationErrorV1::SchemaAdmissionFailed);
    }
    let value = decode_canonical_cbor_v1(&candidate.canonical_result_bytes)
        .map_err(|_| BoundedWorkspaceObservationErrorV1::SchemaAdmissionFailed)?;
    let expected_paths = if candidate.kind == ExternalActionSettlementKindV1::Succeeded {
        let permitted_paths =
            permitted_paths.ok_or(BoundedWorkspaceObservationErrorV1::SchemaAdmissionFailed)?;
        let paths = decode_observation_input(admitted.canonical_operation_input())
            .map_err(|_| BoundedWorkspaceObservationErrorV1::SchemaAdmissionFailed)?;
        let expected = paths.iter().cloned().collect::<BTreeSet<_>>();
        if expected.is_empty()
            || expected.len() != paths.len()
            || expected.iter().any(|path| {
                validate_relative_path(path).is_err() || !permitted_paths.contains(path)
            })
        {
            return Err(BoundedWorkspaceObservationErrorV1::SchemaAdmissionFailed);
        }
        Some(expected)
    } else {
        None
    };
    validate_observation_settlement(
        &value,
        candidate.kind,
        candidate.basis_digest,
        candidate.external_evidence_digest,
        expected_paths.as_ref(),
    )?;
    let expected_evidence = schema_admission_evidence(
        candidate.settlement_schema_digest,
        &candidate.canonical_result_bytes,
    );
    if candidate.schema_admission_evidence_digest != expected_evidence {
        return Err(BoundedWorkspaceObservationErrorV1::SchemaAdmissionFailed);
    }
    Ok(())
}

/// Encodes the operation-specific path request as canonical CBOR.
pub fn encode_bounded_workspace_observation_input_v1(
    paths: impl IntoIterator<Item = String>,
) -> Result<Vec<u8>, BoundedWorkspaceObservationErrorV1> {
    let paths = paths
        .into_iter()
        .map(CanonicalValueV1::Text)
        .collect::<Vec<_>>();
    encode_canonical_cbor_v1(&canonical_map([
        (
            "kind",
            CanonicalValueV1::Text(OBSERVATION_INPUT_KIND.to_owned()),
        ),
        ("paths", CanonicalValueV1::Array(paths)),
    ]))
    .map_err(|_| BoundedWorkspaceObservationErrorV1::Canonical)
}

/// Computes the basis committed by a sorted path/content snapshot.
pub fn bounded_workspace_observation_basis_v1<'a>(
    entries: impl IntoIterator<Item = (&'a str, &'a [u8])>,
) -> Hash {
    let mut entries = entries.into_iter().collect::<Vec<_>>();
    entries.sort_by(|left, right| left.0.cmp(right.0));
    let mut hasher = blake3::Hasher::new();
    hasher.update(OBSERVATION_BASIS_DOMAIN);
    hasher.update(
        &u64::try_from(entries.len())
            .unwrap_or(u64::MAX)
            .to_le_bytes(),
    );
    for (path, bytes) in entries {
        hash_len_prefixed(&mut hasher, path.as_bytes());
        hash_len_prefixed(&mut hasher, bytes);
    }
    hasher.finalize().into()
}

/// Stable adapter/profile failures.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BoundedWorkspaceObservationErrorV1 {
    /// The canonical input or settlement encoding is invalid.
    #[error("bounded observation canonical value is invalid")]
    Canonical,
    /// The claim, compiler request, and adapter profile do not identify one request.
    #[error("bounded observation grant does not match the admitted compiler request")]
    GrantMismatch,
    /// The compiler request is outside the runtime-owned adapter profile.
    #[error("bounded observation request does not match adapter policy")]
    ProfileMismatch,
    /// An input path is empty, absolute, non-normalized, or traverses a parent.
    #[error("bounded observation path is invalid")]
    InvalidPath,
    /// The requested relative path is outside the attenuated path set.
    #[error("bounded observation path is unauthorized")]
    UnauthorizedPath,
    /// A requested path or any traversed component is a symlink.
    #[error("bounded observation refuses symlinks")]
    SymlinkRefused,
    /// A requested path is not one regular file.
    #[error("bounded observation requires regular files")]
    NotRegularFile,
    /// The observed bytes do not match the request's exact basis.
    #[error("bounded observation basis is stale")]
    StaleBasis,
    /// The canonical settlement exceeds the compiler-delegated bound.
    #[error("bounded observation settlement exceeds its byte budget")]
    SettlementBudgetExceeded,
    /// The candidate did not pass the operation-specific settlement validator.
    #[error("bounded observation settlement schema admission failed")]
    SchemaAdmissionFailed,
    /// Capability-rooted filesystem access failed.
    #[error("bounded observation filesystem access failed")]
    Io,
    /// The generic durable protocol refused the transition.
    #[error(transparent)]
    Protocol(#[from] ExternalActionProtocolErrorV1),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ResourceRefV1 {
    coordinate: String,
    digest: Hash,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LocalRefV1 {
    id: String,
    alpha_name: String,
    ty: String,
}

#[derive(Clone, Copy, Debug)]
struct CoreEvaluationBudgetV1 {
    steps: u64,
    allocated_bytes: u64,
    output_bytes: u64,
}

struct BoundedExpressionEvaluatorV1<'a> {
    application_input: &'a CanonicalValueV1,
    budget: CoreEvaluationBudgetV1,
    steps: u64,
    allocated_bytes: u64,
}

impl ResourceRefV1 {
    fn review_digest(&self) -> String {
        format!("sha256:{}", hex::encode(self.digest))
    }
}

#[derive(Debug)]
struct ObservedFileV1 {
    path: String,
    bytes: Vec<u8>,
}

fn expect_map(
    value: &CanonicalValueV1,
) -> Result<&[(CanonicalValueV1, CanonicalValueV1)], EdictExternalActionAdmissionErrorV1> {
    match value {
        CanonicalValueV1::Map(entries) => Ok(entries),
        _ => Err(EdictExternalActionAdmissionErrorV1::ArtifactShape),
    }
}

fn expect_array(
    value: &CanonicalValueV1,
) -> Result<&[CanonicalValueV1], EdictExternalActionAdmissionErrorV1> {
    match value {
        CanonicalValueV1::Array(entries) => Ok(entries),
        _ => Err(EdictExternalActionAdmissionErrorV1::ArtifactShape),
    }
}

fn expect_bytes(value: &CanonicalValueV1) -> Result<&[u8], EdictExternalActionAdmissionErrorV1> {
    match value {
        CanonicalValueV1::Bytes(bytes) => Ok(bytes),
        _ => Err(EdictExternalActionAdmissionErrorV1::InvalidRuntimeValue),
    }
}

fn expect_hash(value: &CanonicalValueV1) -> Result<Hash, EdictExternalActionAdmissionErrorV1> {
    expect_bytes(value)?
        .try_into()
        .map_err(|_| EdictExternalActionAdmissionErrorV1::InvalidRuntimeValue)
}

fn expect_u64(value: &CanonicalValueV1) -> Result<u64, EdictExternalActionAdmissionErrorV1> {
    let CanonicalValueV1::Integer(value) = value else {
        return Err(EdictExternalActionAdmissionErrorV1::InvalidRuntimeValue);
    };
    u64::try_from(*value).map_err(|_| EdictExternalActionAdmissionErrorV1::InvalidRuntimeValue)
}

fn expect_u32(value: &CanonicalValueV1) -> Result<u32, EdictExternalActionAdmissionErrorV1> {
    u32::try_from(expect_u64(value)?)
        .map_err(|_| EdictExternalActionAdmissionErrorV1::InvalidRuntimeValue)
}

fn map_field<'a>(
    entries: &'a [(CanonicalValueV1, CanonicalValueV1)],
    field: &str,
) -> Option<&'a CanonicalValueV1> {
    entries.iter().find_map(|(key, value)| match key {
        CanonicalValueV1::Text(key) if key == field => Some(value),
        _ => None,
    })
}

fn require_field<'a>(
    entries: &'a [(CanonicalValueV1, CanonicalValueV1)],
    field: &str,
) -> Result<&'a CanonicalValueV1, EdictExternalActionAdmissionErrorV1> {
    map_field(entries, field).ok_or(EdictExternalActionAdmissionErrorV1::ArtifactShape)
}

fn require_nonempty_text<'a>(
    entries: &'a [(CanonicalValueV1, CanonicalValueV1)],
    field: &str,
) -> Result<&'a str, EdictExternalActionAdmissionErrorV1> {
    match require_field(entries, field)? {
        CanonicalValueV1::Text(value) if !value.is_empty() => Ok(value),
        _ => Err(EdictExternalActionAdmissionErrorV1::ArtifactShape),
    }
}

fn require_text_field(
    entries: &[(CanonicalValueV1, CanonicalValueV1)],
    field: &str,
    expected: &str,
) -> Result<(), EdictExternalActionAdmissionErrorV1> {
    if require_nonempty_text(entries, field)? == expected {
        Ok(())
    } else {
        Err(EdictExternalActionAdmissionErrorV1::ArtifactShape)
    }
}

fn parse_resource(
    value: &CanonicalValueV1,
) -> Result<ResourceRefV1, EdictExternalActionAdmissionErrorV1> {
    let resource = expect_map(value)?;
    let coordinate = require_nonempty_text(resource, "id")?.to_owned();
    let digest = expect_array(require_field(resource, "digest")?)
        .map_err(|_| EdictExternalActionAdmissionErrorV1::InvalidDigest)?;
    let [CanonicalValueV1::Text(algorithm), CanonicalValueV1::Bytes(bytes)] = digest else {
        return Err(EdictExternalActionAdmissionErrorV1::InvalidDigest);
    };
    if algorithm != "sha256" {
        return Err(EdictExternalActionAdmissionErrorV1::InvalidDigest);
    }
    let digest = bytes
        .as_slice()
        .try_into()
        .map_err(|_| EdictExternalActionAdmissionErrorV1::InvalidDigest)?;
    Ok(ResourceRefV1 { coordinate, digest })
}

fn parse_local_ref(
    value: &CanonicalValueV1,
) -> Result<LocalRefV1, EdictExternalActionAdmissionErrorV1> {
    let reference = expect_map(value)?;
    Ok(LocalRefV1 {
        id: require_nonempty_text(reference, "id")?.to_owned(),
        alpha_name: require_nonempty_text(reference, "alphaName")?.to_owned(),
        ty: require_nonempty_text(reference, "type")?.to_owned(),
    })
}

fn verify_target_derivation(
    core: &[(CanonicalValueV1, CanonicalValueV1)],
    intent_name: &str,
    target_intent: &[(CanonicalValueV1, CanonicalValueV1)],
    target_request: &[(CanonicalValueV1, CanonicalValueV1)],
    target_lawpacks: &[ResourceRefV1],
    target_capabilities: &[ResourceRefV1],
) -> Result<(), EdictExternalActionAdmissionErrorV1> {
    let core_intents = expect_map(require_field(core, "intents")?)?;
    let core_intent = expect_map(
        map_field(core_intents, intent_name)
            .ok_or(EdictExternalActionAdmissionErrorV1::TargetDerivationMismatch)?,
    )?;
    if require_field(core_intent, "inputConstraints")?
        != require_field(target_intent, "inputConstraints")?
        || require_field(core_intent, "coreEvaluationBudget")?
            != require_field(target_intent, "coreEvaluationBudget")?
        || require_field(core_intent, "basis")? != require_field(target_intent, "basis")?
    {
        return Err(EdictExternalActionAdmissionErrorV1::TargetDerivationMismatch);
    }

    let core_body = expect_map(require_field(core_intent, "body")?)?;
    let core_nodes = expect_array(require_field(core_body, "nodes")?)?;
    let [core_request_value] = core_nodes else {
        return Err(EdictExternalActionAdmissionErrorV1::TargetDerivationMismatch);
    };
    let core_request = expect_map(core_request_value)?;
    require_text_field(core_request, "kind", "externalActionRequest")
        .map_err(|_| EdictExternalActionAdmissionErrorV1::TargetDerivationMismatch)?;
    let core_locals = expect_array(require_field(core_body, "locals")?)?;
    let [core_argument, core_binding] = core_locals else {
        return Err(EdictExternalActionAdmissionErrorV1::TargetDerivationMismatch);
    };
    if parse_local_ref(core_argument)?.id != "arg.0"
        || core_binding != require_field(target_request, "binding")?
        || require_field(core_body, "result")? != require_field(target_intent, "result")?
        || require_nonempty_text(target_request, "id")? != format!("{intent_name}.request.0")
    {
        return Err(EdictExternalActionAdmissionErrorV1::TargetDerivationMismatch);
    }
    for field in [
        "binding",
        "operation",
        "inputType",
        "settlementType",
        "inputSchema",
        "settlementSchema",
        "input",
        "authorityScope",
        "basis",
        "budget",
        "reconciliationLaw",
        "state",
        "settlementAdmission",
    ] {
        if require_field(core_request, field)? != require_field(target_request, field)? {
            return Err(EdictExternalActionAdmissionErrorV1::TargetDerivationMismatch);
        }
    }

    let core_lawpacks = core_import_resources(core, "lawpack")?;
    let core_capabilities = core_import_resources(core, "capability")?;
    if canonical_resource_set(core_lawpacks)? != canonical_resource_set(target_lawpacks.to_vec())?
        || canonical_resource_set(core_capabilities)?
            != canonical_resource_set(target_capabilities.to_vec())?
    {
        return Err(EdictExternalActionAdmissionErrorV1::TargetDerivationMismatch);
    }
    Ok(())
}

fn core_import_resources(
    core: &[(CanonicalValueV1, CanonicalValueV1)],
    expected_kind: &str,
) -> Result<Vec<ResourceRefV1>, EdictExternalActionAdmissionErrorV1> {
    let imports = expect_array(require_field(core, "imports")?)?;
    let mut resources = Vec::new();
    for import in imports {
        let import = expect_map(import)?;
        let kind = require_nonempty_text(import, "kind")?;
        if kind == expected_kind {
            resources.push(parse_resource(require_field(import, "ref")?)?);
        }
    }
    Ok(resources)
}

fn canonical_resource_set(
    mut resources: Vec<ResourceRefV1>,
) -> Result<Vec<ResourceRefV1>, EdictExternalActionAdmissionErrorV1> {
    resources.sort();
    if resources.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(EdictExternalActionAdmissionErrorV1::TargetDerivationMismatch);
    }
    Ok(resources)
}

fn parse_evaluation_budget(
    value: &CanonicalValueV1,
) -> Result<CoreEvaluationBudgetV1, EdictExternalActionAdmissionErrorV1> {
    let budget = expect_map(value)?;
    let parsed = CoreEvaluationBudgetV1 {
        steps: expect_u64(require_field(budget, "maxSteps")?)?,
        allocated_bytes: expect_u64(require_field(budget, "maxAllocatedBytes")?)?,
        output_bytes: expect_u64(require_field(budget, "maxOutputBytes")?)?,
    };
    if parsed.steps == 0
        || parsed.allocated_bytes == 0
        || parsed.output_bytes == 0
        || parsed.steps > MAX_CORE_EVALUATION_STEPS_V1
        || parsed.allocated_bytes > MAX_CORE_EVALUATION_ALLOCATED_BYTES_V1
        || parsed.output_bytes > MAX_CORE_EVALUATION_OUTPUT_BYTES_V1
    {
        return Err(EdictExternalActionAdmissionErrorV1::EvaluationBudgetExceeded);
    }
    Ok(parsed)
}

impl<'a> BoundedExpressionEvaluatorV1<'a> {
    fn new(
        application_input: &'a CanonicalValueV1,
        budget: CoreEvaluationBudgetV1,
    ) -> Result<Self, EdictExternalActionAdmissionErrorV1> {
        let input_bytes = retained_value_bytes(application_input);
        if input_bytes > budget.allocated_bytes {
            return Err(EdictExternalActionAdmissionErrorV1::EvaluationBudgetExceeded);
        }
        Ok(Self {
            application_input,
            budget,
            steps: 0,
            allocated_bytes: 0,
        })
    }

    fn evaluate_root(
        &mut self,
        expression: &CanonicalValueV1,
    ) -> Result<CanonicalValueV1, EdictExternalActionAdmissionErrorV1> {
        let value = self.evaluate(expression)?;
        if retained_value_bytes(&value) > self.budget.output_bytes {
            return Err(EdictExternalActionAdmissionErrorV1::EvaluationBudgetExceeded);
        }
        Ok(value)
    }

    fn evaluate(
        &mut self,
        expression: &CanonicalValueV1,
    ) -> Result<CanonicalValueV1, EdictExternalActionAdmissionErrorV1> {
        self.steps = self.steps.saturating_add(1);
        if self.steps > self.budget.steps {
            return Err(EdictExternalActionAdmissionErrorV1::EvaluationBudgetExceeded);
        }
        let expression = expect_map(expression)?;
        match require_nonempty_text(expression, "kind")? {
            "local" => {
                let reference = parse_local_ref(require_field(expression, "ref")?)?;
                if reference.id != "arg.0" {
                    return Err(EdictExternalActionAdmissionErrorV1::UnsupportedExpression);
                }
                self.allocate_value(self.application_input)?;
                Ok(self.application_input.clone())
            }
            "field" => {
                let base = self.evaluate(require_field(expression, "base")?)?;
                let base = expect_map(&base)?;
                let field = require_nonempty_text(expression, "field")?;
                let value = map_field(base, field)
                    .ok_or(EdictExternalActionAdmissionErrorV1::InvalidRuntimeValue)?;
                self.allocate_value(value)?;
                Ok(value.clone())
            }
            "const" => {
                let value = evaluate_core_value(require_field(expression, "value")?)?;
                self.allocate_value(&value)?;
                Ok(value)
            }
            "record" => {
                let fields = expect_map(require_field(expression, "fields")?)?;
                let mut evaluated = Vec::with_capacity(fields.len());
                for (name, value) in fields {
                    let CanonicalValueV1::Text(name) = name else {
                        return Err(EdictExternalActionAdmissionErrorV1::ArtifactShape);
                    };
                    self.allocate_bytes(
                        u64::try_from(name.len())
                            .unwrap_or(u64::MAX)
                            .saturating_add(16),
                    )?;
                    evaluated.push((CanonicalValueV1::Text(name.clone()), self.evaluate(value)?));
                }
                Ok(CanonicalValueV1::Map(evaluated))
            }
            _ => Err(EdictExternalActionAdmissionErrorV1::UnsupportedExpression),
        }
    }

    fn allocate_value(
        &mut self,
        value: &CanonicalValueV1,
    ) -> Result<(), EdictExternalActionAdmissionErrorV1> {
        self.allocate_bytes(retained_value_bytes(value))
    }

    fn allocate_bytes(&mut self, bytes: u64) -> Result<(), EdictExternalActionAdmissionErrorV1> {
        self.allocated_bytes = self.allocated_bytes.saturating_add(bytes);
        if self.allocated_bytes > self.budget.allocated_bytes {
            return Err(EdictExternalActionAdmissionErrorV1::EvaluationBudgetExceeded);
        }
        Ok(())
    }
}

fn retained_value_bytes(value: &CanonicalValueV1) -> u64 {
    let payload = match value {
        CanonicalValueV1::Null | CanonicalValueV1::Bool(_) | CanonicalValueV1::Integer(_) => 16,
        CanonicalValueV1::Bytes(bytes) => u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        CanonicalValueV1::Text(text) => u64::try_from(text.len()).unwrap_or(u64::MAX),
        CanonicalValueV1::Array(values) => values
            .iter()
            .fold(0_u64, |total, value| {
                total.saturating_add(retained_value_bytes(value))
            })
            .saturating_add(
                u64::try_from(values.len())
                    .unwrap_or(u64::MAX)
                    .saturating_mul(8),
            ),
        CanonicalValueV1::Map(entries) => entries
            .iter()
            .fold(0_u64, |total, (key, value)| {
                total
                    .saturating_add(retained_value_bytes(key))
                    .saturating_add(retained_value_bytes(value))
            })
            .saturating_add(
                u64::try_from(entries.len())
                    .unwrap_or(u64::MAX)
                    .saturating_mul(16),
            ),
    };
    payload.saturating_add(16)
}

fn evaluate_core_value(
    value: &CanonicalValueV1,
) -> Result<CanonicalValueV1, EdictExternalActionAdmissionErrorV1> {
    let value = expect_map(value)?;
    match require_nonempty_text(value, "kind")? {
        "null" => Ok(CanonicalValueV1::Null),
        "bool" => match require_field(value, "value")? {
            CanonicalValueV1::Bool(value) => Ok(CanonicalValueV1::Bool(*value)),
            _ => Err(EdictExternalActionAdmissionErrorV1::InvalidRuntimeValue),
        },
        "int" => match require_field(value, "value")? {
            CanonicalValueV1::Integer(value) => Ok(CanonicalValueV1::Integer(*value)),
            _ => Err(EdictExternalActionAdmissionErrorV1::InvalidRuntimeValue),
        },
        "string" => match require_field(value, "value")? {
            CanonicalValueV1::Text(value) => Ok(CanonicalValueV1::Text(value.clone())),
            _ => Err(EdictExternalActionAdmissionErrorV1::InvalidRuntimeValue),
        },
        "bytes" => match require_field(value, "value")? {
            CanonicalValueV1::Bytes(value) => Ok(CanonicalValueV1::Bytes(value.clone())),
            _ => Err(EdictExternalActionAdmissionErrorV1::InvalidRuntimeValue),
        },
        _ => Err(EdictExternalActionAdmissionErrorV1::InvalidRuntimeValue),
    }
}

fn bytes_type_max(value: &str) -> Result<usize, EdictExternalActionAdmissionErrorV1> {
    let value = value
        .strip_prefix("Bytes<max=")
        .and_then(|value| value.strip_suffix('>'))
        .ok_or(EdictExternalActionAdmissionErrorV1::ArtifactShape)?;
    value
        .parse()
        .map_err(|_| EdictExternalActionAdmissionErrorV1::ArtifactShape)
}

fn resource_identity(resource: &ResourceRefV1) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(RESOURCE_ID_DOMAIN);
    hash_len_prefixed(&mut hasher, resource.coordinate.as_bytes());
    hasher.update(&resource.digest);
    hasher.finalize().into()
}

fn target_operation_identity(target_profile: &ResourceRefV1, operation: &ResourceRefV1) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(TARGET_OPERATION_ID_DOMAIN);
    hasher.update(&resource_identity(target_profile));
    hasher.update(&resource_identity(operation));
    hasher.finalize().into()
}

fn input_identity(bytes: &[u8]) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(INPUT_DIGEST_DOMAIN);
    hash_len_prefixed(&mut hasher, bytes);
    hasher.finalize().into()
}

fn decode_observation_input(
    bytes: &[u8],
) -> Result<Vec<String>, BoundedWorkspaceObservationErrorV1> {
    let value = decode_canonical_cbor_v1(bytes)
        .map_err(|_| BoundedWorkspaceObservationErrorV1::Canonical)?;
    let CanonicalValueV1::Map(entries) = value else {
        return Err(BoundedWorkspaceObservationErrorV1::Canonical);
    };
    if entries.len() != 2 {
        return Err(BoundedWorkspaceObservationErrorV1::Canonical);
    }
    let Some(CanonicalValueV1::Text(kind)) = map_field(&entries, "kind") else {
        return Err(BoundedWorkspaceObservationErrorV1::Canonical);
    };
    if kind != OBSERVATION_INPUT_KIND {
        return Err(BoundedWorkspaceObservationErrorV1::Canonical);
    }
    let Some(CanonicalValueV1::Array(paths)) = map_field(&entries, "paths") else {
        return Err(BoundedWorkspaceObservationErrorV1::Canonical);
    };
    paths
        .iter()
        .map(|path| match path {
            CanonicalValueV1::Text(path) => Ok(path.clone()),
            _ => Err(BoundedWorkspaceObservationErrorV1::Canonical),
        })
        .collect()
}

fn validate_relative_path(path: &str) -> Result<(), BoundedWorkspaceObservationErrorV1> {
    if path.is_empty()
        || path.starts_with('/')
        || path.ends_with('/')
        || path.contains("//")
        || path.contains('\\')
        || path
            .split('/')
            .any(|component| component.is_empty() || matches!(component, "." | ".."))
    {
        return Err(BoundedWorkspaceObservationErrorV1::InvalidPath);
    }
    let mut component_count = 0_usize;
    for component in Path::new(path).components() {
        match component {
            Component::Normal(component) if component.to_str().is_some() => {
                component_count = component_count.saturating_add(1);
            }
            _ => return Err(BoundedWorkspaceObservationErrorV1::InvalidPath),
        }
    }
    if component_count == 0 {
        Err(BoundedWorkspaceObservationErrorV1::InvalidPath)
    } else {
        Ok(())
    }
}

fn encode_observation_settlement(
    posture: &str,
    basis: Hash,
    evidence: Hash,
    files: &[ObservedFileV1],
    obstruction: Option<&str>,
) -> Result<Vec<u8>, BoundedWorkspaceObservationErrorV1> {
    let files = files
        .iter()
        .map(|file| {
            canonical_map([
                ("path", CanonicalValueV1::Text(file.path.clone())),
                ("bytes", CanonicalValueV1::Bytes(file.bytes.clone())),
                (
                    "digest",
                    CanonicalValueV1::Bytes(blake3::hash(&file.bytes).as_bytes().to_vec()),
                ),
            ])
        })
        .collect();
    encode_canonical_cbor_v1(&canonical_map([
        (
            "kind",
            CanonicalValueV1::Text(OBSERVATION_SETTLEMENT_KIND.to_owned()),
        ),
        ("posture", CanonicalValueV1::Text(posture.to_owned())),
        ("basis", CanonicalValueV1::Bytes(basis.to_vec())),
        ("evidence", CanonicalValueV1::Bytes(evidence.to_vec())),
        ("files", CanonicalValueV1::Array(files)),
        (
            "obstruction",
            obstruction.map_or(CanonicalValueV1::Null, |value| {
                CanonicalValueV1::Text(value.to_owned())
            }),
        ),
    ]))
    .map_err(|_| BoundedWorkspaceObservationErrorV1::Canonical)
}

fn minimum_terminal_settlement_bytes_v1(
    basis: Hash,
) -> Result<u64, EdictExternalActionAdmissionErrorV1> {
    [
        ("rejected", "settlement-budget-exceeded"),
        ("rejected", "malformed-input"),
        ("failed", "io-failure"),
        ("outcomeUnknown", "outcome-unknown"),
    ]
    .into_iter()
    .map(|(posture, obstruction)| {
        encode_observation_settlement(posture, basis, [0xff; 32], &[], Some(obstruction))
            .map(|bytes| u64::try_from(bytes.len()).unwrap_or(u64::MAX))
            .map_err(|_| EdictExternalActionAdmissionErrorV1::InvalidRuntimeValue)
    })
    .try_fold(0_u64, |maximum, length| {
        length.map(|length| maximum.max(length))
    })
}

fn validate_observation_settlement(
    value: &CanonicalValueV1,
    kind: ExternalActionSettlementKindV1,
    basis: Hash,
    evidence: Hash,
    expected_paths: Option<&BTreeSet<String>>,
) -> Result<(), BoundedWorkspaceObservationErrorV1> {
    let CanonicalValueV1::Map(entries) = value else {
        return Err(BoundedWorkspaceObservationErrorV1::SchemaAdmissionFailed);
    };
    if entries.len() != 6
        || !matches!(
            map_field(entries, "kind"),
            Some(CanonicalValueV1::Text(value)) if value == OBSERVATION_SETTLEMENT_KIND
        )
        || !matches!(
            map_field(entries, "basis"),
            Some(CanonicalValueV1::Bytes(value)) if value.as_slice() == basis
        )
        || !matches!(
            map_field(entries, "evidence"),
            Some(CanonicalValueV1::Bytes(value)) if value.as_slice() == evidence
        )
    {
        return Err(BoundedWorkspaceObservationErrorV1::SchemaAdmissionFailed);
    }
    let expected_posture = match kind {
        ExternalActionSettlementKindV1::Succeeded => "succeeded",
        ExternalActionSettlementKindV1::Rejected => "rejected",
        ExternalActionSettlementKindV1::Failed => "failed",
        ExternalActionSettlementKindV1::OutcomeUnknown => "outcomeUnknown",
    };
    if !matches!(
        map_field(entries, "posture"),
        Some(CanonicalValueV1::Text(value)) if value == expected_posture
    ) {
        return Err(BoundedWorkspaceObservationErrorV1::SchemaAdmissionFailed);
    }
    let Some(CanonicalValueV1::Array(files)) = map_field(entries, "files") else {
        return Err(BoundedWorkspaceObservationErrorV1::SchemaAdmissionFailed);
    };
    if evidence == [0; 32] {
        return Err(BoundedWorkspaceObservationErrorV1::SchemaAdmissionFailed);
    }
    if kind == ExternalActionSettlementKindV1::Succeeded {
        if files.is_empty()
            || !matches!(
                map_field(entries, "obstruction"),
                Some(CanonicalValueV1::Null)
            )
        {
            return Err(BoundedWorkspaceObservationErrorV1::SchemaAdmissionFailed);
        }
        let mut observed = Vec::with_capacity(files.len());
        let mut previous_path: Option<&str> = None;
        for file in files {
            let CanonicalValueV1::Map(file) = file else {
                return Err(BoundedWorkspaceObservationErrorV1::SchemaAdmissionFailed);
            };
            if file.len() != 3 {
                return Err(BoundedWorkspaceObservationErrorV1::SchemaAdmissionFailed);
            }
            let (
                Some(CanonicalValueV1::Text(path)),
                Some(CanonicalValueV1::Bytes(bytes)),
                Some(CanonicalValueV1::Bytes(digest)),
            ) = (
                map_field(file, "path"),
                map_field(file, "bytes"),
                map_field(file, "digest"),
            )
            else {
                return Err(BoundedWorkspaceObservationErrorV1::SchemaAdmissionFailed);
            };
            if validate_relative_path(path).is_err()
                || previous_path.is_some_and(|previous| previous >= path.as_str())
                || digest.as_slice() != blake3::hash(bytes).as_bytes()
            {
                return Err(BoundedWorkspaceObservationErrorV1::SchemaAdmissionFailed);
            }
            previous_path = Some(path);
            observed.push((path.as_str(), bytes.as_slice()));
        }
        if bounded_workspace_observation_basis_v1(observed) != basis || evidence != basis {
            return Err(BoundedWorkspaceObservationErrorV1::SchemaAdmissionFailed);
        }
        let Some(expected_paths) = expected_paths else {
            return Err(BoundedWorkspaceObservationErrorV1::SchemaAdmissionFailed);
        };
        if files.len() != expected_paths.len()
            || files.iter().zip(expected_paths).any(|(file, expected)| {
                !matches!(
                    file,
                    CanonicalValueV1::Map(entries)
                        if matches!(
                            map_field(entries, "path"),
                            Some(CanonicalValueV1::Text(path)) if path == expected
                        )
                )
            })
        {
            return Err(BoundedWorkspaceObservationErrorV1::SchemaAdmissionFailed);
        }
    } else if !files.is_empty()
        || !matches!(
            map_field(entries, "obstruction"),
            Some(CanonicalValueV1::Text(value)) if !value.is_empty()
        )
        || expected_paths.is_some()
    {
        return Err(BoundedWorkspaceObservationErrorV1::SchemaAdmissionFailed);
    }
    Ok(())
}

fn canonical_map<'a>(
    entries: impl IntoIterator<Item = (&'a str, CanonicalValueV1)>,
) -> CanonicalValueV1 {
    CanonicalValueV1::Map(
        entries
            .into_iter()
            .map(|(key, value)| (CanonicalValueV1::Text(key.to_owned()), value))
            .collect(),
    )
}

fn schema_admission_evidence(schema: Hash, bytes: &[u8]) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(OBSERVATION_SCHEMA_EVIDENCE_DOMAIN);
    hasher.update(&schema);
    hash_len_prefixed(&mut hasher, bytes);
    hasher.finalize().into()
}

fn refusal_evidence(code: &str, detail: &str) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(OBSERVATION_REFUSAL_EVIDENCE_DOMAIN);
    hash_len_prefixed(&mut hasher, code.as_bytes());
    hash_len_prefixed(&mut hasher, detail.as_bytes());
    hasher.finalize().into()
}

fn hash_len_prefixed(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    hasher.update(&u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_le_bytes());
    hasher.update(bytes);
}
