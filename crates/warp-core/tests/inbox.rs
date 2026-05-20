// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::unwrap_used)]
//! Runtime-owned ingress integration tests.

use warp_core::{
    make_head_id, make_intent_kind, make_node_id, make_type_id, Engine, EngineBuilder, GlobalTick,
    GraphStore, InboxAddress, InboxPolicy, IngressDisposition, IngressEnvelope, IngressTarget,
    IntentOutcomeObservation, IntentSubmissionDisposition, NodeId, NodeRecord, PlaybackMode,
    ProvenanceEventKind, ProvenanceService, ProvenanceStore, SchedulerCoordinator, SchedulerKind,
    WorldlineId, WorldlineRuntime, WorldlineState, WorldlineTick, WorldlineTickPatchV1, WriterHead,
    WriterHeadKey,
};
#[cfg(feature = "host_test")]
use warp_core::{
    OpticAdmissionTicket, OpticArtifactHandle, RuntimeError, TicketedRuntimeIngressAuthority,
    TicketedRuntimeIngressDisposition, OPTIC_ADMISSION_TICKET_KIND, OPTIC_ARTIFACT_HANDLE_KIND,
};

fn wl(n: u8) -> WorldlineId {
    WorldlineId::from_bytes([n; 32])
}

fn wt(raw: u64) -> WorldlineTick {
    WorldlineTick::from_raw(raw)
}

fn gt(raw: u64) -> GlobalTick {
    GlobalTick::from_raw(raw)
}

fn empty_engine() -> Engine {
    let mut store = GraphStore::default();
    let root = make_node_id("root");
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id("world"),
        },
    );
    EngineBuilder::new(store, root)
        .scheduler(SchedulerKind::Radix)
        .workers(1)
        .build()
}

fn register_head(
    runtime: &mut WorldlineRuntime,
    worldline_id: WorldlineId,
    label: &str,
    public_inbox: Option<&str>,
    is_default_writer: bool,
) -> WriterHeadKey {
    let key = WriterHeadKey {
        worldline_id,
        head_id: make_head_id(label),
    };
    runtime
        .register_writer_head(WriterHead::with_routing(
            key,
            PlaybackMode::Play,
            InboxPolicy::AcceptAll,
            public_inbox.map(|name| InboxAddress(name.to_owned())),
            is_default_writer,
        ))
        .unwrap();
    key
}

fn runtime_store(runtime: &WorldlineRuntime, worldline_id: WorldlineId) -> &GraphStore {
    let frontier = runtime.worldlines().get(&worldline_id).unwrap();
    frontier
        .state()
        .warp_state()
        .store(&frontier.state().root().warp_id)
        .unwrap()
}

fn registered_worldlines_provenance(runtime: &WorldlineRuntime) -> ProvenanceService {
    let mut provenance = ProvenanceService::new();
    for (worldline_id, frontier) in runtime.worldlines().iter() {
        provenance
            .register_worldline(*worldline_id, frontier.state())
            .unwrap();
    }
    provenance
}

#[cfg(feature = "host_test")]
fn admission_ticket(seed: u8) -> OpticAdmissionTicket {
    OpticAdmissionTicket {
        kind: OPTIC_ADMISSION_TICKET_KIND.to_owned(),
        artifact_handle: OpticArtifactHandle {
            kind: OPTIC_ARTIFACT_HANDLE_KIND.to_owned(),
            id: format!("ticketed-runtime-ingress-{seed}"),
        },
        artifact_hash: format!("artifact-hash-{seed}"),
        operation_id: format!("operation-{seed}"),
        requirements_digest: format!("requirements-{seed}"),
        canonical_variables_digest: vec![seed],
        basis_request_digest: [seed; 32],
        aperture_request_digest: [seed.wrapping_add(1); 32],
        budget_request_digest: [seed.wrapping_add(2); 32],
        law_witness_digest: [seed.wrapping_add(3); 32],
        ticket_digest: [seed.wrapping_add(4); 32],
    }
}

