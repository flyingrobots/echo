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
    heads: std::collections::BTreeMap<String, warp_core::WriterHeadKey>,
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
    // One root-level observed slot costs two steps and 64 bytes plus its
    // nonempty encoded value; the create operation also reads 64 bytes. These
    // are necessary lower bounds, not a promise that every aperture fits.
    if package.budget.read_bytes() <= 128 || package.budget.steps() < 3 {
        bail!("insufficient budget for an observation-bound session: compile an observation-capable profile with more than 128 read bytes and at least 3 steps; the full aperture and operation remain subject to the admitted ceiling");
    }
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
        heads: std::collections::BTreeMap::from([("parent".to_owned(), fixture.head)]),
        fixture,
        package,
        package_id,
        grant,
        configuration,
        basis: input.basis,
    };
    if config.retained_alternatives {
        // The initial application operation is retained once. Recovery resolves
        // its original binding; it never re-executes it to rebuild history.
        if session
            .fixture
            .host
            .echo_operation_observation_v1("host-bootstrap")
            .is_err()
        {
            let result = session
                .call(&json!({"op":"observe","attempt":"host-bootstrap","keys":["bootstrap"]}))?;
            if result.get("error").is_some() {
                bail!("bootstrap observation failed");
            }
        }
        let result = session.call(
            &json!({"op":"submit","attempt":"host-bootstrap","key":"bootstrap","value":""}),
        )?;
        if result["outcome"] != "committed" {
            bail!("bootstrap operation failed: {result}");
        }
        for label in ["a", "b"] {
            let head = session
                .fixture
                .host
                .fork_local_operation_strand_v1(session.fixture.head, label)?;
            session.heads.insert(label.to_owned(), head);
        }
    }
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
        if std::env::var_os("ECHO_WAL_PROFILE").is_some() {
            let (scans, frames) = warp_core::causal_wal::filesystem_recovery_work();
            eprintln!("{}", json!({"wal_profile":{"scans":scans,"frames":frames}}));
        }
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
            "status" | "ancestry" => &["op"],
            "use" => &["op", "lane"],
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
            "use" => {
                self.fixture.head = *self
                    .heads
                    .get(text(request, "lane")?)
                    .context("unknown lane")?;
                self.call(&json!({"op":"status"}))
            }
            "ancestry" => {
                use warp_core::ProvenanceStore;
                let provenance = self.fixture.host.provenance();
                let lane = self.fixture.head.worldline_id;
                let len = provenance.len(lane)?;
                if len > 4096 {
                    bail!("ancestry aperture exceeded");
                }
                let commits = (0..len)
                    .map(|tick| {
                        provenance
                            .entry(lane, warp_core::WorldlineTick::from_raw(tick))
                            .map(|entry| hex::encode(entry.expected.commit_hash))
                    })
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                Ok(json!({"worldline":hex::encode(lane.as_bytes()),"commits":commits}))
            }
            "status" => {
                let application_basis = echo_operation_anchored_node_creation_application_basis_v1(
                    self.fixture.node,
                    EchoOperationAnchoredNodeOccupancyV1::Absent,
                );
                let basis = self
                    .fixture
                    .host
                    .echo_operation_evaluation_basis_v1(self.fixture.head, application_basis)?;
                let strand = self
                    .fixture
                    .host
                    .runtime()
                    .strands()
                    .find_by_child_worldline(&self.fixture.head.worldline_id);
                let fork = strand.map(|strand| {
                    let basis = strand.fork_basis_ref();
                    json!({
                        "source_worldline":hex::encode(basis.source_lane_id.as_bytes()),
                        "tick":basis.fork_tick.as_u64(),"commit":hex::encode(basis.commit_hash)
                    })
                });
                Ok(json!({
                    "fork": fork,
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
                let (nodes, commits) = self
                    .fixture
                    .host
                    .echo_operation_observation_change_evidence_v1(text(request, "attempt")?)?;
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
