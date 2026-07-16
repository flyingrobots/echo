// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Deterministic build and structural admission for Echo Edict provider components.

use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::ffi::OsString;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use wasm_encoder::{ComponentSection, CustomSection};
use wasmparser::{
    ComponentExternalKind, ComponentType, ComponentTypeRef, Encoding, InstanceTypeDeclaration,
    Parser, Payload, TypeBounds, Validator, WasmFeatures,
};
use wit_component::{ComponentEncoder, DecodedWasm, StringEncoding};
use wit_parser::{LiveTypes, PackageName, Resolve, WorldId, WorldItem};

/// Name of the sole provider-contract attestation custom section.
pub(crate) const CONTRACT_SECTION_NAME: &str = "edict:target-provider-contract";

/// Exact Edict lowerer contract coordinate carried by the attestation.
pub(crate) const LOWERER_CONTRACT_COORDINATE: &str = "edict:target-provider/lowerer@1.0.0";

/// Exact type-only protocol instance that a provider component may import.
pub(crate) const PROTOCOL_INSTANCE_COORDINATE: &str = "edict:target-provider/protocol@1.0.0";

const PROTOCOL_TYPE_IMPORTS: [&str; 2] = ["lowering-request-v1", "lowering-result-v1"];
const LOWERER_WORLD_NAME: &str = "lowerer";
const LOWERER_WIT_SOURCE: &str =
    include_str!("../../crates/echo-edict-provider-lowerer/wit/edict-target-provider.wit");
const LOWERER_PACKAGE: &str = "echo-edict-provider-lowerer";
const LOWERER_CORE_WASM: &str = "echo_edict_provider_lowerer.wasm";
const LOWER_EXPORT: &str = "lower";

const VERIFIER_CONTRACT_COORDINATE: &str = "edict:target-provider/verifier@1.0.0";
const VERIFIER_PROTOCOL_TYPE_IMPORTS: [&str; 2] =
    ["verification-request-v1", "verification-result-v1"];
const VERIFIER_WORLD_NAME: &str = "verifier";
const VERIFIER_WIT_SOURCE: &str =
    include_str!("../../crates/echo-edict-provider-verifier/wit/edict-target-provider.wit");
const VERIFIER_PACKAGE: &str = "echo-edict-provider-verifier";
const VERIFIER_CORE_WASM: &str = "echo_edict_provider_verifier.wasm";
const VERIFY_EXPORT: &str = "verify";

/// Exact host triple designated to produce the checked component bytes.
pub(crate) const CHECKED_COMPONENT_BUILDER_HOST: &str = "x86_64-unknown-linux-gnu";
const PINNED_RUST_TOOLCHAIN: &str = "1.90.0";
const PINNED_RUSTC_COMMIT: &str = "1159e78c4747b02ef996e55082b704c09b970588";
const PINNED_CARGO_COMMIT: &str = "840b83a10fb0e039a83f4d70ad032892c287570a";

/// Reviewed identity that the portable promotion command is permitted to install.
pub(crate) const APPROVED_CHECKED_COMPONENT_SHA256: &str =
    "03edee44c6bc70eb998c0c17662a214809746af3bba0740f3407c18a4016309e";
pub(crate) const CHECKED_COMPONENT_REPOSITORY_PATH: &str =
    "schemas/edict-provider/components/v1/lowerer.echo-dpo.component.wasm";

/// Repository path reserved for the approved verifier component.
pub(crate) const VERIFIER_CHECKED_COMPONENT_REPOSITORY_PATH: &str =
    "schemas/edict-provider/components/v1/verifier.echo-dpo.component.wasm";
/// Approved SHA-256 identity of the checked verifier component.
pub(crate) const APPROVED_CHECKED_VERIFIER_COMPONENT_SHA256: &str =
    "e13eda6e02d5a46d2aecdec0546d53a7bf66f2580f8d5ec06e5d76710716a27b";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ProviderComponentSpec {
    package: &'static str,
    core_wasm: &'static str,
    contract_coordinate: &'static str,
    protocol_type_imports: [&'static str; 2],
    world_name: &'static str,
    wit_source: &'static str,
    callable_export: &'static str,
    checked_repository_path: &'static str,
    approved_sha256: Option<&'static str>,
}

const LOWERER_SPEC: ProviderComponentSpec = ProviderComponentSpec {
    package: LOWERER_PACKAGE,
    core_wasm: LOWERER_CORE_WASM,
    contract_coordinate: LOWERER_CONTRACT_COORDINATE,
    protocol_type_imports: PROTOCOL_TYPE_IMPORTS,
    world_name: LOWERER_WORLD_NAME,
    wit_source: LOWERER_WIT_SOURCE,
    callable_export: LOWER_EXPORT,
    checked_repository_path: CHECKED_COMPONENT_REPOSITORY_PATH,
    approved_sha256: Some(APPROVED_CHECKED_COMPONENT_SHA256),
};

const VERIFIER_SPEC: ProviderComponentSpec = ProviderComponentSpec {
    package: VERIFIER_PACKAGE,
    core_wasm: VERIFIER_CORE_WASM,
    contract_coordinate: VERIFIER_CONTRACT_COORDINATE,
    protocol_type_imports: VERIFIER_PROTOCOL_TYPE_IMPORTS,
    world_name: VERIFIER_WORLD_NAME,
    wit_source: VERIFIER_WIT_SOURCE,
    callable_export: VERIFY_EXPORT,
    checked_repository_path: VERIFIER_CHECKED_COMPONENT_REPOSITORY_PATH,
    approved_sha256: Some(APPROVED_CHECKED_VERIFIER_COMPONENT_SHA256),
};

fn export_missing_kind(spec: &ProviderComponentSpec) -> ProviderLowererComponentErrorKind {
    if spec.callable_export == VERIFY_EXPORT {
        ProviderLowererComponentErrorKind::VerifyExportMissing
    } else {
        ProviderLowererComponentErrorKind::LowerExportMissing
    }
}

fn export_duplicate_kind(spec: &ProviderComponentSpec) -> ProviderLowererComponentErrorKind {
    if spec.callable_export == VERIFY_EXPORT {
        ProviderLowererComponentErrorKind::VerifyExportDuplicate
    } else {
        ProviderLowererComponentErrorKind::LowerExportDuplicate
    }
}

fn export_invalid_kind(spec: &ProviderComponentSpec) -> ProviderLowererComponentErrorKind {
    if spec.callable_export == VERIFY_EXPORT {
        ProviderLowererComponentErrorKind::VerifyExportInvalid
    } else {
        ProviderLowererComponentErrorKind::LowerExportInvalid
    }
}

/// Stable classification for a component build or audit failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProviderLowererComponentErrorKind {
    /// A path required by the deterministic build is not valid UTF-8.
    InvalidPath,
    /// Cargo could not be started.
    BuildInvocationFailed,
    /// Cargo completed without producing a successful provider-component build.
    BuildFailed,
    /// The expected core WebAssembly artifact could not be read.
    CoreWasmReadFailed,
    /// The pinned Rust compiler could not report its host identity.
    BuilderHostInvocationFailed,
    /// The pinned Rust compiler reported no unique host identity.
    BuilderHostInvalid,
    /// A pinned Rust tool reported a release other than the reviewed release.
    BuilderReleaseMismatch,
    /// A pinned Rust tool reported a commit other than the reviewed commit.
    BuilderCommitMismatch,
    /// Exact checked-byte comparison was attempted on a non-designated host.
    BuilderHostMismatch,
    /// An explicit component candidate could not be read.
    ComponentReadFailed,
    /// Two explicit reproducibility candidates differ byte-for-byte.
    CandidateMismatch,
    /// Two candidate paths resolve to the same underlying file.
    CandidateAliased,
    /// An expected candidate identity is not exact lowercase SHA-256.
    ExpectedDigestInvalid,
    /// Reproducible candidates do not have the explicitly expected digest.
    CandidateDigestMismatch,
    /// No reviewed checked identity has been approved for this component.
    CheckedIdentityUnapproved,
    /// A one-build candidate command targeted the checked repository artifact.
    CheckedOutputRequiresPromotion,
    /// A component command targeted another component's reserved checked artifact.
    ReservedCheckedOutputForbidden,
    /// The core module could not be componentized.
    ComponentEncodingFailed,
    /// The component byte stream is malformed or fails WebAssembly validation.
    ComponentInvalid,
    /// The component is not a top-level WebAssembly component.
    ComponentEncodingInvalid,
    /// The required contract attestation is absent.
    ContractAttestationMissing,
    /// More than one contract attestation is present.
    ContractAttestationDuplicate,
    /// A contract attestation is nested rather than top-level.
    ContractAttestationNested,
    /// The contract attestation carries bytes other than the frozen coordinate.
    ContractAttestationMismatch,
    /// A core WebAssembly import is present.
    CoreImportForbidden,
    /// A component import other than the frozen protocol instance is present.
    ComponentImportForbidden,
    /// More than one frozen protocol instance import is present.
    ProtocolImportDuplicate,
    /// The protocol import is callable, unknown, or not backed by a type-only instance.
    ProtocolImportInvalid,
    /// The protocol instance and its exact type aliases do not form one closure.
    ProtocolImportClosureInvalid,
    /// The candidate component's decoded WIT world could not be authenticated.
    WorldContractInvalid,
    /// The candidate's complete decoded WIT world differs from its frozen selected world.
    WorldContractMismatch,
    /// The required `lower` export is absent.
    LowerExportMissing,
    /// More than one `lower` export is present.
    LowerExportDuplicate,
    /// A top-level export is not the exact callable `lower` export.
    LowerExportInvalid,
    /// The required `verify` export is absent.
    VerifyExportMissing,
    /// More than one `verify` export is present.
    VerifyExportDuplicate,
    /// A top-level export is not the exact callable `verify` export.
    VerifyExportInvalid,
    /// An explicit checked output does not exist.
    OutputMissing,
    /// An explicit checked output could not be read.
    OutputReadFailed,
    /// An explicit checked output differs from the newly built bytes.
    OutputDrift,
    /// An explicit output path is a symlink or is not a regular file.
    OutputTypeInvalid,
    /// An explicit output could not be written.
    OutputWriteFailed,
}

impl ProviderLowererComponentErrorKind {
    fn code(self) -> &'static str {
        match self {
            Self::InvalidPath => "invalid-path",
            Self::BuildInvocationFailed => "build-invocation-failed",
            Self::BuildFailed => "build-failed",
            Self::CoreWasmReadFailed => "core-wasm-read-failed",
            Self::BuilderHostInvocationFailed => "builder-host-invocation-failed",
            Self::BuilderHostInvalid => "builder-host-invalid",
            Self::BuilderReleaseMismatch => "builder-release-mismatch",
            Self::BuilderCommitMismatch => "builder-commit-mismatch",
            Self::BuilderHostMismatch => "builder-host-mismatch",
            Self::ComponentReadFailed => "component-read-failed",
            Self::CandidateMismatch => "candidate-mismatch",
            Self::CandidateAliased => "candidate-aliased",
            Self::ExpectedDigestInvalid => "expected-digest-invalid",
            Self::CandidateDigestMismatch => "candidate-digest-mismatch",
            Self::CheckedIdentityUnapproved => "checked-identity-unapproved",
            Self::CheckedOutputRequiresPromotion => "checked-output-requires-promotion",
            Self::ReservedCheckedOutputForbidden => "reserved-checked-output-forbidden",
            Self::ComponentEncodingFailed => "component-encoding-failed",
            Self::ComponentInvalid => "component-invalid",
            Self::ComponentEncodingInvalid => "component-encoding-invalid",
            Self::ContractAttestationMissing => "contract-attestation-missing",
            Self::ContractAttestationDuplicate => "contract-attestation-duplicate",
            Self::ContractAttestationNested => "contract-attestation-nested",
            Self::ContractAttestationMismatch => "contract-attestation-mismatch",
            Self::CoreImportForbidden => "core-import-forbidden",
            Self::ComponentImportForbidden => "component-import-forbidden",
            Self::ProtocolImportDuplicate => "protocol-import-duplicate",
            Self::ProtocolImportInvalid => "protocol-import-invalid",
            Self::ProtocolImportClosureInvalid => "protocol-import-closure-invalid",
            Self::WorldContractInvalid => "world-contract-invalid",
            Self::WorldContractMismatch => "world-contract-mismatch",
            Self::LowerExportMissing => "lower-export-missing",
            Self::LowerExportDuplicate => "lower-export-duplicate",
            Self::LowerExportInvalid => "lower-export-invalid",
            Self::VerifyExportMissing => "verify-export-missing",
            Self::VerifyExportDuplicate => "verify-export-duplicate",
            Self::VerifyExportInvalid => "verify-export-invalid",
            Self::OutputMissing => "output-missing",
            Self::OutputReadFailed => "output-read-failed",
            Self::OutputDrift => "output-drift",
            Self::OutputTypeInvalid => "output-type-invalid",
            Self::OutputWriteFailed => "output-write-failed",
        }
    }
}

