// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Generic compiler-produced Edict operation witness over the real Echo host.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use blake3::Hasher;
use bytes::Bytes;
use echo_edict_canonical::{
    decode_canonical_cbor_v1, digest_canonical_value_bytes_v1, CanonicalValueV1,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use warp_core::{
    echo_operation_action_envelope_v1, echo_operation_anchored_node_creation_application_basis_v1,
    echo_operation_create_if_absent_target_profile_identity_v1, echo_operation_package_id_v1,
    make_head_id, make_node_id, make_type_id, make_warp_id, AtomPayload, AttachmentValue,
    EchoOperationActionOutcomeV1, EchoOperationAdmissionPolicyV1,
    EchoOperationAnchoredNodeOccupancyV1, EchoOperationBudgetV1,
    EchoOperationInvocationAdmissionPolicyV1, EchoOperationInvocationV1,
    EchoOperationObstructionKindV1, EngineBuilder, GraphStore, InboxPolicy, IngressTarget, NodeId,
    NodeKey, NodeRecord, PlaybackMode, SchedulerKind, TrustedRuntimeHost, TrustedRuntimeWalConfig,
    TypeId, WorldlineId, WorldlineRuntime, WorldlineState, WriterHead, WriterHeadKey,
};

const MAX_ARTIFACT_BYTES: u64 = 1_048_576;
const MAX_INPUT_BYTES: u64 = 65_536;
const PACKAGE_DOMAIN: &str = "echo.operation-package/v1";
const PACKAGE_ROLE: &str = "executable-operation-package.echo";
const MANIFEST_DOMAIN: &str = "edict.lawpack/v1";
const ADAPTER_DOMAIN: &str = "edict.lawpack-adapter/v1";
const CONFIGURATION_DOMAIN: &str = "echo.operation-lowering-configuration/v1";
const PROGRAM_KIND: &str = "anchored-node-attachment-create-if-absent/v1";
const VERIFICATION_REPORT_ABI: &str = "echo.operation-package-verifier-report/v1";
const TARGET_IR_COORDINATE: &str = "echo.span-ir/v1";
const DIAGNOSTIC_ABI: &str = "edict.diagnostics/v1";
const NODE_ID_DERIVATION: &str = "sha256-utf8/v1";
const WARP_ID_SOURCE: &str = "action-lane/v1";
const PRECONDITION_MISMATCH: &str = "echo.executable-operation/precondition-mismatch/v1";

/// Inputs needed to run one exact compiler-produced package.
pub struct RunEdictOperationConfig {
    pub package: PathBuf,
    pub verification_report: PathBuf,
    pub lawpack_manifest: PathBuf,
    pub lawpack_adapter: PathBuf,
    pub target_configuration: PathBuf,
    pub input: PathBuf,
    pub wal_dir: PathBuf,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunEdictOperationReport {
    pub verdict: &'static str,
    pub operation: String,
    pub artifacts: ArtifactReport,
    pub submission: SubmissionReport,
    pub scheduler: SchedulerReport,
    pub state: StateReport,
    pub recovery: RecoveryReport,
    pub duplicate: DuplicateReport,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactReport {
    pub package_sha256: String,
    pub verification_outcome: &'static str,
    pub closure_bound: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionReport {
    pub wal_committed_before_ack: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerReport {
    pub action_count: usize,
    pub singleton_integration: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateReport {
    pub value_utf8: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryReport {
    pub action_recovered: bool,
    pub tick_recovered: bool,
    pub state_recovered: bool,
    pub outcome_recovered: bool,
    pub receipt_recovered: bool,
    pub mutated_initial_state_refused: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateReport {
    pub obstruction: &'static str,
    pub hidden_mutation: bool,
}

struct PackageMetadata {
    operation_coordinate: String,
    lawpack_coordinate: String,
    lawpack_identity: [u8; 32],
    authority_profile_identity: [u8; 32],
    target_profile_identity: [u8; 32],
    budget: EchoOperationBudgetV1,
    required_node_type: TypeId,
    required_attachment_type: TypeId,
    maximum_replacement_bytes: u64,
}

struct TargetConfiguration {
    authority_profile: String,
    required_node_type_profile: String,
    required_attachment_type_profile: String,
    node_key_field: String,
    replacement_field: String,
    maximum_replacement_bytes: u64,
    budget: EchoOperationBudgetV1,
}

struct OperationInput {
    basis: String,
    key: String,
    replacement: Vec<u8>,
}

struct HostFixture {
    host: TrustedRuntimeHost,
    head: WriterHeadKey,
    node: NodeKey,
}

/// Runs the exact package and returns its durable singleton scheduler witness.
pub fn run(config: RunEdictOperationConfig) -> Result<RunEdictOperationReport> {
    let package_bytes = read_bounded(&config.package, MAX_ARTIFACT_BYTES, "package")?;
    let package_sha256 = hex::encode(Sha256::digest(&package_bytes));
    let report_bytes = read_bounded(
        &config.verification_report,
        MAX_ARTIFACT_BYTES,
        "verification report",
    )?;
    let manifest_bytes = read_bounded(
        &config.lawpack_manifest,
        MAX_ARTIFACT_BYTES,
        "lawpack manifest",
    )?;
    let adapter_bytes = read_bounded(
        &config.lawpack_adapter,
        MAX_ARTIFACT_BYTES,
        "lawpack adapter",
    )?;
    let configuration_bytes = read_bounded(
        &config.target_configuration,
        MAX_ARTIFACT_BYTES,
        "target configuration",
    )?;
    let input_bytes = read_bounded(&config.input, MAX_INPUT_BYTES, "operation input")?;

    let package_value =
        decode_canonical_cbor_v1(&package_bytes).context("package is not canonical Edict CBOR")?;
    let package = parse_package(&package_value)?;
    validate_verification_report(&report_bytes, &package_value)?;
    let target_configuration = validate_closure(
        &manifest_bytes,
        &adapter_bytes,
        &configuration_bytes,
        &package.lawpack_coordinate,
        package.lawpack_identity,
    )?;
    validate_package_configuration(&package, &target_configuration)?;
    let input = parse_input(&input_bytes, &target_configuration)?;

    if config.wal_dir.exists() {
        let mut entries = fs::read_dir(&config.wal_dir)
            .with_context(|| format!("failed to inspect {}", config.wal_dir.display()))?;
        if entries.next().transpose()?.is_some() {
            bail!(
                "runtime WAL directory must be empty: {}",
                config.wal_dir.display()
            );
        }
    } else {
        fs::create_dir_all(&config.wal_dir)
            .with_context(|| format!("failed to create {}", config.wal_dir.display()))?;
    }

    let package_id = echo_operation_package_id_v1(&package_bytes);
    let authority_grant_identity = domain_hash(
        b"echo:edict-operation-runner-authority-grant:v1\0",
        &package_id.as_hash(),
    );
    let invocation_policy = EchoOperationInvocationAdmissionPolicyV1::new(
        package.authority_profile_identity,
        authority_grant_identity,
        package.budget,
    );

    let first_submission_id;
    let committed_state_root;
    let scheduler_action_count;
    let wal_committed_before_ack;

    {
        let mut fixture = build_host(&input.basis, &input.key, false)?;
        fixture
            .host
            .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&config.wal_dir))?;
        install_package(&mut fixture.host, &package, package_id, package_bytes)?;
        fixture
            .host
            .install_echo_operation_action_admission_policy_v1(invocation_policy);

        let state_root_before = current_state(&fixture)?.state_root();
        let invocation = invocation(
            &fixture,
            &package,
            package_id,
            authority_grant_identity,
            &input.replacement,
            EchoOperationAnchoredNodeOccupancyV1::Absent,
        )?;
        let envelope = echo_operation_action_envelope_v1(
            IngressTarget::ExactHead { key: fixture.head },
            invocation,
        )
        .context("failed to encode the canonical Action envelope")?;
        first_submission_id = fixture
            .host
            .app()
            .submit_intent_with_runtime_wal_ack(envelope)
            .context("Echo refused durable Action acceptance")?
            .submission_id;

        let accepted = fixture
            .host
            .runtime_wal()
            .context("runtime WAL disappeared after activation")?
            .recover_read_only()
            .context("failed to recover accepted-submission WAL evidence")?;
        wal_committed_before_ack = accepted
            .witnessed_submissions
            .records()
            .iter()
            .any(|record| record.submission.submission_id == first_submission_id);
        if !wal_committed_before_ack
            || !accepted.provenance_entries.is_empty()
            || current_state(&fixture)?.state_root() != state_root_before
        {
            bail!("Action acknowledgement preceded durable acceptance or mutated pre-Tick state");
        }

        let steps = fixture
            .host
            .tick_once()
            .context("scheduler failed to construct the executable-operation Tick")?;
        if steps.len() != 1 || steps[0].admitted_count != 1 {
            bail!("singleton proof produced an unexpected scheduler selection");
        }
        scheduler_action_count = steps[0].admitted_count;
        if !matches!(
            fixture
                .host
                .echo_operation_action_outcome_v1(&first_submission_id),
            Some(EchoOperationActionOutcomeV1::Committed(_))
        ) {
            bail!("scheduler did not publish a committed typed Action outcome");
        }
        let value = node_value(&fixture, package.required_attachment_type)?;
        if value != input.replacement {
            bail!("committed state does not contain the typed replacement bytes");
        }
        committed_state_root = current_state(&fixture)?.state_root();
    }

    let action_recovered;
    let tick_recovered;
    let state_recovered;
    let outcome_recovered;
    let receipt_recovered;
    let hidden_mutation;
    let duplicate_obstruction;

    {
        let mut recovered = build_host(&input.basis, &input.key, false)?;
        recovered
            .host
            .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&config.wal_dir))?;
        let recovered_wal = recovered
            .host
            .runtime_wal()
            .context("recovered runtime WAL is unavailable")?
            .recover_read_only()
            .context("failed to inspect recovered runtime WAL")?;
        action_recovered = recovered_wal
            .witnessed_submissions
            .records()
            .iter()
            .any(|record| record.submission.submission_id == first_submission_id);
        tick_recovered = recovered_wal
            .receipt_correlations
            .iter()
            .any(|record| record.submission_id == first_submission_id);
        state_recovered = current_state(&recovered)?.state_root() == committed_state_root
            && node_value(&recovered, package.required_attachment_type)? == input.replacement;
        outcome_recovered = matches!(
            recovered
                .host
                .echo_operation_action_outcome_v1(&first_submission_id),
            Some(EchoOperationActionOutcomeV1::Committed(_))
        );
        receipt_recovered = recovered_wal.echo_operation_action_outcomes.iter().any(
            |(submission_id, _, outcome)| {
                *submission_id == first_submission_id
                    && matches!(outcome, EchoOperationActionOutcomeV1::Committed(_))
            },
        ) && recovered_wal
            .receipt_correlations
            .iter()
            .any(|record| record.submission_id == first_submission_id);

        recovered
            .host
            .install_echo_operation_action_admission_policy_v1(invocation_policy);
        let state_before_duplicate = current_state(&recovered)?.state_root();
        let value_before_duplicate = node_value(&recovered, package.required_attachment_type)?;
        let duplicate_invocation = invocation(
            &recovered,
            &package,
            package_id,
            authority_grant_identity,
            &input.replacement,
            EchoOperationAnchoredNodeOccupancyV1::NodeAndAttachment,
        )?;
        let duplicate_envelope = echo_operation_action_envelope_v1(
            IngressTarget::ExactHead {
                key: recovered.head,
            },
            duplicate_invocation,
        )
        .context("failed to encode duplicate Action envelope")?;
        let duplicate_submission_id = recovered
            .host
            .app()
            .submit_intent_with_runtime_wal_ack(duplicate_envelope)
            .context("Echo refused durable duplicate Action acceptance")?
            .submission_id;
        let duplicate_steps = recovered
            .host
            .tick_once()
            .context("scheduler failed to decide the duplicate Action")?;
        if duplicate_steps.len() != 1 || duplicate_steps[0].admitted_count != 1 {
            bail!("duplicate proof produced an unexpected scheduler selection");
        }
        duplicate_obstruction = match recovered
            .host
            .echo_operation_action_outcome_v1(&duplicate_submission_id)
        {
            Some(EchoOperationActionOutcomeV1::Obstructed(obstruction))
                if obstruction.kind() == EchoOperationObstructionKindV1::PreconditionMismatch =>
            {
                PRECONDITION_MISMATCH
            }
            outcome => bail!("duplicate Action produced unexpected outcome: {outcome:?}"),
        };
        hidden_mutation = current_state(&recovered)?.state_root() != state_before_duplicate
            || node_value(&recovered, package.required_attachment_type)? != value_before_duplicate;
    }
    if !action_recovered
        || !tick_recovered
        || !state_recovered
        || !outcome_recovered
        || !receipt_recovered
    {
        bail!("fresh host did not recover the complete Action/Tick/state/outcome/Receipt witness");
    }
    if hidden_mutation {
        bail!("obstructed duplicate Action changed committed state");
    }

    let mutated_initial_state_refused = {
        let mut mutated = build_host(&input.basis, &input.key, true)?;
        mutated
            .host
            .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&config.wal_dir))
            .is_err()
    };
    if !mutated_initial_state_refused {
        bail!("runtime WAL recovery accepted a mutated initial state");
    }

    Ok(RunEdictOperationReport {
        verdict: "pass",
        operation: package.operation_coordinate,
        artifacts: ArtifactReport {
            package_sha256,
            verification_outcome: "accepted",
            closure_bound: true,
        },
        submission: SubmissionReport {
            wal_committed_before_ack,
        },
        scheduler: SchedulerReport {
            action_count: scheduler_action_count,
            singleton_integration: true,
        },
        state: StateReport {
            value_utf8: String::from_utf8(input.replacement)
                .context("replacement value is not UTF-8")?,
        },
        recovery: RecoveryReport {
            action_recovered,
            tick_recovered,
            state_recovered,
            outcome_recovered,
            receipt_recovered,
            mutated_initial_state_refused,
        },
        duplicate: DuplicateReport {
            obstruction: duplicate_obstruction,
            hidden_mutation,
        },
    })
}

fn read_bounded(path: &Path, maximum: u64, label: &str) -> Result<Vec<u8>> {
    let file =
        File::open(path).with_context(|| format!("failed to open {label} {}", path.display()))?;
    let metadata = file
        .metadata()
        .with_context(|| format!("failed to inspect {label} {}", path.display()))?;
    if !metadata.is_file() {
        bail!("{label} is not a regular file: {}", path.display());
    }
    let mut bytes = Vec::with_capacity(
        usize::try_from(metadata.len().min(maximum))
            .context("host byte bound does not fit this platform")?,
    );
    file.take(maximum.saturating_add(1))
        .read_to_end(&mut bytes)
        .with_context(|| format!("failed to read {label} {}", path.display()))?;
    if u64::try_from(bytes.len())? > maximum {
        bail!("{label} exceeds the {maximum}-byte host bound");
    }
    Ok(bytes)
}

fn parse_package(value: &CanonicalValueV1) -> Result<PackageMetadata> {
    require_text(value, "schema", "package", "echo.operation-package/v1")?;
    let program_bytes = bytes_field(value, "program", "package")?;
    let program =
        decode_canonical_cbor_v1(program_bytes).context("package program is not canonical")?;
    require_text(
        &program,
        "schema",
        "package program",
        "echo.operation-program/v1",
    )?;
    require_text(&program, "kind", "package program", PROGRAM_KIND)?;
    let budget = map_field(value, "budget_ceiling", "package")?;
    let semantic_closure = map_field(value, "semantic_closure", "package")?;
    Ok(PackageMetadata {
        operation_coordinate: nonempty_text_field(value, "operation_coordinate", "package")?
            .to_owned(),
        lawpack_coordinate: nonempty_text_field(
            semantic_closure,
            "lawpack_coordinate",
            "semantic closure",
        )?
        .to_owned(),
        lawpack_identity: hash_field(semantic_closure, "lawpack_identity", "semantic closure")?,
        authority_profile_identity: hash_field(value, "authority_profile_identity", "package")?,
        target_profile_identity: hash_field(value, "target_profile_identity", "package")?,
        budget: budget_value(
            budget,
            "steps",
            "read_bytes",
            "write_bytes",
            "package budget",
        )?,
        required_node_type: TypeId(hash_field(
            &program,
            "required_node_type",
            "package program",
        )?),
        required_attachment_type: TypeId(hash_field(
            &program,
            "required_attachment_type",
            "package program",
        )?),
        maximum_replacement_bytes: u64_field(&program, "max_replacement_bytes", "package program")?,
    })
}

fn validate_verification_report(bytes: &[u8], package: &CanonicalValueV1) -> Result<()> {
    let report = decode_canonical_cbor_v1(bytes)
        .context("verification report is not canonical Edict CBOR")?;
    require_text(
        &report,
        "apiVersion",
        "verification report",
        VERIFICATION_REPORT_ABI,
    )?;
    require_text(&report, "outcome", "verification report", "accepted")?;
    if !bytes_field(&report, "diagnosticBytes", "verification report")?.is_empty() {
        bail!("accepted verification report contains diagnostics");
    }
    let package_ref = map_field(&report, "package", "verification report")?;
    validate_resource_ref(package_ref, PACKAGE_ROLE, PACKAGE_DOMAIN, package)?;
    let target_ir = map_field(&report, "targetIr", "verification report")?;
    require_resource_identity(target_ir, TARGET_IR_COORDINATE)?;
    let diagnostic_abi = map_field(&report, "diagnosticAbi", "verification report")?;
    require_resource_identity(diagnostic_abi, DIAGNOSTIC_ABI)?;
    Ok(())
}

fn validate_closure(
    manifest_bytes: &[u8],
    adapter_bytes: &[u8],
    configuration_bytes: &[u8],
    package_lawpack_coordinate: &str,
    expected_lawpack_identity: [u8; 32],
) -> Result<TargetConfiguration> {
    let manifest =
        decode_canonical_cbor_v1(manifest_bytes).context("lawpack manifest is not canonical")?;
    let adapter =
        decode_canonical_cbor_v1(adapter_bytes).context("lawpack adapter is not canonical")?;
    let configuration = decode_canonical_cbor_v1(configuration_bytes)
        .context("target configuration is not canonical")?;

    require_text(
        &manifest,
        "apiVersion",
        "lawpack manifest",
        "edict.lawpack/v1",
    )?;
    let lawpack_coordinate = format!(
        "{}@{}",
        nonempty_text_field(&manifest, "id", "lawpack manifest")?,
        nonempty_text_field(&manifest, "version", "lawpack manifest")?
    );
    let manifest_identity = digest_canonical_value_bytes_v1(MANIFEST_DOMAIN, &manifest)?;
    if lawpack_coordinate != package_lawpack_coordinate {
        bail!("package lawpack coordinate does not bind the supplied manifest");
    }
    if manifest_identity != expected_lawpack_identity {
        bail!("package lawpack identity does not bind the supplied manifest");
    }

    let adapters = array_field(&manifest, "targetAdapters", "lawpack manifest")?;
    let matching_adapters = adapters
        .iter()
        .filter_map(|selection| {
            map_field(selection, "adapter", "target adapter selection")
                .ok()
                .filter(|resource| {
                    resource_coordinate(resource).is_ok_and(|coordinate| {
                        resource_ref_matches(resource, coordinate, &adapter)
                    })
                })
        })
        .count();
    if matching_adapters != 1 {
        bail!("lawpack manifest must bind the supplied adapter exactly once");
    }

    require_text(&adapter, "apiVersion", "lawpack adapter", ADAPTER_DOMAIN)?;
    let effects = map_field(&adapter, "effectImplementations", "lawpack adapter")?;
    let implementations = map_entries(effects, "lawpack adapter effect implementations")?;
    let matching_configurations = implementations
        .values()
        .filter_map(|implementation| {
            map_field(
                implementation,
                "targetConfiguration",
                "lawpack adapter effect implementation",
            )
            .ok()
        })
        .filter(|resource| {
            resource_coordinate(resource)
                .is_ok_and(|coordinate| resource_ref_matches(resource, coordinate, &configuration))
        })
        .count();
    if matching_configurations == 0 {
        bail!("lawpack adapter does not bind the supplied target configuration");
    }
    parse_target_configuration(&configuration)
}

fn parse_target_configuration(value: &CanonicalValueV1) -> Result<TargetConfiguration> {
    require_text(
        value,
        "apiVersion",
        "target configuration",
        CONFIGURATION_DOMAIN,
    )?;
    require_text(value, "programKind", "target configuration", PROGRAM_KIND)?;
    let binding = map_field(value, "invocationBinding", "target configuration")?;
    require_text(
        binding,
        "nodeIdDerivation",
        "invocation binding",
        NODE_ID_DERIVATION,
    )?;
    require_text(
        binding,
        "warpIdSource",
        "invocation binding",
        WARP_ID_SOURCE,
    )?;
    let budget = map_field(value, "budgetCeiling", "target configuration")?;
    Ok(TargetConfiguration {
        authority_profile: nonempty_text_field(value, "authorityProfile", "target configuration")?
            .to_owned(),
        required_node_type_profile: nonempty_text_field(
            value,
            "requiredNodeTypeProfile",
            "target configuration",
        )?
        .to_owned(),
        required_attachment_type_profile: nonempty_text_field(
            value,
            "requiredAttachmentTypeProfile",
            "target configuration",
        )?
        .to_owned(),
        node_key_field: nonempty_text_field(binding, "nodeKeyField", "invocation binding")?
            .to_owned(),
        replacement_field: nonempty_text_field(binding, "replacementField", "invocation binding")?
            .to_owned(),
        maximum_replacement_bytes: u64_field(value, "maxReplacementBytes", "target configuration")?,
        budget: budget_value(
            budget,
            "steps",
            "readBytes",
            "writeBytes",
            "target configuration budget",
        )?,
    })
}

fn validate_package_configuration(
    package: &PackageMetadata,
    configuration: &TargetConfiguration,
) -> Result<()> {
    if package.authority_profile_identity != profile_digest(&configuration.authority_profile)
        || package.required_node_type
            != TypeId(profile_digest(&configuration.required_node_type_profile))
        || package.required_attachment_type
            != TypeId(profile_digest(
                &configuration.required_attachment_type_profile,
            ))
        || package.maximum_replacement_bytes != configuration.maximum_replacement_bytes
        || package.budget != configuration.budget
        || package.target_profile_identity
            != echo_operation_create_if_absent_target_profile_identity_v1()
    {
        bail!("package meaning does not match the supplied target configuration");
    }
    Ok(())
}

fn parse_input(bytes: &[u8], configuration: &TargetConfiguration) -> Result<OperationInput> {
    if configuration.node_key_field == configuration.replacement_field
        || configuration.node_key_field == "basis"
        || configuration.replacement_field == "basis"
    {
        bail!("target configuration input fields must be distinct");
    }
    let value: serde_json::Value =
        serde_json::from_slice(bytes).context("operation input is not valid JSON")?;
    let object = value
        .as_object()
        .context("operation input must be one JSON object")?;
    let expected = BTreeSet::from([
        "basis",
        configuration.node_key_field.as_str(),
        configuration.replacement_field.as_str(),
    ]);
    let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    if actual != expected {
        bail!("operation input fields do not match the target configuration");
    }
    let basis = object
        .get("basis")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .context("operation input basis must be a nonempty string")?;
    let key = object
        .get(&configuration.node_key_field)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .context("operation node key must be a nonempty string")?;
    let replacement = object
        .get(&configuration.replacement_field)
        .and_then(serde_json::Value::as_str)
        .context("operation replacement must be a string")?
        .as_bytes()
        .to_vec();
    if u64::try_from(replacement.len())? > configuration.maximum_replacement_bytes {
        bail!("operation replacement exceeds the configured byte bound");
    }
    Ok(OperationInput {
        basis: basis.to_owned(),
        key: key.to_owned(),
        replacement,
    })
}

fn install_package(
    host: &mut TrustedRuntimeHost,
    package: &PackageMetadata,
    package_id: warp_core::EchoOperationPackageIdV1,
    package_bytes: Vec<u8>,
) -> Result<()> {
    let admitted = host
        .admit_echo_operation_package_v1(
            &EchoOperationAdmissionPolicyV1::exact(
                package_id,
                &package.operation_coordinate,
                package.authority_profile_identity,
                package.budget,
            ),
            package_bytes,
        )
        .context("Echo independently refused the compiler-produced package")?;
    host.install_admitted_echo_operation_package_v1(admitted)
        .context("Echo failed to durably install the admitted package")?;
    Ok(())
}

fn build_host(basis: &str, key: &str, mutated: bool) -> Result<HostFixture> {
    let lane_label = format!("edict-operation-lane:{basis}");
    let warp_id = make_warp_id(&lane_label);
    let root_id = make_node_id(&format!("{lane_label}:root"));
    let target_id = NodeId(Sha256::digest(key.as_bytes()).into());
    if target_id == root_id {
        bail!("derived operation node collides with the action-lane root");
    }
    let node = NodeKey {
        warp_id,
        local_id: target_id,
    };
    let mut store = GraphStore::new(warp_id);
    store.insert_node(
        root_id,
        NodeRecord {
            ty: make_type_id("echo.edict-operation-runner/root/v1"),
        },
    );
    if mutated {
        store.set_node_attachment(
            root_id,
            Some(AttachmentValue::Atom(AtomPayload::new(
                make_type_id("echo.edict-operation-runner/mutated-basis/v1"),
                Bytes::from_static(b"mutated"),
            ))),
        );
    }
    let state = WorldlineState::from_root_store(store, root_id)?;
    let worldline_id = WorldlineId::from_bytes(domain_hash(
        b"echo:edict-operation-runner-worldline:v1\0",
        basis.as_bytes(),
    ));
    let head = WriterHeadKey {
        worldline_id,
        head_id: make_head_id(&format!("{lane_label}:head")),
    };
    let mut runtime = WorldlineRuntime::new();
    runtime.register_worldline(worldline_id, state)?;
    runtime.register_writer_head(WriterHead::with_routing(
        head,
        PlaybackMode::Play,
        InboxPolicy::AcceptAll,
        None,
        true,
    ))?;

    let mut engine_store = GraphStore::default();
    let engine_root = make_node_id("edict-operation-runner-engine-root");
    engine_store.insert_node(
        engine_root,
        NodeRecord {
            ty: make_type_id("echo.edict-operation-runner/engine-root/v1"),
        },
    );
    let engine = EngineBuilder::new(engine_store, engine_root)
        .scheduler(SchedulerKind::Radix)
        .workers(1)
        .build();
    Ok(HostFixture {
        host: TrustedRuntimeHost::new(runtime, engine)?,
        head,
        node,
    })
}

fn invocation(
    fixture: &HostFixture,
    package: &PackageMetadata,
    package_id: warp_core::EchoOperationPackageIdV1,
    authority_grant_identity: [u8; 32],
    replacement: &[u8],
    occupancy: EchoOperationAnchoredNodeOccupancyV1,
) -> Result<Vec<u8>> {
    let application_basis =
        echo_operation_anchored_node_creation_application_basis_v1(fixture.node, occupancy);
    let evaluation_basis = fixture
        .host
        .echo_operation_evaluation_basis_v1(fixture.head, application_basis)?;
    EchoOperationInvocationV1::anchored_node_attachment_create_if_absent(
        package_id,
        &package.operation_coordinate,
        evaluation_basis,
        authority_grant_identity,
        package.budget,
        fixture.node,
        replacement.to_vec(),
    )
    .to_canonical_bytes()
    .context("failed to encode canonical executable-operation invocation")
}

fn current_state(fixture: &HostFixture) -> Result<&WorldlineState> {
    fixture
        .host
        .runtime()
        .worldlines()
        .get(&fixture.head.worldline_id)
        .map(warp_core::WorldlineFrontier::state)
        .context("operation worldline is unavailable")
}

fn node_value(fixture: &HostFixture, expected_type: TypeId) -> Result<Vec<u8>> {
    let attachment = current_state(fixture)?
        .store(&fixture.node.warp_id)
        .and_then(|store| store.node_attachment(&fixture.node.local_id))
        .context("operation node attachment is absent")?;
    let AttachmentValue::Atom(atom) = attachment else {
        bail!("operation node attachment is not an atom");
    };
    if atom.type_id != expected_type {
        bail!("operation node attachment has the wrong declared type");
    }
    Ok(atom.bytes.to_vec())
}

fn validate_resource_ref(
    resource: &CanonicalValueV1,
    expected_coordinate: &str,
    digest_domain: &str,
    value: &CanonicalValueV1,
) -> Result<()> {
    require_resource_identity(resource, expected_coordinate)?;
    let actual = resource_digest(resource)?;
    let expected = digest_canonical_value_bytes_v1(digest_domain, value)?;
    if actual != expected {
        bail!("resource {expected_coordinate} does not bind the supplied canonical artifact");
    }
    Ok(())
}

fn resource_ref_matches(
    resource: &CanonicalValueV1,
    expected_coordinate: &str,
    value: &CanonicalValueV1,
) -> bool {
    resource_coordinate(resource).is_ok_and(|coordinate| coordinate == expected_coordinate)
        && resource_digest(resource).is_ok_and(|actual| {
            digest_canonical_value_bytes_v1(expected_coordinate, value)
                .is_ok_and(|expected| actual == expected)
        })
}

fn require_resource_identity(resource: &CanonicalValueV1, expected: &str) -> Result<()> {
    let coordinate = resource_coordinate(resource)?;
    if coordinate != expected {
        bail!("resource coordinate mismatch: expected {expected}, found {coordinate}");
    }
    let _ = resource_digest(resource)?;
    Ok(())
}

fn resource_coordinate(resource: &CanonicalValueV1) -> Result<&str> {
    nonempty_text_field(resource, "id", "resource reference")
}

fn resource_digest(resource: &CanonicalValueV1) -> Result<[u8; 32]> {
    let digest = array_field(resource, "digest", "resource reference")?;
    match digest {
        [CanonicalValueV1::Text(algorithm), CanonicalValueV1::Bytes(bytes)]
            if algorithm == "sha256" =>
        {
            bytes
                .as_slice()
                .try_into()
                .context("resource digest must contain exactly 32 bytes")
        }
        _ => bail!("resource digest must be [\"sha256\", bytes32]"),
    }
}

fn map_entries<'a>(
    value: &'a CanonicalValueV1,
    subject: &str,
) -> Result<BTreeMap<&'a str, &'a CanonicalValueV1>> {
    let CanonicalValueV1::Map(entries) = value else {
        bail!("{subject} must be a canonical map");
    };
    let mut fields = BTreeMap::new();
    for (key, value) in entries {
        let CanonicalValueV1::Text(key) = key else {
            bail!("{subject} contains a non-text map key");
        };
        if fields.insert(key.as_str(), value).is_some() {
            bail!("{subject} repeats field {key}");
        }
    }
    Ok(fields)
}

fn field<'a>(
    value: &'a CanonicalValueV1,
    name: &str,
    subject: &str,
) -> Result<&'a CanonicalValueV1> {
    map_entries(value, subject)?
        .get(name)
        .copied()
        .with_context(|| format!("{subject} is missing {name}"))
}