#[cfg(feature = "host_test")]
fn ticketed_runtime_ingress_authority() -> TicketedRuntimeIngressAuthority {
    TicketedRuntimeIngressAuthority::assume_runtime_owner()
}

#[test]
fn runtime_ingest_commits_without_legacy_graph_inbox_nodes() {
    let mut runtime = WorldlineRuntime::new();
    let mut engine = empty_engine();
    let worldline_id = wl(1);
    runtime
        .register_worldline(worldline_id, WorldlineState::empty())
        .unwrap();
    let head_key = register_head(&mut runtime, worldline_id, "default", None, true);

    let envelope = IngressEnvelope::local_intent(
        IngressTarget::DefaultWriter { worldline_id },
        make_intent_kind("test/runtime"),
        b"runtime-intent".to_vec(),
    );
    assert!(matches!(
        runtime.ingest(envelope.clone()).unwrap(),
        IngressDisposition::Accepted {
            ingress_id,
            head_key: routed_head_key,
            ..
        } if ingress_id == envelope.ingress_id() && routed_head_key == head_key
    ));

    let mut provenance = registered_worldlines_provenance(&runtime);
    let records =
        SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].head_key, head_key);

    let store = runtime_store(&runtime, worldline_id);
    assert!(store.node(&NodeId(envelope.ingress_id())).is_some());
    assert!(store.node(&make_node_id("sim")).is_none());
    assert!(store.node(&make_node_id("sim/inbox")).is_none());
}

#[test]
fn witnessed_submission_does_not_enter_runtime_ingress_before_ticket() {
    let mut runtime = WorldlineRuntime::new();
    let mut engine = empty_engine();
    let worldline_id = wl(1);
    runtime
        .register_worldline(worldline_id, WorldlineState::empty())
        .unwrap();
    let head_key = register_head(&mut runtime, worldline_id, "default", None, true);

    let envelope = IngressEnvelope::local_intent(
        IngressTarget::DefaultWriter { worldline_id },
        make_intent_kind("test/runtime"),
        b"witness-only".to_vec(),
    );
    let disposition = runtime.submit_intent(envelope.clone()).unwrap();

    assert!(matches!(
        disposition,
        IntentSubmissionDisposition::Accepted {
            ingress_id,
            head_key: routed_head_key,
            ..
        } if ingress_id == envelope.ingress_id() && routed_head_key == head_key
    ));
    assert_eq!(runtime.witnessed_submission_count(), 1);
    assert_eq!(
        runtime
            .heads()
            .get(&head_key)
            .unwrap()
            .inbox()
            .pending_count(),
        0
    );
    assert_eq!(runtime.global_tick(), gt(0));

    let mut provenance = registered_worldlines_provenance(&runtime);
    let records =
        SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();
    assert!(records.is_empty());
    let frontier = runtime.worldlines().get(&worldline_id).unwrap();
    assert_eq!(frontier.frontier_tick(), wt(0));
    assert!(frontier.state().tick_history().is_empty());
}

#[test]
#[cfg(feature = "host_test")]
fn ticketed_runtime_ingress_rejects_unknown_submission() {
    let mut runtime = WorldlineRuntime::new();
    let worldline_id = wl(1);
    runtime
        .register_worldline(worldline_id, WorldlineState::empty())
        .unwrap();
    let head_key = register_head(&mut runtime, worldline_id, "default", None, true);
    let envelope = IngressEnvelope::local_intent(
        IngressTarget::DefaultWriter { worldline_id },
        make_intent_kind("test/runtime"),
        b"unknown-submission".to_vec(),
    );

    let err = runtime
        .ingest_ticketed_invocation(
            &ticketed_runtime_ingress_authority(),
            [9; 32],
            &admission_ticket(1),
            envelope,
        )
        .unwrap_err();

    assert!(matches!(
        err,
        RuntimeError::UnknownIntentSubmission(id) if id == [9; 32]
    ));
    assert_eq!(
        runtime
            .heads()
            .get(&head_key)
            .unwrap()
            .inbox()
            .pending_count(),
        0
    );
    assert_eq!(runtime.ticketed_runtime_ingress_count(), 0);
}