/// Stable, typed error returned by component build and audit operations.
#[derive(Debug)]
pub(crate) struct ProviderLowererComponentError {
    component_label: &'static str,
    kind: ProviderLowererComponentErrorKind,
    subject: String,
    reference: Option<String>,
    detail: Option<String>,
}

impl ProviderLowererComponentError {
    fn new(
        kind: ProviderLowererComponentErrorKind,
        subject: impl Into<String>,
        reference: Option<String>,
    ) -> Self {
        Self {
            component_label: "lowerer",
            kind,
            subject: subject.into(),
            reference,
            detail: None,
        }
    }

    fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    fn for_spec(mut self, spec: &ProviderComponentSpec) -> Self {
        self.component_label = spec.world_name;
        self
    }

    /// Returns the stable failure classification.
    #[cfg(test)]
    pub(crate) fn kind(&self) -> ProviderLowererComponentErrorKind {
        self.kind
    }

    /// Returns the stable subject of the failed operation.
    #[cfg(test)]
    pub(crate) fn subject(&self) -> &str {
        &self.subject
    }

    /// Returns the optional stable reference associated with the failure.
    #[cfg(test)]
    pub(crate) fn reference(&self) -> Option<&str> {
        self.reference.as_deref()
    }
}

impl fmt::Display for ProviderLowererComponentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "provider-{}-component:{}: {}",
            self.component_label,
            self.kind.code(),
            self.subject
        )?;
        if let Some(reference) = &self.reference {
            write!(formatter, " ({reference})")?;
        }
        Ok(())
    }
}

impl std::error::Error for ProviderLowererComponentError {}

/// Result alias for deterministic component operations.
pub(crate) type Result<T> = std::result::Result<T, ProviderLowererComponentError>;

/// Exact provider component bytes and their SHA-256 identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProviderLowererComponent {
    bytes: Vec<u8>,
    sha256: [u8; 32],
}

#[derive(Debug)]
pub(crate) struct PinnedRustToolchain {
    cargo: PathBuf,
    rustc: PathBuf,
    host: String,
}

impl PinnedRustToolchain {
    pub(crate) fn host(&self) -> &str {
        &self.host
    }
}

impl ProviderLowererComponent {
    /// Returns the exact final component bytes.
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the lowercase hexadecimal SHA-256 digest.
    pub(crate) fn sha256_hex(&self) -> String {
        digest_hex(&self.sha256)
    }
}

/// Resolves and authenticates the exact Rust toolchain used by component builds.
pub(crate) fn pinned_rust_toolchain() -> Result<PinnedRustToolchain> {
    let rustc = resolve_rustup_tool("rustc")?;
    let cargo = resolve_rustup_tool("cargo")?;
    let rustc_identity = invoke_tool_identity(&rustc, "-vV", "rustc")?;
    let cargo_identity = invoke_tool_identity(&cargo, "-Vv", "cargo")?;
    let rustc_host = authenticate_tool_identity("rustc", &rustc_identity, PINNED_RUSTC_COMMIT)?;
    let cargo_host = authenticate_tool_identity("cargo", &cargo_identity, PINNED_CARGO_COMMIT)?;
    if rustc_host != cargo_host {
        return Err(ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::BuilderHostMismatch,
            cargo_host,
            Some(rustc_host.to_owned()),
        ));
    }
    Ok(PinnedRustToolchain {
        cargo,
        rustc,
        host: rustc_host.to_owned(),
    })
}

/// Resolves the same pinned toolchain with verifier-specific failure context.
pub(crate) fn pinned_verifier_rust_toolchain() -> Result<PinnedRustToolchain> {
    pinned_rust_toolchain().map_err(|error| error.for_spec(&VERIFIER_SPEC))
}

/// Authenticates the pinned compiler host before an exact checked-byte build.
pub(crate) fn require_checked_builder() -> Result<PinnedRustToolchain> {
    let toolchain = pinned_rust_toolchain()?;
    ensure_checked_builder_host(toolchain.host())?;
    Ok(toolchain)
}

/// Authenticates the designated verifier builder host and pinned toolchain.
pub(crate) fn require_verifier_checked_builder() -> Result<PinnedRustToolchain> {
    require_checked_builder().map_err(|error| error.for_spec(&VERIFIER_SPEC))
}

fn resolve_rustup_tool(tool: &str) -> Result<PathBuf> {
    let output = Command::new("rustup")
        .args(["which", "--toolchain", PINNED_RUST_TOOLCHAIN, tool])
        .output()
        .map_err(|error| {
            ProviderLowererComponentError::new(
                ProviderLowererComponentErrorKind::BuilderHostInvocationFailed,
                format!("rustup which {tool}"),
                Some(PINNED_RUST_TOOLCHAIN.to_owned()),
            )
            .with_detail(error.to_string())
        })?;
    if !output.status.success() {
        return Err(ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::BuilderHostInvocationFailed,
            format!("rustup which {tool}"),
            Some(PINNED_RUST_TOOLCHAIN.to_owned()),
        )
        .with_detail(String::from_utf8_lossy(&output.stderr)));
    }

    let resolved = String::from_utf8(output.stdout).map_err(|error| {
        ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::BuilderHostInvalid,
            format!("rustup which {tool}"),
            Some("utf-8-absolute-path".to_owned()),
        )
        .with_detail(error.to_string())
    })?;
    let path = parse_resolved_tool_path(&resolved).ok_or_else(|| {
        ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::BuilderHostInvalid,
            format!("rustup which {tool}"),
            Some("one-absolute-path".to_owned()),
        )
    })?;
    fs::canonicalize(&path).map_err(|error| {
        ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::BuilderHostInvalid,
            path.display().to_string(),
            Some(tool.to_owned()),
        )
        .with_detail(error.to_string())
    })
}

fn parse_resolved_tool_path(output: &str) -> Option<PathBuf> {
    let mut lines = output.lines();
    let path = lines.next()?;
    if path.is_empty() || lines.next().is_some() {
        return None;
    }
    let path = PathBuf::from(path);
    path.is_absolute().then_some(path)
}

fn invoke_tool_identity(path: &Path, version_arg: &str, tool: &str) -> Result<String> {
    let output = Command::new(path)
        .arg(version_arg)
        .output()
        .map_err(|error| {
            ProviderLowererComponentError::new(
                ProviderLowererComponentErrorKind::BuilderHostInvocationFailed,
                path.display().to_string(),
                Some(tool.to_owned()),
            )
            .with_detail(error.to_string())
        })?;
    if !output.status.success() {
        return Err(ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::BuilderHostInvocationFailed,
            path.display().to_string(),
            Some(tool.to_owned()),
        )
        .with_detail(String::from_utf8_lossy(&output.stderr)));
    }
    String::from_utf8(output.stdout).map_err(|error| {
        ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::BuilderHostInvalid,
            path.display().to_string(),
            Some(tool.to_owned()),
        )
        .with_detail(error.to_string())
    })
}

fn authenticate_tool_identity<'a>(
    tool: &str,
    identity: &'a str,
    expected_commit: &str,
) -> Result<&'a str> {
    let release = parse_identity_field(identity, "release").ok_or_else(|| {
        ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::BuilderHostInvalid,
            tool,
            Some("release".to_owned()),
        )
    })?;
    if release != PINNED_RUST_TOOLCHAIN {
        return Err(ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::BuilderReleaseMismatch,
            release,
            Some(PINNED_RUST_TOOLCHAIN.to_owned()),
        ));
    }
    let commit = parse_identity_field(identity, "commit-hash").ok_or_else(|| {
        ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::BuilderHostInvalid,
            tool,
            Some("commit-hash".to_owned()),
        )
    })?;
    if commit != expected_commit {
        return Err(ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::BuilderCommitMismatch,
            commit,
            Some(expected_commit.to_owned()),
        ));
    }
    parse_identity_field(identity, "host").ok_or_else(|| {
        ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::BuilderHostInvalid,
            tool,
            Some("host".to_owned()),
        )
    })
}

#[cfg(test)]
fn parse_rustc_host(version: &str) -> Option<&str> {
    parse_identity_field(version, "host")
}

fn parse_identity_field<'a>(identity: &'a str, field: &str) -> Option<&'a str> {
    let prefix = format!("{field}: ");
    let mut values = identity
        .lines()
        .filter_map(|line| line.strip_prefix(&prefix));
    let value = values.next()?;
    if value.is_empty() || values.next().is_some() {
        return None;
    }
    Some(value)
}

fn ensure_checked_builder_host(host: &str) -> Result<()> {
    if host == CHECKED_COMPONENT_BUILDER_HOST {
        return Ok(());
    }
    Err(ProviderLowererComponentError::new(
        ProviderLowererComponentErrorKind::BuilderHostMismatch,
        host,
        Some(CHECKED_COMPONENT_BUILDER_HOST.to_owned()),
    ))
}

/// Reads and fully audits an explicit component candidate.
pub(crate) fn read_component(input_path: &Path) -> Result<ProviderLowererComponent> {
    authenticated_component(read_component_bytes(input_path)?)
}

/// Reads and fully audits an explicit verifier component candidate.
pub(crate) fn read_verifier_component(input_path: &Path) -> Result<ProviderLowererComponent> {
    read_component_bytes(input_path)
        .and_then(authenticated_verifier_component)
        .map_err(|error| error.for_spec(&VERIFIER_SPEC))
}

fn read_component_bytes(input_path: &Path) -> Result<Vec<u8>> {
    fs::read(input_path).map_err(|error| {
        ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::ComponentReadFailed,
            input_path.display().to_string(),
            None,
        )
        .with_detail(error.to_string())
    })
}

pub(crate) fn ensure_designated_candidate_output(
    output_path: &Path,
    checked_path: &Path,
) -> Result<()> {
    ensure_output_does_not_alias(
        output_path,
        checked_path,
        ProviderLowererComponentErrorKind::CheckedOutputRequiresPromotion,
    )
}

fn ensure_output_does_not_alias(
    output_path: &Path,
    reserved_path: &Path,
    kind: ProviderLowererComponentErrorKind,
) -> Result<()> {
    let aliases_reserved = match same_file::is_same_file(output_path, reserved_path) {
        Ok(aliases) => aliases,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            normalized_destination(output_path)? == normalized_destination(reserved_path)?
        }
        Err(error) => {
            return Err(ProviderLowererComponentError::new(
                ProviderLowererComponentErrorKind::OutputReadFailed,
                output_path.display().to_string(),
                Some(reserved_path.display().to_string()),
            )
            .with_detail(error.to_string()));
        }
    };
    if aliases_reserved {
        return Err(ProviderLowererComponentError::new(
            kind,
            output_path.display().to_string(),
            Some(reserved_path.display().to_string()),
        ));
    }
    Ok(())
}

/// Refuses an output that aliases another component's reserved checked path.
pub(crate) fn ensure_reserved_checked_output(
    output_path: &Path,
    reserved_path: &Path,
) -> Result<()> {
    ensure_output_does_not_alias(
        output_path,
        reserved_path,
        ProviderLowererComponentErrorKind::ReservedCheckedOutputForbidden,
    )
}

/// Refuses a verifier output that aliases another component's reserved checked path.
pub(crate) fn ensure_verifier_reserved_checked_output(
    output_path: &Path,
    reserved_path: &Path,
) -> Result<()> {
    ensure_reserved_checked_output(output_path, reserved_path)
        .map_err(|error| error.for_spec(&VERIFIER_SPEC))
}

/// Refuses a one-build verifier candidate that aliases its checked path.
pub(crate) fn ensure_verifier_designated_candidate_output(
    output_path: &Path,
    checked_path: &Path,
) -> Result<()> {
    ensure_designated_candidate_output(output_path, checked_path)
        .map_err(|error| error.for_spec(&VERIFIER_SPEC))
}

