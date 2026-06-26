// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Integration tests for `echo-cli` binary.
//!
//! These tests run the actual binary via `assert_cmd` and verify exit codes,
//! help output, and error messages.

#![allow(deprecated)] // assert_cmd::cargo::cargo_bin deprecation — no stable replacement in v2.x

use std::error::Error;
use std::fs;
use std::path::Path;

use assert_cmd::cargo::cargo_bin;
use predicates::prelude::*;
use tempfile::TempDir;
use warp_core::causal_wal::{
    build_submission_acceptance_transaction, build_tick_transaction, AffectedFrontier,
    AffectedFrontierKind, FilesystemWalStore, Lsn, PayloadCodecId, PayloadSchemaId,
    SubmissionAcceptanceRecord, TickReceiptRecord, WalAppendAuthority, WalDurabilityMode,
    WalReceiptCorrelationRecord, WalSegmentId, WalStorePort, WalTickDecision,
    WalTransactionBuilder, WalTransactionId, WalTransactionKind, WriterEpochId, WriterEpochRequest,
};
use warp_core::wsc::{build_one_warp_input, write_wsc_one_warp};
use warp_core::{
    make_edge_id, make_node_id, make_type_id, make_warp_id, EdgeRecord, GraphStore, Hash,
    NodeRecord,
};

#[path = "support/runtime_wal_fixture.rs"]
mod runtime_wal_fixture;
use runtime_wal_fixture::{
    accepted_pending_fixture as runtime_wal_accepted_pending_fixture,
    decided_applied_fixture as runtime_wal_decided_applied_fixture,
};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn echo_cli() -> assert_cmd::Command {
    assert_cmd::Command::new(cargo_bin("echo-cli"))
}

fn make_demo_wsc() -> TestResult<Vec<u8>> {
    let warp = make_warp_id("test");
    let node_ty = make_type_id("Actor");
    let child_ty = make_type_id("Item");
    let edge_ty = make_type_id("HasItem");
    let root = make_node_id("root");
    let child1 = make_node_id("child1");
    let child2 = make_node_id("child2");

    let mut store = GraphStore::new(warp);
    store.insert_node(root, NodeRecord { ty: node_ty });
    store.insert_node(child1, NodeRecord { ty: child_ty });
    store.insert_node(child2, NodeRecord { ty: child_ty });
    store.insert_edge(
        root,
        EdgeRecord {
            id: make_edge_id("root->child1"),
            from: root,
            to: child1,
            ty: edge_ty,
        },
    );
    store.insert_edge(
        root,
        EdgeRecord {
            id: make_edge_id("root->child2"),
            from: root,
            to: child2,
            ty: edge_ty,
        },
    );

    let input = build_one_warp_input(&store, root);
    Ok(write_wsc_one_warp(&input, [0u8; 32], 42)?)
}

fn write_demo_snapshot() -> TestResult<TempDir> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("state.wsc"), make_demo_wsc()?)?;
    Ok(temp)
}

fn wal_doctor_json(root: &Path) -> TestResult<serde_json::Value> {
    let root = root.to_str().ok_or("WAL root path is not UTF-8")?;
    let assert = echo_cli()
        .args(["--format", "json", "wal", "doctor", root])
        .assert()
        .success();
    Ok(serde_json::from_slice(&assert.get_output().stdout)?)
}

fn wal_submission_posture_json(
    root: &Path,
    submission_id: Hash,
    canonical_envelope_digest: Hash,
) -> TestResult<serde_json::Value> {
    let root = root.to_str().ok_or("WAL root path is not UTF-8")?;
    let assert = echo_cli()
        .args([
            "--format",
            "json",
            "wal",
            "submission-posture",
            root,
            "--submission-id",
            &hex::encode(submission_id),
            "--canonical-envelope-digest",
            &hex::encode(canonical_envelope_digest),
        ])
        .assert()
        .success();
    Ok(serde_json::from_slice(&assert.get_output().stdout)?)
}

