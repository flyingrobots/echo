// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Runtime-owned immutable observation and logical-request bindings.
use super::{
    BTreeMap, Error, Hash, TrustedRuntimeHost, TrustedRuntimeWal, WalAppendAuthority,
    WalTransactionId, WalTransactionKind,
};
use crate::EchoOperationAnchoredNodeOccupancyV1 as Occupancy;
use crate::{
    EchoOperationInvocation, EchoOperationObservationV1, NodeKey, ProvenanceStore, WriterHeadKey,
};
use echo_edict_canonical::{
    decode_canonical_cbor_v1, encode_canonical_cbor_v1, CanonicalValueV1 as V,
};

/// Failure to retain or resolve an immutable operation context.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("operation context: {0}")]
pub struct EchoOperationContextErrorV1(String);

fn error(value: impl std::fmt::Display) -> EchoOperationContextErrorV1 {
    EchoOperationContextErrorV1(value.to_string())
}

type Result<T> = std::result::Result<T, EchoOperationContextErrorV1>;
const SCHEMA: &str = "echo.operation-context/v1";

/// Observations one WAL root retains over its whole lifetime.
///
/// Contexts are never retired: a retained observation must keep resolving for
/// every later retry of a request bound to it, and no retirement rule exists
/// yet (#854). Once a root holds this many, it refuses every new observation,
/// including after reopening. A host that needs more starts a new WAL root.
pub(super) const MAX_OBSERVATIONS_PER_WAL_ROOT: usize = 1024;

/// Logical request bindings one WAL root retains over its whole lifetime,
/// under the same rule as [`MAX_OBSERVATIONS_PER_WAL_ROOT`].
pub(super) const MAX_REQUESTS_PER_WAL_ROOT: usize = 4096;

fn field_text(value: &V) -> Result<&str> {
    if let V::Text(value) = value {
        Ok(value)
    } else {
        Err(error("invalid context text"))
    }
}
fn field_bytes(value: &V) -> Result<&[u8]> {
    if let V::Bytes(value) = value {
        Ok(value)
    } else {
        Err(error("invalid context bytes"))
    }
}
fn check_id(id: &str) -> Result<()> {
    if id.is_empty() || id.len() > 128 {
        return Err(error("identity must contain 1..128 bytes"));
    }
    Ok(())
}

#[derive(Debug, Clone, Default)]
pub(super) struct Contexts {
    observations: BTreeMap<String, EchoOperationObservationV1>,
    requests: BTreeMap<String, (String, Hash, Vec<u8>)>,
    /// Durable link from a logical request to the submission that carries it
    /// (#854, R1).
    submissions: BTreeMap<String, Hash>,
}

impl Contexts {
    pub(super) fn from_recovery(report: &crate::causal_wal::RecoveryScanReport) -> Result<Self> {
        let mut contexts = Self::default();
        for transaction in &report.transactions {
            contexts.apply_frames(&transaction.frames)?;
        }
        Ok(contexts)
    }

    pub(super) fn apply_frames(&mut self, frames: &[crate::causal_wal::WalFrame]) -> Result<()> {
        for frame in frames {
            if frame.header.record_kind
                != crate::causal_wal::WalRecordKind::ExecutableOperationContextRetained
            {
                continue;
            }
            let V::Array(fields) =
                decode_canonical_cbor_v1(&frame.payload.canonical_bytes).map_err(error)?
            else {
                return Err(error("invalid context record"));
            };
            if fields.len() < 4 || field_text(&fields[0])? != SCHEMA {
                return Err(error("invalid context schema"));
            }
            let id = field_text(&fields[2])?.to_owned();
            check_id(&id)?;
            match field_text(&fields[1])? {
                "observation" if fields.len() == 4 => {
                    let observation = EchoOperationObservationV1::decode(field_bytes(&fields[3])?)
                        .map_err(error)?;
                    if self.observations.insert(id, observation).is_some() {
                        return Err(error("duplicate observation identity"));
                    }
                }
                "request" if fields.len() == 6 => {
                    let attempt = field_text(&fields[3])?.to_owned();
                    let digest: Hash = field_bytes(&fields[4])?.try_into().map_err(error)?;
                    let invocation_bytes = field_bytes(&fields[5])?.to_vec();
                    let invocation =
                        EchoOperationInvocation::from_canonical_bytes(&invocation_bytes)
                            .map_err(error)?;
                    let observation = self
                        .observations
                        .get(&attempt)
                        .ok_or_else(|| error("request observation unavailable"))?;
                    if !invocation.has_observation(observation)
                        || invocation
                            .observed_semantic_identity(observation)
                            .map_err(error)?
                            != digest
                    {
                        return Err(error("request identity mismatch"));
                    }
                    if self
                        .requests
                        .insert(id, (attempt, digest, invocation_bytes))
                        .is_some()
                    {
                        return Err(error("duplicate request identity"));
                    }
                }
                "submission" if fields.len() == 4 => {
                    if !self.requests.contains_key(&id) {
                        return Err(error("submission link names an unbound request"));
                    }
                    let submission: Hash = field_bytes(&fields[3])?.try_into().map_err(error)?;
                    if self.submissions.insert(id, submission).is_some() {
                        return Err(error("duplicate submission link"));
                    }
                }
                _ => return Err(error("invalid context record kind")),
            }
        }
        Ok(())
    }
}