#[test]
#[cfg(feature = "host_test")]
fn ticketed_invocation_ingests_runtime_envelope_without_ticking() {
    let mut runtime = WorldlineRuntime::new();
    let worldline_id = wl(1);
    runtime
        .register_worldline(worldline_id, WorldlineState::empty())
        .unwrap();
    let head_key = register_head(&mut runtime, worldline_id, "default", None, true);
    let envelope = IngressEnvelope::local_intent(
        IngressTarget::DefaultWriter { worldline_id },
        make_intent_kind("test/runtime"),
        b"ticketed-ingress".to_vec(),
    );
    let submission = match runtime.submit_intent(envelope.clone()).unwrap() {
        IntentSubmissionDisposition::Accepted { submission_id, .. } => submission_id,
        IntentSubmissionDisposition::Duplicate { .. } => panic!("first submission duplicated"),
    };
    let ticket = admission_ticket(2);

    let disposition = runtime
        .ingest_ticketed_invocation(
            &ticketed_runtime_ingress_authority(),
            submission,
            &ticket,
            envelope.clone(),
        )
        .unwrap();

    let record = match disposition {
        TicketedRuntimeIngressDisposition::Staged { record, ingress } => {
            assert!(matches!(
                ingress,
                IngressDisposition::Accepted {
                    ingress_id,
                    head_key: routed_head_key,
                    submission_id,
                    ..
                } if ingress_id == envelope.ingress_id()
                    && routed_head_key == head_key
                    && submission_id == submission
            ));
            record
        }
        TicketedRuntimeIngressDisposition::Duplicate { .. } => {
            panic!("first ticketed runtime ingress duplicated")
        }
    };

    assert_eq!(record.submission_id, submission);
    assert_eq!(record.ticket_digest, ticket.ticket_digest);
    assert_eq!(record.ingress_id, envelope.ingress_id());
    assert_eq!(record.head_key, head_key);
    assert_eq!(runtime.ticketed_runtime_ingress_count(), 1);
    assert_eq!(
        runtime
            .heads()
            .get(&head_key)
            .unwrap()
            .inbox()
            .pending_count(),
        1
    );
    assert_eq!(runtime.global_tick(), gt(0));
    let frontier = runtime.worldlines().get(&worldline_id).unwrap();
    assert_eq!(frontier.frontier_tick(), wt(0));
    assert!(frontier.state().tick_history().is_empty());
}

#[test]
#[cfg(feature = "host_test")]
fn ticketed_ingress_preserves_submission_and_ticket_identity() {
    let mut runtime = WorldlineRuntime::new();
    let worldline_id = wl(1);
    runtime
        .register_worldline(worldline_id, WorldlineState::empty())
        .unwrap();
    register_head(&mut runtime, worldline_id, "default", None, true);
    let envelope = IngressEnvelope::local_intent(
        IngressTarget::DefaultWriter { worldline_id },
        make_intent_kind("test/runtime"),
        b"stable-ticketed-ingress".to_vec(),
    );
    let submission = match runtime.submit_intent(envelope.clone()).unwrap() {
        IntentSubmissionDisposition::Accepted { submission_id, .. } => submission_id,
        IntentSubmissionDisposition::Duplicate { .. } => panic!("first submission duplicated"),
    };
    let ticket = admission_ticket(3);

    let first = runtime
        .ingest_ticketed_invocation(
            &ticketed_runtime_ingress_authority(),
            submission,
            &ticket,
            envelope.clone(),
        )
        .unwrap();
    let duplicate = runtime
        .ingest_ticketed_invocation(
            &ticketed_runtime_ingress_authority(),
            submission,
            &ticket,
            envelope,
        )
        .unwrap();

    let first_record = match first {
        TicketedRuntimeIngressDisposition::Staged { record, .. } => record,
        TicketedRuntimeIngressDisposition::Duplicate { .. } => {
            panic!("first ticketed ingress duplicated")
        }
    };
    let duplicate_record = match duplicate {
        TicketedRuntimeIngressDisposition::Duplicate { record } => record,
        TicketedRuntimeIngressDisposition::Staged { .. } => {
            panic!("duplicate ticketed ingress staged twice")
        }
    };

    assert_eq!(first_record, duplicate_record);
    assert_eq!(runtime.ticketed_runtime_ingress_count(), 1);
}

