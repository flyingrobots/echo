// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Basis-bound, capability-rooted application of compiler-authored patches.
//!
//! Edict contributes canonical request data but receives no filesystem
//! authority. Echo durably records and claims the request before this adapter
//! can validate or mutate one exact regular file. Ambiguous attempts are
//! reconciled by observation and are never blindly reapplied.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Component, Path};

use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt, OpenOptionsSyncExt};
use cap_std::{
    ambient_authority,
    fs::{Dir, Metadata, OpenOptions},
};
use echo_edict_canonical::{decode_canonical_cbor_v1, encode_canonical_cbor_v1, CanonicalValueV1};
use thiserror::Error;

use crate::causal_wal::WalStorePort;
use crate::external_action::{
    admit_external_action_settlement, AdmittedExternalActionSettlementV1,
    ExternalActionAdapterBindingV1, ExternalActionAdapterIdV1, ExternalActionClaimGrantV1,
    ExternalActionCoordinatorV1, ExternalActionOperationIdV1, ExternalActionProtocolErrorV1,
    ExternalActionSettlementCandidateV1, ExternalActionSettlementKindV1,
    ExternalActionTransactionContextV1,
};
use crate::external_action_adapter::{
    bounded_workspace_observation_basis_v1, AdmittedEdictExternalActionRequestV1,
};
use crate::Hash;

const PATCH_INPUT_KIND: &str = "validatedWorkspacePatchInput";
const PATCH_SETTLEMENT_KIND: &str = "validatedWorkspacePatchSettlement";
const PATCH_AUTHORITY_DOMAIN: &[u8] = b"echo.validated-workspace-patch.authority/v1";
const PATCH_SCHEMA_EVIDENCE_DOMAIN: &[u8] = b"echo.validated-workspace-patch.schema-evidence/v1";
const PATCH_EXTERNAL_EVIDENCE_DOMAIN: &[u8] =
    b"echo.validated-workspace-patch.external-evidence/v1";
const EXTERNAL_ACTION_INPUT_DOMAIN: &[u8] = b"echo.external-action.input/v1";
const MAX_CANONICAL_PATCH_INPUT_BYTES: u64 = 65_536;

/// Runtime-owner profile binding one patch adapter to compiler identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValidatedWorkspacePatchProfileV1 {
    /// Exact compiler-declared operation family.
    pub operation_id: ExternalActionOperationIdV1,
    /// Exact compiler-declared input schema.
    pub input_schema_digest: Hash,
    /// Exact compiler-declared settlement schema.
    pub settlement_schema_digest: Hash,
    /// Exact compiler-declared reconciliation law.
    pub reconciliation_law_digest: Hash,
    /// Exact writable-path policy delegated by the runtime owner.
    pub authority_scope_digest: Hash,
    /// Runtime-owned adapter identity.
    pub adapter_id: ExternalActionAdapterIdV1,
    /// Maximum existing or replacement file size admitted by this adapter.
    pub max_file_bytes: u64,
}

/// Capability-rooted adapter that can replace one exact regular file.
pub struct ValidatedWorkspacePatchAdapterV1 {
    root: Dir,
    permitted_paths: BTreeSet<String>,
    profile: ValidatedWorkspacePatchProfileV1,
}