impl TrustedRuntimeWal {
    fn operation_contexts(&self) -> Result<std::borrow::Cow<'_, Contexts>> {
        if let Some((prefix, contexts)) = &self.operation_context_index {
            if *prefix == self.previous_committed_transaction_digest {
                return Ok(std::borrow::Cow::Borrowed(contexts));
            }
        }
        // A read after an uncertain append resolves retained truth, but does not
        // restore permission to append from the old writer cursor.
        let report = self.store.recover_read_only().map_err(error)?;
        Ok(std::borrow::Cow::Owned(Contexts::from_recovery(&report)?))
    }

    fn reconcile_operation_context_prefix(&mut self) -> Result<()> {
        if self
            .operation_context_index
            .as_ref()
            .is_none_or(|(prefix, _)| *prefix != self.previous_committed_transaction_digest)
        {
            self.refresh_cursor_from_store_for_writer().map_err(error)?;
        }
        Ok(())
    }

    fn retain_operation_context(&mut self, fields: Vec<V>) -> Result<()> {
        self.reconcile_operation_context_prefix()?;
        let bytes = encode_canonical_cbor_v1(&V::Array(fields)).map_err(error)?;
        let transaction_id = WalTransactionId::from_hash(*blake3::hash(&bytes).as_bytes());
        let mut builder = self.builder(
            WalTransactionKind::RuntimePosture,
            WalAppendAuthority::RuntimeControl,
            transaction_id,
        );
        builder
            .push_record(
                crate::causal_wal::WalRecordKind::ExecutableOperationContextRetained,
                bytes,
            )
            .map_err(error)?;
        self.append_transaction(builder.commit(Vec::new()).map_err(error)?)
            .map_err(error)?;
        Ok(())
    }
}

impl TrustedRuntimeHost {
    /// Retains a bounded observation before supplying it to an attempt.
    ///
    /// The trusted host controls the aperture. Reusing an attempt cannot replace
    /// its observations. Missing WAL material obstructs instead of recapturing.
    pub fn retain_echo_operation_observation_v1(
        &mut self,
        attempt: &str,
        head: WriterHeadKey,
        nodes: &[NodeKey],
    ) -> Result<EchoOperationObservationV1> {
        check_id(attempt)?;
        let wal = self
            .runtime_wal
            .as_mut()
            .ok_or_else(|| error("durable WAL required"))?;
        wal.reconcile_operation_context_prefix()?;
        let contexts = wal.operation_contexts()?;
        if contexts.observations.contains_key(attempt) {
            return Err(error("attempt observation is immutable"));
        }
        if contexts.observations.len() >= MAX_OBSERVATIONS_PER_WAL_ROOT {
            return Err(error(format!(
                "context admission limit reached: this WAL root has retained \
                 {MAX_OBSERVATIONS_PER_WAL_ROOT} observations, and contexts are never retired"
            )));
        }
        let first = nodes
            .first()
            .ok_or_else(|| error("observation needs an aperture"))?;
        let state = self
            .runtime
            .worldlines()
            .get(&head.worldline_id)
            .ok_or_else(|| error("worldline unavailable"))?
            .state();
        let store = state
            .store(&first.warp_id)
            .ok_or_else(|| error("observation warp unavailable"))?;
        let occupancy = match (
            store.node(&first.local_id).is_some(),
            store.node_attachment(&first.local_id).is_some(),
        ) {
            (false, false) => Occupancy::Absent,
            (true, false) => Occupancy::NodeOnly,
            (false, true) => Occupancy::AttachmentOnly,
            (true, true) => Occupancy::NodeAndAttachment,
        };
        let application_basis =
            crate::echo_operation_anchored_node_creation_application_basis_v1(*first, occupancy);
        let basis = self
            .echo_operation_evaluation_basis_v1(head, application_basis)
            .map_err(error)?;
        let observation =
            EchoOperationObservationV1::capture(state, basis, nodes).map_err(error)?;
        self.runtime_wal
            .as_mut()
            .ok_or_else(|| error("durable WAL required"))?
            .retain_operation_context(vec![
                V::Text(SCHEMA.into()),
                V::Text("observation".into()),
                V::Text(attempt.into()),
                V::Bytes(observation.encode().map_err(error)?),
            ])?;
        Ok(observation)
    }

