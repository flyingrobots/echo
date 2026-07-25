// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Thirty-second Hello Echo evidence capsule.

use anyhow::{bail, Context, Result};
use bytes::Bytes;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::Instant;
use warp_core::inbox::INTENT_ATTACHMENT_TYPE;
use warp_core::wsc::{build_one_warp_input, validate_wsc, write_wsc_one_warp, WscFile};
use warp_core::{
    derive_witnessed_suffix_shell_digest, export_suffix, import_suffix, make_edge_id,
    make_intent_kind, make_node_id, make_type_id, AtomPayload, AttachmentKey, AttachmentValue,
    CausalSuffixBundle, ConflictPolicy, EdgeRecord, ExportSuffixRequest, Footprint, GraphStore,
    GraphView, Hash, ImportSuffixRequest, IngressEnvelope, IngressTarget, NodeId, NodeKey,
    NodeRecord, PatternGraph, ProvenanceRef, RewriteRule, TickDelta, TickReceipt,
    TickReceiptDisposition, TickReceiptRejection, WarpOp, WitnessedSuffixAdmissionContext,
    WitnessedSuffixAdmissionOutcome, WitnessedSuffixExportContext,
    WitnessedSuffixLocalAdmissionPosture, WorldlineId, WorldlineState, WorldlineTick,
};

const COUNTER_RULE_NAME: &str = "cmd/hello_echo_counter";
const COUNTER_VALUE_TYPE: &str = "hello-echo/counter-value/v1";
const COUNTER_NODE: &str = "hello-echo/counter";
const INTENT_PREFIX: &str = "hello-echo.intent/v1;";
const WSC_SCHEMA: &str = "hello-echo/wsc-schema/v1";

pub(crate) struct HelloEchoConfig {
    pub(crate) out_dir: PathBuf,
    pub(crate) max_ms: u64,
}

#[derive(Serialize)]
pub(crate) struct HelloEchoReport {
    pub(crate) demo: &'static str,
    pub(crate) verdict: &'static str,
    pub(crate) elapsed_ms: u128,
    pub(crate) max_ms: u64,
    pub(crate) deterministic_evidence_digest: String,
    pub(crate) features: Vec<FeatureEvidence>,
    pub(crate) worldline: WorldlineReport,
    pub(crate) ingress: IngressReport,
    pub(crate) receipt: ReceiptReport,
    pub(crate) counter: CounterReport,
    pub(crate) hashes: HashReport,
    pub(crate) patch: PatchReport,
    pub(crate) wsc: WscReport,
    pub(crate) continuum: ContinuumReport,
    pub(crate) inspection: InspectionReport,
}

#[derive(Serialize)]
pub(crate) struct FeatureEvidence {
    pub(crate) feature: &'static str,
    pub(crate) witness: String,
}

#[derive(Serialize)]
pub(crate) struct WorldlineReport {
    pub(crate) id: String,
    pub(crate) root_warp: String,
    pub(crate) root_node: String,
    pub(crate) current_tick: u64,
    pub(crate) explicit_basis: String,
}