fn digest(label: &str) -> warp_core::Hash {
    let mut out = [0_u8; 32];
    for (index, byte) in label.bytes().enumerate() {
        let rotate = match index % 8 {
            0 => 0,
            1 => 1,
            2 => 2,
            3 => 3,
            4 => 4,
            5 => 5,
            6 => 6,
            _ => 7,
        };
        out[index % out.len()] = out[index % out.len()].wrapping_add(byte);
        out[(index * 7) % out.len()] ^= byte.rotate_left(rotate);
    }
    out
}

fn writer_epoch_request() -> WriterEpochRequest {
    WriterEpochRequest {
        epoch_id: WriterEpochId::from_hash(digest("epoch")),
        storage_fencing_token: digest("fencing"),
        process_identity: digest("process"),
        host_identity: digest("host"),
        started_at_lsn: Lsn::from_raw(0),
        previous_epoch_id: None,
        previous_epoch_final_commit_digest: None,
        lease_or_lock_evidence: digest("lease"),
    }
}

fn filesystem_wal_with_committed_submission() -> TestResult<TempDir> {
    let temp = TempDir::new()?;
    let epoch = writer_epoch_request();
    let mut store = FilesystemWalStore::open(temp.path(), WalSegmentId::from_raw(1))?;
    store.acquire_writer_epoch(epoch.clone())?;
    let transaction = build_submission_acceptance_transaction(
        WalTransactionBuilder::new(
            epoch.epoch_id,
            WalSegmentId::from_raw(1),
            WalTransactionId::from_hash(digest("transaction")),
            WalTransactionKind::SubmissionIntake,
            WalAppendAuthority::SubmissionIntake,
            Lsn::from_raw(0),
            digest("previous-frame"),
            digest("previous-commit"),
            WalDurabilityMode::Buffered,
            PayloadCodecId::from_hash(digest("codec")),
            PayloadSchemaId::from_hash(digest("schema")),
            1,
            1,
            digest("domain"),
        ),
        SubmissionAcceptanceRecord {
            submission_id: digest("submission"),
            canonical_envelope_digest: digest("envelope"),
            idempotency_key_digest: None,
            acceptance_evidence_digest: digest("evidence"),
        },
        vec![AffectedFrontier {
            kind: AffectedFrontierKind::SubmissionQueue,
            before_digest: digest("frontier-before"),
            after_digest: digest("frontier-after"),
        }],
    )?;
    store.append_transaction(transaction)?;
    Ok(temp)
}

fn filesystem_wal_with_decided_submission() -> TestResult<TempDir> {
    filesystem_wal_with_decided_submission_decision("decided", WalTickDecision::Applied)
}