    /// Resolves exactly the retained reading, without refreshing its basis.
    pub fn echo_operation_observation_v1(
        &self,
        attempt: &str,
    ) -> Result<EchoOperationObservationV1> {
        self.runtime_wal
            .as_ref()
            .ok_or_else(|| error("durable WAL required"))?
            .operation_contexts()?
            .observations
            .get(attempt)
            .cloned()
            .ok_or_else(|| error("observation unavailable; re-observation requires a new attempt"))
    }

    /// Returns support changed now or written since this attempt's reading.
    pub fn echo_operation_observation_changes_v1(&self, attempt: &str) -> Result<Vec<NodeKey>> {
        self.echo_operation_observation_change_evidence_v1(attempt)
            .map(|(nodes, _)| nodes)
    }

    /// Retained commits which wrote the changed support after this reading.
    /// Unrelated commits and their payloads are excluded from the result.
    pub fn echo_operation_observation_change_commits_v1(&self, attempt: &str) -> Result<Vec<Hash>> {
        self.echo_operation_observation_change_evidence_v1(attempt)
            .map(|(_, commits)| commits)
    }

    /// Resolves aperture changes and their commit evidence together.
    ///
    /// Native retained writes remain discoverable after a value is restored.
    /// Current-value differences also remain visible. Missing retained provenance
    /// obstructs instead of being interpreted as an unchanged observation.
    pub fn echo_operation_observation_change_evidence_v1(
        &self,
        attempt: &str,
    ) -> Result<(Vec<NodeKey>, Vec<Hash>)> {
        let observation = self.echo_operation_observation_v1(attempt)?;
        let lane = observation.basis().writer_head().worldline_id;
        let worldline = self
            .runtime
            .worldlines()
            .get(&lane)
            .ok_or_else(|| error("worldline unavailable"))?;
        let mut changed = observation
            .changed_nodes(worldline.state())
            .map_err(error)?
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        let mut commits = Vec::new();
        // This index is populated by native commitment and reconstructed by WAL
        // recovery before the host becomes available. Do not reconstruct it for
        // each change query.
        for tick in
            observation.basis().worldline_tick().as_u64()..worldline.frontier_tick().as_u64()
        {
            let entry = self
                .provenance()
                .entry(lane, crate::WorldlineTick::from_raw(tick))
                .map_err(error)?;
            let Some(patch) = entry.patch else {
                continue;
            };
            let mut relevant = false;
            for (node, _) in observation.readings() {
                if patch.out_slots.contains(&crate::SlotId::Node(node))
                    || patch.out_slots.contains(&crate::SlotId::Attachment(
                        crate::AttachmentKey::node_alpha(node),
                    ))
                {
                    changed.insert(node);
                    relevant = true;
                }
            }
            if relevant {
                commits.push(entry.expected.commit_hash);
            }
        }
        Ok((changed.into_iter().collect(), commits))
    }