/// Read-only reconciliation handle for a claimed patch attempt.
pub struct ValidatedWorkspacePatchReconcilerV1 {
    root: Dir,
    permitted_paths: BTreeSet<String>,
    profile: ValidatedWorkspacePatchProfileV1,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ValidatedPatchInputV1 {
    path: String,
    expected_content_digest: Hash,
    replacement: Vec<u8>,
    replacement_digest: Hash,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SettlementEvidenceV1 {
    posture: &'static str,
    path: Option<String>,
    request_basis: Hash,
    evidence: Hash,
    before_content_digest: Option<Hash>,
    after_content_digest: Option<Hash>,
    resulting_basis: Option<Hash>,
    obstruction: Option<String>,
}

impl ValidatedWorkspacePatchAdapterV1 {
    /// Opens one workspace root and retains only exact path-relative authority.
    pub fn open(
        root: &Path,
        permitted_paths: impl IntoIterator<Item = String>,
        profile: ValidatedWorkspacePatchProfileV1,
    ) -> Result<Self, ValidatedWorkspacePatchErrorV1> {
        validate_profile(profile)?;
        let permitted_paths = validate_permitted_paths(permitted_paths)?;
        if validated_workspace_patch_authority_v1(permitted_paths.iter().map(String::as_str))
            != profile.authority_scope_digest
        {
            return Err(ValidatedWorkspacePatchErrorV1::ProfileMismatch);
        }
        let root = Dir::open_ambient_dir(root, ambient_authority())
            .map_err(|_| ValidatedWorkspacePatchErrorV1::Io)?;
        Ok(Self {
            root,
            permitted_paths,
            profile,
        })
    }

    /// Returns the registry binding for this exact attenuated adapter.
    #[must_use]
    pub const fn adapter_binding(&self) -> ExternalActionAdapterBindingV1 {
        adapter_binding_for(self.profile)
    }

    /// Validates and applies one basis-bound patch after request and claim durability.
    pub fn apply(
        &self,
        grant: &ExternalActionClaimGrantV1,
        admitted: &AdmittedEdictExternalActionRequestV1,
    ) -> Result<ExternalActionSettlementCandidateV1, ValidatedWorkspacePatchErrorV1> {
        validate_grant(self.profile, grant, admitted)?;
        let input = match decode_patch_input(admitted.canonical_operation_input()) {
            Ok(input) => input,
            Err(ValidatedWorkspacePatchErrorV1::Canonical) => {
                return self.obstruction(grant, None, "malformed-input", None);
            }
            Err(error) => return Err(error),
        };
        if let Some(code) = validate_requested_path(&input.path, &self.permitted_paths) {
            return self.obstruction(grant, Some(&input.path), code, None);
        }
        if u64::try_from(input.replacement.len()).unwrap_or(u64::MAX) > self.profile.max_file_bytes
        {
            return self.obstruction(
                grant,
                Some(&input.path),
                "replacement-budget-exceeded",
                None,
            );
        }

        let (parent, file_name) = match open_parent_nofollow(&self.root, &input.path) {
            Ok(value) => value,
            Err(error) => return self.filesystem_obstruction(grant, &input.path, error),
        };
        let (before, metadata) =
            match read_regular_file(&parent, &file_name, self.profile.max_file_bytes) {
                Ok(value) => value,
                Err(error) => return self.filesystem_obstruction(grant, &input.path, error),
            };
        let before_digest: Hash = blake3::hash(&before).into();
        let observed_basis = validated_workspace_patch_basis_v1(&input.path, &before);
        if before_digest != input.expected_content_digest
            || observed_basis != grant.request().basis_digest
        {
            return self.obstruction(
                grant,
                Some(&input.path),
                "stale-basis",
                Some((observed_basis, before_digest)),
            );
        }

        let temp_name = temporary_name(&file_name, grant);
        if let Err(error) = stage_replacement(&parent, &temp_name, &input.replacement, &metadata) {
            return self.filesystem_obstruction(grant, &input.path, error);
        }

        let pre_rename = read_regular_file(&parent, &file_name, self.profile.max_file_bytes);
        let pre_rename_matches = pre_rename
            .as_ref()
            .is_ok_and(|(bytes, _)| bytes.as_slice() == before.as_slice());
        if !pre_rename_matches {
            let _ = parent.remove_file(&temp_name);
            return self.obstruction(
                grant,
                Some(&input.path),
                "stale-basis",
                Some((observed_basis, before_digest)),
            );
        }

        if parent.rename(&temp_name, &parent, &file_name).is_err() {
            let _ = parent.remove_file(&temp_name);
            return self.outcome_unknown(grant, Some(&input.path), "rename-outcome-unknown", None);
        }
        if sync_directory(&parent).is_err() {
            return self.outcome_unknown(
                grant,
                Some(&input.path),
                "directory-sync-outcome-unknown",
                None,
            );
        }

        let (after, _) = match read_regular_file(&parent, &file_name, self.profile.max_file_bytes) {
            Ok(value) => value,
            Err(_) => {
                return self.outcome_unknown(
                    grant,
                    Some(&input.path),
                    "postcondition-unreadable",
                    None,
                );
            }
        };
        let after_digest: Hash = blake3::hash(&after).into();
        let resulting_basis = validated_workspace_patch_basis_v1(&input.path, &after);
        if after_digest != input.replacement_digest || after != input.replacement {
            return self.outcome_unknown(
                grant,
                Some(&input.path),
                "postcondition-mismatch",
                Some((resulting_basis, after_digest)),
            );
        }
        self.candidate(
            grant,
            ExternalActionSettlementKindV1::Succeeded,
            SettlementEvidenceV1 {
                posture: "succeeded",
                path: Some(input.path),
                request_basis: grant.request().basis_digest,
                evidence: resulting_basis,
                before_content_digest: Some(before_digest),
                after_content_digest: Some(after_digest),
                resulting_basis: Some(resulting_basis),
                obstruction: None,
            },
        )
    }

    /// Validates the operation schema and durably admits the settlement.
    pub fn admit_settlement(
        &self,
        store: &mut impl WalStorePort,
        coordinator: &mut ExternalActionCoordinatorV1,
        context: ExternalActionTransactionContextV1,
        admitted: &AdmittedEdictExternalActionRequestV1,
        grant: ExternalActionClaimGrantV1,
        candidate: ExternalActionSettlementCandidateV1,
    ) -> Result<AdmittedExternalActionSettlementV1, ValidatedWorkspacePatchErrorV1> {
        validate_grant(self.profile, &grant, admitted)?;
        validate_candidate(
            self.profile,
            Some(&self.permitted_paths),
            &grant,
            admitted,
            &candidate,
        )?;
        Ok(admit_external_action_settlement(
            store,
            coordinator,
            context,
            grant,
            candidate,
        )?)
    }

    fn filesystem_obstruction(
        &self,
        grant: &ExternalActionClaimGrantV1,
        path: &str,
        error: ValidatedWorkspacePatchErrorV1,
    ) -> Result<ExternalActionSettlementCandidateV1, ValidatedWorkspacePatchErrorV1> {
        let code = match error {
            ValidatedWorkspacePatchErrorV1::SymlinkRefused => "symlink-refused",
            ValidatedWorkspacePatchErrorV1::NotRegularFile => "not-regular-file",
            ValidatedWorkspacePatchErrorV1::FileBudgetExceeded => "file-budget-exceeded",
            ValidatedWorkspacePatchErrorV1::InvalidPath => "invalid-path",
            _ => "io-failure",
        };
        let kind = if code == "io-failure" {
            ExternalActionSettlementKindV1::Failed
        } else {
            ExternalActionSettlementKindV1::Rejected
        };
        self.candidate(
            grant,
            kind,
            SettlementEvidenceV1 {
                posture: posture_for(kind),
                path: Some(path.to_owned()),
                request_basis: grant.request().basis_digest,
                evidence: external_evidence(code, path),
                before_content_digest: None,
                after_content_digest: None,
                resulting_basis: None,
                obstruction: Some(code.to_owned()),
            },
        )
    }

    fn obstruction(
        &self,
        grant: &ExternalActionClaimGrantV1,
        path: Option<&str>,
        code: &str,
        observed: Option<(Hash, Hash)>,
    ) -> Result<ExternalActionSettlementCandidateV1, ValidatedWorkspacePatchErrorV1> {
        let (evidence, before_content_digest) = observed.map_or_else(
            || (external_evidence(code, path.unwrap_or("")), None),
            |(basis, digest)| (basis, Some(digest)),
        );
        self.candidate(
            grant,
            ExternalActionSettlementKindV1::Rejected,
            SettlementEvidenceV1 {
                posture: "rejected",
                path: path.map(str::to_owned),
                request_basis: grant.request().basis_digest,
                evidence,
                before_content_digest,
                after_content_digest: None,
                resulting_basis: None,
                obstruction: Some(code.to_owned()),
            },
        )
    }

    fn outcome_unknown(
        &self,
        grant: &ExternalActionClaimGrantV1,
        path: Option<&str>,
        code: &str,
        observed: Option<(Hash, Hash)>,
    ) -> Result<ExternalActionSettlementCandidateV1, ValidatedWorkspacePatchErrorV1> {
        let (evidence, before_content_digest) = observed.map_or_else(
            || (external_evidence(code, path.unwrap_or("")), None),
            |(basis, digest)| (basis, Some(digest)),
        );
        self.candidate(
            grant,
            ExternalActionSettlementKindV1::OutcomeUnknown,
            SettlementEvidenceV1 {
                posture: "outcomeUnknown",
                path: path.map(str::to_owned),
                request_basis: grant.request().basis_digest,
                evidence,
                before_content_digest,
                after_content_digest: None,
                resulting_basis: None,
                obstruction: Some(code.to_owned()),
            },
        )
    }

    fn candidate(
        &self,
        grant: &ExternalActionClaimGrantV1,
        kind: ExternalActionSettlementKindV1,
        evidence: SettlementEvidenceV1,
    ) -> Result<ExternalActionSettlementCandidateV1, ValidatedWorkspacePatchErrorV1> {
        build_candidate(self.profile, grant, kind, evidence)
    }
}

impl ValidatedWorkspacePatchReconcilerV1 {
    /// Opens a read-only reconciliation handle for one exact path policy.
    pub fn open(
        root: &Path,
        permitted_paths: impl IntoIterator<Item = String>,
        profile: ValidatedWorkspacePatchProfileV1,
    ) -> Result<Self, ValidatedWorkspacePatchErrorV1> {
        validate_profile(profile)?;
        let permitted_paths = validate_permitted_paths(permitted_paths)?;
        if validated_workspace_patch_authority_v1(permitted_paths.iter().map(String::as_str))
            != profile.authority_scope_digest
        {
            return Err(ValidatedWorkspacePatchErrorV1::ProfileMismatch);
        }
        let root = Dir::open_ambient_dir(root, ambient_authority())
            .map_err(|_| ValidatedWorkspacePatchErrorV1::Io)?;
        Ok(Self {
            root,
            permitted_paths,
            profile,
        })
    }

    /// Returns the registry binding for the exact retained adapter identity.
    #[must_use]
    pub const fn adapter_binding(&self) -> ExternalActionAdapterBindingV1 {
        adapter_binding_for(self.profile)
    }

    /// Observes the postcondition of an ambiguous attempt without reapplying it.
    pub fn reconcile(
        &self,
        grant: &ExternalActionClaimGrantV1,
        admitted: &AdmittedEdictExternalActionRequestV1,
    ) -> Result<ExternalActionSettlementCandidateV1, ValidatedWorkspacePatchErrorV1> {
        validate_grant(self.profile, grant, admitted)?;
        let input = match decode_patch_input(admitted.canonical_operation_input()) {
            Ok(input) => input,
            Err(_) => {
                return build_obstruction_candidate(self.profile, grant, None, "malformed-input");
            }
        };
        if let Some(code) = validate_requested_path(&input.path, &self.permitted_paths) {
            return build_obstruction_candidate(self.profile, grant, Some(&input.path), code);
        }
        let (parent, file_name) = match open_parent_nofollow(&self.root, &input.path) {
            Ok(value) => value,
            Err(_) => {
                return build_outcome_unknown_candidate(
                    self.profile,
                    grant,
                    Some(&input.path),
                    "postcondition-unreadable",
                    None,
                );
            }
        };
        let (bytes, _) = match read_regular_file(&parent, &file_name, self.profile.max_file_bytes) {
            Ok(value) => value,
            Err(_) => {
                return build_outcome_unknown_candidate(
                    self.profile,
                    grant,
                    Some(&input.path),
                    "postcondition-unreadable",
                    None,
                );
            }
        };
        let observed_digest: Hash = blake3::hash(&bytes).into();
        let observed_basis = validated_workspace_patch_basis_v1(&input.path, &bytes);
        if observed_digest == input.replacement_digest && bytes == input.replacement {
            return build_candidate(
                self.profile,
                grant,
                ExternalActionSettlementKindV1::Succeeded,
                SettlementEvidenceV1 {
                    posture: "succeeded",
                    path: Some(input.path),
                    request_basis: grant.request().basis_digest,
                    evidence: observed_basis,
                    before_content_digest: Some(input.expected_content_digest),
                    after_content_digest: Some(observed_digest),
                    resulting_basis: Some(observed_basis),
                    obstruction: None,
                },
            );
        }
        build_candidate(
            self.profile,
            grant,
            ExternalActionSettlementKindV1::OutcomeUnknown,
            SettlementEvidenceV1 {
                posture: "outcomeUnknown",
                path: Some(input.path),
                request_basis: grant.request().basis_digest,
                evidence: observed_basis,
                before_content_digest: Some(observed_digest),
                after_content_digest: None,
                resulting_basis: None,
                obstruction: Some("postcondition-not-observed".to_owned()),
            },
        )
    }

    /// Validates and durably admits a reconciled terminal settlement.
    pub fn admit_settlement(
        &self,
        store: &mut impl WalStorePort,
        coordinator: &mut ExternalActionCoordinatorV1,
        context: ExternalActionTransactionContextV1,
        admitted: &AdmittedEdictExternalActionRequestV1,
        grant: ExternalActionClaimGrantV1,
        candidate: ExternalActionSettlementCandidateV1,
    ) -> Result<AdmittedExternalActionSettlementV1, ValidatedWorkspacePatchErrorV1> {
        validate_grant(self.profile, &grant, admitted)?;
        validate_candidate(
            self.profile,
            Some(&self.permitted_paths),
            &grant,
            admitted,
            &candidate,
        )?;
        Ok(admit_external_action_settlement(
            store,
            coordinator,
            context,
            grant,
            candidate,
        )?)
    }
}

/// Encodes one canonical, content-addressed file replacement.
pub fn encode_validated_workspace_patch_input_v1(
    path: String,
    expected_content_digest: Hash,
    replacement: Vec<u8>,
) -> Result<Vec<u8>, ValidatedWorkspacePatchErrorV1> {
    validate_relative_path(&path)?;
    let replacement_digest: Hash = blake3::hash(&replacement).into();
    let bytes = encode_canonical_cbor_v1(&canonical_map([
        ("kind", CanonicalValueV1::Text(PATCH_INPUT_KIND.to_owned())),
        ("path", CanonicalValueV1::Text(path)),
        (
            "expectedContentDigest",
            CanonicalValueV1::Bytes(expected_content_digest.to_vec()),
        ),
        ("replacement", CanonicalValueV1::Bytes(replacement)),
        (
            "replacementDigest",
            CanonicalValueV1::Bytes(replacement_digest.to_vec()),
        ),
    ]))
    .map_err(|_| ValidatedWorkspacePatchErrorV1::Canonical)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_CANONICAL_PATCH_INPUT_BYTES {
        return Err(ValidatedWorkspacePatchErrorV1::FileBudgetExceeded);
    }
    Ok(bytes)
}

/// Commits one exact relative path and its complete file bytes.
#[must_use]
pub fn validated_workspace_patch_basis_v1(path: &str, bytes: &[u8]) -> Hash {
    bounded_workspace_observation_basis_v1([(path, bytes)])
}

/// Commits the exact writable aperture and immutable patch safety policy.
#[must_use]
pub fn validated_workspace_patch_authority_v1<'a>(
    paths: impl IntoIterator<Item = &'a str>,
) -> Hash {
    let mut paths = paths.into_iter().collect::<Vec<_>>();
    paths.sort_unstable();
    paths.dedup();
    let mut hasher = blake3::Hasher::new();
    hasher.update(PATCH_AUTHORITY_DOMAIN);
    hash_len_prefixed(&mut hasher, b"single-file-replace");
    hash_len_prefixed(&mut hasher, b"no-follow");
    hash_len_prefixed(&mut hasher, b"regular-file-only");
    hash_len_prefixed(&mut hasher, b"ci-workflow-forbidden");
    for path in paths {
        hash_len_prefixed(&mut hasher, path.as_bytes());
    }
    hasher.finalize().into()
}

/// Stable adapter and schema failures.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ValidatedWorkspacePatchErrorV1 {
    /// The canonical input or settlement encoding is invalid.
    #[error("validated workspace patch canonical value is invalid")]
    Canonical,
    /// The durable grant does not identify the admitted compiler request.
    #[error("validated workspace patch grant does not match the admitted request")]
    GrantMismatch,
    /// The compiler request is outside the runtime-owned adapter profile.
    #[error("validated workspace patch request does not match adapter policy")]
    ProfileMismatch,
    /// A path is empty, absolute, non-normalized, or traverses a parent.
    #[error("validated workspace patch path is invalid")]
    InvalidPath,
    /// A path is outside the exact attenuated writable aperture.
    #[error("validated workspace patch path is unauthorized")]
    UnauthorizedPath,
    /// CI workflow paths are outside this operation family.
    #[error("validated workspace patch refuses CI workflow paths")]
    CiWorkflowPathRefused,
    /// A path or traversed component is a symlink.
    #[error("validated workspace patch refuses symlinks")]
    SymlinkRefused,
    /// The target must be one regular file.
    #[error("validated workspace patch requires one regular file")]
    NotRegularFile,
    /// Existing or replacement bytes exceed the runtime-owned bound.
    #[error("validated workspace patch file exceeds its byte budget")]
    FileBudgetExceeded,
    /// The candidate did not pass operation-specific settlement admission.
    #[error("validated workspace patch settlement schema admission failed")]
    SchemaAdmissionFailed,
    /// Capability-rooted filesystem access failed.
    #[error("validated workspace patch filesystem access failed")]
    Io,
    /// The generic durable protocol refused the transition.
    #[error(transparent)]
    Protocol(#[from] ExternalActionProtocolErrorV1),
}

const fn adapter_binding_for(
    profile: ValidatedWorkspacePatchProfileV1,
) -> ExternalActionAdapterBindingV1 {
    ExternalActionAdapterBindingV1 {
        adapter_id: profile.adapter_id,
        operation_id: profile.operation_id,
        authority_scope_digest: profile.authority_scope_digest,
    }
}

fn validate_profile(
    profile: ValidatedWorkspacePatchProfileV1,
) -> Result<(), ValidatedWorkspacePatchErrorV1> {
    if profile.operation_id.as_hash() == [0; 32]
        || profile.input_schema_digest == [0; 32]
        || profile.settlement_schema_digest == [0; 32]
        || profile.reconciliation_law_digest == [0; 32]
        || profile.authority_scope_digest == [0; 32]
        || profile.adapter_id.as_hash() == [0; 32]
        || profile.max_file_bytes == 0
        || profile.max_file_bytes > MAX_CANONICAL_PATCH_INPUT_BYTES
    {
        return Err(ValidatedWorkspacePatchErrorV1::ProfileMismatch);
    }
    Ok(())
}

fn validate_permitted_paths(
    paths: impl IntoIterator<Item = String>,
) -> Result<BTreeSet<String>, ValidatedWorkspacePatchErrorV1> {
    let paths = paths.into_iter().collect::<BTreeSet<_>>();
    if paths.is_empty() {
        return Err(ValidatedWorkspacePatchErrorV1::ProfileMismatch);
    }
    for path in &paths {
        validate_relative_path(path)?;
        if is_ci_workflow_path(path) {
            return Err(ValidatedWorkspacePatchErrorV1::CiWorkflowPathRefused);
        }
    }
    Ok(paths)
}

fn validate_grant(
    profile: ValidatedWorkspacePatchProfileV1,
    grant: &ExternalActionClaimGrantV1,
    admitted: &AdmittedEdictExternalActionRequestV1,
) -> Result<(), ValidatedWorkspacePatchErrorV1> {
    let request = grant.request();
    if request != admitted.request()
        || request.input_digest
            != external_action_input_identity(admitted.canonical_operation_input())
        || grant.claim().adapter_id != profile.adapter_id
    {
        return Err(ValidatedWorkspacePatchErrorV1::GrantMismatch);
    }
    if request.operation_id != profile.operation_id
        || request.input_schema_digest != profile.input_schema_digest
        || request.settlement_schema_digest != profile.settlement_schema_digest
        || request.reconciliation_law_digest != profile.reconciliation_law_digest
        || request.authority_scope_digest != profile.authority_scope_digest
    {
        return Err(ValidatedWorkspacePatchErrorV1::ProfileMismatch);
    }
    Ok(())
}

fn validate_candidate(
    profile: ValidatedWorkspacePatchProfileV1,
    permitted_paths: Option<&BTreeSet<String>>,
    grant: &ExternalActionClaimGrantV1,
    admitted: &AdmittedEdictExternalActionRequestV1,
    candidate: &ExternalActionSettlementCandidateV1,
) -> Result<(), ValidatedWorkspacePatchErrorV1> {
    if candidate.request_id != grant.request().request_id()
        || candidate.attempt_id != grant.claim().attempt_id
        || candidate.adapter_id != profile.adapter_id
        || candidate.settlement_schema_digest != profile.settlement_schema_digest
        || candidate.basis_digest != grant.request().basis_digest
        || candidate.declared_result_digest
            != Hash::from(blake3::hash(&candidate.canonical_result_bytes))
    {
        return Err(ValidatedWorkspacePatchErrorV1::SchemaAdmissionFailed);
    }
    let evidence = decode_settlement(&candidate.canonical_result_bytes)?;
    if evidence.posture != posture_for(candidate.kind)
        || evidence.request_basis != candidate.basis_digest
        || evidence.evidence != candidate.external_evidence_digest
    {
        return Err(ValidatedWorkspacePatchErrorV1::SchemaAdmissionFailed);
    }
    if evidence.evidence == [0; 32] {
        return Err(ValidatedWorkspacePatchErrorV1::SchemaAdmissionFailed);
    }
    let input = decode_patch_input(admitted.canonical_operation_input()).ok();
    if candidate.kind == ExternalActionSettlementKindV1::Succeeded {
        let Some(input) = input.as_ref() else {
            return Err(ValidatedWorkspacePatchErrorV1::SchemaAdmissionFailed);
        };
        let Some(permitted_paths) = permitted_paths else {
            return Err(ValidatedWorkspacePatchErrorV1::SchemaAdmissionFailed);
        };
        let expected_resulting_basis =
            validated_workspace_patch_basis_v1(&input.path, &input.replacement);
        if evidence.path.as_deref() != Some(input.path.as_str())
            || !permitted_paths.contains(&input.path)
            || evidence.before_content_digest != Some(input.expected_content_digest)
            || evidence.after_content_digest != Some(input.replacement_digest)
            || evidence.resulting_basis != Some(expected_resulting_basis)
            || evidence.evidence != expected_resulting_basis
            || evidence.obstruction.is_some()
        {
            return Err(ValidatedWorkspacePatchErrorV1::SchemaAdmissionFailed);
        }
    } else if evidence.obstruction.as_deref().is_none_or(str::is_empty)
        || evidence.resulting_basis.is_some()
        || evidence.after_content_digest.is_some()
    {
        return Err(ValidatedWorkspacePatchErrorV1::SchemaAdmissionFailed);
    } else if let Some(input) = input {
        if evidence.path.as_deref() != Some(input.path.as_str()) {
            return Err(ValidatedWorkspacePatchErrorV1::SchemaAdmissionFailed);
        }
    } else if evidence.path.is_some() || evidence.obstruction.as_deref() != Some("malformed-input")
    {
        return Err(ValidatedWorkspacePatchErrorV1::SchemaAdmissionFailed);
    }
    let expected_schema_evidence = schema_admission_evidence(
        profile.settlement_schema_digest,
        &candidate.canonical_result_bytes,
    );
    if candidate.schema_admission_evidence_digest != expected_schema_evidence {
        return Err(ValidatedWorkspacePatchErrorV1::SchemaAdmissionFailed);
    }
    Ok(())
}

fn build_obstruction_candidate(
    profile: ValidatedWorkspacePatchProfileV1,
    grant: &ExternalActionClaimGrantV1,
    path: Option<&str>,
    code: &str,
) -> Result<ExternalActionSettlementCandidateV1, ValidatedWorkspacePatchErrorV1> {
    build_candidate(
        profile,
        grant,
        ExternalActionSettlementKindV1::Rejected,
        SettlementEvidenceV1 {
            posture: "rejected",
            path: path.map(str::to_owned),
            request_basis: grant.request().basis_digest,
            evidence: external_evidence(code, path.unwrap_or("")),
            before_content_digest: None,
            after_content_digest: None,
            resulting_basis: None,
            obstruction: Some(code.to_owned()),
        },
    )
}

fn build_outcome_unknown_candidate(
    profile: ValidatedWorkspacePatchProfileV1,
    grant: &ExternalActionClaimGrantV1,
    path: Option<&str>,
    code: &str,
    observed: Option<(Hash, Hash)>,
) -> Result<ExternalActionSettlementCandidateV1, ValidatedWorkspacePatchErrorV1> {
    let (evidence, before_content_digest) = observed.map_or_else(
        || (external_evidence(code, path.unwrap_or("")), None),
        |(basis, digest)| (basis, Some(digest)),
    );
    build_candidate(
        profile,
        grant,
        ExternalActionSettlementKindV1::OutcomeUnknown,
        SettlementEvidenceV1 {
            posture: "outcomeUnknown",
            path: path.map(str::to_owned),
            request_basis: grant.request().basis_digest,
            evidence,
            before_content_digest,
            after_content_digest: None,
            resulting_basis: None,
            obstruction: Some(code.to_owned()),
        },
    )
}

fn build_candidate(
    profile: ValidatedWorkspacePatchProfileV1,
    grant: &ExternalActionClaimGrantV1,
    kind: ExternalActionSettlementKindV1,
    evidence: SettlementEvidenceV1,
) -> Result<ExternalActionSettlementCandidateV1, ValidatedWorkspacePatchErrorV1> {
    let result = encode_settlement(&evidence)?;
    if u64::try_from(result.len()).unwrap_or(u64::MAX) > grant.request().budget.max_settlement_bytes
    {
        return Err(ValidatedWorkspacePatchErrorV1::FileBudgetExceeded);
    }
    let schema_evidence = schema_admission_evidence(profile.settlement_schema_digest, &result);
    Ok(ExternalActionSettlementCandidateV1::new(
        grant.request().request_id(),
        grant.claim().attempt_id,
        profile.adapter_id,
        kind,
        profile.settlement_schema_digest,
        grant.request().basis_digest,
        result,
        schema_evidence,
        evidence.evidence,
    ))
}

fn decode_patch_input(
    bytes: &[u8],
) -> Result<ValidatedPatchInputV1, ValidatedWorkspacePatchErrorV1> {
    let value =
        decode_canonical_cbor_v1(bytes).map_err(|_| ValidatedWorkspacePatchErrorV1::Canonical)?;
    let CanonicalValueV1::Map(entries) = value else {
        return Err(ValidatedWorkspacePatchErrorV1::Canonical);
    };
    if entries.len() != 5
        || !matches!(
            map_field(&entries, "kind"),
            Some(CanonicalValueV1::Text(value)) if value == PATCH_INPUT_KIND
        )
    {
        return Err(ValidatedWorkspacePatchErrorV1::Canonical);
    }
    let Some(CanonicalValueV1::Text(path)) = map_field(&entries, "path") else {
        return Err(ValidatedWorkspacePatchErrorV1::Canonical);
    };
    let Some(CanonicalValueV1::Bytes(expected_content_digest)) =
        map_field(&entries, "expectedContentDigest")
    else {
        return Err(ValidatedWorkspacePatchErrorV1::Canonical);
    };
    let Some(CanonicalValueV1::Bytes(replacement)) = map_field(&entries, "replacement") else {
        return Err(ValidatedWorkspacePatchErrorV1::Canonical);
    };
    let Some(CanonicalValueV1::Bytes(replacement_digest)) =
        map_field(&entries, "replacementDigest")
    else {
        return Err(ValidatedWorkspacePatchErrorV1::Canonical);
    };
    let expected_content_digest = expected_content_digest
        .as_slice()
        .try_into()
        .map_err(|_| ValidatedWorkspacePatchErrorV1::Canonical)?;
    let replacement_digest: Hash = replacement_digest
        .as_slice()
        .try_into()
        .map_err(|_| ValidatedWorkspacePatchErrorV1::Canonical)?;
    if replacement_digest != Hash::from(blake3::hash(replacement)) {
        return Err(ValidatedWorkspacePatchErrorV1::Canonical);
    }
    Ok(ValidatedPatchInputV1 {
        path: path.clone(),
        expected_content_digest,
        replacement: replacement.clone(),
        replacement_digest,
    })
}

fn encode_settlement(
    evidence: &SettlementEvidenceV1,
) -> Result<Vec<u8>, ValidatedWorkspacePatchErrorV1> {
    encode_canonical_cbor_v1(&canonical_map([
        (
            "kind",
            CanonicalValueV1::Text(PATCH_SETTLEMENT_KIND.to_owned()),
        ),
        (
            "posture",
            CanonicalValueV1::Text(evidence.posture.to_owned()),
        ),
        (
            "path",
            evidence
                .path
                .as_ref()
                .map_or(CanonicalValueV1::Null, |path| {
                    CanonicalValueV1::Text(path.clone())
                }),
        ),
        (
            "requestBasis",
            CanonicalValueV1::Bytes(evidence.request_basis.to_vec()),
        ),
        (
            "evidence",
            CanonicalValueV1::Bytes(evidence.evidence.to_vec()),
        ),
        (
            "beforeContentDigest",
            optional_hash(evidence.before_content_digest),
        ),
        (
            "afterContentDigest",
            optional_hash(evidence.after_content_digest),
        ),
        ("resultingBasis", optional_hash(evidence.resulting_basis)),
        (
            "obstruction",
            evidence
                .obstruction
                .as_ref()
                .map_or(CanonicalValueV1::Null, |value| {
                    CanonicalValueV1::Text(value.clone())
                }),
        ),
    ]))
    .map_err(|_| ValidatedWorkspacePatchErrorV1::Canonical)
}

fn decode_settlement(bytes: &[u8]) -> Result<SettlementEvidenceV1, ValidatedWorkspacePatchErrorV1> {
    let value =
        decode_canonical_cbor_v1(bytes).map_err(|_| ValidatedWorkspacePatchErrorV1::Canonical)?;
    let CanonicalValueV1::Map(entries) = value else {
        return Err(ValidatedWorkspacePatchErrorV1::SchemaAdmissionFailed);
    };
    if entries.len() != 9
        || !matches!(
            map_field(&entries, "kind"),
            Some(CanonicalValueV1::Text(value)) if value == PATCH_SETTLEMENT_KIND
        )
    {
        return Err(ValidatedWorkspacePatchErrorV1::SchemaAdmissionFailed);
    }
    let Some(CanonicalValueV1::Text(posture)) = map_field(&entries, "posture") else {
        return Err(ValidatedWorkspacePatchErrorV1::SchemaAdmissionFailed);
    };
    let posture = match posture.as_str() {
        "succeeded" => "succeeded",
        "rejected" => "rejected",
        "failed" => "failed",
        "outcomeUnknown" => "outcomeUnknown",
        _ => return Err(ValidatedWorkspacePatchErrorV1::SchemaAdmissionFailed),
    };
    Ok(SettlementEvidenceV1 {
        posture,
        path: optional_text(map_field(&entries, "path"))?,
        request_basis: required_hash(map_field(&entries, "requestBasis"))?,
        evidence: required_hash(map_field(&entries, "evidence"))?,
        before_content_digest: optional_hash_value(map_field(&entries, "beforeContentDigest"))?,
        after_content_digest: optional_hash_value(map_field(&entries, "afterContentDigest"))?,
        resulting_basis: optional_hash_value(map_field(&entries, "resultingBasis"))?,
        obstruction: optional_text(map_field(&entries, "obstruction"))?,
    })
}

fn validate_requested_path(path: &str, permitted_paths: &BTreeSet<String>) -> Option<&'static str> {
    if validate_relative_path(path).is_err() {
        Some("invalid-path")
    } else if is_ci_workflow_path(path) {
        Some("ci-workflow-refused")
    } else if !permitted_paths.contains(path) {
        Some("unauthorized-path")
    } else {
        None
    }
}

