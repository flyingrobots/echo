// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Deterministic test kernel used by the DIND harness.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::cast_possible_truncation,
    clippy::unnecessary_wraps,
    clippy::match_wildcard_for_single_variants
)]

use echo_dry_tests::build_motion_demo_engine;
use warp_core::{make_node_id, ApplyResult, DispatchDisposition, Engine};

/// Auto-generated codec definitions.
#[path = "codecs.generated.rs"]
mod codecs;

/// Schema content hash (BLAKE3) for the generated codecs module.
///
/// This 64-character hex string uniquely identifies the schema version used to
/// generate the binary codecs. It is embedded in ELOG files for compatibility
/// checking during replay.
pub use codecs::SCHEMA_HASH;
/// DIND test rules and state management.
pub mod rules;
/// Auto-generated type ID constants.
#[path = "type_ids.generated.rs"]
mod type_ids;

use rules::{
    ball_physics_rule, drop_ball_rule, route_push_rule, set_theme_rule, toast_rule, toggle_nav_rule,
};

#[cfg(feature = "dind_ops")]
use rules::put_kv_rule;

/// The deterministic kernel used for DIND scenarios.
pub struct EchoKernel {
    engine: Engine,
}

impl Default for EchoKernel {
    fn default() -> Self {
        Self::new()
    }
}

impl EchoKernel {
    /// Create a new kernel instance with all DIND rules registered.
    pub fn new() -> Self {
        let mut e = build_motion_demo_engine();
        e.register_rule(toast_rule())
            .expect("toast_rule registration failed");
        e.register_rule(route_push_rule())
            .expect("route_push_rule registration failed");
        e.register_rule(set_theme_rule())
            .expect("set_theme_rule registration failed");
        e.register_rule(toggle_nav_rule())
            .expect("toggle_nav_rule registration failed");
        e.register_rule(drop_ball_rule())
            .expect("drop_ball_rule registration failed");
        e.register_rule(ball_physics_rule())
            .expect("ball_physics_rule registration failed");
        #[cfg(feature = "dind_ops")]
        e.register_rule(put_kv_rule())
            .expect("put_kv_rule registration failed");
        e.register_rule(warp_core::inbox::ack_pending_rule())
            .expect("ack_pending_rule registration failed");

        Self { engine: e }
    }

    /// Dispatch an intent (canonical bytes) with an auto-assigned sequence number.
    pub fn dispatch_intent(&mut self, intent_bytes: &[u8]) {
        // Canonical ingress: content-addressed, idempotent on `intent_id`.
        // Bytes are opaque to the core engine; validation is the caller's responsibility.
        let _ = self
            .engine
            .ingest_intent(intent_bytes)
            .expect("ingest intent");
    }

    /// Run a deterministic step.
    pub fn step(&mut self) -> bool {
        let tx = self.engine.begin();
        let ball_id = make_node_id("ball");
        let mut dirty = false;

        // Consume exactly one pending intent per tick, using canonical `intent_id` order.
        let dispatch = self
            .engine
            .dispatch_next_intent(tx)
            .expect("dispatch_next_intent");
        if matches!(dispatch, DispatchDisposition::Consumed { .. }) {
            dirty = true;
        }

        if matches!(
            self.engine
                .apply(tx, rules::BALL_PHYSICS_RULE_NAME, &ball_id)
                .expect("apply physics rule"),
            ApplyResult::Applied
        ) {
            dirty = true;
        }

        if dirty {
            // Commit must succeed in test kernel; failure indicates corruption.
            self.engine
                .commit(tx)
                .expect("commit failed - test kernel corruption");
            true
        } else {
            self.engine.abort(tx);
            false
        }
    }

    /// Access the underlying engine (read-only).
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Access the underlying engine (mutable).
    pub fn engine_mut(&mut self) -> &mut Engine {
        &mut self.engine
    }

    /// Canonical state hash of the root warp.
    ///
    /// # Panics
    /// Panics if the root warp does not exist (indicates test kernel misconfiguration).
    pub fn state_hash(&self) -> [u8; 32] {
        self.engine
            .state()
            .store(&warp_core::make_warp_id("root"))
            .expect("root warp must exist in test kernel")
            .canonical_state_hash()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        path::{Path, PathBuf},
        process::{self, Child, Command},
        thread,
        time::Duration,
    };

