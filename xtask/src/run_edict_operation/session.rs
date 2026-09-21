// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Bounded JSONL driver for the native executable-operation host.
use super::{
    application_result_report, bail, build_host, current_state, decode_canonical_cbor_v1,
    domain_hash, echo_operation_action_envelope_v1,
    echo_operation_anchored_node_creation_application_basis_v1, echo_operation_package_id_v1, fs,
    install_package, parse_input, parse_package, read_bounded, validate_closure,
    validate_package_configuration, validate_verification_report, Context, Digest,
    EchoOperationActionOutcomeV1, EchoOperationAnchoredNodeOccupancyV1,
    EchoOperationInvocationAdmissionPolicyV1, EchoOperationInvocationV1, HostFixture,
    IngressTarget, NodeId, NodeKey, PackageMetadata, Result, RunEdictOperationConfig, Sha256,
    TargetConfiguration, TrustedRuntimeWalConfig, MAX_ARTIFACT_BYTES, MAX_INPUT_BYTES,
};
use serde_json::{json, Value};
use std::io::{BufRead, Read, Write};

struct Session {
    fixture: HostFixture,
    package: PackageMetadata,
    package_id: warp_core::EchoOperationPackageIdV1,
    grant: [u8; 32],
    configuration: TargetConfiguration,
    basis: String,
}

pub fn serve(config: RunEdictOperationConfig) -> Result<()> {
    let package_bytes = read_bounded(&config.package, MAX_ARTIFACT_BYTES, "package")?;
    let package_value = decode_canonical_cbor_v1(&package_bytes)?;
    let package = parse_package(&package_value)?;
    validate_verification_report(
        &read_bounded(
            &config.verification_report,
            MAX_ARTIFACT_BYTES,
            "verification report",
        )?,
        &package_value,
        &package.operation_coordinate,
        package.target_ir_identity,
        package.result_projection_identity,
    )?;
    let configuration = validate_closure(
        &read_bounded(&config.lawpack_manifest, MAX_ARTIFACT_BYTES, "manifest")?,
        &read_bounded(&config.lawpack_adapter, MAX_ARTIFACT_BYTES, "adapter")?,
        &read_bounded(
            &config.target_configuration,
            MAX_ARTIFACT_BYTES,
            "configuration",
        )?,
        &package.lawpack_coordinate,
        package.lawpack_identity,
        package.target_intrinsic,
    )?;
    validate_package_configuration(&package, &configuration)?;
    let input = parse_input(
        &read_bounded(&config.input, MAX_INPUT_BYTES, "bootstrap")?,
        &configuration,
    )?;
    fs::create_dir_all(&config.wal_dir)?;
    let mut fixture = build_host(&input.basis, &input.key, false)?;
    fixture
        .host
        .enable_runtime_wal(TrustedRuntimeWalConfig::filesystem(&config.wal_dir))?;
    let package_id = echo_operation_package_id_v1(&package_bytes);
    if fixture
        .host
        .engine()
        .installed_echo_operation_package_v1(package_id)
        .is_none()
    {
        install_package(&mut fixture.host, &package, package_id, package_bytes)?;
    }
    let grant = domain_hash(
        b"echo:edict-operation-runner-authority-grant:v1\0",
        &package_id.as_hash(),
    );
    fixture
        .host
        .install_echo_operation_action_admission_policy_v1(
            EchoOperationInvocationAdmissionPolicyV1::new(
                package.authority_profile_identity,
                grant,
                package.budget,
            ),
        );
    let mut session = Session {
        fixture,
        package,
        package_id,
        grant,
        configuration,
        basis: input.basis,
    };
    let mut input = std::io::stdin().lock();
    loop {
        let mut line = Vec::new();
        if input
            .by_ref()
            .take(MAX_INPUT_BYTES + 1)
            .read_until(b'\n', &mut line)?
            == 0
        {
            break;
        }
        if u64::try_from(line.len())? > MAX_INPUT_BYTES {
            bail!("request exceeds input limit");
        }
        let result = serde_json::from_slice(&line)
            .context("invalid JSON request")
            .and_then(|request| session.call(&request));
        let response = result.unwrap_or_else(|error| json!({"error": format!("{error:#}")}));
        println!("{}", serde_json::to_string(&response)?);
        std::io::stdout().flush()?;
    }
    Ok(())
}

fn text<'a>(request: &'a Value, key: &str) -> Result<&'a str> {
    request
        .get(key)
        .and_then(Value::as_str)
        .with_context(|| format!("missing string {key}"))
}

impl Session {
    fn node(&self, key: &str) -> NodeKey {
        NodeKey {
            warp_id: self.fixture.node.warp_id,
            local_id: NodeId(Sha256::digest(key.as_bytes()).into()),
        }
    }