#[derive(Serialize)]
pub(crate) struct IngressReport {
    pub(crate) intent_kind: String,
    pub(crate) submitted: usize,
    pub(crate) ingress_ids: Vec<String>,
    pub(crate) payloads: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct ReceiptReport {
    pub(crate) tx: u64,
    pub(crate) entries: Vec<ReceiptEntryReport>,
    pub(crate) applied_count: usize,
    pub(crate) rejected_count: usize,
    pub(crate) conflict_count: usize,
}

#[derive(Serialize)]
pub(crate) struct ReceiptEntryReport {
    pub(crate) index: usize,
    pub(crate) rule_id: String,
    pub(crate) scope: String,
    pub(crate) disposition: &'static str,
    pub(crate) blocked_by: Vec<u32>,
}

#[derive(Serialize)]
pub(crate) struct CounterReport {
    pub(crate) node: String,
    pub(crate) value: u64,
    pub(crate) submitted_increment_total: u64,
    pub(crate) applied_increment_total: u64,
}

#[derive(Serialize)]
pub(crate) struct HashReport {
    pub(crate) commit_hash: String,
    pub(crate) state_root: String,
    pub(crate) plan_digest: String,
    pub(crate) decision_digest: String,
    pub(crate) receipt_digest: String,
    pub(crate) decision_matches_receipt: bool,
    pub(crate) rewrites_digest: String,
    pub(crate) patch_digest: String,
    pub(crate) patch_matches_snapshot: bool,
    pub(crate) wsc_file_digest: String,
    pub(crate) wsc_warp_state_root: String,
}

#[derive(Serialize)]
pub(crate) struct PatchReport {
    pub(crate) ops: usize,
    pub(crate) in_slots: usize,
    pub(crate) out_slots: usize,
    pub(crate) commit_status: &'static str,
}

#[derive(Serialize)]
pub(crate) struct WscReport {
    pub(crate) path: String,
    pub(crate) verified: bool,
    pub(crate) tick: u64,
    pub(crate) warp_count: usize,
    pub(crate) nodes: usize,
    pub(crate) edges: usize,
    pub(crate) node_attachments: usize,
    pub(crate) edge_attachments: usize,
    pub(crate) bytes: usize,
}

#[derive(Serialize)]
pub(crate) struct ContinuumReport {
    pub(crate) bundle_digest: String,
    pub(crate) shell_digest: String,
    pub(crate) source_entries: usize,
    pub(crate) outcome: &'static str,
    pub(crate) admitted_refs: usize,
}

#[derive(Serialize)]
pub(crate) struct InspectionReport {
    pub(crate) inspect_tree: String,
    pub(crate) verify_wsc: String,
}

pub(crate) fn run(config: HelloEchoConfig) -> Result<HelloEchoReport> {
    let start = Instant::now();
    let mut artifacts = run_capsule(&config.out_dir)?;
    artifacts.elapsed_ms = start.elapsed().as_millis();

    if artifacts.elapsed_ms > u128::from(config.max_ms) {
        bail!(
            "Hello Echo demo exceeded budget: {}ms > {}ms",
            artifacts.elapsed_ms,
            config.max_ms
        );
    }

    Ok(artifacts.into_report(config.max_ms))
}

struct CapsuleArtifacts {
    elapsed_ms: u128,
    worldline_id: WorldlineId,
    ingress: Vec<IngressEnvelope>,
    state: WorldlineState,
    outcome: warp_core::CommitOutcome,
    counter_value: u64,
    wsc_path: PathBuf,
    wsc_file_digest: Hash,
    wsc_warp_state_root: Hash,
    wsc_verified: bool,
    wsc_counts: WscCounts,
    continuum: ContinuumArtifacts,
}

struct WscCounts {
    tick: u64,
    warp_count: usize,
    nodes: usize,
    edges: usize,
    node_attachments: usize,
    edge_attachments: usize,
    bytes: usize,
}

struct ContinuumArtifacts {
    bundle: CausalSuffixBundle,
    outcome: &'static str,
    admitted_refs: usize,
}

impl CapsuleArtifacts {
    fn into_report(self, max_ms: u64) -> HelloEchoReport {
        let root = self.state.root();
        let receipt = self.outcome.receipt;
        let patch = self.outcome.patch;
        let snapshot = self.outcome.snapshot;
        let ingress_ids = self
            .ingress
            .iter()
            .map(|envelope| hash_hex(&envelope.ingress_id()))
            .collect::<Vec<_>>();
        let payloads = self
            .ingress
            .iter()
            .map(|envelope| match envelope.payload() {
                warp_core::IngressPayload::LocalIntent { intent_bytes, .. } => {
                    String::from_utf8_lossy(intent_bytes).into_owned()
                }
            })
            .collect::<Vec<_>>();
        let receipt_report = receipt_report(&receipt);
        let submitted_increment_total = payloads
            .iter()
            .filter_map(|payload| parse_amount(payload.as_bytes()))
            .sum::<u64>();
        let applied_increment_total = self.counter_value;
        let deterministic_evidence_digest = deterministic_evidence_digest(
            &snapshot.hash,
            &receipt.digest(),
            &patch.digest(),
            &self.wsc_file_digest,
            &self.continuum.bundle.bundle_digest,
            self.counter_value,
        );
        let wsc_path = self.wsc_path.display().to_string();
        let wsc_warp_state_root = hash_hex(&self.wsc_warp_state_root);
        let features = feature_matrix(
            self.ingress.len(),
            receipt_report.applied_count,
            receipt_report.conflict_count,
            patch.ops().len(),
            &wsc_path,
            self.continuum.outcome,
        );
        let inspect_tree = format!("cargo run -p warp-cli -- inspect {wsc_path} --tree");
        let verify_wsc =
            format!("cargo run -p warp-cli -- verify {wsc_path} --expected {wsc_warp_state_root}");

        HelloEchoReport {
            demo: "hello-echo",
            verdict: "pass",
            elapsed_ms: self.elapsed_ms,
            max_ms,
            deterministic_evidence_digest,
            features,
            worldline: WorldlineReport {
                id: hex::encode(self.worldline_id.as_bytes()),
                root_warp: hash_hex(root.warp_id.as_bytes()),
                root_node: hash_hex(root.local_id.as_bytes()),
                current_tick: self.state.current_tick().as_u64(),
                explicit_basis: "u0 -> tick:1".to_string(),
            },
            ingress: IngressReport {
                intent_kind: hash_hex(make_intent_kind("hello-echo/increment").as_hash()),
                submitted: self.ingress.len(),
                ingress_ids,
                payloads,
            },
            receipt: receipt_report,
            counter: CounterReport {
                node: hash_hex(counter_node_id().as_bytes()),
                value: self.counter_value,
                submitted_increment_total,
                applied_increment_total,
            },
            hashes: HashReport {
                commit_hash: hash_hex(&snapshot.hash),
                state_root: hash_hex(&snapshot.state_root),
                plan_digest: hash_hex(&snapshot.plan_digest),
                decision_digest: hash_hex(&snapshot.decision_digest),
                receipt_digest: hash_hex(&receipt.digest()),
                decision_matches_receipt: snapshot.decision_digest == receipt.digest(),
                rewrites_digest: hash_hex(&snapshot.rewrites_digest),
                patch_digest: hash_hex(&patch.digest()),
                patch_matches_snapshot: snapshot.patch_digest == patch.digest(),
                wsc_file_digest: hash_hex(&self.wsc_file_digest),
                wsc_warp_state_root,
            },
            patch: PatchReport {
                ops: patch.ops().len(),
                in_slots: patch.in_slots().len(),
                out_slots: patch.out_slots().len(),
                commit_status: "committed",
            },
            wsc: WscReport {
                path: wsc_path,
                verified: self.wsc_verified,
                tick: self.wsc_counts.tick,
                warp_count: self.wsc_counts.warp_count,
                nodes: self.wsc_counts.nodes,
                edges: self.wsc_counts.edges,
                node_attachments: self.wsc_counts.node_attachments,
                edge_attachments: self.wsc_counts.edge_attachments,
                bytes: self.wsc_counts.bytes,
            },
            continuum: ContinuumReport {
                bundle_digest: hash_hex(&self.continuum.bundle.bundle_digest),
                shell_digest: hash_hex(&self.continuum.bundle.source_suffix.witness_digest),
                source_entries: self.continuum.bundle.source_suffix.source_entries.len(),
                outcome: self.continuum.outcome,
                admitted_refs: self.continuum.admitted_refs,
            },
            inspection: InspectionReport {
                inspect_tree,
                verify_wsc,
            },
        }
    }
}

fn run_capsule(out_dir: &Path) -> Result<CapsuleArtifacts> {
    std::fs::create_dir_all(out_dir)
        .with_context(|| format!("failed to create {}", out_dir.display()))?;

    let root_node = make_node_id("hello-echo/root");
    let store = initial_store(root_node);
    let mut engine = warp_core::Engine::new(store.clone(), root_node);
    engine
        .register_rule(counter_rule())
        .context("failed to register Hello Echo counter rule")?;

    let mut state =
        WorldlineState::from_root_store(store, root_node).context("failed to build worldline")?;
    let worldline_id = WorldlineId::from_bytes([0xE0; 32]);
    let intent_kind = make_intent_kind("hello-echo/increment");
    let ingress = vec![
        IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            intent_kind,
            intent_bytes("alice", 1),
        ),
        IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            intent_kind,
            intent_bytes("bob", 1),
        ),
    ];

    let outcome = engine
        .commit_with_state(&mut state, &ingress)
        .context("failed to commit Hello Echo runtime tick")?;
    let counter_value = read_counter_from_state(&state)?;
    let (wsc_path, wsc_file_digest, wsc_warp_state_root, wsc_verified, wsc_counts) =
        write_and_validate_wsc(&state, out_dir)?;
    let continuum = continuum_shell(worldline_id, &outcome.snapshot)?;

    Ok(CapsuleArtifacts {
        elapsed_ms: 0,
        worldline_id,
        ingress,
        state,
        outcome,
        counter_value,
        wsc_path,
        wsc_file_digest,
        wsc_warp_state_root,
        wsc_verified,
        wsc_counts,
        continuum,
    })
}