    use warp_core::{
        causal_wal::{
            build_submission_acceptance_transaction, recover_filesystem_store,
            recover_submission_index, AffectedFrontier, AffectedFrontierKind, FilesystemWalStore,
            Lsn, PayloadCodecId, PayloadSchemaId, RecoveredSubmissionPosture, RecoveryAccessMode,
            RecoveryTailPosture, SubmissionAcceptanceRecord, WalAppendAuthority,
            WalCommittedTransaction, WalDurabilityMode, WalSegmentId, WalStorePort,
            WalTransactionBuilder, WalTransactionId, WalTransactionKind, WriterEpochId,
            WriterEpochRequest,
        },
        Hash,
    };

    const CHILD_MODE_ENV: &str = "ECHO_DIND_WAL_CRASHPOINT_CHILD";
    const WAL_ROOT_ENV: &str = "ECHO_DIND_WAL_CRASHPOINT_ROOT";
    const READY_MARKER_ENV: &str = "ECHO_DIND_WAL_CRASHPOINT_READY";
    const AFTER_COMMIT_MODE: &str = "after_wal_commit";
    const BEFORE_COMMIT_MODE: &str = "before_wal_commit";

    #[test]
    fn wal_process_crashpoints() {
        if let Ok(mode) = env::var(CHILD_MODE_ENV) {
            run_wal_crashpoint_child(&mode);
        }

        let root = crashpoint_root();
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create crashpoint root");

        let after_root = root.join("after-wal-commit");
        let after_acceptance = acceptance("after-wal-commit");
        run_and_kill_child(AFTER_COMMIT_MODE, &after_root);
        let after_report = recover_filesystem_store(&after_root, RecoveryAccessMode::ReadOnly)
            .expect("recover after-commit WAL root");
        let after_index =
            recover_submission_index(&after_report).expect("recover after-commit index");
        let after_entry = after_index
            .get(&after_acceptance.submission_id)
            .expect("after-commit submission recovered");
        assert_eq!(after_report.tail_posture, RecoveryTailPosture::Clean);
        assert_eq!(after_entry.acceptance, after_acceptance);
        assert_eq!(
            after_entry.posture,
            RecoveredSubmissionPosture::AcceptedPending
        );

        let before_root = root.join("before-wal-commit");
        let before_acceptance = acceptance("before-wal-commit");
        run_and_kill_child(BEFORE_COMMIT_MODE, &before_root);
        let before_report = recover_filesystem_store(&before_root, RecoveryAccessMode::ReadOnly)
            .expect("recover before-commit WAL root");
        let before_index =
            recover_submission_index(&before_report).expect("recover before-commit index");
        assert_eq!(
            before_report.tail_posture,
            RecoveryTailPosture::WouldTruncateAll
        );
        assert!(before_index.get(&before_acceptance.submission_id).is_none());
        assert!(before_index.is_empty());

        fs::remove_dir_all(&root).expect("remove crashpoint root");
    }

    fn run_wal_crashpoint_child(mode: &str) -> ! {
        let root = PathBuf::from(env::var_os(WAL_ROOT_ENV).expect("WAL root env"));
        let marker = PathBuf::from(env::var_os(READY_MARKER_ENV).expect("ready marker env"));
        fs::create_dir_all(&root).expect("create child WAL root");
        let mut store = FilesystemWalStore::open(&root, WalSegmentId::from_raw(1))
            .expect("open child WAL store");
        store
            .acquire_writer_epoch(writer_epoch_request())
            .expect("acquire writer epoch");
        match mode {
            AFTER_COMMIT_MODE => {
                store
                    .append_transaction(submission_transaction(
                        "after-wal-commit",
                        Lsn::from_raw(0),
                    ))
                    .expect("append committed child transaction");
            }
            BEFORE_COMMIT_MODE => {
                let transaction = submission_transaction("before-wal-commit", Lsn::from_raw(0));
                store
                    .append_uncommitted_frame(epoch_id(), transaction.frames[0].clone())
                    .expect("append uncommitted child frame");
            }
            other => panic!("unknown child crashpoint mode: {other}"),
        }
        fs::write(marker, b"ready").expect("write ready marker");
        loop {
            thread::sleep(Duration::from_secs(60));
        }
    }