fn normalized_destination(path: &Path) -> Result<PathBuf> {
    match fs::canonicalize(path) {
        Ok(path) => Ok(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let parent = path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."));
            let file_name = path.file_name().ok_or_else(|| {
                ProviderLowererComponentError::new(
                    ProviderLowererComponentErrorKind::InvalidPath,
                    path.display().to_string(),
                    Some("file-name".to_owned()),
                )
            })?;
            fs::canonicalize(parent)
                .map(|parent| parent.join(file_name))
                .map_err(|error| {
                    ProviderLowererComponentError::new(
                        ProviderLowererComponentErrorKind::OutputReadFailed,
                        path.display().to_string(),
                        Some("canonical-parent".to_owned()),
                    )
                    .with_detail(error.to_string())
                })
        }
        Err(error) => Err(ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::OutputReadFailed,
            path.display().to_string(),
            Some("canonical-path".to_owned()),
        )
        .with_detail(error.to_string())),
    }
}

/// Audits two explicit candidates and admits only one exact expected identity.
pub(crate) fn read_reproducible_candidates(
    first_path: &Path,
    second_path: &Path,
) -> Result<ProviderLowererComponent> {
    read_reproducible_candidates_for(&LOWERER_SPEC, first_path, second_path)
}

fn read_reproducible_candidates_for(
    spec: &ProviderComponentSpec,
    first_path: &Path,
    second_path: &Path,
) -> Result<ProviderLowererComponent> {
    let first = read_component_bytes(first_path)?;
    let second = read_component_bytes(second_path)?;
    let aliased = same_file::is_same_file(first_path, second_path).map_err(|error| {
        ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::ComponentReadFailed,
            first_path.display().to_string(),
            Some(second_path.display().to_string()),
        )
        .with_detail(error.to_string())
    })?;
    if aliased {
        return Err(ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::CandidateAliased,
            first_path.display().to_string(),
            Some(second_path.display().to_string()),
        ));
    }
    let approved_sha256 = require_approved_checked_digest(spec)?;
    ensure_reproducible_candidates_for(spec, &first, &second, approved_sha256)?;
    authenticated_component_for(spec, first)
}

/// Audits, authenticates, and intentionally writes two reproducible candidates.
pub(crate) fn promote_reproducible_candidates(
    first_path: &Path,
    second_path: &Path,
    output_path: &Path,
) -> Result<(ProviderLowererComponent, ComponentOutputStatus)> {
    let component = read_reproducible_candidates(first_path, second_path)?;
    let status = sync_output(output_path, component.bytes(), ComponentOutputMode::Write)?;
    Ok((component, status))
}

/// Fails closed until an exact verifier identity is approved, then promotes it.
pub(crate) fn promote_verifier_reproducible_candidates(
    first_path: &Path,
    second_path: &Path,
    output_path: &Path,
) -> Result<(ProviderLowererComponent, ComponentOutputStatus)> {
    require_verifier_checked_identity()?;
    let component = read_reproducible_candidates_for(&VERIFIER_SPEC, first_path, second_path)
        .map_err(|error| error.for_spec(&VERIFIER_SPEC))?;
    let status = sync_output(output_path, component.bytes(), ComponentOutputMode::Write)
        .map_err(|error| error.for_spec(&VERIFIER_SPEC))?;
    Ok((component, status))
}

/// Requires the verifier's checked identity to have been explicitly approved.
pub(crate) fn require_verifier_checked_identity() -> Result<&'static str> {
    require_approved_checked_digest(&VERIFIER_SPEC).map_err(|error| error.for_spec(&VERIFIER_SPEC))
}

/// Binds one already authenticated lowerer component to the approved identity.
pub(crate) fn require_lowerer_component_identity(
    component: &ProviderLowererComponent,
) -> Result<()> {
    require_component_identity_for(&LOWERER_SPEC, component)
        .map_err(|error| error.for_spec(&LOWERER_SPEC))
}

/// Binds one already authenticated verifier component to the approved identity.
pub(crate) fn require_verifier_component_identity(
    component: &ProviderLowererComponent,
) -> Result<()> {
    require_component_identity_for(&VERIFIER_SPEC, component)
        .map_err(|error| error.for_spec(&VERIFIER_SPEC))
}

fn require_component_identity_for(
    spec: &ProviderComponentSpec,
    component: &ProviderLowererComponent,
) -> Result<()> {
    let approved_sha256 = require_approved_checked_digest(spec)?;
    if approved_sha256.len() != 64
        || !approved_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::ExpectedDigestInvalid,
            approved_sha256,
            Some("lowercase-sha256".to_owned()),
        ));
    }

    let observed_sha256 = component.sha256_hex();
    if observed_sha256 != approved_sha256 {
        return Err(ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::CandidateDigestMismatch,
            approved_sha256,
            Some(observed_sha256),
        ));
    }
    Ok(())
}

fn require_approved_checked_digest(spec: &ProviderComponentSpec) -> Result<&'static str> {
    spec.approved_sha256.ok_or_else(|| {
        ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::CheckedIdentityUnapproved,
            spec.contract_coordinate,
            Some(spec.checked_repository_path.to_owned()),
        )
    })
}

#[cfg(test)]
fn ensure_reproducible_candidates(
    first: &[u8],
    second: &[u8],
    expected_sha256: &str,
) -> Result<()> {
    ensure_reproducible_candidates_for(&LOWERER_SPEC, first, second, expected_sha256)
}

fn ensure_reproducible_candidates_for(
    spec: &ProviderComponentSpec,
    first: &[u8],
    second: &[u8],
    expected_sha256: &str,
) -> Result<()> {
    if first != second {
        return Err(ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::CandidateMismatch,
            spec.contract_coordinate,
            Some(format!(
                "first:{};second:{}",
                digest_hex(&sha256(first)),
                digest_hex(&sha256(second))
            )),
        ));
    }
    if expected_sha256.len() != 64
        || !expected_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::ExpectedDigestInvalid,
            expected_sha256,
            Some("lowercase-sha256".to_owned()),
        ));
    }
    let approved_sha256 = require_approved_checked_digest(spec)?;
    if expected_sha256 != approved_sha256 {
        return Err(ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::CandidateDigestMismatch,
            approved_sha256,
            Some(expected_sha256.to_owned()),
        ));
    }
    let observed = digest_hex(&sha256(first));
    if observed != expected_sha256 {
        return Err(ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::CandidateDigestMismatch,
            expected_sha256,
            Some(observed),
        ));
    }
    Ok(())
}

/// Structural facts established by a specification-selected component audit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ComponentAudit {
    /// Number of exact top-level contract attestations.
    pub(crate) contract_attestations: u32,
    /// Number of exact type-only protocol imports (zero or one).
    pub(crate) protocol_imports: u32,
    /// Number of exact equality-bounded protocol type aliases (zero or two).
    pub(crate) protocol_type_imports: u32,
    /// Number of exact callable exports selected by the component specification.
    pub(crate) callable_exports: u32,
}

/// Whether an explicit output should be checked or updated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ComponentOutputMode {
    /// Report missing or stale bytes without changing the output.
    Check,
    /// Write exact bytes when the output is missing or stale.
    Write,
}

/// Outcome of checking or writing an explicit component output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ComponentOutputStatus {
    /// The explicit output already contained the exact bytes.
    Current,
    /// The explicit output was written with the exact bytes.
    Written,
}

/// Builds, componentizes, attests, audits, and hashes the provider lowerer.
///
/// Both paths are explicit inputs. This boundary performs no repository or
/// output discovery. Cargo is pinned to Rust 1.90.0, uses the lockfile, and
/// receives a caller-owned target directory.
pub(crate) fn build_component(
    repository_root: &Path,
    target_directory: &Path,
    toolchain: &PinnedRustToolchain,
) -> Result<ProviderLowererComponent> {
    build_component_for(&LOWERER_SPEC, repository_root, target_directory, toolchain)
}

/// Builds, componentizes, attests, and audits the provider verifier.
pub(crate) fn build_verifier_component(
    repository_root: &Path,
    target_directory: &Path,
    toolchain: &PinnedRustToolchain,
) -> Result<ProviderLowererComponent> {
    build_component_for(&VERIFIER_SPEC, repository_root, target_directory, toolchain)
        .map_err(|error| error.for_spec(&VERIFIER_SPEC))
}

fn build_component_for(
    spec: &ProviderComponentSpec,
    repository_root: &Path,
    target_directory: &Path,
    toolchain: &PinnedRustToolchain,
) -> Result<ProviderLowererComponent> {
    let target_directory = absolute_target_directory(repository_root, target_directory);
    let cargo_home = target_directory.join("cargo-home");
    let encoded_rustflags =
        encoded_build_rustflags(repository_root, &target_directory, &cargo_home)?;

    let mut command = Command::new(&toolchain.cargo);
    remove_ambient_cargo_build_overrides(&mut command, std::env::vars_os().map(|(name, _)| name));
    bind_pinned_toolchain(&mut command, toolchain);
    let output = command
        .args([
            "build",
            "-p",
            spec.package,
            "--target",
            "wasm32-unknown-unknown",
            "--release",
            "--locked",
        ])
        .current_dir(repository_root)
        .env("CARGO_HOME", &cargo_home)
        .env("CARGO_TARGET_DIR", &target_directory)
        .env("CARGO_INCREMENTAL", "0")
        .env("CARGO_ENCODED_RUSTFLAGS", encoded_rustflags)
        .env("SOURCE_DATE_EPOCH", "1")
        .env_remove("RUSTFLAGS")
        .env_remove("RUSTC_BOOTSTRAP")
        .output()
        .map_err(|error| {
            ProviderLowererComponentError::new(
                ProviderLowererComponentErrorKind::BuildInvocationFailed,
                spec.package,
                Some(toolchain.cargo.display().to_string()),
            )
            .with_detail(error.to_string())
        })?;

    if !output.status.success() {
        return Err(ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::BuildFailed,
            spec.package,
            Some("wasm32-unknown-unknown/release".to_owned()),
        )
        .with_detail(String::from_utf8_lossy(&output.stderr)));
    }

    let core_path = target_directory
        .join("wasm32-unknown-unknown")
        .join("release")
        .join(spec.core_wasm);
    let core_bytes = fs::read(&core_path).map_err(|error| {
        ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::CoreWasmReadFailed,
            core_path.display().to_string(),
            Some(spec.core_wasm.to_owned()),
        )
        .with_detail(error.to_string())
    })?;

    if spec == &LOWERER_SPEC {
        componentize(&core_bytes)
    } else {
        componentize_for(spec, &core_bytes)
    }
}

fn encoded_build_rustflags(
    repository_root: &Path,
    target_directory: &Path,
    cargo_home: &Path,
) -> Result<String> {
    let repository_root_text = path_text(repository_root)?;
    let target_directory_text = path_text(target_directory)?;
    let cargo_home_text = path_text(cargo_home)?;

    // rustc uses the last matching remap, so broad roots precede nested roots.
    Ok([
        format!("--remap-path-prefix={repository_root_text}=/echo"),
        format!("--remap-path-prefix={target_directory_text}=/target"),
        format!("--remap-path-prefix={cargo_home_text}=/cargo"),
    ]
    .join("\u{1f}"))
}

fn remove_ambient_cargo_build_overrides(
    command: &mut Command,
    names: impl IntoIterator<Item = OsString>,
) {
    for name in names {
        let Some(name_text) = name.to_str() else {
            continue;
        };
        if ["CARGO_PROFILE_", "CARGO_BUILD_", "CARGO_TARGET_"]
            .iter()
            .any(|prefix| name_text.starts_with(prefix))
        {
            command.env_remove(name);
        }
    }
}

fn bind_pinned_toolchain(command: &mut Command, toolchain: &PinnedRustToolchain) {
    command
        .env("RUSTC", &toolchain.rustc)
        .env("CARGO_BUILD_RUSTC", &toolchain.rustc)
        .env("RUSTC_WRAPPER", "")
        .env("RUSTC_WORKSPACE_WRAPPER", "")
        .env_remove("CARGO_BUILD_RUSTC_WRAPPER")
        .env_remove("CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER");
}

/// Componentizes explicit core Wasm bytes and appends the exact attestation.
pub(crate) fn componentize(core_bytes: &[u8]) -> Result<ProviderLowererComponent> {
    componentize_for(&LOWERER_SPEC, core_bytes)
}