fn initial_store(root: NodeId) -> GraphStore {
    let counter = counter_node_id();
    let mut store = GraphStore::default();
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id("hello-echo/world"),
        },
    );
    store.insert_node(
        counter,
        NodeRecord {
            ty: make_type_id("hello-echo/counter"),
        },
    );
    store.insert_edge(
        root,
        EdgeRecord {
            id: make_edge_id("hello-echo/root/counter"),
            from: root,
            to: counter,
            ty: make_type_id("hello-echo/contains"),
        },
    );
    store.set_node_attachment(counter, Some(counter_attachment(0)));
    store
}

fn counter_rule() -> RewriteRule {
    RewriteRule {
        id: make_type_id("rule:cmd/hello_echo_counter").0,
        name: COUNTER_RULE_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: counter_matcher,
        executor: counter_executor,
        compute_footprint: counter_footprint,
        factor_mask: 1,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

fn counter_matcher(view: GraphView<'_>, scope: &NodeId) -> bool {
    read_intent_amount(&view, scope).is_some()
}

fn counter_executor(view: GraphView<'_>, scope: &NodeId, delta: &mut TickDelta) {
    let Some(amount) = read_intent_amount(&view, scope) else {
        return;
    };
    let current = view
        .node_attachment(&counter_node_id())
        .and_then(counter_from_attachment)
        .unwrap_or(0);
    let Some(next) = current.checked_add(amount) else {
        return;
    };
    let key = AttachmentKey::node_alpha(NodeKey {
        warp_id: view.warp_id(),
        local_id: counter_node_id(),
    });
    delta.push(WarpOp::SetAttachment {
        key,
        value: Some(counter_attachment(next)),
    });
}

fn counter_footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
    let warp_id = view.warp_id();
    let event = NodeKey {
        warp_id,
        local_id: *scope,
    };
    let counter = NodeKey {
        warp_id,
        local_id: counter_node_id(),
    };
    let mut footprint = Footprint::default();
    footprint.n_read.insert(event);
    footprint.n_read.insert(counter);
    footprint.a_read.insert(AttachmentKey::node_alpha(event));
    footprint.a_read.insert(AttachmentKey::node_alpha(counter));
    footprint.a_write.insert(AttachmentKey::node_alpha(counter));
    footprint.factor_mask = 1;
    footprint
}

fn read_intent_amount(view: &GraphView<'_>, scope: &NodeId) -> Option<u64> {
    let attachment = view.node_attachment(scope)?;
    let AttachmentValue::Atom(atom) = attachment else {
        return None;
    };
    if atom.type_id != make_type_id(INTENT_ATTACHMENT_TYPE) {
        return None;
    }
    parse_amount(atom.bytes.as_ref())
}

fn parse_amount(bytes: &[u8]) -> Option<u64> {
    let text = std::str::from_utf8(bytes).ok()?;
    let rest = text.strip_prefix(INTENT_PREFIX)?;
    rest.split(';').find_map(|part| {
        part.strip_prefix("amount=")
            .and_then(|amount| amount.parse::<u64>().ok())
    })
}

fn intent_bytes(client: &str, amount: u64) -> Vec<u8> {
    format!("{INTENT_PREFIX}op=increment;amount={amount};client={client}").into_bytes()
}

fn counter_node_id() -> NodeId {
    make_node_id(COUNTER_NODE)
}

fn counter_attachment(value: u64) -> AttachmentValue {
    AttachmentValue::Atom(AtomPayload::new(
        make_type_id(COUNTER_VALUE_TYPE),
        Bytes::copy_from_slice(&value.to_le_bytes()),
    ))
}

fn counter_from_attachment(value: &AttachmentValue) -> Option<u64> {
    let AttachmentValue::Atom(atom) = value else {
        return None;
    };
    if atom.type_id != make_type_id(COUNTER_VALUE_TYPE) || atom.bytes.len() != 8 {
        return None;
    }
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(atom.bytes.as_ref());
    Some(u64::from_le_bytes(bytes))
}

fn read_counter_from_state(state: &WorldlineState) -> Result<u64> {
    let store = state
        .store(&state.root().warp_id)
        .context("worldline root store is missing")?;
    let value = store
        .node_attachment(&counter_node_id())
        .and_then(counter_from_attachment)
        .context("counter attachment is missing or malformed")?;
    Ok(value)
}

fn write_and_validate_wsc(
    state: &WorldlineState,
    out_dir: &Path,
) -> Result<(PathBuf, Hash, Hash, bool, WscCounts)> {
    let store = state
        .store(&state.root().warp_id)
        .context("worldline root store is missing")?;
    let input = build_one_warp_input(store, state.root().local_id);
    let bytes = write_wsc_one_warp(
        &input,
        make_type_id(WSC_SCHEMA).0,
        state.current_tick().as_u64(),
    )
    .context("failed to write WSC bytes")?;
    let path = out_dir.join("counter.wsc");
    std::fs::write(&path, &bytes).with_context(|| format!("failed to write {}", path.display()))?;

    let file =
        WscFile::open(&path).with_context(|| format!("failed to open {}", path.display()))?;
    validate_wsc(&file).with_context(|| format!("failed to validate {}", path.display()))?;
    let view = file.warp_view(0).context("failed to read WSC warp 0")?;
    let node_attachments = view
        .nodes()
        .iter()
        .enumerate()
        .map(|(index, _)| view.node_attachments(index).len())
        .sum();
    let edge_attachments = view
        .edges()
        .iter()
        .enumerate()
        .map(|(index, _)| view.edge_attachments(index).len())
        .sum();
    let counts = WscCounts {
        tick: file.tick(),
        warp_count: file.warp_count(),
        nodes: view.nodes().len(),
        edges: view.edges().len(),
        node_attachments,
        edge_attachments,
        bytes: bytes.len(),
    };
    Ok((
        path,
        blake3::hash(&bytes).into(),
        store.canonical_state_hash(),
        true,
        counts,
    ))
}

fn continuum_shell(
    worldline_id: WorldlineId,
    snapshot: &warp_core::Snapshot,
) -> Result<ContinuumArtifacts> {
    let base = ProvenanceRef {
        worldline_id,
        worldline_tick: WorldlineTick::ZERO,
        commit_hash: warp_core::digest_len0_u64(),
    };
    let target = ProvenanceRef {
        worldline_id,
        worldline_tick: WorldlineTick::from_raw(1),
        commit_hash: snapshot.hash,
    };
    let export_request = ExportSuffixRequest {
        source_worldline_id: worldline_id,
        base_frontier: base,
        target_frontier: Some(target),
        basis_report: None,
    };
    let export_context = DemoSuffixExportContext {
        source_entry: target,
        boundary_witness: base,
    };
    let bundle = export_suffix(&export_request, &export_context)
        .map_err(|obstruction| anyhow::anyhow!("suffix export obstructed: {obstruction:?}"))?;
    let import_request = ImportSuffixRequest {
        bundle: bundle.clone(),
        target_worldline_id: worldline_id,
        target_basis: base,
        basis_report: None,
    };
    let admission_context = DemoSuffixAdmissionContext {
        target_basis: base,
        admitted_ref: target,
    };
    let import = import_suffix(&import_request, &admission_context);
    let (outcome, admitted_refs) = match &import.admission.outcome {
        WitnessedSuffixAdmissionOutcome::Admitted { admitted_refs, .. } => {
            ("admitted", admitted_refs.len())
        }
        WitnessedSuffixAdmissionOutcome::Staged { staged_refs, .. } => {
            ("staged", staged_refs.len())
        }
        WitnessedSuffixAdmissionOutcome::Plural { candidate_refs, .. } => {
            ("plural", candidate_refs.len())
        }
        WitnessedSuffixAdmissionOutcome::Conflict { .. } => ("conflict", 0),
        WitnessedSuffixAdmissionOutcome::Obstructed { .. } => ("obstructed", 0),
    };
    Ok(ContinuumArtifacts {
        bundle,
        outcome,
        admitted_refs,
    })
}

struct DemoSuffixExportContext {
    source_entry: ProvenanceRef,
    boundary_witness: ProvenanceRef,
}

impl WitnessedSuffixExportContext for DemoSuffixExportContext {
    fn source_entries(&self, _request: &ExportSuffixRequest) -> Option<Vec<ProvenanceRef>> {
        Some(vec![self.source_entry])
    }

    fn boundary_witness(&self, _request: &ExportSuffixRequest) -> Option<ProvenanceRef> {
        Some(self.boundary_witness)
    }
}

struct DemoSuffixAdmissionContext {
    target_basis: ProvenanceRef,
    admitted_ref: ProvenanceRef,
}

impl WitnessedSuffixAdmissionContext for DemoSuffixAdmissionContext {
    fn source_shell_digest(&self, shell: &warp_core::WitnessedSuffixShell) -> Option<Hash> {
        Some(derive_witnessed_suffix_shell_digest(shell))
    }

    fn resolve_target_basis(&self, target_basis: ProvenanceRef) -> Option<ProvenanceRef> {
        (target_basis == self.target_basis).then_some(target_basis)
    }

    fn local_admission_posture(
        &self,
        _request: &warp_core::WitnessedSuffixAdmissionRequest,
    ) -> WitnessedSuffixLocalAdmissionPosture {
        WitnessedSuffixLocalAdmissionPosture::Admissible {
            admitted_refs: vec![self.admitted_ref],
        }
    }
}

fn receipt_report(receipt: &TickReceipt) -> ReceiptReport {
    let mut applied_count = 0usize;
    let mut rejected_count = 0usize;
    let mut conflict_count = 0usize;
    let mut entries = Vec::with_capacity(receipt.entries().len());

    for (index, entry) in receipt.entries().iter().enumerate() {
        let disposition = match entry.disposition {
            TickReceiptDisposition::Applied => {
                applied_count += 1;
                "applied"
            }
            TickReceiptDisposition::Rejected(TickReceiptRejection::FootprintConflict) => {
                rejected_count += 1;
                conflict_count += 1;
                "rejected:footprint-conflict"
            }
            TickReceiptDisposition::Rejected(
                TickReceiptRejection::ExecutableOperationObstruction,
            ) => {
                rejected_count += 1;
                "rejected:executable-operation-obstruction"
            }
        };
        entries.push(ReceiptEntryReport {
            index,
            rule_id: hash_hex(&entry.rule_id),
            scope: hash_hex(entry.scope.local_id.as_bytes()),
            disposition,
            blocked_by: receipt.blocked_by(index).to_vec(),
        });
    }

    ReceiptReport {
        tx: receipt.tx().value(),
        entries,
        applied_count,
        rejected_count,
        conflict_count,
    }
}

fn feature_matrix(
    ingress_count: usize,
    applied_count: usize,
    conflict_count: usize,
    patch_ops: usize,
    wsc_path: &str,
    continuum_outcome: &str,
) -> Vec<FeatureEvidence> {
    vec![
        FeatureEvidence {
            feature: "explicit causal basis",
            witness: "worldline reports U0 -> tick:1 and root warp/node coordinates".to_string(),
        },
        FeatureEvidence {
            feature: "canonical ingress",
            witness: format!("{ingress_count} content-addressed local intent envelopes"),
        },
        FeatureEvidence {
            feature: "runtime-owned tick",
            witness: "Engine::commit_with_state admitted and committed the scheduler tick"
                .to_string(),
        },
        FeatureEvidence {
            feature: "domain semantics outside Echo core",
            witness: format!("repo-owned {COUNTER_RULE_NAME} rule hosts toy counter semantics"),
        },
        FeatureEvidence {
            feature: "receipt evidence",
            witness: format!("{applied_count} applied candidate and {conflict_count} obstruction"),
        },
        FeatureEvidence {
            feature: "footprint conflict detection",
            witness: "two increment intents target the same counter attachment slot".to_string(),
        },
        FeatureEvidence {
            feature: "canonical hashing",
            witness:
                "report includes commit, state, plan, decision, receipt, patch, and WSC digests"
                    .to_string(),
        },
        FeatureEvidence {
            feature: "replayable patch boundary",
            witness: format!("{patch_ops} canonical patch operations emitted"),
        },
        FeatureEvidence {
            feature: "WSC materialized reading",
            witness: format!("validated single-warp WSC artifact at {wsc_path}"),
        },
        FeatureEvidence {
            feature: "witnessed suffix posture",
            witness: format!("shape-only Continuum shell classified as {continuum_outcome}"),
        },
    ]
}

fn deterministic_evidence_digest(
    commit_hash: &Hash,
    receipt_digest: &Hash,
    patch_digest: &Hash,
    wsc_file_digest: &Hash,
    suffix_bundle_digest: &Hash,
    counter_value: u64,
) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"hello-echo/evidence/v1");
    hasher.update(commit_hash);
    hasher.update(receipt_digest);
    hasher.update(patch_digest);
    hasher.update(wsc_file_digest);
    hasher.update(suffix_bundle_digest);
    hasher.update(&counter_value.to_le_bytes());
    hash_hex(hasher.finalize().as_bytes())
}

fn hash_hex(hash: &Hash) -> String {
    hex::encode(hash)
}