    fn run_and_kill_child(mode: &str, wal_root: &Path) {
        fs::create_dir_all(wal_root).expect("create WAL root");
        let marker = wal_root.join("ready");
        let mut child = Command::new(env::current_exe().expect("current test binary"))
            .arg("tests::wal_process_crashpoints")
            .arg("--exact")
            .arg("--nocapture")
            .env(CHILD_MODE_ENV, mode)
            .env(WAL_ROOT_ENV, wal_root)
            .env(READY_MARKER_ENV, &marker)
            .spawn()
            .expect("spawn WAL crashpoint child");
        wait_for_ready_marker(&mut child, &marker);
        child.kill().expect("kill WAL crashpoint child");
        let status = child.wait().expect("wait for killed child");
        assert!(!status.success(), "child should have been killed");
    }

    fn wait_for_ready_marker(child: &mut Child, marker: &Path) {
        for _ in 0..200 {
            if marker.exists() {
                return;
            }
            if let Some(status) = child.try_wait().expect("poll child") {
                panic!("child exited before ready marker: {status}");
            }
            thread::sleep(Duration::from_millis(50));
        }
        let _ = child.kill();
        panic!("timed out waiting for ready marker at {}", marker.display());
    }

    fn crashpoint_root() -> PathBuf {
        env::current_dir()
            .expect("current dir")
            .join("target")
            .join("echo-dind-wal-crashpoints")
            .join(process::id().to_string())
    }

    fn submission_transaction(label: &str, first_lsn: Lsn) -> WalCommittedTransaction {
        build_submission_acceptance_transaction(
            builder(
                transaction_id(label),
                first_lsn,
                WalAppendAuthority::SubmissionIntake,
                WalTransactionKind::SubmissionIntake,
            ),
            acceptance(label),
            vec![frontier(label)],
        )
        .expect("build submission transaction")
    }

    fn acceptance(label: &str) -> SubmissionAcceptanceRecord {
        SubmissionAcceptanceRecord {
            submission_id: digest(&format!("submission:{label}")),
            canonical_envelope_digest: digest(&format!("envelope:{label}")),
            idempotency_key_digest: Some(digest(&format!("idempotency:{label}"))),
            acceptance_evidence_digest: digest(&format!("accepted:{label}")),
        }
    }

    fn builder(
        transaction_id: WalTransactionId,
        first_lsn: Lsn,
        authority: WalAppendAuthority,
        transaction_kind: WalTransactionKind,
    ) -> WalTransactionBuilder {
        WalTransactionBuilder::new(
            epoch_id(),
            WalSegmentId::from_raw(1),
            transaction_id,
            transaction_kind,
            authority,
            first_lsn,
            digest("genesis-frame"),
            digest("genesis-commit"),
            WalDurabilityMode::StrictFilesystem,
            PayloadCodecId::from_hash(digest("codec:echo-dind-wal-crashpoint")),
            PayloadSchemaId::from_hash(digest("schema:echo-dind-wal-crashpoint")),
            1,
            1,
            digest("domain:echo-dind-wal-crashpoint"),
        )
    }

    fn writer_epoch_request() -> WriterEpochRequest {
        WriterEpochRequest {
            epoch_id: epoch_id(),
            storage_fencing_token: digest("fence:echo-dind-wal-crashpoint"),
            process_identity: digest("process:echo-dind-wal-crashpoint"),
            host_identity: digest("host:echo-dind-wal-crashpoint"),
            started_at_lsn: Lsn::from_raw(0),
            previous_epoch_id: None,
            previous_epoch_final_commit_digest: None,
            lease_or_lock_evidence: digest("lease:echo-dind-wal-crashpoint"),
        }
    }

    fn frontier(label: &str) -> AffectedFrontier {
        AffectedFrontier {
            kind: AffectedFrontierKind::SubmissionQueue,
            before_digest: digest(&format!("{label}:submission:before")),
            after_digest: digest(&format!("{label}:submission:after")),
        }
    }

    fn transaction_id(label: &str) -> WalTransactionId {
        WalTransactionId::from_hash(digest(&format!("tx:{label}")))
    }

    fn epoch_id() -> WriterEpochId {
        WriterEpochId::from_hash(digest("epoch:echo-dind-wal-crashpoint"))
    }

    fn digest(label: &str) -> Hash {
        let mut out = [0_u8; 32];
        for (index, byte) in label.as_bytes().iter().enumerate() {
            out[index % 32] = out[index % 32]
                .wrapping_add(*byte)
                .wrapping_add(index as u8);
        }
        out
    }
}