fn componentize_for(
    spec: &ProviderComponentSpec,
    core_bytes: &[u8],
) -> Result<ProviderLowererComponent> {
    let encoder = ComponentEncoder::default()
        .validate(true)
        .merge_imports_based_on_semver(false)
        .module(core_bytes)
        .map_err(|error| {
            ProviderLowererComponentError::new(
                ProviderLowererComponentErrorKind::ComponentEncodingFailed,
                spec.package,
                Some("core-module".to_owned()),
            )
            .with_detail(error.to_string())
        })?;
    let mut encoder = encoder;
    let mut bytes = encoder.encode().map_err(|error| {
        ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::ComponentEncodingFailed,
            spec.package,
            Some("component".to_owned()),
        )
        .with_detail(error.to_string())
    })?;

    if spec == &LOWERER_SPEC {
        append_contract_attestation(&mut bytes);
    } else {
        append_contract_attestation_for(spec, &mut bytes);
    }
    authenticated_component_for(spec, bytes)
}

fn authenticated_component(bytes: Vec<u8>) -> Result<ProviderLowererComponent> {
    audit_component(&bytes)?;
    let sha256 = sha256(&bytes);
    Ok(ProviderLowererComponent { bytes, sha256 })
}

fn authenticated_verifier_component(bytes: Vec<u8>) -> Result<ProviderLowererComponent> {
    audit_verifier_component(&bytes)?;
    let sha256 = sha256(&bytes);
    Ok(ProviderLowererComponent { bytes, sha256 })
}

fn authenticated_component_for(
    spec: &ProviderComponentSpec,
    bytes: Vec<u8>,
) -> Result<ProviderLowererComponent> {
    audit_component_for(spec, &bytes)?;
    let sha256 = sha256(&bytes);
    Ok(ProviderLowererComponent { bytes, sha256 })
}

/// Audits exact attestation, import, export, and WebAssembly validity claims.
pub(crate) fn audit_component(bytes: &[u8]) -> Result<ComponentAudit> {
    audit_component_for(&LOWERER_SPEC, bytes)
}

/// Audits exact verifier attestation, imports, export, and WebAssembly validity.
pub(crate) fn audit_verifier_component(bytes: &[u8]) -> Result<ComponentAudit> {
    audit_component_for(&VERIFIER_SPEC, bytes).map_err(|error| error.for_spec(&VERIFIER_SPEC))
}

fn audit_component_for(spec: &ProviderComponentSpec, bytes: &[u8]) -> Result<ComponentAudit> {
    let mut depth = 0_u32;
    let mut outer_is_component = false;
    let mut contract_attestations = 0_u32;
    let mut protocol_imports = 0_u32;
    let mut protocol_type_imports = [false; 2];
    let mut callable_exports = 0_u32;
    let mut top_level_types = Vec::new();

    for payload in Parser::new(0).parse_all(bytes) {
        let payload = payload.map_err(|error| component_invalid_for(spec, error.to_string()))?;
        match payload {
            Payload::Version { encoding, .. } => {
                depth = depth.checked_add(1).ok_or_else(|| {
                    component_invalid_for(spec, "component nesting depth overflow".to_owned())
                })?;
                if depth == 1 {
                    outer_is_component = encoding == Encoding::Component;
                }
            }
            Payload::End(_) => {
                depth = depth.checked_sub(1).ok_or_else(|| {
                    component_invalid_for(spec, "component nesting depth underflow".to_owned())
                })?;
            }
            Payload::CustomSection(section) if section.name() == CONTRACT_SECTION_NAME => {
                if depth != 1 {
                    return Err(ProviderLowererComponentError::new(
                        ProviderLowererComponentErrorKind::ContractAttestationNested,
                        CONTRACT_SECTION_NAME,
                        Some(format!("depth:{depth}")),
                    ));
                }
                contract_attestations += 1;
                if contract_attestations > 1 {
                    return Err(ProviderLowererComponentError::new(
                        ProviderLowererComponentErrorKind::ContractAttestationDuplicate,
                        CONTRACT_SECTION_NAME,
                        Some(spec.contract_coordinate.to_owned()),
                    ));
                }
                if section.data() != spec.contract_coordinate.as_bytes() {
                    return Err(ProviderLowererComponentError::new(
                        ProviderLowererComponentErrorKind::ContractAttestationMismatch,
                        CONTRACT_SECTION_NAME,
                        Some(spec.contract_coordinate.to_owned()),
                    ));
                }
            }
            Payload::ImportSection(section) => {
                let import = section
                    .into_imports()
                    .next()
                    .transpose()
                    .map_err(|error| component_invalid_for(spec, error.to_string()))?;
                if let Some(import) = import {
                    return Err(ProviderLowererComponentError::new(
                        ProviderLowererComponentErrorKind::CoreImportForbidden,
                        "core-import",
                        Some(format!("{}::{}", import.module, import.name)),
                    ));
                }
            }
            Payload::ComponentTypeSection(section) if depth == 1 => {
                for component_type in section {
                    let component_type = component_type
                        .map_err(|error| component_invalid_for(spec, error.to_string()))?;
                    top_level_types.push(type_only_instance(&component_type));
                }
            }
            Payload::ComponentImportSection(section) => {
                for import in section {
                    let import =
                        import.map_err(|error| component_invalid_for(spec, error.to_string()))?;
                    if depth != 1 || import.name.implements.is_some() {
                        return Err(ProviderLowererComponentError::new(
                            ProviderLowererComponentErrorKind::ComponentImportForbidden,
                            import.name.name,
                            Some(format!("{};depth:{depth}", import.ty.kind().desc())),
                        ));
                    }

                    if import.name.name == PROTOCOL_INSTANCE_COORDINATE {
                        protocol_imports += 1;
                        if protocol_imports > 1 {
                            return Err(ProviderLowererComponentError::new(
                                ProviderLowererComponentErrorKind::ProtocolImportDuplicate,
                                PROTOCOL_INSTANCE_COORDINATE,
                                None,
                            ));
                        }

                        let ComponentTypeRef::Instance(type_index) = import.ty else {
                            return Err(ProviderLowererComponentError::new(
                                ProviderLowererComponentErrorKind::ProtocolImportInvalid,
                                PROTOCOL_INSTANCE_COORDINATE,
                                Some(import.ty.kind().desc().to_owned()),
                            ));
                        };
                        if top_level_types.get(type_index as usize) != Some(&true) {
                            return Err(ProviderLowererComponentError::new(
                                ProviderLowererComponentErrorKind::ProtocolImportInvalid,
                                PROTOCOL_INSTANCE_COORDINATE,
                                Some(format!("type-index:{type_index}")),
                            ));
                        }
                        continue;
                    }

                    let Some(alias_index) = spec
                        .protocol_type_imports
                        .iter()
                        .position(|name| *name == import.name.name)
                    else {
                        return Err(ProviderLowererComponentError::new(
                            ProviderLowererComponentErrorKind::ComponentImportForbidden,
                            import.name.name,
                            Some(import.ty.kind().desc().to_owned()),
                        ));
                    };
                    if protocol_type_imports[alias_index] {
                        return Err(ProviderLowererComponentError::new(
                            ProviderLowererComponentErrorKind::ProtocolImportDuplicate,
                            import.name.name,
                            Some("type-alias".to_owned()),
                        ));
                    }
                    if !matches!(import.ty, ComponentTypeRef::Type(TypeBounds::Eq(_))) {
                        return Err(ProviderLowererComponentError::new(
                            ProviderLowererComponentErrorKind::ProtocolImportInvalid,
                            import.name.name,
                            Some(import.ty.kind().desc().to_owned()),
                        ));
                    }
                    protocol_type_imports[alias_index] = true;
                }
            }
            Payload::ComponentExportSection(section) if depth == 1 => {
                for export in section {
                    let export =
                        export.map_err(|error| component_invalid_for(spec, error.to_string()))?;
                    if export.name.name != spec.callable_export
                        || export.name.implements.is_some()
                        || export.kind != ComponentExternalKind::Func
                    {
                        return Err(ProviderLowererComponentError::new(
                            export_invalid_kind(spec),
                            export.name.name,
                            Some(export.kind.desc().to_owned()),
                        ));
                    }
                    callable_exports += 1;
                    if callable_exports > 1 {
                        return Err(ProviderLowererComponentError::new(
                            export_duplicate_kind(spec),
                            spec.callable_export,
                            None,
                        ));
                    }
                }
            }
            _ => {}
        }
    }

    if !outer_is_component {
        return Err(ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::ComponentEncodingInvalid,
            spec.package,
            Some("component-model".to_owned()),
        ));
    }
    if contract_attestations != 1 {
        return Err(ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::ContractAttestationMissing,
            CONTRACT_SECTION_NAME,
            Some(spec.contract_coordinate.to_owned()),
        ));
    }
    let protocol_type_import_count = protocol_type_imports
        .iter()
        .fold(0_u32, |count, present| count + u32::from(*present));
    if !matches!(
        (protocol_imports, protocol_type_import_count),
        (0, 0) | (1, 2)
    ) {
        return Err(ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::ProtocolImportClosureInvalid,
            PROTOCOL_INSTANCE_COORDINATE,
            Some(format!(
                "instances:{protocol_imports};type-aliases:{protocol_type_import_count}"
            )),
        ));
    }
    if callable_exports != 1 {
        return Err(ProviderLowererComponentError::new(
            export_missing_kind(spec),
            spec.callable_export,
            None,
        ));
    }

    Validator::new_with_features(WasmFeatures::all())
        .validate_all(bytes)
        .map_err(|error| component_invalid_for(spec, error.to_string()))?;
    authenticate_component_world_for(spec, bytes)?;

    Ok(ComponentAudit {
        contract_attestations,
        protocol_imports,
        protocol_type_imports: protocol_type_import_count,
        callable_exports,
    })
}

/// Checks or writes exact component bytes at a caller-selected output path.
pub(crate) fn sync_output(
    output_path: &Path,
    bytes: &[u8],
    mode: ComponentOutputMode,
) -> Result<ComponentOutputStatus> {
    validate_output_type(output_path)?;
    match fs::read(output_path) {
        Ok(existing) if existing == bytes => Ok(ComponentOutputStatus::Current),
        Ok(existing) if mode == ComponentOutputMode::Check => {
            Err(ProviderLowererComponentError::new(
                ProviderLowererComponentErrorKind::OutputDrift,
                output_path.display().to_string(),
                Some(format!(
                    "expected:{};observed:{}",
                    digest_hex(&sha256(bytes)),
                    digest_hex(&sha256(&existing))
                )),
            ))
        }
        Ok(_) => write_output(output_path, bytes),
        Err(error)
            if error.kind() == std::io::ErrorKind::NotFound
                && mode == ComponentOutputMode::Check =>
        {
            Err(ProviderLowererComponentError::new(
                ProviderLowererComponentErrorKind::OutputMissing,
                output_path.display().to_string(),
                None,
            ))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            write_output(output_path, bytes)
        }
        Err(error) => Err(ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::OutputReadFailed,
            output_path.display().to_string(),
            None,
        )
        .with_detail(error.to_string())),
    }
}

/// Checks or writes explicit verifier component bytes without path discovery.
pub(crate) fn sync_verifier_output(
    output_path: &Path,
    bytes: &[u8],
    mode: ComponentOutputMode,
) -> Result<ComponentOutputStatus> {
    sync_output(output_path, bytes, mode).map_err(|error| error.for_spec(&VERIFIER_SPEC))
}

fn write_output(output_path: &Path, bytes: &[u8]) -> Result<ComponentOutputStatus> {
    write_output_with(output_path, |temporary| temporary.write_all(bytes))
}

fn write_output_with(
    output_path: &Path,
    writer: impl FnOnce(&mut File) -> std::io::Result<()>,
) -> Result<ComponentOutputStatus> {
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|error| {
                ProviderLowererComponentError::new(
                    ProviderLowererComponentErrorKind::OutputWriteFailed,
                    output_path.display().to_string(),
                    Some("create-parent".to_owned()),
                )
                .with_detail(error.to_string())
            })?;
        }
    }

    let (temporary_path, mut temporary) = create_temporary_sibling(output_path)?;
    if let Err(error) = writer(&mut temporary).and_then(|()| temporary.sync_all()) {
        drop(temporary);
        let _ = fs::remove_file(&temporary_path);
        return Err(ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::OutputWriteFailed,
            output_path.display().to_string(),
            Some("write-temporary".to_owned()),
        )
        .with_detail(error.to_string()));
    }
    drop(temporary);

    fs::rename(&temporary_path, output_path).map_err(|error| {
        let _ = fs::remove_file(&temporary_path);
        ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::OutputWriteFailed,
            output_path.display().to_string(),
            Some("atomic-replace".to_owned()),
        )
        .with_detail(error.to_string())
    })?;
    Ok(ComponentOutputStatus::Written)
}

