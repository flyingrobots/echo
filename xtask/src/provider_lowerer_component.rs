// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Deterministic build and structural admission for the Echo Edict lowerer component.

use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::fmt;
use std::fs;
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

/// Exact type-only protocol instance that the lowerer component may import.
pub(crate) const PROTOCOL_INSTANCE_COORDINATE: &str = "edict:target-provider/protocol@1.0.0";

const PROTOCOL_TYPE_IMPORTS: [&str; 2] = ["lowering-request-v1", "lowering-result-v1"];
const LOWERER_WORLD_NAME: &str = "lowerer";
const LOWERER_WIT_SOURCE: &str =
    include_str!("../../crates/echo-edict-provider-lowerer/wit/edict-target-provider.wit");
const LOWERER_PACKAGE: &str = "echo-edict-provider-lowerer";
const LOWERER_CORE_WASM: &str = "echo_edict_provider_lowerer.wasm";
const LOWER_EXPORT: &str = "lower";

/// Stable classification for a component build or audit failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProviderLowererComponentErrorKind {
    /// A path required by the deterministic build is not valid UTF-8.
    InvalidPath,
    /// Cargo could not be started.
    BuildInvocationFailed,
    /// Cargo completed without producing a successful lowerer build.
    BuildFailed,
    /// The expected core WebAssembly artifact could not be read.
    CoreWasmReadFailed,
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
    /// The candidate's complete decoded WIT world differs from the frozen lowerer world.
    WorldContractMismatch,
    /// The required `lower` export is absent.
    LowerExportMissing,
    /// More than one `lower` export is present.
    LowerExportDuplicate,
    /// A top-level export is not the exact callable `lower` export.
    LowerExportInvalid,
    /// An explicit checked output does not exist.
    OutputMissing,
    /// An explicit checked output could not be read.
    OutputReadFailed,
    /// An explicit checked output differs from the newly built bytes.
    OutputDrift,
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
            Self::OutputMissing => "output-missing",
            Self::OutputReadFailed => "output-read-failed",
            Self::OutputDrift => "output-drift",
            Self::OutputWriteFailed => "output-write-failed",
        }
    }
}

/// Stable, typed error returned by component build and audit operations.
#[derive(Debug)]
pub(crate) struct ProviderLowererComponentError {
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
            "provider-lowerer-component:{}: {}",
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

/// Exact lowerer component bytes and their SHA-256 identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProviderLowererComponent {
    bytes: Vec<u8>,
    sha256: [u8; 32],
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

/// Structural facts established by [`audit_component`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ComponentAudit {
    /// Number of exact top-level contract attestations.
    pub(crate) contract_attestations: u32,
    /// Number of exact type-only protocol imports (zero or one).
    pub(crate) protocol_imports: u32,
    /// Number of exact equality-bounded protocol type aliases (zero or two).
    pub(crate) protocol_type_imports: u32,
    /// Number of exact callable `lower` exports.
    pub(crate) lower_exports: u32,
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
) -> Result<ProviderLowererComponent> {
    let target_directory = absolute_target_directory(repository_root, target_directory);
    let repository_root_text = path_text(repository_root)?;
    let target_directory_text = path_text(&target_directory)?;

    let encoded_rustflags = [
        format!("--remap-path-prefix={repository_root_text}=/echo"),
        format!("--remap-path-prefix={target_directory_text}=/target"),
    ]
    .join("\u{1f}");

    let output = Command::new("cargo")
        .args([
            "+1.90.0",
            "build",
            "-p",
            LOWERER_PACKAGE,
            "--target",
            "wasm32-unknown-unknown",
            "--release",
            "--locked",
        ])
        .current_dir(repository_root)
        .env("CARGO_TARGET_DIR", &target_directory)
        .env("CARGO_INCREMENTAL", "0")
        .env("CARGO_ENCODED_RUSTFLAGS", encoded_rustflags)
        .env("SOURCE_DATE_EPOCH", "1")
        .env_remove("RUSTFLAGS")
        .env_remove("RUSTC_WRAPPER")
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        .output()
        .map_err(|error| {
            ProviderLowererComponentError::new(
                ProviderLowererComponentErrorKind::BuildInvocationFailed,
                LOWERER_PACKAGE,
                Some("cargo +1.90.0".to_owned()),
            )
            .with_detail(error.to_string())
        })?;

    if !output.status.success() {
        return Err(ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::BuildFailed,
            LOWERER_PACKAGE,
            Some("wasm32-unknown-unknown/release".to_owned()),
        )
        .with_detail(String::from_utf8_lossy(&output.stderr)));
    }

    let core_path = target_directory
        .join("wasm32-unknown-unknown")
        .join("release")
        .join(LOWERER_CORE_WASM);
    let core_bytes = fs::read(&core_path).map_err(|error| {
        ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::CoreWasmReadFailed,
            core_path.display().to_string(),
            Some(LOWERER_CORE_WASM.to_owned()),
        )
        .with_detail(error.to_string())
    })?;

    componentize(&core_bytes)
}