    /// Binds a logical request before ingress acceptance. Exact retry returns
    /// the original invocation, including its original submission basis.
    /// Changed semantic input under the same identity is refused.
    pub fn bind_echo_operation_request_v1(
        &mut self,
        request: &str,
        attempt: &str,
        invocation: EchoOperationInvocation,
    ) -> Result<Vec<u8>> {
        check_id(request)?;
        let wal = self
            .runtime_wal
            .as_mut()
            .ok_or_else(|| error("durable WAL required"))?;
        wal.reconcile_operation_context_prefix()?;
        let contexts = wal.operation_contexts()?;
        let observation = contexts
            .observations
            .get(attempt)
            .ok_or_else(|| error("observation unavailable"))?;
        let semantic = invocation
            .observed_semantic_identity(observation)
            .map_err(error)?;
        if let Some((original_attempt, original_semantic, original_bytes)) =
            contexts.requests.get(request)
        {
            if original_attempt != attempt || original_semantic != &semantic {
                return Err(error("logical request identity reused with changed input"));
            }
            return Ok(original_bytes.clone());
        }
        if contexts.requests.len() >= MAX_REQUESTS_PER_WAL_ROOT {
            return Err(error(format!(
                "request admission limit reached: this WAL root has retained \
                 {MAX_REQUESTS_PER_WAL_ROOT} request bindings, and contexts are never retired"
            )));
        }
        let bytes = invocation
            .with_observation(observation.clone())
            .to_canonical_bytes()
            .map_err(error)?;
        self.runtime_wal
            .as_mut()
            .ok_or_else(|| error("durable WAL required"))?
            .retain_operation_context(vec![
                V::Text(SCHEMA.into()),
                V::Text("request".into()),
                V::Text(request.into()),
                V::Text(attempt.into()),
                V::Bytes(semantic.to_vec()),
                V::Bytes(bytes.clone()),
            ])?;
        Ok(bytes)
    }

    /// Submits the Action bound to `request` to `target` and durably links
    /// the request to that submission (#854, R1).
    ///
    /// The submission is accepted durably first, then the link is retained.
    /// Retrying after a failure between the two re-derives the same submission
    /// (the envelope is the same content) and completes the link. A request is
    /// linked to exactly one submission: submitting it to a different target
    /// afterwards is refused.
    ///
    /// # Errors
    ///
    /// Refuses an unknown request, a request already linked to another
    /// submission, and any failure to submit or to retain the link durably.
    pub fn submit_bound_echo_operation_request_v1(
        &mut self,
        request: &str,
        target: crate::IngressTarget,
    ) -> Result<Hash> {
        check_id(request)?;
        let wal = self
            .runtime_wal
            .as_mut()
            .ok_or_else(|| error("durable WAL required"))?;
        wal.reconcile_operation_context_prefix()?;
        let contexts = wal.operation_contexts()?;
        let (_, _, bytes) = contexts
            .requests
            .get(request)
            .ok_or_else(|| error("unknown logical request"))?;
        let bytes = bytes.clone();
        let linked = contexts.submissions.get(request).copied();
        let envelope = crate::echo_operation::echo_operation_action_envelope_v1(target, bytes)
            .map_err(error)?;
        let submission = self
            .app()
            .submit_intent_with_runtime_wal_ack(envelope)
            .map_err(error)?
            .submission_id;
        if let Some(linked) = linked {
            return if linked == submission {
                Ok(submission)
            } else {
                Err(error("logical request already submitted to another target"))
            };
        }
        self.runtime_wal
            .as_mut()
            .ok_or_else(|| error("durable WAL required"))?
            .retain_operation_context(vec![
                V::Text(SCHEMA.into()),
                V::Text("submission".into()),
                V::Text(request.into()),
                V::Bytes(submission.to_vec()),
            ])?;
        Ok(submission)
    }

    /// Returns the submission durably linked to `request` and its recorded
    /// outcome, if one has been decided (#854, R1).
    ///
    /// `Ok(None)` means the request has not been submitted through
    /// [`Self::submit_bound_echo_operation_request_v1`].
    ///
    /// # Errors
    ///
    /// Refuses a malformed request identity or an unreadable WAL.
    pub fn echo_operation_request_outcome_v1(
        &self,
        request: &str,
    ) -> Result<Option<(Hash, Option<crate::EchoOperationActionOutcomeV1>)>> {
        check_id(request)?;
        let wal = self
            .runtime_wal
            .as_ref()
            .ok_or_else(|| error("durable WAL required"))?;
        let contexts = wal.operation_contexts()?;
        Ok(contexts.submissions.get(request).map(|submission| {
            (
                *submission,
                self.echo_operation_action_outcome_v1(submission).cloned(),
            )
        }))
    }
}
