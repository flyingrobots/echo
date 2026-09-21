// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Bounded trusted-local native strand retention and recovery.
use super::{
    restore_provenance_entries, Hash, ProvenanceService, ProvenanceStore, RuntimeWalActivationGap,
    TrustedRuntimeHost, TrustedRuntimeHostError, TrustedRuntimeWal, TrustedRuntimeWalError,
    TrustedRuntimeWalRecovery, WalAppendAuthority, WalTransactionId, WalTransactionKind,
    WorldlineRuntime,
};
use crate::causal_wal::{StrandForkRecord, WalRecordKind};
use crate::playback::SessionId;
use crate::{
    ActorId, AuthorityBinding, AuthorityDomainId, AuthorityDomainRef, CausalPosture,
    ForkStrandRequest, InboxPolicy, OriginId, PlaybackMode, RetentionContractId, SealStrength,
    SessionContext, StrandId, WorldlineTick, WriterHead, WriterHeadKey,
};

fn invalid() -> TrustedRuntimeHostError {
    TrustedRuntimeWalError::RuntimeAuthorityNotDurable {
        gap: RuntimeWalActivationGap::Provenance,
    }
    .into()
}
fn digest(domain: &[u8], identity: &[u8]) -> Hash {
    let mut hash = blake3::Hasher::new();
    hash.update(domain);
    hash.update(identity);
    *hash.finalize().as_bytes()
}
fn request(record: &StrandForkRecord) -> Result<ForkStrandRequest, TrustedRuntimeHostError> {
    let identity = *record.strand_id.as_bytes();
    let origin = OriginId::from_bytes(identity);
    let session = SessionContext::new(
        SessionId(identity),
        origin,
        ActorId::from_bytes(identity),
        AuthorityDomainRef::new(origin, AuthorityDomainId::from_bytes(identity)),
        AuthorityBinding::LocalUnbound { origin },
        SealStrength::Advisory,
        CausalPosture::AuthorOnly,
        None,
        RetentionContractId::from_bytes(identity),
    )
    .map_err(|_| invalid())?;
    if record.writer_heads.len() != 1
        || record.writer_heads[0].worldline_id != record.child_worldline_id
        || record.child_worldline_id.as_bytes() != &digest(b"echo:local-strand-child:v1", &identity)
        || record.writer_heads[0].head_id.as_bytes()
            != &digest(b"echo:local-strand-head:v1", &identity)
        || record.topology_intent_id != identity
        || record.idempotency_key_digest != Some(identity)
        || record.retention_posture_digest != digest(b"echo:local-author-only-strand:v1", &identity)
        || record.issuer_evidence_digest != identity
    {
        return Err(invalid());
    }
    ForkStrandRequest::from_session_default(
        record.strand_id,
        record.source_worldline_id,
        record.fork_tick,
        record.child_worldline_id,
        vec![WriterHead::with_routing(
            record.writer_heads[0],
            PlaybackMode::Play,
            InboxPolicy::AcceptAll,
            None,
            true,
        )],
        &session,
    )
    .map_err(|_| invalid())
}