/// Componentizes explicit core Wasm bytes and appends the exact attestation.
pub(crate) fn componentize(core_bytes: &[u8]) -> Result<ProviderLowererComponent> {
    let encoder = ComponentEncoder::default()
        .validate(true)
        .merge_imports_based_on_semver(false)
        .module(core_bytes)
        .map_err(|error| {
            ProviderLowererComponentError::new(
                ProviderLowererComponentErrorKind::ComponentEncodingFailed,
                LOWERER_PACKAGE,
                Some("core-module".to_owned()),
            )
            .with_detail(error.to_string())
        })?;
    let mut encoder = encoder;
    let mut bytes = encoder.encode().map_err(|error| {
        ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::ComponentEncodingFailed,
            LOWERER_PACKAGE,
            Some("component".to_owned()),
        )
        .with_detail(error.to_string())
    })?;

    append_contract_attestation(&mut bytes);
    audit_component(&bytes)?;
    let sha256 = sha256(&bytes);
    Ok(ProviderLowererComponent { bytes, sha256 })
}

/// Audits exact attestation, import, export, and WebAssembly validity claims.
pub(crate) fn audit_component(bytes: &[u8]) -> Result<ComponentAudit> {
    let mut depth = 0_u32;
    let mut outer_is_component = false;
    let mut contract_attestations = 0_u32;
    let mut protocol_imports = 0_u32;
    let mut protocol_type_imports = [false; PROTOCOL_TYPE_IMPORTS.len()];
    let mut lower_exports = 0_u32;
    let mut top_level_types = Vec::new();

    for payload in Parser::new(0).parse_all(bytes) {
        let payload = payload.map_err(|error| component_invalid(error.to_string()))?;
        match payload {
            Payload::Version { encoding, .. } => {
                depth = depth.checked_add(1).ok_or_else(|| {
                    component_invalid("component nesting depth overflow".to_owned())
                })?;
                if depth == 1 {
                    outer_is_component = encoding == Encoding::Component;
                }
            }
            Payload::End(_) => {
                depth = depth.checked_sub(1).ok_or_else(|| {
                    component_invalid("component nesting depth underflow".to_owned())
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
                        Some(LOWERER_CONTRACT_COORDINATE.to_owned()),
                    ));
                }
                if section.data() != LOWERER_CONTRACT_COORDINATE.as_bytes() {
                    return Err(ProviderLowererComponentError::new(
                        ProviderLowererComponentErrorKind::ContractAttestationMismatch,
                        CONTRACT_SECTION_NAME,
                        Some(LOWERER_CONTRACT_COORDINATE.to_owned()),
                    ));
                }
            }
            Payload::ImportSection(section) => {
                let import = section
                    .into_imports()
                    .next()
                    .transpose()
                    .map_err(|error| component_invalid(error.to_string()))?;
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
                    let component_type =
                        component_type.map_err(|error| component_invalid(error.to_string()))?;
                    top_level_types.push(type_only_instance(&component_type));
                }
            }
            Payload::ComponentImportSection(section) => {
                for import in section {
                    let import = import.map_err(|error| component_invalid(error.to_string()))?;
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

                    let Some(alias_index) = PROTOCOL_TYPE_IMPORTS
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
                    let export = export.map_err(|error| component_invalid(error.to_string()))?;
                    if export.name.name != LOWER_EXPORT
                        || export.name.implements.is_some()
                        || export.kind != ComponentExternalKind::Func
                    {
                        return Err(ProviderLowererComponentError::new(
                            ProviderLowererComponentErrorKind::LowerExportInvalid,
                            export.name.name,
                            Some(export.kind.desc().to_owned()),
                        ));
                    }
                    lower_exports += 1;
                    if lower_exports > 1 {
                        return Err(ProviderLowererComponentError::new(
                            ProviderLowererComponentErrorKind::LowerExportDuplicate,
                            LOWER_EXPORT,
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
            LOWERER_PACKAGE,
            Some("component-model".to_owned()),
        ));
    }
    if contract_attestations != 1 {
        return Err(ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::ContractAttestationMissing,
            CONTRACT_SECTION_NAME,
            Some(LOWERER_CONTRACT_COORDINATE.to_owned()),
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
    if lower_exports != 1 {
        return Err(ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::LowerExportMissing,
            LOWER_EXPORT,
            None,
        ));
    }

    Validator::new_with_features(WasmFeatures::all())
        .validate_all(bytes)
        .map_err(|error| component_invalid(error.to_string()))?;
    authenticate_component_world(bytes)?;

    Ok(ComponentAudit {
        contract_attestations,
        protocol_imports,
        protocol_type_imports: protocol_type_import_count,
        lower_exports,
    })
}

/// Checks or writes exact component bytes at a caller-selected output path.
pub(crate) fn sync_output(
    output_path: &Path,
    bytes: &[u8],
    mode: ComponentOutputMode,
) -> Result<ComponentOutputStatus> {
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

fn write_output(output_path: &Path, bytes: &[u8]) -> Result<ComponentOutputStatus> {
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
    fs::write(output_path, bytes).map_err(|error| {
        ProviderLowererComponentError::new(
            ProviderLowererComponentErrorKind::OutputWriteFailed,
            output_path.display().to_string(),
            Some("write".to_owned()),
        )
        .with_detail(error.to_string())
    })?;
    Ok(ComponentOutputStatus::Written)
}

fn append_contract_attestation(bytes: &mut Vec<u8>) {
    CustomSection {
        name: Cow::Borrowed(CONTRACT_SECTION_NAME),
        data: Cow::Borrowed(LOWERER_CONTRACT_COORDINATE.as_bytes()),
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

fn authenticate_component_world(bytes: &[u8]) -> Result<()> {
    let (mut expected_resolve, expected_world) =
        parse_lowerer_world(LOWERER_WIT_SOURCE, "frozen-wit")?;
    let expected_package = expected_resolve.worlds[expected_world]
        .package
        .map(|package| expected_resolve.packages[package].name.clone())
        .ok_or_else(|| world_contract_invalid("frozen-wit", "lowerer world has no package"))?;
    canonicalize_world_type_closure(&mut expected_resolve, expected_world);
    let expected = encode_world_contract(&expected_resolve, expected_world, "frozen-wit")?;

    let decoded = wit_component::decode(bytes).map_err(|error| {
        world_contract_invalid("candidate-component", "component WIT decode failed")
            .with_detail(error.to_string())
    })?;
    let DecodedWasm::Component(mut observed_resolve, observed_world) = decoded else {
        return Err(world_contract_invalid(
            "candidate-component",
            "decoded bytes are a WIT package rather than a component",
        ));
    };
    normalize_decoded_world_identity(&mut observed_resolve, observed_world, expected_package)?;
    canonicalize_world_type_closure(&mut observed_resolve, observed_world);
    let observed = encode_world_contract(&observed_resolve, observed_world, "candidate-component")?;
    compare_world_contracts(&expected, &observed)
}

#[cfg(test)]
fn authenticate_wit_source_for_test(source: &str) -> Result<()> {
    let (mut expected_resolve, expected_world) =
        parse_lowerer_world(LOWERER_WIT_SOURCE, "frozen-wit")?;
    canonicalize_world_type_closure(&mut expected_resolve, expected_world);
    let expected = encode_world_contract(&expected_resolve, expected_world, "frozen-wit")?;
    let (mut observed_resolve, observed_world) = parse_lowerer_world(source, "candidate-wit")?;
    canonicalize_world_type_closure(&mut observed_resolve, observed_world);
    let observed = encode_world_contract(&observed_resolve, observed_world, "candidate-wit")?;
    compare_world_contracts(&expected, &observed)
}

fn parse_lowerer_world(source: &str, reference: &str) -> Result<(Resolve, WorldId)> {
    let mut resolve = Resolve::default();
    let package = resolve
        .push_str("edict-target-provider.wit", source)
        .map_err(|error| {
            world_contract_invalid(reference, "WIT parse failed").with_detail(error.to_string())
        })?;
    let world = resolve
        .select_world(&[package], Some(LOWERER_WORLD_NAME))
        .map_err(|error| {
            world_contract_invalid(reference, "lowerer world selection failed")
                .with_detail(error.to_string())
        })?;
    Ok((resolve, world))
}

fn normalize_decoded_world_identity(
    resolve: &mut Resolve,
    world: WorldId,
    package_name: PackageName,
) -> Result<()> {
    let package = resolve.worlds[world].package.ok_or_else(|| {
        world_contract_invalid("candidate-component", "decoded world has no package")
    })?;
    let previous_world_name = std::mem::replace(
        &mut resolve.worlds[world].name,
        LOWERER_WORLD_NAME.to_owned(),
    );
    let package_worlds = &mut resolve.packages[package].worlds;
    if package_worlds.get(&previous_world_name) == Some(&world) {
        package_worlds.shift_remove(&previous_world_name);
        package_worlds.insert(LOWERER_WORLD_NAME.to_owned(), world);
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

fn encode_world_contract(resolve: &Resolve, world: WorldId, reference: &str) -> Result<Vec<u8>> {
    wit_component::metadata::encode(resolve, world, StringEncoding::UTF8, None).map_err(|error| {
        world_contract_invalid(reference, "canonical world encoding failed")
            .with_detail(error.to_string())
    })
}

fn compare_world_contracts(expected: &[u8], observed: &[u8]) -> Result<()> {
    if expected == observed {
        return Ok(());
    }
    Err(ProviderLowererComponentError::new(
        ProviderLowererComponentErrorKind::WorldContractMismatch,
        LOWERER_CONTRACT_COORDINATE,
        Some(format!(
            "expected:{};observed:{}",
            digest_hex(&sha256(expected)),
            digest_hex(&sha256(observed))
        )),
    ))
}

fn world_contract_invalid(
    reference: impl Into<String>,
    detail: impl Into<String>,
) -> ProviderLowererComponentError {
    ProviderLowererComponentError::new(
        ProviderLowererComponentErrorKind::WorldContractInvalid,
        LOWERER_CONTRACT_COORDINATE,
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

fn component_invalid(detail: String) -> ProviderLowererComponentError {
    ProviderLowererComponentError::new(
        ProviderLowererComponentErrorKind::ComponentInvalid,
        LOWERER_PACKAGE,
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

    fn temporary_output(label: &str) -> PathBuf {
        let nonce = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "echo-provider-lowerer-component-{}-{label}-{nonce}.wasm",
            std::process::id()
        ))
    }
}