fn validate_relative_path(path: &str) -> Result<(), ValidatedWorkspacePatchErrorV1> {
    if path.is_empty()
        || path.starts_with('/')
        || path.ends_with('/')
        || path.contains("//")
        || path.contains('\\')
        || path
            .split('/')
            .any(|component| component.is_empty() || matches!(component, "." | ".."))
    {
        return Err(ValidatedWorkspacePatchErrorV1::InvalidPath);
    }
    let mut count = 0_usize;
    for component in Path::new(path).components() {
        match component {
            Component::Normal(value) if value.to_str().is_some() => count = count.saturating_add(1),
            _ => return Err(ValidatedWorkspacePatchErrorV1::InvalidPath),
        }
    }
    if count == 0 {
        Err(ValidatedWorkspacePatchErrorV1::InvalidPath)
    } else {
        Ok(())
    }
}

fn is_ci_workflow_path(path: &str) -> bool {
    path == ".github/workflows" || path.starts_with(".github/workflows/")
}

fn open_parent_nofollow(
    root: &Dir,
    path: &str,
) -> Result<(Dir, OsString), ValidatedWorkspacePatchErrorV1> {
    validate_relative_path(path)?;
    let components = Path::new(path).components().collect::<Vec<_>>();
    let Some((file_name, parents)) = components.split_last() else {
        return Err(ValidatedWorkspacePatchErrorV1::InvalidPath);
    };
    let Component::Normal(file_name) = file_name else {
        return Err(ValidatedWorkspacePatchErrorV1::InvalidPath);
    };
    let mut parent = root
        .try_clone()
        .map_err(|_| ValidatedWorkspacePatchErrorV1::Io)?;
    for component in parents {
        let Component::Normal(component) = component else {
            return Err(ValidatedWorkspacePatchErrorV1::InvalidPath);
        };
        let metadata = parent
            .symlink_metadata(component)
            .map_err(|_| ValidatedWorkspacePatchErrorV1::Io)?;
        if metadata.file_type().is_symlink() {
            return Err(ValidatedWorkspacePatchErrorV1::SymlinkRefused);
        }
        parent = parent
            .open_dir_nofollow(component)
            .map_err(|_| ValidatedWorkspacePatchErrorV1::Io)?;
    }
    Ok((parent, file_name.to_os_string()))
}