impl TrustedRuntimeHost {
    /// Forks a retained AuthorOnly strand under the trusted local host's fixed
    /// advisory authority profile. This is not an authenticated admission API.
    /// Reusing an existing strand identity returns its original writer head.
    ///
    /// # Errors
    /// Refuses missing WAL/source history, invalid labels, or more than 64 strands.
    pub fn fork_local_operation_strand_v1(
        &mut self,
        source: WriterHeadKey,
        label: &str,
    ) -> Result<WriterHeadKey, TrustedRuntimeHostError> {
        if label.is_empty() || label.len() > 64 || self.runtime.heads().get(&source).is_none() {
            return Err(invalid());
        }
        let identity = digest(source.worldline_id.as_bytes(), label.as_bytes());
        let strand_id = StrandId::from_bytes(identity);
        if let Some(strand) = self.runtime.strands().get(&strand_id) {
            return strand.writer_heads().first().copied().ok_or_else(invalid);
        }
        if self.runtime.strands().len() >= 64 {
            return Err(invalid());
        }
        let tip = self
            .provenance
            .tip_ref(source.worldline_id)?
            .ok_or_else(invalid)?;
        let entry = self
            .provenance
            .entry(source.worldline_id, tip.worldline_tick)?;
        let head = WriterHeadKey {
            worldline_id: crate::WorldlineId::from_bytes(digest(
                b"echo:local-strand-child:v1",
                &identity,
            )),
            head_id: crate::head::HeadId::from_bytes(digest(
                b"echo:local-strand-head:v1",
                &identity,
            )),
        };
        let record = StrandForkRecord {
            topology_intent_id: identity,
            strand_id,
            source_worldline_id: source.worldline_id,
            fork_tick: tip.worldline_tick,
            source_commit_hash: tip.commit_hash,
            source_boundary_hash: entry.expected.state_root,
            child_worldline_id: head.worldline_id,
            writer_heads: vec![head],
            retention_posture_digest: digest(b"echo:local-author-only-strand:v1", &identity),
            issuer_evidence_digest: identity,
            idempotency_key_digest: Some(identity),
        };
        let mut runtime = self.runtime.clone();
        let mut provenance = self.provenance.clone();
        runtime.fork_strand(&mut provenance, request(&record)?)?;
        let wal = self.runtime_wal.as_mut().ok_or_else(invalid)?;
        wal.refresh_cursor_from_store_for_writer()?;
        let mut builder = wal.builder(
            WalTransactionKind::TopologyIntent,
            WalAppendAuthority::TrustedScheduler,
            WalTransactionId::from_hash(identity),
        );
        builder
            .push_record(
                WalRecordKind::TopologyStrandForkRecorded,
                record.to_payload_bytes(),
            )
            .map_err(TrustedRuntimeWalError::from)?;
        wal.append_transaction(
            builder
                .commit(Vec::new())
                .map_err(TrustedRuntimeWalError::from)?,
        )?;
        self.runtime = runtime;
        self.provenance = provenance;
        Ok(head)
    }
}

pub(super) fn restore(
    runtime: &WorldlineRuntime,
    provenance: &ProvenanceService,
    wal: &TrustedRuntimeWal,
    recovery: &mut TrustedRuntimeWalRecovery,
) -> Result<(WorldlineRuntime, ProvenanceService), TrustedRuntimeHostError> {
    let mut runtime = runtime.clone();
    let mut provenance = provenance.clone();
    let scan = wal
        .store
        .recover_read_only()
        .map_err(TrustedRuntimeWalError::from)?;
    let mut count = 0;
    for transaction in scan.transactions {
        for frame in transaction.frames {
            if frame.header.record_kind != WalRecordKind::TopologyStrandForkRecorded {
                continue;
            }
            count += 1;
            if count > 64 {
                return Err(invalid());
            }
            let record = StrandForkRecord::from_payload_bytes(&frame.payload.canonical_bytes)
                .map_err(|_| invalid())?;
            let fork_request = request(&record)?;
            if runtime.strands().contains(&record.strand_id) {
                return Err(invalid());
            }
            let source_entries = recovery
                .provenance_entries
                .iter()
                .filter(|entry| entry.worldline_id == record.source_worldline_id)
                .cloned()
                .collect::<Vec<_>>();
            restore_provenance_entries(&mut provenance, &source_entries)?;
            runtime.restore_causal_runtime_history(&provenance, &source_entries, &[])?;
            let receipt = runtime.fork_strand(&mut provenance, fork_request)?;
            if receipt.fork_basis_ref.commit_hash != record.source_commit_hash
                || receipt.fork_basis_ref.boundary_hash != record.source_boundary_hash
            {
                return Err(invalid());
            }
            // The copied prefix is derived from the validated native fork and
            // retained source entries, not reconstructed from application bytes.
            for tick in 0..=record.fork_tick.as_u64() {
                recovery.provenance_entries.push(
                    provenance.entry(record.child_worldline_id, WorldlineTick::from_raw(tick))?,
                );
            }
        }
    }
    recovery
        .provenance_entries
        .sort_by_key(|entry| (entry.worldline_id, entry.worldline_tick));
    Ok((runtime, provenance))
}