fn filesystem_wal_with_decided_submission_decision(
    label: &str,
    decision: WalTickDecision,
) -> TestResult<TempDir> {
    let temp = TempDir::new()?;
    let epoch = writer_epoch_request();
    let mut store = FilesystemWalStore::open(temp.path(), WalSegmentId::from_raw(1))?;
    store.acquire_writer_epoch(epoch.clone())?;
    let acceptance = build_submission_acceptance_transaction(
        WalTransactionBuilder::new(
            epoch.epoch_id,
            WalSegmentId::from_raw(1),
            WalTransactionId::from_hash(digest(&format!("transaction:{label}:accepted"))),
            WalTransactionKind::SubmissionIntake,
            WalAppendAuthority::SubmissionIntake,
            Lsn::from_raw(0),
            digest("previous-frame"),
            digest("previous-commit"),
            WalDurabilityMode::Buffered,
            PayloadCodecId::from_hash(digest("codec")),
            PayloadSchemaId::from_hash(digest("schema")),
            1,
            1,
            digest("domain"),
        ),
        SubmissionAcceptanceRecord {
            submission_id: digest(&format!("submission:{label}")),
            canonical_envelope_digest: digest(&format!("envelope:{label}")),
            idempotency_key_digest: None,
            acceptance_evidence_digest: digest(&format!("evidence:{label}")),
        },
        vec![AffectedFrontier {
            kind: AffectedFrontierKind::SubmissionQueue,
            before_digest: digest("submission-frontier-before"),
            after_digest: digest("submission-frontier-after"),
        }],
    )?;
    store.append_transaction(acceptance)?;
    let receipt = TickReceiptRecord {
        submission_id: digest(&format!("submission:{label}")),
        ticket_digest: digest(&format!("ticket:{label}")),
        receipt_digest: digest(&format!("receipt:{label}")),
        decision,
    };
    let correlation = WalReceiptCorrelationRecord {
        submission_id: receipt.submission_id,
        ticket_digest: receipt.ticket_digest,
        receipt_digest: receipt.receipt_digest,
    };
    let tick = build_tick_transaction(
        WalTransactionBuilder::new(
            epoch.epoch_id,
            WalSegmentId::from_raw(1),
            WalTransactionId::from_hash(digest(&format!("transaction:{label}:ticked"))),
            WalTransactionKind::SchedulerTick,
            WalAppendAuthority::TrustedScheduler,
            Lsn::from_raw(2),
            digest("tick-previous-frame"),
            digest("tick-previous-commit"),
            WalDurabilityMode::Buffered,
            PayloadCodecId::from_hash(digest("codec")),
            PayloadSchemaId::from_hash(digest("schema")),
            1,
            1,
            digest("domain"),
        ),
        receipt,
        correlation,
        digest(&format!("state-delta:{label}")),
        vec![
            AffectedFrontier {
                kind: AffectedFrontierKind::ReceiptIndex,
                before_digest: digest("receipt-frontier-before"),
                after_digest: digest("receipt-frontier-after"),
            },
            AffectedFrontier {
                kind: AffectedFrontierKind::RuntimeState,
                before_digest: digest("runtime-frontier-before"),
                after_digest: digest("runtime-frontier-after"),
            },
        ],
    )?;
    store.append_transaction(tick)?;
    Ok(temp)
}

#[test]
fn help_shows_all_subcommands() {
    echo_cli()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Echo developer CLI"))
        .stdout(predicate::str::contains("verify"))
        .stdout(predicate::str::contains("bench"))
        .stdout(predicate::str::contains("inspect"))
        .stdout(predicate::str::contains("wal"));
}

#[test]
fn help_output_has_no_trailing_whitespace() {
    let assert = echo_cli().arg("--help").assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let offenders = stdout
        .lines()
        .enumerate()
        .filter(|(_, line)| line.ends_with(' '))
        .map(|(index, _)| format!("line {}", index + 1))
        .collect::<Vec<_>>();

    assert!(
        offenders.is_empty(),
        "help output contains trailing whitespace on {}",
        offenders.join(", ")
    );
}

#[test]
fn help_matches_golden() {
    let assert = echo_cli().arg("--help").assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(stdout, include_str!("golden/echo-cli-help.txt"));
}

#[test]
fn help_golden_has_no_trailing_whitespace() {
    let golden = include_str!("golden/echo-cli-help.txt");
    let offenders = golden
        .lines()
        .enumerate()
        .filter(|(_, line)| line.ends_with(' '))
        .map(|(index, _)| format!("line {}", index + 1))
        .collect::<Vec<_>>();

    assert!(
        offenders.is_empty(),
        "help golden contains trailing whitespace on {}",
        offenders.join(", ")
    );
}