fn map_field<'a>(
    value: &'a CanonicalValueV1,
    name: &str,
    subject: &str,
) -> Result<&'a CanonicalValueV1> {
    let value = field(value, name, subject)?;
    if matches!(value, CanonicalValueV1::Map(_)) {
        Ok(value)
    } else {
        bail!("{subject}.{name} must be a map");
    }
}

fn array_field<'a>(
    value: &'a CanonicalValueV1,
    name: &str,
    subject: &str,
) -> Result<&'a [CanonicalValueV1]> {
    match field(value, name, subject)? {
        CanonicalValueV1::Array(values) => Ok(values),
        _ => bail!("{subject}.{name} must be an array"),
    }
}

fn nonempty_text_field<'a>(
    value: &'a CanonicalValueV1,
    name: &str,
    subject: &str,
) -> Result<&'a str> {
    match field(value, name, subject)? {
        CanonicalValueV1::Text(value) if !value.is_empty() => Ok(value),
        _ => bail!("{subject}.{name} must be nonempty text"),
    }
}

fn require_text(value: &CanonicalValueV1, name: &str, subject: &str, expected: &str) -> Result<()> {
    let actual = nonempty_text_field(value, name, subject)?;
    if actual != expected {
        bail!("{subject}.{name} must equal {expected}, found {actual}");
    }
    Ok(())
}