fn read_regular_file(
    parent: &Dir,
    file_name: &OsString,
    max_bytes: u64,
) -> Result<(Vec<u8>, Metadata), ValidatedWorkspacePatchErrorV1> {
    let metadata = parent
        .symlink_metadata(file_name)
        .map_err(|_| ValidatedWorkspacePatchErrorV1::Io)?;
    if metadata.file_type().is_symlink() {
        return Err(ValidatedWorkspacePatchErrorV1::SymlinkRefused);
    }
    if !metadata.is_file() {
        return Err(ValidatedWorkspacePatchErrorV1::NotRegularFile);
    }
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No).nonblock(true);
    let mut file = parent
        .open_with(file_name, &options)
        .map_err(|_| ValidatedWorkspacePatchErrorV1::Io)?;
    let opened_metadata = file
        .metadata()
        .map_err(|_| ValidatedWorkspacePatchErrorV1::Io)?;
    if !opened_metadata.is_file() {
        return Err(ValidatedWorkspacePatchErrorV1::NotRegularFile);
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| ValidatedWorkspacePatchErrorV1::Io)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > max_bytes {
        return Err(ValidatedWorkspacePatchErrorV1::FileBudgetExceeded);
    }
    Ok((bytes, opened_metadata))
}

fn stage_replacement(
    parent: &Dir,
    temp_name: &OsString,
    replacement: &[u8],
    metadata: &Metadata,
) -> Result<(), ValidatedWorkspacePatchErrorV1> {
    let mut options = OpenOptions::new();
    options
        .write(true)
        .create_new(true)
        .follow(FollowSymlinks::No);
    let mut file = parent
        .open_with(temp_name, &options)
        .map_err(|_| ValidatedWorkspacePatchErrorV1::Io)?;
    if let Err(error) = file.write_all(replacement) {
        let _ = parent.remove_file(temp_name);
        return Err(if error.kind() == std::io::ErrorKind::InvalidInput {
            ValidatedWorkspacePatchErrorV1::FileBudgetExceeded
        } else {
            ValidatedWorkspacePatchErrorV1::Io
        });
    }
    if parent
        .set_permissions(temp_name, metadata.permissions())
        .is_err()
    {
        let _ = parent.remove_file(temp_name);
        return Err(ValidatedWorkspacePatchErrorV1::Io);
    }
    if file.sync_all().is_err() {
        let _ = parent.remove_file(temp_name);
        return Err(ValidatedWorkspacePatchErrorV1::Io);
    }
    Ok(())
}

