// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! External-consumer witness for trusted causal-anchor admission.

#![cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
#![allow(clippy::expect_used)]

use std::fmt::Write;

use warp_core::{
    make_node_id, make_type_id, CausalAnchorAdmissionRequest, CausalAnchorAppRootRole,
    CausalAnchorCasRole, CausalAnchorGraphRole, CausalAnchorPurpose, CausalAnchorRoot,
    CausalAnchorRootSupportGrant, CausalAnchorRootSupportPolicy, CausalAnchorSubject,
    CausalFrontierRef, EngineBuilder, GraphStore, Hash, NodeRecord, RecoveredCausalAnchorAdmission,
    TrustedRuntimeHost, WorldlineRuntime, CAUSAL_ANCHOR_SCHEMA_VERSION,
};

fn empty_engine() -> warp_core::Engine {
    let mut store = GraphStore::default();
    let root = make_node_id("causal-anchor-external-consumer/root");
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id("causal-anchor-external-consumer/world"),
        },
    );
    EngineBuilder::new(store, root).workers(1).build()
}

fn request(basis_frontier: CausalFrontierRef) -> CausalAnchorAdmissionRequest {
    CausalAnchorAdmissionRequest {
        schema_version: CAUSAL_ANCHOR_SCHEMA_VERSION,
        subject: CausalAnchorSubject::new("jedit", "BufferWorldline", "worldline:main"),
        basis_frontier,
        retained_roots: vec![CausalAnchorRoot::AppSubjectRoot {
            app_id: "jedit".to_owned(),
            subject_kind: "RopeHead".to_owned(),
            id: "head:0001".to_owned(),
            role: CausalAnchorAppRootRole::Authority,
        }],
        materialization_roots: vec![CausalAnchorRoot::CasObject {
            id: [0x44; 32],
            role: CausalAnchorCasRole::Materialization,
        }],
        purpose: CausalAnchorPurpose::UserSave,
    }
}

fn support_policy(request: &CausalAnchorAdmissionRequest) -> CausalAnchorRootSupportPolicy {
    CausalAnchorRootSupportPolicy::new(
        request
            .retained_roots
            .iter()
            .cloned()
            .map(|root| CausalAnchorRootSupportGrant::retained(request.subject.clone(), root))
            .chain(request.materialization_roots.iter().cloned().map(|root| {
                CausalAnchorRootSupportGrant::materialization(request.subject.clone(), root)
            })),
    )
}

fn hash_hex(hash: &Hash) -> String {
    hex::encode(hash)
}

fn purpose_label(purpose: CausalAnchorPurpose) -> &'static str {
    match purpose {
        CausalAnchorPurpose::Recovery => "recovery",
        CausalAnchorPurpose::Retention => "retention",
        CausalAnchorPurpose::Export => "export",
        CausalAnchorPurpose::UserSave => "user-save",
        CausalAnchorPurpose::Autosave => "autosave",
        CausalAnchorPurpose::Debug => "debug",
        CausalAnchorPurpose::CacheWarm => "cache-warm",
    }
}

fn cas_role_label(role: CausalAnchorCasRole) -> &'static str {
    match role {
        CausalAnchorCasRole::Materialization => "materialization",
        CausalAnchorCasRole::Manifest => "manifest",
        CausalAnchorCasRole::Index => "index",
    }
}

fn graph_role_label(role: CausalAnchorGraphRole) -> &'static str {
    match role {
        CausalAnchorGraphRole::Authority => "authority",
        CausalAnchorGraphRole::Evidence => "evidence",
        CausalAnchorGraphRole::Index => "index",
    }
}

fn app_role_label(role: CausalAnchorAppRootRole) -> &'static str {
    match role {
        CausalAnchorAppRootRole::Authority => "authority",
        CausalAnchorAppRootRole::Evidence => "evidence",
    }
}

fn root_label(root: &CausalAnchorRoot) -> String {
    match root {
        CausalAnchorRoot::CasObject { id, role } => {
            format!("echo.cas.Object:{}:{}", cas_role_label(*role), hash_hex(id))
        }
        CausalAnchorRoot::GraphFact { id, role } => {
            format!(
                "echo.graph.Fact:{}:{}",
                graph_role_label(*role),
                hash_hex(id)
            )
        }
        CausalAnchorRoot::AppSubjectRoot {
            app_id,
            subject_kind,
            id,
            role,
        } => format!(
            "app.subject.Root:{}:{app_id}:{subject_kind}:{id}",
            app_role_label(*role)
        ),
    }
}

