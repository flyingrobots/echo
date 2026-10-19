// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![cfg(feature = "host_test")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
// The helper returns the runtime API's own error type unchanged.
#![allow(clippy::result_large_err)]
//! Comparison bases for witnessed submission and ticketed staging (T27, T34).
//!
//! Each case builds two runtimes with equal committed state and offers both
//! the same request through the same API. A test-only [`AdmissionReference`]
//! names the state each API reads. Where the references are equal, the
//! results and transitions must be equal (T27). Where only the committed
//! state is equal, the cases show which omitted layer changes the result
//! (T34). The reference is not a production digest.

use std::collections::BTreeSet;

use warp_core::{
    make_head_id, make_intent_kind, ContinuumOccurrence, Hash, InboxPolicy, IngressEnvelope,
    IngressSubmissionGeneration, IngressTarget, IntentSubmissionDisposition,
    IntentSubmissionRecord, OpticAdmissionTicket, OpticArtifactHandle, PlaybackMode, RuntimeError,
    TicketedRuntimeIngressAuthority, TicketedRuntimeIngressDisposition,
    TicketedRuntimeIngressRecord, WitnessedSubmissionPersistenceRecord,
    WitnessedSubmissionPersistenceSnapshot, WorldlineId, WorldlineRuntime, WorldlineState,
    WorldlineTick, WriterHead, WriterHeadKey, OPTIC_ADMISSION_TICKET_KIND,
    OPTIC_ARTIFACT_HANDLE_KIND,
};

const KIND: &str = "test/comparison";

fn worldline() -> WorldlineId {
    WorldlineId::from_bytes([7; 32])
}

fn head(label: &str) -> WriterHeadKey {
    WriterHeadKey {
        worldline_id: worldline(),
        head_id: make_head_id(label),
    }
}

/// One worldline with two writer heads, `first` and `second`.
fn runtime_with(policy: InboxPolicy, default_writer: &str) -> WorldlineRuntime {
    let mut runtime = WorldlineRuntime::new();
    runtime
        .register_worldline(worldline(), WorldlineState::empty())
        .unwrap();
    for label in ["first", "second"] {
        runtime
            .register_writer_head(WriterHead::with_routing(
                head(label),
                PlaybackMode::Play,
                policy.clone(),
                None,
                label == default_writer,
            ))
            .unwrap();
    }
    runtime
}

fn runtime() -> WorldlineRuntime {
    runtime_with(InboxPolicy::AcceptAll, "first")
}

fn intent(label: &str) -> IngressEnvelope {
    IngressEnvelope::local_intent(
        IngressTarget::DefaultWriter {
            worldline_id: worldline(),
        },
        make_intent_kind(KIND),
        label.as_bytes().to_vec(),
    )
}

fn occurred(label: &str, key: u8, applied: u8) -> IngressEnvelope {
    IngressEnvelope::continuum_occurred_intent(
        IngressTarget::DefaultWriter {
            worldline_id: worldline(),
        },
        make_intent_kind(KIND),
        label.as_bytes().to_vec(),
        Vec::new(),
        ContinuumOccurrence::new([key; 32], [applied; 32]),
    )
}