#[test]
#[cfg(feature = "host_test")]
fn receipt_correlation_does_not_exist_before_scheduler_tick() {
    let mut runtime = WorldlineRuntime::new();
    let worldline_id = wl(1);
    runtime
        .register_worldline(worldline_id, WorldlineState::empty())
        .unwrap();
    register_head(&mut runtime, worldline_id, "default", None, true);
    let envelope = IngressEnvelope::local_intent(
        IngressTarget::DefaultWriter { worldline_id },
        make_intent_kind("test/runtime"),
        b"pending-receipt-correlation".to_vec(),
    );
    let submission = match runtime.submit_intent(envelope.clone()).unwrap() {
        IntentSubmissionDisposition::Accepted { submission_id, .. } => submission_id,
        IntentSubmissionDisposition::Duplicate { .. } => panic!("first submission duplicated"),
    };
    let ticket = admission_ticket(4);
    let staged = runtime
        .ingest_ticketed_invocation(
            &ticketed_runtime_ingress_authority(),
            submission,
            &ticket,
            envelope,
        )
        .unwrap();
    let ticketed_ingress_id = match staged {
        TicketedRuntimeIngressDisposition::Staged { record, .. } => record.ticketed_ingress_id,
        TicketedRuntimeIngressDisposition::Duplicate { .. } => {
            panic!("first ticketed ingress duplicated")
        }
    };

    assert!(runtime
        .receipt_correlation_for_ticketed_ingress(&ticketed_ingress_id)
        .is_none());
    assert!(runtime
        .receipt_correlation_for_submission(&submission)
        .is_none());
    assert_eq!(runtime.receipt_correlation_count(), 0);
}

#[test]
fn unknown_submission_outcome_observation_is_unknown() {
    let runtime = WorldlineRuntime::new();
    let unknown_submission = [42; 32];

    assert!(matches!(
        runtime.observe_intent_outcome(&unknown_submission),
        IntentOutcomeObservation::UnknownSubmission { submission_id }
            if submission_id == unknown_submission
    ));
}

#[test]
fn witnessed_submission_outcome_observation_is_pending_without_tick() {
    let mut runtime = WorldlineRuntime::new();
    let worldline_id = wl(1);
    runtime
        .register_worldline(worldline_id, WorldlineState::empty())
        .unwrap();
    register_head(&mut runtime, worldline_id, "default", None, true);
    let envelope = IngressEnvelope::local_intent(
        IngressTarget::DefaultWriter { worldline_id },
        make_intent_kind("test/runtime"),
        b"pending-outcome".to_vec(),
    );
    let (submission, generation) = match runtime.submit_intent(envelope).unwrap() {
        IntentSubmissionDisposition::Accepted {
            submission_id,
            submission_generation,
            ..
        } => (submission_id, submission_generation),
        IntentSubmissionDisposition::Duplicate { .. } => panic!("first submission duplicated"),
    };

    assert!(matches!(
        runtime.observe_intent_outcome(&submission),
        IntentOutcomeObservation::Pending {
            submission_id,
            submission_generation,
            ticketed_ingress_id: None,
        } if submission_id == submission && submission_generation == generation
    ));
}