fn temporary_name(file_name: &OsString, grant: &ExternalActionClaimGrantV1) -> OsString {
    let mut name = OsString::from(".");
    name.push(file_name);
    name.push(".echo-patch-");
    name.push(hex::encode(grant.claim().attempt_id.as_hash()));
    name
}

fn sync_directory(parent: &Dir) -> Result<(), ValidatedWorkspacePatchErrorV1> {
    parent
        .try_clone()
        .map_err(|_| ValidatedWorkspacePatchErrorV1::Io)?
        .into_std_file()
        .sync_all()
        .map_err(|_| ValidatedWorkspacePatchErrorV1::Io)
}

fn external_action_input_identity(bytes: &[u8]) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(EXTERNAL_ACTION_INPUT_DOMAIN);
    hash_len_prefixed(&mut hasher, bytes);
    hasher.finalize().into()
}

fn schema_admission_evidence(schema: Hash, bytes: &[u8]) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(PATCH_SCHEMA_EVIDENCE_DOMAIN);
    hasher.update(&schema);
    hash_len_prefixed(&mut hasher, bytes);
    hasher.finalize().into()
}

fn external_evidence(code: &str, detail: &str) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(PATCH_EXTERNAL_EVIDENCE_DOMAIN);
    hash_len_prefixed(&mut hasher, code.as_bytes());
    hash_len_prefixed(&mut hasher, detail.as_bytes());
    hasher.finalize().into()
}