fn validate_output_type(output_path: &Path) -> Result<()> {
    match fs::symlink_metadata(output_path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(()),
        Ok(metadata) => {
            let kind = if metadata.file_type().is_symlink() {
                "symlink"
            } else if metadata.file_type().is_dir() {
                "directory"
            } else {
                "non-regular"
            };
            Err(ProviderLowererComponentError::new(
                ProviderLowererComponentErrorKind::OutputTypeInvalid,
                output_path.display().to_string(),
                Some(kind.to_owned()),
            ))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::OutputReadFailed,
            output_path.display().to_string(),
            Some("metadata".to_owned()),
        )
        .with_detail(error.to_string())),
    }
}

fn create_temporary_sibling(output_path: &Path) -> Result<(PathBuf, File)> {
    let parent = output_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = output_path.file_name().ok_or_else(|| {
        ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::InvalidPath,
            output_path.display().to_string(),
            Some("file-name".to_owned()),
        )
    })?;

    for nonce in 0_u16..128 {
        let mut temporary_name = OsString::from(".");
        temporary_name.push(file_name);
        temporary_name.push(format!(".tmp-{}-{nonce}", std::process::id()));
        let temporary_path = parent.join(temporary_name);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
        {
            Ok(file) => return Ok((temporary_path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(ProviderLowererComponentError::new(
                    ProviderLowererComponentErrorKind::OutputWriteFailed,
                    output_path.display().to_string(),
                    Some("create-temporary".to_owned()),
                )
                .with_detail(error.to_string()));
            }
        }
    }

    Err(ProviderLowererComponentError::new(
        ProviderLowererComponentErrorKind::OutputWriteFailed,
        output_path.display().to_string(),
        Some("temporary-name-exhausted".to_owned()),
    ))
}

fn append_contract_attestation(bytes: &mut Vec<u8>) {
    append_contract_attestation_for(&LOWERER_SPEC, bytes);
}

fn append_contract_attestation_for(spec: &ProviderComponentSpec, bytes: &mut Vec<u8>) {
    CustomSection {
        name: Cow::Borrowed(CONTRACT_SECTION_NAME),
        data: Cow::Borrowed(spec.contract_coordinate.as_bytes()),
    }
    .append_to_component(bytes);
}

fn type_only_instance(component_type: &ComponentType<'_>) -> bool {
    let ComponentType::Instance(declarations) = component_type else {
        return false;
    };
    let mut export_count = 0_u32;
    for declaration in declarations {
        if let InstanceTypeDeclaration::Export { ty, .. } = declaration {
            export_count += 1;
            if !matches!(ty, ComponentTypeRef::Type(_)) {
                return false;
            }
        }
    }
    export_count > 0
}

fn authenticate_component_world_for(spec: &ProviderComponentSpec, bytes: &[u8]) -> Result<()> {
    let (mut expected_resolve, expected_world) =
        parse_component_world(spec, spec.wit_source, "frozen-wit")?;
    let expected_package = expected_resolve.worlds[expected_world]
        .package
        .map(|package| expected_resolve.packages[package].name.clone())
        .ok_or_else(|| {
            world_contract_invalid_for(spec, "frozen-wit", "selected world has no package")
        })?;
    canonicalize_world_type_closure(&mut expected_resolve, expected_world);
    let expected = encode_world_contract(spec, &expected_resolve, expected_world, "frozen-wit")?;

    let decoded = wit_component::decode(bytes).map_err(|error| {
        world_contract_invalid_for(spec, "candidate-component", "component WIT decode failed")
            .with_detail(error.to_string())
    })?;
    let DecodedWasm::Component(mut observed_resolve, observed_world) = decoded else {
        return Err(world_contract_invalid_for(
            spec,
            "candidate-component",
            "decoded bytes are a WIT package rather than a component",
        ));
    };
    normalize_decoded_world_identity(
        spec,
        &mut observed_resolve,
        observed_world,
        expected_package,
    )?;
    canonicalize_world_type_closure(&mut observed_resolve, observed_world);
    let observed = encode_world_contract(
        spec,
        &observed_resolve,
        observed_world,
        "candidate-component",
    )?;
    compare_world_contracts(spec, &expected, &observed)
}

#[cfg(test)]
fn authenticate_wit_source_for_test(source: &str) -> Result<()> {
    authenticate_wit_source_for_spec(&LOWERER_SPEC, source)
}

#[cfg(test)]
fn authenticate_wit_source_for_spec(spec: &ProviderComponentSpec, source: &str) -> Result<()> {
    let (mut expected_resolve, expected_world) =
        parse_component_world(spec, spec.wit_source, "frozen-wit")?;
    canonicalize_world_type_closure(&mut expected_resolve, expected_world);
    let expected = encode_world_contract(spec, &expected_resolve, expected_world, "frozen-wit")?;
    let (mut observed_resolve, observed_world) =
        parse_component_world(spec, source, "candidate-wit")?;
    canonicalize_world_type_closure(&mut observed_resolve, observed_world);
    let observed = encode_world_contract(spec, &observed_resolve, observed_world, "candidate-wit")?;
    compare_world_contracts(spec, &expected, &observed)
}

fn parse_component_world(
    spec: &ProviderComponentSpec,
    source: &str,
    reference: &str,
) -> Result<(Resolve, WorldId)> {
    let mut resolve = Resolve::default();
    let package = resolve
        .push_str("edict-target-provider.wit", source)
        .map_err(|error| {
            world_contract_invalid_for(spec, reference, "WIT parse failed")
                .with_detail(error.to_string())
        })?;
    let world = resolve
        .select_world(&[package], Some(spec.world_name))
        .map_err(|error| {
            world_contract_invalid_for(spec, reference, "component world selection failed")
                .with_detail(error.to_string())
        })?;
    Ok((resolve, world))
}

fn normalize_decoded_world_identity(
    spec: &ProviderComponentSpec,
    resolve: &mut Resolve,
    world: WorldId,
    package_name: PackageName,
) -> Result<()> {
    let package = resolve.worlds[world].package.ok_or_else(|| {
        world_contract_invalid_for(spec, "candidate-component", "decoded world has no package")
    })?;
    let previous_world_name =
        std::mem::replace(&mut resolve.worlds[world].name, spec.world_name.to_owned());
    let package_worlds = &mut resolve.packages[package].worlds;
    if package_worlds.get(&previous_world_name) == Some(&world) {
        package_worlds.shift_remove(&previous_world_name);
        package_worlds.insert(spec.world_name.to_owned(), world);
    }
    resolve.packages[package].name = package_name;
    Ok(())
}

fn canonicalize_world_type_closure(resolve: &mut Resolve, world: WorldId) {
    let live_types = {
        let mut live = LiveTypes::default();
        for item in resolve.worlds[world]
            .imports
            .values()
            .chain(resolve.worlds[world].exports.values())
        {
            match item {
                WorldItem::Function(function) => live.add_func(resolve, function),
                WorldItem::Type { id, .. } => live.add_type_id(resolve, *id),
                WorldItem::Interface { id, .. } => {
                    for function in resolve.interfaces[*id].functions.values() {
                        live.add_func(resolve, function);
                    }
                }
            }
        }
        live.iter().collect::<Vec<_>>()
    };

    for (_, interface) in &mut resolve.interfaces {
        let declared_types = std::mem::take(&mut interface.types);
        for live_type in &live_types {
            if let Some((name, id)) = declared_types
                .iter()
                .find(|(_, declared_type)| *declared_type == live_type)
            {
                interface.types.insert(name.clone(), *id);
            }
        }
    }
}

fn encode_world_contract(
    spec: &ProviderComponentSpec,
    resolve: &Resolve,
    world: WorldId,
    reference: &str,
) -> Result<Vec<u8>> {
    wit_component::metadata::encode(resolve, world, StringEncoding::UTF8, None).map_err(|error| {
        world_contract_invalid_for(spec, reference, "canonical world encoding failed")
            .with_detail(error.to_string())
    })
}

fn compare_world_contracts(
    spec: &ProviderComponentSpec,
    expected: &[u8],
    observed: &[u8],
) -> Result<()> {
    if expected == observed {
        return Ok(());
    }
    Err(ProviderLowererComponentError::new(
        ProviderLowererComponentErrorKind::WorldContractMismatch,
        spec.contract_coordinate,
        Some(format!(
            "expected:{};observed:{}",
            digest_hex(&sha256(expected)),
            digest_hex(&sha256(observed))
        )),
    ))
}

fn world_contract_invalid_for(
    spec: &ProviderComponentSpec,
    reference: impl Into<String>,
    detail: impl Into<String>,
) -> ProviderLowererComponentError {
    ProviderLowererComponentError::new(
        ProviderLowererComponentErrorKind::WorldContractInvalid,
        spec.contract_coordinate,
        Some(reference.into()),
    )
    .with_detail(detail)
}

fn absolute_target_directory(repository_root: &Path, target_directory: &Path) -> PathBuf {
    if target_directory.is_absolute() {
        target_directory.to_owned()
    } else {
        repository_root.join(target_directory)
    }
}

fn path_text(path: &Path) -> Result<&str> {
    path.to_str().ok_or_else(|| {
        ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::InvalidPath,
            path.display().to_string(),
            Some("utf-8".to_owned()),
        )
    })
}