#[test]
#[cfg(feature = "host_test")]
fn ticketed_ingress_correlates_tick_receipt_after_scheduler_owned_tick() {
    let mut runtime = WorldlineRuntime::new();
    let mut engine = empty_engine();
    let worldline_id = wl(1);
    runtime
        .register_worldline(worldline_id, WorldlineState::empty())
        .unwrap();
    let head_key = register_head(&mut runtime, worldline_id, "default", None, true);
    let envelope = IngressEnvelope::local_intent(
        IngressTarget::DefaultWriter { worldline_id },
        make_intent_kind("test/runtime"),
        b"correlate-after-tick".to_vec(),
    );
    let submission = match runtime.submit_intent(envelope.clone()).unwrap() {
        IntentSubmissionDisposition::Accepted { submission_id, .. } => submission_id,
        IntentSubmissionDisposition::Duplicate { .. } => panic!("first submission duplicated"),
    };
    let ticket = admission_ticket(5);
    let staged = runtime
        .ingest_ticketed_invocation(
            &ticketed_runtime_ingress_authority(),
            submission,
            &ticket,
            envelope.clone(),
        )
        .unwrap();
    let ticketed_ingress_id = match staged {
        TicketedRuntimeIngressDisposition::Staged { record, .. } => record.ticketed_ingress_id,
        TicketedRuntimeIngressDisposition::Duplicate { .. } => {
            panic!("first ticketed ingress duplicated")
        }
    };

    let mut provenance = registered_worldlines_provenance(&runtime);
    let records =
        SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();

    assert_eq!(records.len(), 1);
    let correlation = runtime
        .receipt_correlation_for_ticketed_ingress(&ticketed_ingress_id)
        .expect("ticketed ingress should correlate after scheduler tick");
    let frontier = runtime.worldlines().get(&worldline_id).unwrap();
    let (_, receipt, _) = frontier
        .state()
        .tick_history()
        .last()
        .expect("scheduler-owned tick should be recorded");
    assert_eq!(correlation.ticketed_ingress_id, ticketed_ingress_id);
    assert_eq!(correlation.submission_id, submission);
    assert_eq!(correlation.ticket_digest, ticket.ticket_digest);
    assert_eq!(correlation.ingress_id, envelope.ingress_id());
    assert_eq!(correlation.head_key, head_key);
    assert_eq!(
        correlation.commit_global_tick,
        records[0].commit_global_tick
    );
    assert_eq!(
        correlation.worldline_tick_after,
        records[0].worldline_tick_after
    );
    assert_eq!(correlation.tick_receipt_digest, receipt.digest());
    assert_eq!(correlation.commit_hash, records[0].commit_hash);
    assert_eq!(
        runtime
            .receipt_correlation_for_submission(&submission)
            .unwrap(),
        correlation
    );
}

#[test]
#[cfg(feature = "host_test")]
fn ticketed_submission_outcome_observation_is_decided_after_scheduler_tick() {
    let mut runtime = WorldlineRuntime::new();
    let mut engine = empty_engine();
    let worldline_id = wl(1);
    runtime
        .register_worldline(worldline_id, WorldlineState::empty())
        .unwrap();
    register_head(&mut runtime, worldline_id, "default", None, true);
    let envelope = IngressEnvelope::local_intent(
        IngressTarget::DefaultWriter { worldline_id },
        make_intent_kind("test/runtime"),
        b"decided-outcome".to_vec(),
    );
    let submission = match runtime.submit_intent(envelope.clone()).unwrap() {
        IntentSubmissionDisposition::Accepted { submission_id, .. } => submission_id,
        IntentSubmissionDisposition::Duplicate { .. } => panic!("first submission duplicated"),
    };
    let ticket = admission_ticket(6);
    runtime
        .ingest_ticketed_invocation(
            &ticketed_runtime_ingress_authority(),
            submission,
            &ticket,
            envelope,
        )
        .unwrap();

    let mut provenance = registered_worldlines_provenance(&runtime);
    SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();

    let correlation = runtime
        .receipt_correlation_for_submission(&submission)
        .expect("submission should have receipt correlation")
        .clone();
    assert!(matches!(
        runtime.observe_intent_outcome(&submission),
        IntentOutcomeObservation::Decided { correlation: observed }
            if observed == correlation
    ));
}