fn bytes_field<'a>(value: &'a CanonicalValueV1, name: &str, subject: &str) -> Result<&'a [u8]> {
    match field(value, name, subject)? {
        CanonicalValueV1::Bytes(bytes) => Ok(bytes),
        _ => bail!("{subject}.{name} must be bytes"),
    }
}

fn hash_field(value: &CanonicalValueV1, name: &str, subject: &str) -> Result<[u8; 32]> {
    bytes_field(value, name, subject)?
        .try_into()
        .with_context(|| format!("{subject}.{name} must contain exactly 32 bytes"))
}

fn u64_field(value: &CanonicalValueV1, name: &str, subject: &str) -> Result<u64> {
    match field(value, name, subject)? {
        CanonicalValueV1::Integer(value) => {
            u64::try_from(*value).with_context(|| format!("{subject}.{name} must be a uint"))
        }
        _ => bail!("{subject}.{name} must be a uint"),
    }
}

fn budget_value(
    value: &CanonicalValueV1,
    steps: &str,
    read_bytes: &str,
    write_bytes: &str,
    subject: &str,
) -> Result<EchoOperationBudgetV1> {
    let budget = EchoOperationBudgetV1::new(
        u64_field(value, steps, subject)?,
        u64_field(value, read_bytes, subject)?,
        u64_field(value, write_bytes, subject)?,
    );
    if budget.steps() == 0 {
        bail!("{subject} must grant at least one step");
    }
    Ok(budget)
}

fn profile_digest(label: &str) -> [u8; 32] {
    domain_hash(b"echo:operation-profile:v1\0", label.as_bytes())
}

fn domain_hash(domain: &[u8], bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(domain);
    hasher.update(&(bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
    hasher.finalize().into()
}