fn ticket(seed: u8) -> OpticAdmissionTicket {
    OpticAdmissionTicket {
        kind: OPTIC_ADMISSION_TICKET_KIND.to_owned(),
        artifact_handle: OpticArtifactHandle {
            kind: OPTIC_ARTIFACT_HANDLE_KIND.to_owned(),
            id: format!("comparison-ticket-{seed}"),
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

fn accept(runtime: &mut WorldlineRuntime, envelope: &IngressEnvelope) -> IntentSubmissionRecord {
    match runtime.submit_intent(envelope.clone()).unwrap() {
        IntentSubmissionDisposition::Accepted {
            submission_id,
            ingress_id,
            head_key,
            submission_generation,
        } => IntentSubmissionRecord {
            submission_id,
            ingress_id,
            head_key,
            submission_generation,
        },
        other => panic!("the fixture submission is new: {other:?}"),
    }
}

fn stage(
    runtime: &mut WorldlineRuntime,
    submission_id: Hash,
    seed: u8,
    envelope: &IngressEnvelope,
) -> Result<TicketedRuntimeIngressDisposition, RuntimeError> {
    runtime.ingest_ticketed_invocation(
        &TicketedRuntimeIngressAuthority::assume_runtime_owner(),
        submission_id,
        &ticket(seed),
        envelope.clone(),
    )
}

/// The state the two APIs read, as a test-only reference tuple.
///
/// `witnessed` carries the generations, which stand in for the allocator in
/// these fixtures: no fixture restores an allocator above its records.
#[derive(Debug, PartialEq, Eq)]
struct AdmissionReference {
    state_root: Hash,
    frontier_tick: WorldlineTick,
    witnessed: Vec<IntentSubmissionRecord>,
    pending_witnessed: usize,
    staged: Vec<TicketedRuntimeIngressRecord>,
    inbox_pending: Vec<(WriterHeadKey, usize)>,
}

impl AdmissionReference {
    fn of(runtime: &WorldlineRuntime) -> Self {
        let frontier = runtime.worldlines().get(&worldline()).unwrap();
        Self {
            state_root: frontier.state().state_root(),
            frontier_tick: frontier.frontier_tick(),
            witnessed: runtime.witnessed_submissions().cloned().collect(),
            pending_witnessed: runtime.pending_witnessed_submission_count(),
            staged: runtime
                .ticketed_runtime_ingress_records()
                .cloned()
                .collect(),
            inbox_pending: ["first", "second"]
                .map(|label| {
                    let key = head(label);
                    (
                        key,
                        runtime.heads().get(&key).unwrap().inbox().pending_count(),
                    )
                })
                .to_vec(),
        }
    }

    fn committed(&self) -> (Hash, WorldlineTick) {
        (self.state_root, self.frontier_tick)
    }
}

/// Result fields, as the canonical variant and its fields. `RuntimeError` has
/// no equality, so both sides compare through the same formatting.
fn shown<T: std::fmt::Debug>(result: &Result<T, RuntimeError>) -> String {
    format!("{result:?}")
}

/// A history with all three admission layers occupied: one accepted
/// submission, one staged submission and nothing ticked.
fn layered() -> (
    WorldlineRuntime,
    IntentSubmissionRecord,
    IntentSubmissionRecord,
) {
    let mut runtime = runtime();
    let staged = accept(&mut runtime, &intent("staged"));
    let accepted = accept(&mut runtime, &intent("accepted"));
    stage(&mut runtime, staged.submission_id, 1, &intent("staged")).unwrap();
    (runtime, staged, accepted)
}

#[test]
/// T27: equal admission references give equal results and equal transitions
/// for each API and request in the battery, including refusals.
fn t27_equal_admission_references_give_equal_results() {
    type Request =
        fn(&mut WorldlineRuntime, &IntentSubmissionRecord, &IntentSubmissionRecord) -> String;
    // Each request names the variant both runtimes must return, so the
    // comparison cannot pass vacuously on one repeated outcome.
    let battery: [(&str, &str, Request); 7] = [
        ("submit staged", "Ok(Duplicate", |r, _, _| {
            shown(&r.submit_intent(intent("staged")))
        }),
        ("submit accepted", "Ok(Duplicate", |r, _, _| {
            shown(&r.submit_intent(intent("accepted")))
        }),
        ("submit new", "Ok(Accepted", |r, _, _| {
            shown(&r.submit_intent(intent("new")))
        }),
        ("restage same ticket", "Ok(Duplicate", |r, s, _| {
            shown(&stage(r, s.submission_id, 1, &intent("staged")))
        }),
        (
            "restage other ticket",
            "Err(TicketedIngressAlreadyStaged",
            |r, s, _| shown(&stage(r, s.submission_id, 2, &intent("staged"))),
        ),
        ("stage accepted", "Ok(Staged", |r, _, a| {
            shown(&stage(r, a.submission_id, 3, &intent("accepted")))
        }),
        ("stage unknown", "Err(UnknownIntentSubmission", |r, _, _| {
            shown(&stage(r, [9; 32], 4, &intent("new")))
        }),
    ];
    for (name, variant, request) in battery {
        let (mut left, left_staged, left_accepted) = layered();
        let (mut right, right_staged, right_accepted) = layered();
        assert_eq!(
            AdmissionReference::of(&left),
            AdmissionReference::of(&right)
        );
        let left_result = request(&mut left, &left_staged, &left_accepted);
        let right_result = request(&mut right, &right_staged, &right_accepted);
        assert!(left_result.starts_with(variant), "{name}: {left_result}");
        assert_eq!(left_result, right_result, "{name}: result");
        assert_eq!(
            AdmissionReference::of(&left),
            AdmissionReference::of(&right),
            "{name}: transition"
        );
    }
}

#[test]
/// T34(a): accepted but unstaged. The committed state and even the resulting
/// state agree; the returned variant does not.
fn t34_accepted_unstaged_changes_the_submission_result() {
    let mut held = runtime();
    let original = accept(&mut held, &intent("x"));
    let mut fresh = runtime();
    assert_eq!(
        AdmissionReference::of(&held).committed(),
        AdmissionReference::of(&fresh).committed()
    );

    let retry = held.submit_intent(intent("x")).unwrap();
    let first = fresh.submit_intent(intent("x")).unwrap();
    assert!(matches!(
        retry,
        IntentSubmissionDisposition::Duplicate { submission_id, submission_generation, .. }
            if submission_id == original.submission_id
                && submission_generation == original.submission_generation
    ));
    assert!(matches!(
        first,
        IntentSubmissionDisposition::Accepted { .. }
    ));
    assert_eq!(
        AdmissionReference::of(&held),
        AdmissionReference::of(&fresh),
        "a transition comparison alone would miss the different variant"
    );
}

#[test]
/// T34(b): staged but unticked. Both runtimes share the witnessed layer; only
/// staging differs. Witnessed submission does not read the staging layer;
/// ticketed staging does, and distinguishes same and different tickets.
fn t34_staged_unticked_changes_only_the_staging_result() {
    let mut staged = runtime();
    let record = accept(&mut staged, &intent("x"));
    stage(&mut staged, record.submission_id, 1, &intent("x")).unwrap();
    let mut unstaged = runtime();
    assert_eq!(accept(&mut unstaged, &intent("x")), record);

    let before_staged = AdmissionReference::of(&staged);
    let before_unstaged = AdmissionReference::of(&unstaged);
    assert_eq!(before_staged.committed(), before_unstaged.committed());
    assert_eq!(before_staged.witnessed, before_unstaged.witnessed);
    assert_eq!(
        before_staged.pending_witnessed,
        before_unstaged.pending_witnessed
    );
    assert_ne!(before_staged.staged, before_unstaged.staged);

    assert_eq!(
        shown(&staged.submit_intent(intent("x"))),
        shown(&unstaged.submit_intent(intent("x"))),
        "witnessed submission reads no staging state"
    );

    let mut unstaged_other = runtime();
    accept(&mut unstaged_other, &intent("x"));
    assert!(matches!(
        stage(&mut staged, record.submission_id, 1, &intent("x")),
        Ok(TicketedRuntimeIngressDisposition::Duplicate { .. })
    ));
    assert!(matches!(
        stage(&mut staged, record.submission_id, 2, &intent("x")),
        Err(RuntimeError::TicketedIngressAlreadyStaged(id)) if id == record.submission_id
    ));
    assert_eq!(
        AdmissionReference::of(&staged),
        before_staged,
        "refusals write nothing"
    );
    assert!(matches!(
        stage(&mut unstaged, record.submission_id, 1, &intent("x")),
        Ok(TicketedRuntimeIngressDisposition::Staged { .. })
    ));
    assert!(matches!(
        stage(&mut unstaged_other, record.submission_id, 2, &intent("x")),
        Ok(TicketedRuntimeIngressDisposition::Staged { .. })
    ));
}

#[test]
/// T34(c): an occurrence key bound to other material refuses submission, and
/// refuses restaging before the staged-duplicate return even with the same
/// ticket. The applied digest is not part of the ingress id.
fn t34_occurrence_binding_precedes_duplicate_returns() {
    let mut bound = runtime();
    let original = accept(&mut bound, &occurred("x", 1, 0xa1));
    stage(
        &mut bound,
        original.submission_id,
        1,
        &occurred("x", 1, 0xa1),
    )
    .unwrap();
    let mut fresh = runtime();
    assert_eq!(
        AdmissionReference::of(&bound).committed(),
        AdmissionReference::of(&fresh).committed()
    );
    assert_eq!(
        occurred("x", 1, 0xa1).ingress_id(),
        occurred("x", 1, 0xb2).ingress_id(),
        "the applied digest does not change the ingress id"
    );

    let conflict = |result: String| result.contains("ContinuumOccurrenceConflict");
    assert!(conflict(shown(
        &bound.submit_intent(occurred("y", 1, 0xa1))
    )));
    assert!(matches!(
        fresh.submit_intent(occurred("y", 1, 0xa1)),
        Ok(IntentSubmissionDisposition::Accepted { .. })
    ));
    assert!(conflict(shown(&stage(
        &mut bound,
        original.submission_id,
        1,
        &occurred("x", 1, 0xb2),
    ))));
    assert!(matches!(
        stage(
            &mut bound,
            original.submission_id,
            1,
            &occurred("x", 1, 0xa1)
        ),
        Ok(TicketedRuntimeIngressDisposition::Duplicate { .. })
    ));
}

#[test]
/// T34(d): routing and policy are configuration, not committed state.
fn t34_routing_and_policy_change_results_at_equal_committed_state() {
    let mut first = runtime_with(InboxPolicy::AcceptAll, "first");
    let mut second = runtime_with(InboxPolicy::AcceptAll, "second");
    let mut filtered = runtime_with(
        InboxPolicy::KindFilter(BTreeSet::from([make_intent_kind("test/other")])),
        "first",
    );
    let committed = AdmissionReference::of(&first).committed();
    assert_eq!(AdmissionReference::of(&second).committed(), committed);
    assert_eq!(AdmissionReference::of(&filtered).committed(), committed);

    let routed_first = accept(&mut first, &intent("x"));
    let routed_second = accept(&mut second, &intent("x"));
    assert_eq!(routed_first.head_key, head("first"));
    assert_eq!(routed_second.head_key, head("second"));
    assert_ne!(routed_first.submission_id, routed_second.submission_id);

    assert!(matches!(
        filtered.submit_intent(intent("x")),
        Err(RuntimeError::RejectedByPolicy(key)) if key == head("first")
    ));
    assert_eq!(AdmissionReference::of(&filtered).witnessed, Vec::new());
}

#[test]
/// T34(e): the default full-result comparison includes the allocator. At the
/// last generation, a new submission refuses while a retry of a known one
/// still returns its duplicate.
fn t34_allocation_exhaustion_is_part_of_the_submission_basis() {
    let mut source = runtime();
    let known = accept(&mut source, &intent("known"));
    let envelope = source
        .witnessed_submission_envelope(&known.submission_id)
        .unwrap()
        .clone();
    let at_last = |generation: IngressSubmissionGeneration| {
        let mut restored = runtime();
        restored
            .restore_witnessed_submission_persistence(WitnessedSubmissionPersistenceSnapshot::new(
                vec![WitnessedSubmissionPersistenceRecord {
                    submission: IntentSubmissionRecord {
                        submission_generation: generation,
                        ..known.clone()
                    },
                    envelope: envelope.clone(),
                }],
            ))
            .unwrap();
        restored
    };
    let mut exhausted = at_last(IngressSubmissionGeneration::MAX);
    let mut ordinary = at_last(IngressSubmissionGeneration::from_raw(1));
    assert_eq!(
        AdmissionReference::of(&exhausted).committed(),
        AdmissionReference::of(&ordinary).committed()
    );

    assert!(matches!(
        exhausted.submit_intent(intent("new")),
        Err(RuntimeError::IntentSubmissionGenerationOverflow)
    ));
    assert!(matches!(
        ordinary.submit_intent(intent("new")),
        Ok(IntentSubmissionDisposition::Accepted { submission_generation, .. })
            if submission_generation == IngressSubmissionGeneration::from_raw(2)
    ));
    assert!(matches!(
        exhausted.submit_intent(intent("known")),
        Ok(IntentSubmissionDisposition::Duplicate { submission_generation, .. })
            if submission_generation == IngressSubmissionGeneration::MAX
    ));
    assert!(
        matches!(
            stage(&mut exhausted, known.submission_id, 1, &intent("known")),
            Ok(TicketedRuntimeIngressDisposition::Staged { .. })
        ),
        "staging a known submission allocates no generation"
    );
}

#[test]
/// T34(f): the witnessed-submission persistence image carries the witnessed
/// layer and not the staging layer, as its contract says. A restored runtime
/// stages again where the original reports a duplicate.
fn t34_persistence_image_restores_witnessed_but_not_staged_work() {
    let mut original = runtime();
    let record = accept(&mut original, &intent("x"));
    stage(&mut original, record.submission_id, 1, &intent("x")).unwrap();
    let mut restored = runtime();
    restored
        .restore_witnessed_submission_persistence(
            original
                .witnessed_submission_persistence_snapshot()
                .unwrap(),
        )
        .unwrap();

    let before_original = AdmissionReference::of(&original);
    let before_restored = AdmissionReference::of(&restored);
    assert_eq!(before_original.witnessed, before_restored.witnessed);
    assert_eq!(before_restored.staged, Vec::new());
    assert_eq!(
        before_restored.inbox_pending,
        vec![(head("first"), 0), (head("second"), 0)]
    );
    assert_eq!(
        before_original.inbox_pending,
        vec![(head("first"), 1), (head("second"), 0)]
    );

    assert!(matches!(
        stage(&mut original, record.submission_id, 1, &intent("x")),
        Ok(TicketedRuntimeIngressDisposition::Duplicate { .. })
    ));
    assert!(matches!(
        stage(&mut restored, record.submission_id, 1, &intent("x")),
        Ok(TicketedRuntimeIngressDisposition::Staged { .. })
    ));
}