#[test]
fn verify_help_lists_snapshot_arg() {
    echo_cli()
        .args(["verify", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("snapshot"));
}

#[test]
fn bench_help_lists_filter() {
    echo_cli()
        .args(["bench", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("filter"))
        .stdout(predicate::str::contains("baseline"));
}

#[test]
fn inspect_help_lists_tree_flag() {
    echo_cli()
        .args(["inspect", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("tree"))
        .stdout(predicate::str::contains("raw"));
}

#[test]
fn wal_doctor_help_lists_read_only_doctor() {
    echo_cli()
        .args(["wal", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("doctor"))
        .stdout(predicate::str::contains("submission-posture"));
}

#[test]
fn wal_doctor_json_reports_read_only_empty_store() -> TestResult {
    let assert = echo_cli()
        .args(["--format", "json", "wal", "doctor"])
        .assert()
        .success();
    let json: serde_json::Value = serde_json::from_slice(&assert.get_output().stdout)?;

    assert_eq!(json["posture"], "Recoverable");
    assert_eq!(json["tail_posture"], "Clean");
    assert_eq!(json["committed_transactions_replayed"], 0);
    assert_eq!(json["obstruction_count"], 0);
    Ok(())
}

#[test]
fn wal_doctor_json_reports_committed_filesystem_wal() -> TestResult {
    let temp = filesystem_wal_with_committed_submission()?;
    let assert = echo_cli()
        .args([
            "--format",
            "json",
            "wal",
            "doctor",
            temp.path().to_str().ok_or("temp path is not UTF-8")?,
        ])
        .assert()
        .success();
    let json: serde_json::Value = serde_json::from_slice(&assert.get_output().stdout)?;

    assert_eq!(json["posture"], "Recoverable");
    assert_eq!(json["tail_posture"], "Clean");
    assert_eq!(json["committed_transactions_replayed"], 1);
    assert_eq!(json["obstruction_count"], 0);
    Ok(())
}

#[test]
fn wal_submission_posture_json_reports_generic_recovered_status() -> TestResult {
    let temp = filesystem_wal_with_decided_submission()?;
    let assert = echo_cli()
        .args([
            "--format",
            "json",
            "wal",
            "submission-posture",
            temp.path().to_str().ok_or("temp path is not UTF-8")?,
            "--submission-id",
            &hex::encode(digest("submission:decided")),
            "--canonical-envelope-digest",
            &hex::encode(digest("envelope:decided")),
        ])
        .assert()
        .success();
    let json: serde_json::Value = serde_json::from_slice(&assert.get_output().stdout)?;

    assert_eq!(json["retry_posture"], "AlreadyDecidedApplied");
    assert_eq!(json["recovered_posture"], "DecidedApplied");
    assert_eq!(
        json["receipt_digest"],
        hex::encode(digest("receipt:decided"))
    );
    assert_eq!(json["ticket_digest"], hex::encode(digest("ticket:decided")));
    Ok(())
}

#[test]
fn wal_submission_posture_runtime_ack_root_reports_accepted_pending_json() -> TestResult {
    let fixture = runtime_wal_accepted_pending_fixture()?;
    let doctor = wal_doctor_json(fixture.root.path())?;
    assert_eq!(doctor["posture"], "Recoverable");
    assert_eq!(doctor["tail_posture"], "Clean");
    assert_eq!(doctor["committed_transactions_replayed"], 1);
    assert_eq!(doctor["obstruction_count"], 0);

    let json = wal_submission_posture_json(
        fixture.root.path(),
        fixture.submission_id,
        fixture.canonical_envelope_digest,
    )?;
    assert_eq!(json["retry_posture"], "AlreadyAcceptedPending");
    assert_eq!(json["recovered_posture"], "AcceptedPending");
    assert!(json["receipt_digest"].is_null());
    assert!(json["ticket_digest"].is_null());
    Ok(())
}

#[test]
fn wal_submission_posture_runtime_ack_root_reports_decided_applied_json() -> TestResult {
    let fixture = runtime_wal_decided_applied_fixture()?;
    let json = wal_submission_posture_json(
        fixture.root.path(),
        fixture.submission_id,
        fixture.canonical_envelope_digest,
    )?;

    assert_eq!(json["retry_posture"], "AlreadyDecidedApplied");
    assert_eq!(json["recovered_posture"], "DecidedApplied");
    assert_eq!(
        json["receipt_digest"],
        hex::encode(fixture.receipt_digest.ok_or("missing applied receipt")?)
    );
    assert_eq!(
        json["ticket_digest"],
        hex::encode(fixture.ticket_digest.ok_or("missing applied ticket")?)
    );
    Ok(())
}

// Runtime-produced filesystem roots cover accepted/applied today. Rejected and
// obstructed stay as direct WAL taxonomy fixtures until the trusted host can
// emit those decisions without expanding this CLI contract slice.
#[test]
fn wal_submission_posture_json_reports_obstructed_recovered_status() -> TestResult {
    let temp =
        filesystem_wal_with_decided_submission_decision("obstructed", WalTickDecision::Obstructed)?;
    let json = wal_submission_posture_json(
        temp.path(),
        digest("submission:obstructed"),
        digest("envelope:obstructed"),
    )?;

    assert_eq!(json["retry_posture"], "AlreadyObstructed");
    assert_eq!(json["recovered_posture"], "Obstructed");
    assert_eq!(
        json["receipt_digest"],
        hex::encode(digest("receipt:obstructed"))
    );
    assert_eq!(
        json["ticket_digest"],
        hex::encode(digest("ticket:obstructed"))
    );
    Ok(())
}

#[test]
fn wal_submission_posture_json_reports_decided_rejected_status() -> TestResult {
    let temp = filesystem_wal_with_decided_submission_decision(
        "rejected",
        WalTickDecision::RejectedFootprintConflict,
    )?;
    let json = wal_submission_posture_json(
        temp.path(),
        digest("submission:rejected"),
        digest("envelope:rejected"),
    )?;

    assert_eq!(json["retry_posture"], "AlreadyDecidedRejected");
    assert_eq!(json["recovered_posture"], "DecidedRejected");
    assert_eq!(
        json["receipt_digest"],
        hex::encode(digest("receipt:rejected"))
    );
    assert_eq!(
        json["ticket_digest"],
        hex::encode(digest("ticket:rejected"))
    );
    Ok(())
}

#[test]
fn wal_submission_posture_text_reports_canonical_envelope_digest() -> TestResult {
    let temp = filesystem_wal_with_decided_submission()?;
    let envelope_digest = hex::encode(digest("envelope:decided"));
    let assert = echo_cli()
        .args([
            "wal",
            "submission-posture",
            temp.path().to_str().ok_or("temp path is not UTF-8")?,
            "--submission-id",
            &hex::encode(digest("submission:decided")),
            "--canonical-envelope-digest",
            &envelope_digest,
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

    assert!(stdout.contains(&format!("Canonical envelope: {envelope_digest}")));
    Ok(())
}

#[test]
fn wal_submission_posture_json_reports_not_accepted_without_app_nouns() -> TestResult {
    let temp = filesystem_wal_with_committed_submission()?;
    let assert = echo_cli()
        .args([
            "--format",
            "json",
            "wal",
            "submission-posture",
            temp.path().to_str().ok_or("temp path is not UTF-8")?,
            "--submission-id",
            &hex::encode(digest("submission:missing")),
            "--canonical-envelope-digest",
            &hex::encode(digest("envelope:missing")),
        ])
        .assert()
        .success();
    let json: serde_json::Value = serde_json::from_slice(&assert.get_output().stdout)?;

    assert_eq!(json["retry_posture"], "NotAccepted");
    assert!(json["recovered_posture"].is_null());
    assert!(json["receipt_digest"].is_null());
    assert!(json["ticket_digest"].is_null());
    Ok(())
}

#[test]
fn wal_submission_posture_json_suppresses_recovered_fields_for_envelope_conflict() -> TestResult {
    let temp = filesystem_wal_with_decided_submission()?;
    let assert = echo_cli()
        .args([
            "--format",
            "json",
            "wal",
            "submission-posture",
            temp.path().to_str().ok_or("temp path is not UTF-8")?,
            "--submission-id",
            &hex::encode(digest("submission:decided")),
            "--canonical-envelope-digest",
            &hex::encode(digest("envelope:conflicting")),
        ])
        .assert()
        .success();
    let json: serde_json::Value = serde_json::from_slice(&assert.get_output().stdout)?;

    assert_eq!(json["retry_posture"], "ConflictSameIdDifferentEnvelope");
    assert!(json["recovered_posture"].is_null());
    assert!(json["receipt_digest"].is_null());
    assert!(json["ticket_digest"].is_null());
    Ok(())
}

#[test]
fn inspect_text_reports_metadata_stats_and_tree() -> TestResult {
    let temp = write_demo_snapshot()?;
    let assert = echo_cli()
        .current_dir(temp.path())
        .args(["inspect", "state.wsc", "--tree"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

    assert!(stdout.contains("echo-cli inspect"));
    assert!(stdout.contains("File: state.wsc"));
    assert!(stdout.contains("Tick: 42"));
    assert!(
        stdout.contains("Schema: 0000000000000000000000000000000000000000000000000000000000000000")
    );
    assert!(stdout.contains("Warps: 1"));
    assert!(stdout
        .contains("ID:         6939dc0fbdb5004cb5d9d1aca2d096042456f4257b88ee8c7fdbfca163f10f11"));
    assert!(stdout
        .contains("Root node:  401e1d8fcbc26350901be9100a153e8eaf644560386edf68f876ffc1335cccf0"));
    assert!(stdout
        .contains("State root: 5934ceb0b331755a406e85fbe1a6dda3d6ce5278f7c1802713a50c4e754c84a6"));
    assert!(stdout.contains("Nodes:      3"));
    assert!(stdout.contains("Edges:      2"));
    assert!(stdout.contains("Components: 1"));
    assert!(stdout.contains("Node types:"));
    assert!(stdout.contains("1e27b4d0: 1"));
    assert!(stdout.contains("d9f7db5f: 2"));
    assert!(stdout.contains("Edge types:"));
    assert!(stdout.contains("8e4ee065: 2"));
    assert!(stdout.contains("Tree:"));
    assert!(stdout.contains("[401e1d8f] type=1e27b4d0"));
    assert!(stdout
        .lines()
        .any(|line| line.starts_with("    ") && line.contains("type=d9f7db5f")));

    Ok(())
}

#[test]
fn inspect_json_reports_structured_metadata_and_stats() -> TestResult {
    let temp = write_demo_snapshot()?;
    let assert = echo_cli()
        .current_dir(temp.path())
        .args(["--format", "json", "inspect", "state.wsc"])
        .assert()
        .success();
    let json: serde_json::Value = serde_json::from_slice(&assert.get_output().stdout)?;

    assert_eq!(json["metadata"]["file"], "state.wsc");
    assert_eq!(json["metadata"]["tick"], 42);
    assert_eq!(
        json["metadata"]["schema_hash"],
        "0000000000000000000000000000000000000000000000000000000000000000"
    );
    assert_eq!(json["metadata"]["warp_count"], 1);
    assert_eq!(json["warps"][0]["total_nodes"], 3);
    assert_eq!(json["warps"][0]["total_edges"], 2);
    assert_eq!(json["warps"][0]["connected_components"], 1);
    assert_eq!(json["warps"][0]["node_types"]["1e27b4d0"], 1);
    assert_eq!(json["warps"][0]["node_types"]["d9f7db5f"], 2);
    assert_eq!(json["warps"][0]["edge_types"]["8e4ee065"], 2);
    assert!(json.get("tree").is_none());

    let node_type_sum = json["warps"][0]["node_types"]
        .as_object()
        .into_iter()
        .flat_map(serde_json::Map::values)
        .filter_map(serde_json::Value::as_u64)
        .sum::<u64>();
    assert_eq!(node_type_sum, 3);

    Ok(())
}

#[test]
fn inspect_corrupt_snapshot_exits_nonzero_without_panic() -> TestResult {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("bad.wsc"), b"not a wsc")?;

    echo_cli()
        .current_dir(temp.path())
        .args(["inspect", "bad.wsc"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("failed to open WSC file")
                .or(predicate::str::contains("WSC validation failed")),
        );

    Ok(())
}

#[test]
fn unknown_subcommand_exits_2() {
    echo_cli().arg("bogus").assert().code(2);
}

#[test]
fn no_subcommand_exits_2() {
    echo_cli().assert().code(2);
}

#[test]
fn verify_missing_file_exits_nonzero() {
    echo_cli()
        .args(["verify", "/nonexistent/path/state.wsc"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to open WSC file"));
}

#[test]
fn format_flag_is_global() {
    // --format should work before and after the subcommand.
    echo_cli()
        .args(["--format", "json", "verify", "--help"])
        .assert()
        .success();
}