fn posture_for(kind: ExternalActionSettlementKindV1) -> &'static str {
    match kind {
        ExternalActionSettlementKindV1::Succeeded => "succeeded",
        ExternalActionSettlementKindV1::Rejected => "rejected",
        ExternalActionSettlementKindV1::Failed => "failed",
        ExternalActionSettlementKindV1::OutcomeUnknown => "outcomeUnknown",
    }
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

fn map_field<'a>(
    entries: &'a [(CanonicalValueV1, CanonicalValueV1)],
    field: &str,
) -> Option<&'a CanonicalValueV1> {
    entries.iter().find_map(|(key, value)| match key {
        CanonicalValueV1::Text(key) if key == field => Some(value),
        _ => None,
    })
}

fn optional_hash(value: Option<Hash>) -> CanonicalValueV1 {
    value.map_or(CanonicalValueV1::Null, |value| {
        CanonicalValueV1::Bytes(value.to_vec())
    })
}

fn required_hash(value: Option<&CanonicalValueV1>) -> Result<Hash, ValidatedWorkspacePatchErrorV1> {
    let Some(CanonicalValueV1::Bytes(value)) = value else {
        return Err(ValidatedWorkspacePatchErrorV1::SchemaAdmissionFailed);
    };
    value
        .as_slice()
        .try_into()
        .map_err(|_| ValidatedWorkspacePatchErrorV1::SchemaAdmissionFailed)
}

fn optional_hash_value(
    value: Option<&CanonicalValueV1>,
) -> Result<Option<Hash>, ValidatedWorkspacePatchErrorV1> {
    match value {
        Some(CanonicalValueV1::Null) => Ok(None),
        Some(value) => required_hash(Some(value)).map(Some),
        None => Err(ValidatedWorkspacePatchErrorV1::SchemaAdmissionFailed),
    }
}

fn optional_text(
    value: Option<&CanonicalValueV1>,
) -> Result<Option<String>, ValidatedWorkspacePatchErrorV1> {
    match value {
        Some(CanonicalValueV1::Null) => Ok(None),
        Some(CanonicalValueV1::Text(value)) => Ok(Some(value.clone())),
        _ => Err(ValidatedWorkspacePatchErrorV1::SchemaAdmissionFailed),
    }
}

fn hash_len_prefixed(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    hasher.update(&u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_le_bytes());
    hasher.update(bytes);
}