    fn call(&mut self, request: &Value) -> Result<Value> {
        let allowed: &[&str] = match text(request, "op")? {
            "status" => &["op"],
            "observe" => &["op", "attempt", "keys"],
            "changes" | "reading" => &["op", "attempt"],
            "submit" => &["op", "attempt", "key", "value", "request_id"],
            "outcome" => &["op", "submission_id"],
            _ => bail!("unknown operation"),
        };
        if request
            .as_object()
            .context("request must be an object")?
            .keys()
            .any(|key| !allowed.contains(&key.as_str()))
        {
            bail!("unexpected request field; observation bindings are runtime-owned");
        }
        match text(request, "op")? {
            "status" => {
                let application_basis = echo_operation_anchored_node_creation_application_basis_v1(
                    self.fixture.node,
                    EchoOperationAnchoredNodeOccupancyV1::Absent,
                );
                let basis = self
                    .fixture
                    .host
                    .echo_operation_evaluation_basis_v1(self.fixture.head, application_basis)?;
                Ok(json!({
                    "worldline": hex::encode(self.fixture.head.worldline_id.as_bytes()),
                    "state_root": hex::encode(current_state(&self.fixture)?.state_root()),
                    "commit_id": hex::encode(basis.commit_id()),
                    "tick": basis.worldline_tick().as_u64(),
                }))
            }
            "observe" => {
                let attempt = text(request, "attempt")?;
                let keys = request
                    .get("keys")
                    .and_then(Value::as_array)
                    .context("missing keys")?;
                let nodes = keys
                    .iter()
                    .map(|key| {
                        key.as_str()
                            .map(|key| self.node(key))
                            .context("key must be a string")
                    })
                    .collect::<Result<Vec<_>>>()?;
                let observation = self.fixture.host.retain_echo_operation_observation_v1(
                    attempt,
                    self.fixture.head,
                    &nodes,
                )?;
                let readings = observation
                    .readings()
                    .map(|(node, value)| {
                        json!({
                            "node": hex::encode(node.local_id.0), "value_cbor": hex::encode(value)
                        })
                    })
                    .collect::<Vec<_>>();
                Ok(json!({"attempt":attempt, "readings":readings}))
            }
            "changes" => {
                let nodes = self
                    .fixture
                    .host
                    .echo_operation_observation_changes_v1(text(request, "attempt")?)?;
                let commits = self
                    .fixture
                    .host
                    .echo_operation_observation_change_commits_v1(text(request, "attempt")?)?;
                Ok(
                    json!({"changed":!nodes.is_empty(), "nodes":nodes.iter().map(|node| hex::encode(node.local_id.0)).collect::<Vec<_>>(), "commits":commits.iter().map(hex::encode).collect::<Vec<_>>()}),
                )
            }
            "reading" => {
                let observation = self
                    .fixture
                    .host
                    .echo_operation_observation_v1(text(request, "attempt")?)?;
                Ok(json!({"attempt":text(request,"attempt")?,
                    "observation_commit":hex::encode(observation.basis().commit_id()),
                    "readings":observation.readings().map(|(node, value)| json!({"node":hex::encode(node.local_id.0),"value_cbor":hex::encode(value)})).collect::<Vec<_>>() }))
            }
            "submit" => {
                let attempt = text(request, "attempt")?;
                let key = text(request, "key")?;
                let input = parse_input(
                    &serde_json::to_vec(
                        &json!({"basis":self.basis, "key":key, "value":text(request,"value")?}),
                    )?,
                    &self.configuration,
                )?;
                let node = self.node(key);
                let store = current_state(&self.fixture)?
                    .store(&node.warp_id)
                    .context("warp unavailable")?;
                let occupancy = match (
                    store.node(&node.local_id).is_some(),
                    store.node_attachment(&node.local_id).is_some(),
                ) {
                    (false, false) => EchoOperationAnchoredNodeOccupancyV1::Absent,
                    (true, false) => EchoOperationAnchoredNodeOccupancyV1::NodeOnly,
                    (false, true) => EchoOperationAnchoredNodeOccupancyV1::AttachmentOnly,
                    (true, true) => EchoOperationAnchoredNodeOccupancyV1::NodeAndAttachment,
                };
                let application_basis =
                    echo_operation_anchored_node_creation_application_basis_v1(node, occupancy);
                let basis = self
                    .fixture
                    .host
                    .echo_operation_evaluation_basis_v1(self.fixture.head, application_basis)?;
                let invocation = EchoOperationInvocationV1::anchored_node_attachment_create_if_absent_with_application_input(
                    self.package_id, &self.package.operation_coordinate, basis, self.grant, self.package.budget,
                    node, input.replacement, input.canonical_bytes,
                );
                let request_id = request
                    .get("request_id")
                    .and_then(Value::as_str)
                    .unwrap_or(attempt);
                let invocation = self
                    .fixture
                    .host
                    .bind_echo_operation_request_v1(request_id, attempt, invocation)?;
                let envelope = echo_operation_action_envelope_v1(
                    IngressTarget::ExactHead {
                        key: self.fixture.head,
                    },
                    invocation,
                )?;
                let submission = self
                    .fixture
                    .host
                    .app()
                    .submit_intent_with_runtime_wal_ack(envelope)?
                    .submission_id;
                if self
                    .fixture
                    .host
                    .echo_operation_action_outcome_v1(&submission)
                    .is_none()
                {
                    self.fixture.host.tick_once()?;
                }
                self.outcome(submission)
            }
            "outcome" => {
                let bytes: [u8; 32] = hex::decode(text(request, "submission_id")?)?
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("invalid submission identity"))?;
                self.outcome(bytes)
            }
            _ => bail!("unknown operation"),
        }
    }

    fn outcome(&self, submission: [u8; 32]) -> Result<Value> {
        let disposition = self
            .fixture
            .host
            .echo_operation_action_outcome_v1(&submission)
            .context("outcome unavailable")?;
        let mut result = json!({"submission_id":hex::encode(submission)});
        match disposition {
            EchoOperationActionOutcomeV1::Committed(receipt) => {
                result["outcome"] = json!("committed");
                result["commit_id"] = json!(hex::encode(receipt.commit_id()));
                result["receipt_digest"] = json!(hex::encode(receipt.digest()));
                result["application_result"] = serde_json::to_value(application_result_report(
                    receipt
                        .committed_application_result()
                        .context("application result unavailable")?,
                ))?;
            }
            EchoOperationActionOutcomeV1::Obstructed(obstruction) => {
                result["outcome"] = json!(format!("{:?}", obstruction.kind()));
            }
            EchoOperationActionOutcomeV1::RejectedFootprintConflict(_) => {
                result["outcome"] = json!("FootprintConflict");
            }
        }
        Ok(result)
    }
}