#[test]
#[cfg(feature = "host_test")]
fn legacy_ingress_without_ticket_does_not_create_receipt_correlation() {
    let mut runtime = WorldlineRuntime::new();
    let mut engine = empty_engine();
    let worldline_id = wl(1);
    runtime
        .register_worldline(worldline_id, WorldlineState::empty())
        .unwrap();
    register_head(&mut runtime, worldline_id, "default", None, true);
    let envelope = IngressEnvelope::local_intent(
        IngressTarget::DefaultWriter { worldline_id },
        make_intent_kind("test/runtime"),
        b"legacy-uncorrelated".to_vec(),
    );

    assert!(matches!(
        runtime.ingest(envelope).unwrap(),
        IngressDisposition::Accepted { .. }
    ));
    let mut provenance = registered_worldlines_provenance(&runtime);
    let records =
        SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();

    assert_eq!(records.len(), 1);
    assert_eq!(runtime.receipt_correlation_count(), 0);
}

#[test]
fn runtime_ingest_is_idempotent_per_resolved_head_after_commit() {
    let mut runtime = WorldlineRuntime::new();
    let mut engine = empty_engine();
    let worldline_id = wl(1);
    runtime
        .register_worldline(worldline_id, WorldlineState::empty())
        .unwrap();
    let default_key = register_head(&mut runtime, worldline_id, "default", None, true);
    let named_key = register_head(&mut runtime, worldline_id, "orders", Some("orders"), false);

    let kind = make_intent_kind("test/runtime");
    let default_env = IngressEnvelope::local_intent(
        IngressTarget::DefaultWriter { worldline_id },
        kind,
        b"same-intent".to_vec(),
    );
    let named_env = IngressEnvelope::local_intent(
        IngressTarget::InboxAddress {
            worldline_id,
            inbox: InboxAddress("orders".to_owned()),
        },
        kind,
        b"same-intent".to_vec(),
    );
    let default_ingress_id = default_env.ingress_id();

    assert!(matches!(
        runtime.ingest(default_env.clone()).unwrap(),
        IngressDisposition::Accepted {
            ingress_id,
            head_key,
            ..
        } if ingress_id == default_ingress_id && head_key == default_key
    ));
    let mut provenance = registered_worldlines_provenance(&runtime);
    SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();

    assert!(matches!(
        runtime.ingest(default_env).unwrap(),
        IngressDisposition::Duplicate {
            ingress_id,
            head_key,
            ..
        } if ingress_id == default_ingress_id && head_key == default_key
    ));
    assert!(matches!(
        runtime.ingest(named_env.clone()).unwrap(),
        IngressDisposition::Accepted {
            ingress_id,
            head_key,
            ..
        } if ingress_id == named_env.ingress_id() && head_key == named_key
    ));
    SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();
    let named_ingress_id = named_env.ingress_id();
    assert!(matches!(
        runtime.ingest(named_env).unwrap(),
        IngressDisposition::Duplicate {
            ingress_id,
            head_key,
            ..
        } if ingress_id == named_ingress_id && head_key == named_key
    ));
}

#[test]
fn runtime_ingest_keeps_distinct_intents_as_distinct_event_nodes() {
    let mut runtime = WorldlineRuntime::new();
    let mut engine = empty_engine();
    let worldline_id = wl(1);
    runtime
        .register_worldline(worldline_id, WorldlineState::empty())
        .unwrap();
    register_head(&mut runtime, worldline_id, "default", None, true);

    let intent_a = IngressEnvelope::local_intent(
        IngressTarget::DefaultWriter { worldline_id },
        make_intent_kind("test/runtime"),
        b"intent-alpha".to_vec(),
    );
    let intent_b = IngressEnvelope::local_intent(
        IngressTarget::DefaultWriter { worldline_id },
        make_intent_kind("test/runtime"),
        b"intent-beta".to_vec(),
    );

    runtime.ingest(intent_a.clone()).unwrap();
    runtime.ingest(intent_b.clone()).unwrap();

    let mut provenance = registered_worldlines_provenance(&runtime);
    let records =
        SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].admitted_count, 2);

    let store = runtime_store(&runtime, worldline_id);
    assert!(store.node(&NodeId(intent_a.ingress_id())).is_some());
    assert!(store.node(&NodeId(intent_b.ingress_id())).is_some());
}