fn component_invalid_for(
    spec: &ProviderComponentSpec,
    detail: String,
) -> ProviderLowererComponentError {
    ProviderLowererComponentError::new(
        ProviderLowererComponentErrorKind::ComponentInvalid,
        spec.package,
        Some("wasmparser".to_owned()),
    )
    .with_detail(detail)
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn digest_hex(digest: &[u8; 32]) -> String {
    hex::encode(digest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use wasm_encoder::Component;

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    fn require_component_error<T>(
        result: Result<T>,
        message: &str,
    ) -> std::result::Result<ProviderLowererComponentError, Box<dyn std::error::Error>> {
        match result {
            Err(error) => Ok(error),
            Ok(_) => Err(std::io::Error::other(message).into()),
        }
    }

    #[test]
    fn provider_verifier_spec_is_exact_and_approved() {
        assert_eq!(VERIFIER_SPEC.package, "echo-edict-provider-verifier");
        assert_eq!(VERIFIER_SPEC.core_wasm, "echo_edict_provider_verifier.wasm");
        assert_eq!(VERIFIER_SPEC.world_name, "verifier");
        assert_eq!(
            VERIFIER_SPEC.contract_coordinate,
            "edict:target-provider/verifier@1.0.0"
        );
        assert_eq!(
            VERIFIER_SPEC.protocol_type_imports,
            ["verification-request-v1", "verification-result-v1"]
        );
        assert_eq!(VERIFIER_SPEC.callable_export, "verify");
        assert_eq!(
            VERIFIER_SPEC.checked_repository_path,
            "schemas/edict-provider/components/v1/verifier.echo-dpo.component.wasm"
        );
        assert_eq!(
            VERIFIER_SPEC.approved_sha256,
            Some(APPROVED_CHECKED_VERIFIER_COMPONENT_SHA256)
        );
        assert_eq!(
            sha256(VERIFIER_SPEC.wit_source.as_bytes()),
            sha256(LOWERER_SPEC.wit_source.as_bytes())
        );
    }

    #[test]
    fn provider_checked_identities_are_exact() -> std::result::Result<(), Box<dyn std::error::Error>>
    {
        let lowerer = authenticated_component_for(
            &LOWERER_SPEC,
            include_bytes!(
                "../../schemas/edict-provider/components/v1/lowerer.echo-dpo.component.wasm"
            )
            .to_vec(),
        )?;
        require_lowerer_component_identity(&lowerer)?;
        assert_eq!(
            require_approved_checked_digest(&VERIFIER_SPEC)?,
            APPROVED_CHECKED_VERIFIER_COMPONENT_SHA256
        );
        assert_eq!(
            require_verifier_checked_identity()?,
            APPROVED_CHECKED_VERIFIER_COMPONENT_SHA256
        );
        Ok(())
    }

    #[test]
    fn authenticated_verifier_requires_a_well_formed_approved_digest(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let component = authenticated_component_for(
            &VERIFIER_SPEC,
            include_bytes!(
                "../../schemas/edict-provider/components/v1/verifier.echo-dpo.component.wasm"
            )
            .to_vec(),
        )?;

        let stale_spec = ProviderComponentSpec {
            approved_sha256: Some(
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            ..VERIFIER_SPEC
        };
        let stale = require_component_error(
            require_component_identity_for(&stale_spec, &component),
            "a stale approved verifier digest must reject the authenticated component",
        )?;
        assert_eq!(
            stale.kind(),
            ProviderLowererComponentErrorKind::CandidateDigestMismatch
        );

        let malformed_spec = ProviderComponentSpec {
            approved_sha256: Some("not-a-lowercase-sha256"),
            ..VERIFIER_SPEC
        };
        let malformed = require_component_error(
            require_component_identity_for(&malformed_spec, &component),
            "a malformed approved verifier digest must reject the authenticated component",
        )?;
        assert_eq!(
            malformed.kind(),
            ProviderLowererComponentErrorKind::ExpectedDigestInvalid
        );
        Ok(())
    }

    #[test]
    fn wrong_verifier_signature_cannot_receive_the_frozen_attestation(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let source = VERIFIER_SPEC.wit_source.replacen(
            "export verify: func(request: verification-request-v1) -> verification-result-v1;",
            "export verify: func(request: verification-result-v1) -> verification-request-v1;",
            1,
        );

        let error = require_component_error(
            authenticate_wit_source_for_spec(&VERIFIER_SPEC, &source),
            "a wrong verifier signature must fail world authentication",
        )?;
        assert_eq!(
            error.kind(),
            ProviderLowererComponentErrorKind::WorldContractMismatch
        );
        assert_eq!(error.subject(), VERIFIER_SPEC.contract_coordinate);
        Ok(())
    }

    #[test]
    fn lowerer_public_identity_remains_the_merged_spec() {
        assert_eq!(
            LOWERER_SPEC.contract_coordinate,
            LOWERER_CONTRACT_COORDINATE
        );
        assert_eq!(
            LOWERER_SPEC.approved_sha256,
            Some(APPROVED_CHECKED_COMPONENT_SHA256)
        );
        assert_eq!(
            LOWERER_SPEC.checked_repository_path,
            CHECKED_COMPONENT_REPOSITORY_PATH
        );
        assert_eq!(LOWERER_SPEC.callable_export, "lower");
    }

    #[test]
    fn verifier_spec_drives_exact_attestation_and_export_admission(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut bytes = Component::new().finish();
        append_contract_attestation_for(&VERIFIER_SPEC, &mut bytes);

        let mut sections = Vec::new();
        for payload in Parser::new(0).parse_all(&bytes) {
            match payload? {
                Payload::CustomSection(section) if section.name() == CONTRACT_SECTION_NAME => {
                    sections.push(section.data().to_vec());
                }
                _ => {}
            }
        }
        assert_eq!(sections, [VERIFIER_CONTRACT_COORDINATE.as_bytes()]);

        let lowerer_error = require_component_error(
            audit_component(&bytes),
            "the verifier attestation must not authenticate as a lowerer",
        )?;
        assert_eq!(
            lowerer_error.kind(),
            ProviderLowererComponentErrorKind::ContractAttestationMismatch
        );
        assert_eq!(lowerer_error.subject(), CONTRACT_SECTION_NAME);
        assert_eq!(lowerer_error.reference(), Some(LOWERER_CONTRACT_COORDINATE));

        let verifier_error = require_component_error(
            audit_verifier_component(&bytes),
            "a verifier without its callable export must fail",
        )?;
        assert_eq!(
            verifier_error.kind(),
            ProviderLowererComponentErrorKind::VerifyExportMissing
        );
        assert_eq!(verifier_error.subject(), VERIFY_EXPORT);
        assert_eq!(verifier_error.reference(), None);
        Ok(())
    }

    #[test]
    fn verifier_world_rejects_world_export_and_alias_drift(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        authenticate_wit_source_for_spec(&VERIFIER_SPEC, VERIFIER_SPEC.wit_source)?;

        let renamed_world =
            VERIFIER_SPEC
                .wit_source
                .replacen("world verifier {", "world verifier-drift {", 1);
        let error = require_component_error(
            authenticate_wit_source_for_spec(&VERIFIER_SPEC, &renamed_world),
            "a renamed verifier world must fail closed",
        )?;
        assert_eq!(
            error.kind(),
            ProviderLowererComponentErrorKind::WorldContractInvalid
        );
        assert_eq!(error.subject(), VERIFIER_CONTRACT_COORDINATE);

        let renamed_export = VERIFIER_SPEC.wit_source.replacen(
            "  export verify: func(",
            "  export verify-drift: func(",
            1,
        );
        let error = require_component_error(
            authenticate_wit_source_for_spec(&VERIFIER_SPEC, &renamed_export),
            "a renamed verifier export must fail world equality",
        )?;
        assert_eq!(
            error.kind(),
            ProviderLowererComponentErrorKind::WorldContractMismatch
        );
        assert_eq!(error.subject(), VERIFIER_CONTRACT_COORDINATE);

        let lowerer_alias_closure = VERIFIER_SPEC.wit_source.replacen(
            concat!(
                "  use protocol.{verification-request-v1, verification-result-v1};\n\n",
                "  export verify: func(request: verification-request-v1) -> verification-result-v1;",
            ),
            concat!(
                "  use protocol.{lowering-request-v1, lowering-result-v1};\n\n",
                "  export verify: func(request: lowering-request-v1) -> lowering-result-v1;",
            ),
            1,
        );
        let error = require_component_error(
            authenticate_wit_source_for_spec(&VERIFIER_SPEC, &lowerer_alias_closure),
            "the lowerer alias closure must not authenticate as the verifier",
        )?;
        assert_eq!(
            error.kind(),
            ProviderLowererComponentErrorKind::WorldContractMismatch
        );
        assert_eq!(error.subject(), VERIFIER_CONTRACT_COORDINATE);
        Ok(())
    }

    #[test]
    fn checked_verifier_component_promotion_is_exact_and_idempotent(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let bytes = include_bytes!(
            "../../schemas/edict-provider/components/v1/verifier.echo-dpo.component.wasm"
        );
        assert_eq!(
            digest_hex(&sha256(bytes)),
            APPROVED_CHECKED_VERIFIER_COMPONENT_SHA256
        );
        let first = temporary_output("verifier-promotion-valid-a");
        let second = temporary_output("verifier-promotion-valid-b");
        let output = temporary_output("verifier-promotion-output");
        fs::write(&first, bytes)?;
        fs::write(&second, bytes)?;

        let (component, status) =
            promote_verifier_reproducible_candidates(&first, &second, &output)?;
        assert_eq!(status, ComponentOutputStatus::Written);
        assert_eq!(component.bytes(), bytes);
        let (_, status) = promote_verifier_reproducible_candidates(&first, &second, &output)?;
        assert_eq!(status, ComponentOutputStatus::Current);
        assert_eq!(fs::read(&output)?, bytes);

        fs::remove_file(first)?;
        fs::remove_file(second)?;
        fs::remove_file(output)?;
        Ok(())
    }

    #[test]
    fn lowerer_componentize_wrapper_preserves_its_error_surface(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let error = require_component_error(
            componentize(b"not-core-wasm"),
            "invalid core Wasm must still fail componentization",
        )?;
        assert_eq!(
            error.kind(),
            ProviderLowererComponentErrorKind::ComponentEncodingFailed
        );
        assert_eq!(error.subject(), LOWERER_PACKAGE);
        Ok(())
    }

    #[test]
    fn appends_one_exact_top_level_attestation(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut bytes = Component::new().finish();
        append_contract_attestation(&mut bytes);

        let mut sections = Vec::new();
        for payload in Parser::new(0).parse_all(&bytes) {
            match payload? {
                Payload::CustomSection(section) if section.name() == CONTRACT_SECTION_NAME => {
                    sections.push(section.data().to_vec());
                }
                _ => {}
            }
        }

        assert_eq!(sections, [LOWERER_CONTRACT_COORDINATE.as_bytes()]);
        Ok(())
    }

    #[test]
    fn rejects_duplicate_contract_attestations(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut bytes = Component::new().finish();
        append_contract_attestation(&mut bytes);
        append_contract_attestation(&mut bytes);

        let error = match audit_component(&bytes) {
            Err(error) => error,
            Ok(_) => {
                return Err(std::io::Error::other("duplicate attestation must fail").into());
            }
        };

        assert_eq!(
            error.kind(),
            ProviderLowererComponentErrorKind::ContractAttestationDuplicate
        );
        assert_eq!(error.subject(), CONTRACT_SECTION_NAME);
        assert_eq!(error.reference(), Some(LOWERER_CONTRACT_COORDINATE));
        Ok(())
    }

    #[test]
    fn rejects_mismatched_contract_attestation_bytes(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut bytes = Component::new().finish();
        CustomSection {
            name: Cow::Borrowed(CONTRACT_SECTION_NAME),
            data: Cow::Borrowed(b"edict:target-provider/lowerer@9.9.9"),
        }
        .append_to_component(&mut bytes);

        let error = match audit_component(&bytes) {
            Err(error) => error,
            Ok(_) => return Err(std::io::Error::other("wrong attestation must fail").into()),
        };

        assert_eq!(
            error.kind(),
            ProviderLowererComponentErrorKind::ContractAttestationMismatch
        );
        Ok(())
    }

    #[test]
    fn zero_import_topology_requires_only_the_lower_export(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut bytes = Component::new().finish();
        append_contract_attestation(&mut bytes);

        let error = match audit_component(&bytes) {
            Err(error) => error,
            Ok(_) => {
                return Err(
                    std::io::Error::other("component without lower export must fail").into(),
                );
            }
        };

        assert_eq!(
            error.kind(),
            ProviderLowererComponentErrorKind::LowerExportMissing
        );
        Ok(())
    }

    #[test]
    fn wrong_lower_parameter_and_result_cannot_receive_the_frozen_attestation(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let source =
            include_str!("../../crates/echo-edict-provider-lowerer/wit/edict-target-provider.wit")
                .replacen(
                    "export lower: func(request: lowering-request-v1) -> lowering-result-v1;",
                    "export lower: func(request: lowering-result-v1) -> lowering-request-v1;",
                    1,
                );

        let error = match authenticate_wit_source_for_test(&source) {
            Err(error) => error,
            Ok(()) => {
                return Err(std::io::Error::other(
                    "wrong lower signature received the frozen attestation",
                )
                .into());
            }
        };

        assert_eq!(
            error.kind(),
            ProviderLowererComponentErrorKind::WorldContractMismatch
        );
        assert_eq!(error.subject(), LOWERER_CONTRACT_COORDINATE);
        Ok(())
    }

    #[test]
    fn reachable_alias_graph_change_cannot_receive_the_frozen_attestation(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let source =
            include_str!("../../crates/echo-edict-provider-lowerer/wit/edict-target-provider.wit")
                .replacen(
                    "    core: bound-artifact,\n    target-profile: bound-artifact,",
                    "    core: string,\n    target-profile: bound-artifact,",
                    1,
                );

        let error = match authenticate_wit_source_for_test(&source) {
            Err(error) => error,
            Ok(()) => {
                return Err(std::io::Error::other(
                    "unrelated alias graph received the frozen attestation",
                )
                .into());
            }
        };

        assert_eq!(
            error.kind(),
            ProviderLowererComponentErrorKind::WorldContractMismatch
        );
        assert_eq!(error.subject(), LOWERER_CONTRACT_COORDINATE);
        Ok(())
    }

    #[test]
    fn check_reports_drift_without_rewriting() -> std::result::Result<(), Box<dyn std::error::Error>>
    {
        let output = temporary_output("check-drift");
        fs::write(&output, b"old")?;

        let error = match sync_output(&output, b"new", ComponentOutputMode::Check) {
            Err(error) => error,
            Ok(_) => return Err(std::io::Error::other("check must report drift").into()),
        };

        assert_eq!(error.kind(), ProviderLowererComponentErrorKind::OutputDrift);
        assert_eq!(fs::read(&output)?, b"old");
        fs::remove_file(output)?;
        Ok(())
    }

    #[test]
    fn write_is_exact_and_idempotent() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let output = temporary_output("write");

        assert_eq!(
            sync_output(&output, b"component", ComponentOutputMode::Write)?,
            ComponentOutputStatus::Written
        );
        assert_eq!(
            sync_output(&output, b"component", ComponentOutputMode::Write)?,
            ComponentOutputStatus::Current
        );
        assert_eq!(fs::read(&output)?, b"component");
        fs::remove_file(output)?;
        Ok(())
    }

    #[test]
    fn failed_atomic_replacement_preserves_the_previous_output(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let output = temporary_output("atomic-write-failure");
        fs::write(&output, b"preserve-me")?;

        let error = match write_output_with(&output, |temporary| {
            temporary.write_all(b"partial")?;
            Err(std::io::Error::other("injected write failure"))
        }) {
            Err(error) => error,
            Ok(_) => return Err(std::io::Error::other("partial output was installed").into()),
        };

        assert_eq!(
            error.kind(),
            ProviderLowererComponentErrorKind::OutputWriteFailed
        );
        assert_eq!(fs::read(&output)?, b"preserve-me");
        fs::remove_file(output)?;
        Ok(())
    }

    #[test]
    fn non_regular_output_paths_fail_closed() -> std::result::Result<(), Box<dyn std::error::Error>>
    {
        let output = temporary_output("non-regular");
        fs::create_dir(&output)?;

        let error = match sync_output(&output, b"component", ComponentOutputMode::Write) {
            Err(error) => error,
            Ok(_) => return Err(std::io::Error::other("directory output was accepted").into()),
        };
        assert_eq!(
            error.kind(),
            ProviderLowererComponentErrorKind::OutputTypeInvalid
        );
        fs::remove_dir(output)?;
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn symlink_outputs_fail_without_mutating_the_link_target(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::symlink;

        let target = temporary_output("symlink-target");
        let output = temporary_output("symlink-output");
        fs::write(&target, b"preserve-target")?;
        symlink(&target, &output)?;

        let error = match sync_output(&output, b"component", ComponentOutputMode::Write) {
            Err(error) => error,
            Ok(_) => return Err(std::io::Error::other("symlink output was accepted").into()),
        };
        assert_eq!(
            error.kind(),
            ProviderLowererComponentErrorKind::OutputTypeInvalid
        );
        assert_eq!(error.reference(), Some("symlink"));
        assert_eq!(fs::read(&target)?, b"preserve-target");
        assert!(fs::symlink_metadata(&output)?.file_type().is_symlink());
        fs::remove_file(output)?;
        fs::remove_file(target)?;
        Ok(())
    }

    #[test]
    fn provider_component_build_flags_remap_cargo_home_to_one_stable_prefix(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            encoded_build_rustflags(
                Path::new("/workspace/echo"),
                Path::new("/workspace/echo/target/provider-a"),
                Path::new("/workspace/echo/target/provider-a/cargo-home"),
            )?,
            [
                "--remap-path-prefix=/workspace/echo=/echo",
                "--remap-path-prefix=/workspace/echo/target/provider-a=/target",
                "--remap-path-prefix=/workspace/echo/target/provider-a/cargo-home=/cargo",
            ]
            .join("\u{1f}")
        );
        assert_eq!(
            encoded_build_rustflags(
                Path::new("/checkout-b/echo"),
                Path::new("/var/tmp/provider-b"),
                Path::new("/var/tmp/provider-b/cargo-home"),
            )?,
            [
                "--remap-path-prefix=/checkout-b/echo=/echo",
                "--remap-path-prefix=/var/tmp/provider-b=/target",
                "--remap-path-prefix=/var/tmp/provider-b/cargo-home=/cargo",
            ]
            .join("\u{1f}")
        );
        Ok(())
    }

    #[test]
    fn checked_builder_policy_is_explicit_and_fails_closed(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            parse_rustc_host(
                "rustc 1.90.0 (1159e78c4 2025-09-14)\nbinary: rustc\nhost: x86_64-unknown-linux-gnu\n"
            ),
            Some(CHECKED_COMPONENT_BUILDER_HOST)
        );
        assert_eq!(
            parse_rustc_host("host: x86_64-unknown-linux-gnu\nhost: aarch64-apple-darwin\n"),
            None
        );

        assert!(ensure_checked_builder_host(CHECKED_COMPONENT_BUILDER_HOST).is_ok());
        let error = match ensure_checked_builder_host("aarch64-apple-darwin") {
            Err(error) => error,
            Ok(()) => {
                return Err(
                    std::io::Error::other("a non-designated builder must fail closed").into(),
                );
            }
        };
        assert_eq!(
            error.kind(),
            ProviderLowererComponentErrorKind::BuilderHostMismatch
        );
        assert_eq!(error.subject(), "aarch64-apple-darwin");
        assert_eq!(error.reference(), Some(CHECKED_COMPONENT_BUILDER_HOST));
        Ok(())
    }

    #[test]
    fn pinned_tool_paths_and_identities_fail_closed(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            parse_resolved_tool_path("/toolchains/1.90.0/bin/rustc\n"),
            Some(PathBuf::from("/toolchains/1.90.0/bin/rustc"))
        );
        for malformed in ["", "relative/rustc\n", "/first/rustc\n/second/rustc\n"] {
            assert_eq!(parse_resolved_tool_path(malformed), None);
        }

        let valid = format!(
            "release: {PINNED_RUST_TOOLCHAIN}\ncommit-hash: {PINNED_RUSTC_COMMIT}\nhost: {CHECKED_COMPONENT_BUILDER_HOST}\n"
        );
        assert_eq!(
            authenticate_tool_identity("rustc", &valid, PINNED_RUSTC_COMMIT)?,
            CHECKED_COMPONENT_BUILDER_HOST
        );

        let duplicate_release = format!(
            "release: {PINNED_RUST_TOOLCHAIN}\nrelease: {PINNED_RUST_TOOLCHAIN}\ncommit-hash: {PINNED_RUSTC_COMMIT}\nhost: {CHECKED_COMPONENT_BUILDER_HOST}\n"
        );
        let error =
            match authenticate_tool_identity("rustc", &duplicate_release, PINNED_RUSTC_COMMIT) {
                Err(error) => error,
                Ok(_) => return Err(std::io::Error::other("duplicate release was accepted").into()),
            };
        assert_eq!(
            error.kind(),
            ProviderLowererComponentErrorKind::BuilderHostInvalid
        );

        let wrong_release = format!(
            "release: 1.91.0\ncommit-hash: {PINNED_RUSTC_COMMIT}\nhost: {CHECKED_COMPONENT_BUILDER_HOST}\n"
        );
        let error = match authenticate_tool_identity("rustc", &wrong_release, PINNED_RUSTC_COMMIT) {
            Err(error) => error,
            Ok(_) => return Err(std::io::Error::other("wrong release was accepted").into()),
        };
        assert_eq!(
            error.kind(),
            ProviderLowererComponentErrorKind::BuilderReleaseMismatch
        );

        let wrong_commit = format!(
            "release: {PINNED_RUST_TOOLCHAIN}\ncommit-hash: deadbeef\nhost: {CHECKED_COMPONENT_BUILDER_HOST}\n"
        );
        let error = match authenticate_tool_identity("rustc", &wrong_commit, PINNED_RUSTC_COMMIT) {
            Err(error) => error,
            Ok(_) => return Err(std::io::Error::other("wrong commit was accepted").into()),
        };
        assert_eq!(
            error.kind(),
            ProviderLowererComponentErrorKind::BuilderCommitMismatch
        );
        Ok(())
    }

    #[test]
    fn installed_pinned_toolchain_has_one_authenticated_identity(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let toolchain = pinned_rust_toolchain()?;
        assert!(toolchain.cargo.is_absolute());
        assert!(toolchain.rustc.is_absolute());
        assert!(!toolchain.host().is_empty());
        Ok(())
    }

    #[test]
    fn ambient_cargo_build_overrides_are_removed_by_prefix_family() {
        let mut command = Command::new("cargo");
        let overridden = [
            "CARGO_PROFILE_RELEASE_OPT_LEVEL",
            "CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_DEBUG",
            "CARGO_BUILD_TARGET",
            "CARGO_BUILD_JOBS",
            "CARGO_TARGET_DIR",
            "CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS",
        ];
        for name in overridden {
            command.env(name, "ambient");
        }
        command.env("CARGO_NET_OFFLINE", "true");

        remove_ambient_cargo_build_overrides(
            &mut command,
            overridden
                .into_iter()
                .chain(["CARGO_NET_OFFLINE"])
                .map(OsString::from),
        );

        for name in overridden {
            assert!(command_environment_is_removed(&command, name));
        }
        assert_eq!(
            command_environment_value(&command, "CARGO_NET_OFFLINE"),
            Some(std::ffi::OsStr::new("true"))
        );
    }

    #[test]
    fn cargo_build_is_bound_to_the_authenticated_toolchain() {
        let toolchain = PinnedRustToolchain {
            cargo: PathBuf::from("/approved/cargo"),
            rustc: PathBuf::from("/approved/rustc"),
            host: CHECKED_COMPONENT_BUILDER_HOST.to_owned(),
        };
        let mut command = Command::new(&toolchain.cargo);
        for name in [
            "RUSTC",
            "CARGO_BUILD_RUSTC",
            "RUSTC_WRAPPER",
            "CARGO_BUILD_RUSTC_WRAPPER",
            "RUSTC_WORKSPACE_WRAPPER",
            "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
        ] {
            command.env(name, "/ambient/override");
        }

        bind_pinned_toolchain(&mut command, &toolchain);

        assert_eq!(command.get_program(), toolchain.cargo.as_os_str());
        assert_eq!(
            command_environment_value(&command, "RUSTC"),
            Some(toolchain.rustc.as_os_str())
        );
        assert_eq!(
            command_environment_value(&command, "CARGO_BUILD_RUSTC"),
            Some(toolchain.rustc.as_os_str())
        );
        for name in ["RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"] {
            assert_eq!(
                command_environment_value(&command, name),
                Some(std::ffi::OsStr::new(""))
            );
        }
        for name in [
            "CARGO_BUILD_RUSTC_WRAPPER",
            "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
        ] {
            assert!(command_environment_is_removed(&command, name));
        }
    }

    fn command_environment_value<'a>(
        command: &'a Command,
        name: &str,
    ) -> Option<&'a std::ffi::OsStr> {
        command
            .get_envs()
            .find(|(key, _)| *key == std::ffi::OsStr::new(name))
            .and_then(|(_, value)| value)
    }

    fn command_environment_is_removed(command: &Command, name: &str) -> bool {
        command
            .get_envs()
            .any(|(key, value)| key == std::ffi::OsStr::new(name) && value.is_none())
    }

    #[test]
    fn explicit_component_candidates_are_audited_before_use(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let candidate = temporary_output("candidate-invalid");
        fs::write(&candidate, b"not-a-component")?;

        let error = match read_component(&candidate) {
            Err(error) => error,
            Ok(_) => {
                return Err(
                    std::io::Error::other("an unaudited candidate became a component").into(),
                );
            }
        };
        assert_eq!(
            error.kind(),
            ProviderLowererComponentErrorKind::ComponentInvalid
        );
        fs::remove_file(candidate)?;
        Ok(())
    }

    #[test]
    fn failed_promotion_never_touches_the_output(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let first = temporary_output("promotion-invalid-a");
        let second = temporary_output("promotion-invalid-b");
        let output = temporary_output("promotion-preserved");
        fs::write(&first, b"not-a-component")?;
        fs::write(&second, b"not-a-component")?;
        fs::write(&output, b"preserve-me")?;
        let error = match promote_reproducible_candidates(&first, &second, &output) {
            Err(error) => error,
            Ok(_) => {
                return Err(std::io::Error::other("an invalid candidate was promoted").into());
            }
        };
        assert_eq!(
            error.kind(),
            ProviderLowererComponentErrorKind::CandidateDigestMismatch
        );
        assert_eq!(fs::read(&output)?, b"preserve-me");
        fs::remove_file(first)?;
        fs::remove_file(second)?;
        fs::remove_file(output)?;
        Ok(())
    }

    #[test]
    fn promotion_rejects_one_candidate_supplied_twice(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let candidate = temporary_output("promotion-aliased");
        fs::write(&candidate, b"one-candidate")?;
        let error = match read_reproducible_candidates(&candidate, &candidate) {
            Err(error) => error,
            Ok(_) => {
                return Err(std::io::Error::other(
                    "one candidate path satisfied the two-candidate boundary",
                )
                .into());
            }
        };
        assert_eq!(
            error.kind(),
            ProviderLowererComponentErrorKind::CandidateAliased
        );
        fs::remove_file(candidate)?;
        Ok(())
    }

    #[test]
    fn promotion_rejects_two_paths_to_one_underlying_file(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let first = temporary_output("promotion-hardlink-a");
        let second = temporary_output("promotion-hardlink-b");
        fs::write(&first, b"one-underlying-file")?;
        fs::hard_link(&first, &second)?;

        let error = match read_reproducible_candidates(&first, &second) {
            Err(error) => error,
            Ok(_) => {
                return Err(std::io::Error::other(
                    "hard-linked paths satisfied the two-candidate boundary",
                )
                .into());
            }
        };
        assert_eq!(
            error.kind(),
            ProviderLowererComponentErrorKind::CandidateAliased
        );
        fs::remove_file(first)?;
        fs::remove_file(second)?;
        Ok(())
    }

    #[test]
    fn designated_candidate_build_cannot_target_the_checked_output(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let checked = temporary_output("checked-route");
        fs::write(&checked, b"checked")?;

        let error = match ensure_designated_candidate_output(&checked, &checked) {
            Err(error) => error,
            Ok(()) => {
                return Err(std::io::Error::other(
                    "one designated build could write the checked artifact",
                )
                .into());
            }
        };
        assert_eq!(
            error.kind(),
            ProviderLowererComponentErrorKind::CheckedOutputRequiresPromotion
        );
        let alias = temporary_output("checked-route-hardlink");
        fs::hard_link(&checked, &alias)?;
        let error = match ensure_designated_candidate_output(&alias, &checked) {
            Err(error) => error,
            Ok(()) => {
                return Err(std::io::Error::other(
                    "hard-linked candidate output bypassed promotion",
                )
                .into());
            }
        };
        assert_eq!(
            error.kind(),
            ProviderLowererComponentErrorKind::CheckedOutputRequiresPromotion
        );
        fs::remove_file(alias)?;
        fs::remove_file(checked)?;
        Ok(())
    }

    #[test]
    fn promotion_requires_two_equal_candidates_and_the_expected_digest(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let first = b"first";
        let second = b"second";
        let first_digest = digest_hex(&sha256(first));
        let second_digest = digest_hex(&sha256(second));

        let error = match ensure_reproducible_candidates(first, second, &first_digest) {
            Err(error) => error,
            Ok(()) => {
                return Err(std::io::Error::other("different candidates were promotable").into());
            }
        };
        assert_eq!(
            error.kind(),
            ProviderLowererComponentErrorKind::CandidateMismatch
        );

        let error = match ensure_reproducible_candidates(first, first, "not-a-sha256") {
            Err(error) => error,
            Ok(()) => {
                return Err(
                    std::io::Error::other("a malformed expected digest was promotable").into(),
                );
            }
        };
        assert_eq!(
            error.kind(),
            ProviderLowererComponentErrorKind::ExpectedDigestInvalid
        );

        let error = match ensure_reproducible_candidates(first, first, &second_digest) {
            Err(error) => error,
            Ok(()) => {
                return Err(std::io::Error::other(
                    "an unexpected candidate identity was promotable",
                )
                .into());
            }
        };
        assert_eq!(
            error.kind(),
            ProviderLowererComponentErrorKind::CandidateDigestMismatch
        );
        let error = match ensure_reproducible_candidates(first, first, &first_digest) {
            Err(error) => error,
            Ok(()) => {
                return Err(std::io::Error::other(
                    "a caller-selected digest became release authority",
                )
                .into());
            }
        };
        assert_eq!(
            error.kind(),
            ProviderLowererComponentErrorKind::CandidateDigestMismatch
        );
        Ok(())
    }

    #[test]
    fn packaged_lowerer_resources_match_the_checked_provider_corpus() {
        let pairs: [(&[u8], &[u8]); 4] = [
            (
                include_bytes!("../../crates/echo-edict-provider-lowerer/resources/target-profile.echo-dpo.cbor"),
                include_bytes!("../../schemas/edict-provider/generated/v1/primary/target-profile.echo-dpo.cbor"),
            ),
            (
                include_bytes!("../../crates/echo-edict-provider-lowerer/resources/lawpack.echo-dpo.cbor"),
                include_bytes!("../../schemas/edict-provider/generated/v1/primary/lawpack.echo-dpo.cbor"),
            ),
            (
                include_bytes!("../../crates/echo-edict-provider-lowerer/resources/authority-facts.echo-dpo.cbor"),
                include_bytes!("../../schemas/edict-provider/generated/v1/primary/authority-facts.echo-dpo.cbor"),
            ),
            (
                include_bytes!("../../crates/echo-edict-provider-lowerer/resources/authority-facts.echo-lawpack.cbor"),
                include_bytes!("../../schemas/edict-provider/generated/v1/primary/authority-facts.echo-lawpack.cbor"),
            ),
        ];

        for (packaged, checked) in pairs {
            assert_eq!(packaged, checked);
        }
    }

    #[test]
    fn checked_component_promotion_is_exact_and_idempotent(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let bytes = include_bytes!(
            "../../schemas/edict-provider/components/v1/lowerer.echo-dpo.component.wasm"
        );
        assert_eq!(
            digest_hex(&sha256(bytes)),
            APPROVED_CHECKED_COMPONENT_SHA256
        );
        let first = temporary_output("promotion-valid-a");
        let second = temporary_output("promotion-valid-b");
        let output = temporary_output("promotion-output");
        fs::write(&first, bytes)?;
        fs::write(&second, bytes)?;

        let (component, status) = promote_reproducible_candidates(&first, &second, &output)?;
        assert_eq!(status, ComponentOutputStatus::Written);
        assert_eq!(component.bytes(), bytes);
        let (_, status) = promote_reproducible_candidates(&first, &second, &output)?;
        assert_eq!(status, ComponentOutputStatus::Current);
        assert_eq!(fs::read(&output)?, bytes);
        fs::remove_file(first)?;
        fs::remove_file(second)?;
        fs::remove_file(output)?;
        Ok(())
    }

    #[test]
    fn checked_component_contains_no_physical_builder_paths() {
        let bytes = include_bytes!(
            "../../schemas/edict-provider/components/v1/lowerer.echo-dpo.component.wasm"
        );
        for forbidden in [
            b"/usr/local/cargo".as_slice(),
            b"/home/runner/.cargo".as_slice(),
            b"/target/cargo-home".as_slice(),
        ] {
            assert!(!bytes.windows(forbidden.len()).any(|part| part == forbidden));
        }
        let canonical = b"/cargo/registry/src/";
        assert!(bytes.windows(canonical.len()).any(|part| part == canonical));
    }

    #[test]
    fn repository_routes_preserve_separate_build_propositions() {
        let designated_image = "docker.io/library/rust@sha256:3914072ca0c3b8aad871db9169a651ccfce30cf58303e5d6f2db16d1d8a7e58f";
        let ci = include_str!("../../.github/workflows/ci.yml");
        assert!(ci.contains("cargo xtask provider-lowerer-component check"));
        assert!(ci.contains("runs-on: ubuntu-24.04"));
        assert!(ci.contains(designated_image));
        assert!(ci.contains("options: --platform linux/amd64"));
        assert!(ci.contains(concat!(
            "- name: build, audit, and check exact component bytes\n",
            "              env:\n",
            "                  GIT_CONFIG_COUNT: 1\n",
            "                  GIT_CONFIG_KEY_0: safe.directory\n",
            "                  GIT_CONFIG_VALUE_0: ${{ github.workspace }}",
        )));
        assert!(ci.contains("    build-edict-provider-verifier:"));
        assert!(ci.contains("name: Build and check Edict provider verifier"));
        assert!(ci.contains("cargo xtask provider-verifier-component check"));
        assert!(ci.contains("--target-dir target/provider-verifier-component"));
        assert!(ci.contains(
            "--output schemas/edict-provider/components/v1/verifier.echo-dpo.component.wasm"
        ));
        assert!(ci.contains(
            "cargo +1.90.0 clippy -p echo-edict-provider-verifier --target wasm32-unknown-unknown --lib -- -D warnings -D missing_docs"
        ));
        assert!(ci.matches(designated_image).count() >= 2);
        assert!(ci.matches("options: --platform linux/amd64").count() >= 2);

        let determinism = include_str!("../../.github/workflows/det-gates.yml");
        assert!(determinism.contains(designated_image));
        assert!(determinism.contains("options: --platform linux/amd64"));
        assert!(determinism.contains(concat!(
            "- name: Build isolated candidate\n",
            "        env:\n",
            "          CANDIDATE: ${{ matrix.candidate }}\n",
            "          GIT_CONFIG_COUNT: 1\n",
            "          GIT_CONFIG_KEY_0: safe.directory\n",
            "          GIT_CONFIG_VALUE_0: ${{ github.workspace }}",
        )));
        assert!(determinism.contains("candidate: [1, 2]"));
        assert!(determinism.contains("build-repro-candidate-${{ matrix.candidate }}"));
        assert!(determinism.matches("overwrite: true").count() >= 2);
        assert!(determinism.contains("cargo xtask provider-lowerer-component designated-build"));
        assert!(determinism.contains("cargo xtask provider-lowerer-component promote"));
        assert!(determinism.contains("--write"));
        assert!(determinism.contains(
            "cmp build1.lowerer.component.wasm schemas/edict-provider/components/v1/lowerer.echo-dpo.component.wasm"
        ));
        assert!(determinism.contains(
            "cmp build2.lowerer.component.wasm schemas/edict-provider/components/v1/lowerer.echo-dpo.component.wasm"
        ));
        assert!(determinism.contains("cargo xtask provider-verifier-component designated-build"));
        assert!(determinism.contains("--target-dir target/provider-verifier-repro"));
        assert!(determinism.contains("--output target/verifier.echo-dpo.component.wasm"));
        assert!(determinism.contains(
            "sha256sum target/verifier.echo-dpo.component.wasm > \"verifier-hash${CANDIDATE}.txt\""
        ));
        assert!(determinism.contains(
            "cp target/verifier.echo-dpo.component.wasm \"build${CANDIDATE}.verifier.component.wasm\""
        ));
        assert!(determinism.contains("verifier-hash${{ matrix.candidate }}.txt"));
        assert!(determinism.contains("build${{ matrix.candidate }}.verifier.component.wasm"));
        assert!(determinism.contains("diff verifier-hash1.txt verifier-hash2.txt"));
        assert!(determinism
            .contains("cmp build1.verifier.component.wasm build2.verifier.component.wasm"));
        assert!(determinism.contains("cargo xtask provider-verifier-component promote"));
        assert!(determinism.contains("--candidate-a build1.verifier.component.wasm"));
        assert!(determinism.contains("--candidate-b build2.verifier.component.wasm"));
        assert!(determinism.contains("--output target/verifier.echo-dpo.promoted.component.wasm"));
        assert!(determinism.contains(
            "cmp build1.verifier.component.wasm target/verifier.echo-dpo.promoted.component.wasm"
        ));
        assert!(determinism.contains(
            "cmp build1.verifier.component.wasm schemas/edict-provider/components/v1/verifier.echo-dpo.component.wasm"
        ));
        assert!(determinism.contains(
            "cmp build2.verifier.component.wasm schemas/edict-provider/components/v1/verifier.echo-dpo.component.wasm"
        ));
        assert!(determinism.contains(concat!(
            "            verifier-hash1.txt\n",
            "            verifier-hash2.txt\n",
            "            build1.verifier.component.wasm\n",
            "            build2.verifier.component.wasm",
        )));
        assert!(determinism.contains(concat!(
            "              gathered-artifacts/build-repro-artifacts/verifier-hash1.txt\n",
            "              gathered-artifacts/build-repro-artifacts/verifier-hash2.txt\n",
            "              gathered-artifacts/build-repro-artifacts/build1.verifier.component.wasm\n",
            "              gathered-artifacts/build-repro-artifacts/build2.verifier.component.wasm",
        )));

        let host = include_str!("../../scripts/verify-edict-provider-host-v1.sh");
        assert!(host.contains("provider-lowerer-component build"));
        assert!(host.contains("provider-lowerer-component audit"));
        assert!(!host.contains("provider-lowerer-component promote"));
    }

    fn temporary_output(label: &str) -> PathBuf {
        let nonce = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "echo-provider-lowerer-component-{}-{label}-{nonce}.wasm",
            std::process::id()
        ))
    }
}