fn render_golden(admission: &RecoveredCausalAnchorAdmission) -> String {
    let fact = admission.fact();
    let claim = fact.claim();
    let receipt = admission.receipt();
    let subject = claim.subject();
    let retained_root = &claim.retained_roots()[0];
    let materialization_root = &claim.materialization_roots()[0];
    let mut output = String::new();

    writeln!(output, "schema_version={}", claim.schema_version()).expect("write to String");
    writeln!(output, "subject.app_id={}", subject.app_id).expect("write to String");
    writeln!(output, "subject.kind={}", subject.subject_kind).expect("write to String");
    writeln!(output, "subject.id={}", subject.subject_id).expect("write to String");
    writeln!(
        output,
        "basis_frontier={}",
        hash_hex(&claim.basis_frontier().frontier_digest)
    )
    .expect("write to String");
    writeln!(output, "retained_root={}", root_label(retained_root)).expect("write to String");
    writeln!(
        output,
        "materialization_root={}",
        root_label(materialization_root)
    )
    .expect("write to String");
    writeln!(output, "purpose={}", purpose_label(claim.purpose())).expect("write to String");
    writeln!(output, "claim_digest={}", hash_hex(claim.claim_digest())).expect("write to String");
    writeln!(
        output,
        "anchor_id={}",
        hash_hex(fact.anchor_id().as_bytes())
    )
    .expect("write to String");
    writeln!(output, "anchor_digest={}", hash_hex(fact.anchor_digest())).expect("write to String");
    writeln!(
        output,
        "admitted_by_receipt_id={}",
        hash_hex(fact.admitted_by_receipt_id().as_bytes())
    )
    .expect("write to String");
    writeln!(
        output,
        "receipt.id={}",
        hash_hex(receipt.receipt_id().as_bytes())
    )
    .expect("write to String");
    writeln!(
        output,
        "receipt.anchor_id={}",
        hash_hex(receipt.anchor_id().as_bytes())
    )
    .expect("write to String");
    writeln!(
        output,
        "receipt.claim_digest={}",
        hash_hex(receipt.claim_digest())
    )
    .expect("write to String");
    writeln!(
        output,
        "receipt.basis_frontier={}",
        hash_hex(&receipt.basis_frontier().frontier_digest)
    )
    .expect("write to String");
    writeln!(
        output,
        "receipt.support_policy_digest={}",
        hash_hex(receipt.support_policy_digest())
    )
    .expect("write to String");
    writeln!(
        output,
        "receipt.writer_epoch_id={}",
        hash_hex(receipt.writer_epoch_id())
    )
    .expect("write to String");
    writeln!(
        output,
        "receipt.wal_transaction_id={}",
        hash_hex(receipt.wal_transaction_id())
    )
    .expect("write to String");
    writeln!(output, "receipt.wal_first_lsn={}", receipt.wal_first_lsn()).expect("write to String");
    writeln!(
        output,
        "transaction_id={}",
        hash_hex(&admission.transaction_id().as_hash())
    )
    .expect("write to String");
    writeln!(
        output,
        "committed_lsn={}",
        admission.committed_lsn().as_u64()
    )
    .expect("write to String");
    writeln!(
        output,
        "commit_digest={}",
        hash_hex(admission.commit_digest())
    )
    .expect("write to String");

    output
}

#[test]
fn jim_consumes_echo_admitted_anchor_identity_without_reimplementing_it() {
    let mut host = TrustedRuntimeHost::new(WorldlineRuntime::new(), empty_engine())
        .expect("trusted host should initialize");
    host.enable_in_memory_runtime_wal()
        .expect("runtime WAL should initialize");
    let basis = host
        .app()
        .current_causal_anchor_basis()
        .expect("current durable basis should exist");
    let request = request(basis);
    host.install_causal_anchor_root_support_policy(support_policy(&request));

    let admission = host
        .app()
        .admit_causal_anchor(request)
        .expect("Echo should admit the supported current-basis request");
    let recovered = host
        .app()
        .causal_anchor_by_id(admission.fact().anchor_id())
        .expect("anchor lookup should recover")
        .expect("admitted anchor should exist");

    assert_eq!(recovered, admission);
    assert_eq!(
        admission.fact().anchor_id(),
        admission.receipt().anchor_id()
    );
    assert_eq!(
        admission.fact().admitted_by_receipt_id(),
        admission.receipt().receipt_id()
    );
    assert_eq!(
        admission.fact().claim().claim_digest(),
        admission.receipt().claim_digest()
    );
    assert_eq!(
        render_golden(&admission),
        include_str!("fixtures/causal_anchor_admission_v1.txt")
    );
}