#[test]
fn runtime_commit_patch_replays_to_post_state() {
    let mut runtime = WorldlineRuntime::new();
    let mut engine = empty_engine();
    let worldline_id = wl(1);
    runtime
        .register_worldline(worldline_id, WorldlineState::empty())
        .unwrap();
    register_head(&mut runtime, worldline_id, "default", None, true);

    runtime
        .ingest(IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            make_intent_kind("test/runtime"),
            b"patch-replay".to_vec(),
        ))
        .unwrap();

    let mut provenance = registered_worldlines_provenance(&runtime);
    let records =
        SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();
    assert_eq!(records.len(), 1);

    let frontier = runtime.worldlines().get(&worldline_id).unwrap();
    let (snapshot, _receipt, patch) = frontier.state().tick_history().last().unwrap().clone();

    let mut replay_state = frontier.state().initial_state().clone();
    patch.apply_to_state(&mut replay_state).unwrap();
    let replay_root = engine
        .snapshot_for_state(&WorldlineState::new(replay_state, *frontier.state().root()).unwrap())
        .state_root;

    assert_eq!(
        replay_root, snapshot.state_root,
        "runtime tick patch must replay to the committed post-state"
    );
}

#[test]
fn runtime_commit_provenance_matches_worldline_state_mirror() {
    let mut runtime = WorldlineRuntime::new();
    let mut engine = empty_engine();
    let worldline_id = wl(1);
    runtime
        .register_worldline(worldline_id, WorldlineState::empty())
        .unwrap();
    let head_key = register_head(&mut runtime, worldline_id, "default", None, true);

    runtime
        .ingest(IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            make_intent_kind("test/runtime"),
            b"mirror-consistency".to_vec(),
        ))
        .unwrap();

    let mut provenance = registered_worldlines_provenance(&runtime);
    let records =
        SchedulerCoordinator::super_tick(&mut runtime, &mut provenance, &mut engine).unwrap();
    assert_eq!(records.len(), 1);

    let frontier = runtime.worldlines().get(&worldline_id).unwrap();
    let state = frontier.state();
    let (snapshot, _receipt, patch) = state.tick_history().last().unwrap().clone();
    let entry = provenance.entry(worldline_id, wt(0)).unwrap();
    let expected_outputs = state
        .last_materialization()
        .iter()
        .map(|channel| (channel.channel, channel.data.clone()))
        .collect::<Vec<_>>();

    assert_eq!(entry.worldline_id, worldline_id);
    assert_eq!(entry.worldline_tick, wt(0));
    assert_eq!(entry.commit_global_tick, runtime.global_tick());
    assert_eq!(entry.head_key, Some(head_key));
    assert!(matches!(entry.event_kind, ProvenanceEventKind::LocalCommit));
    assert!(
        entry.parents.is_empty(),
        "first local commit should be parentless"
    );
    assert_eq!(entry.expected.state_root, snapshot.state_root);
    assert_eq!(entry.expected.patch_digest, snapshot.patch_digest);
    assert_eq!(entry.expected.commit_hash, snapshot.hash);
    assert_eq!(entry.outputs, expected_outputs);

    let expected_patch = WorldlineTickPatchV1 {
        header: warp_core::WorldlineTickHeaderV1 {
            commit_global_tick: runtime.global_tick(),
            policy_id: patch.policy_id(),
            rule_pack_id: patch.rule_pack_id(),
            plan_digest: snapshot.plan_digest,
            decision_digest: snapshot.decision_digest,
            rewrites_digest: snapshot.rewrites_digest,
        },
        warp_id: snapshot.root.warp_id,
        ops: patch.ops().to_vec(),
        in_slots: patch.in_slots().to_vec(),
        out_slots: patch.out_slots().to_vec(),
        patch_digest: patch.digest(),
    };
    assert_eq!(entry.patch, Some(expected_patch));
}
