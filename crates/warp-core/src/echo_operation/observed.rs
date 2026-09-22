// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Bounded observation preconditions for executable actions.
use super::*;

/// A bounded reading supplied before an executable operation is proposed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EchoOperationObservationV1 {
    basis: EchoOperationEvaluationBasisV1,
    reads: Vec<(NodeKey, Vec<u8>)>,
}

pub(super) const OBSERVED_SCHEMA: &str = "echo.operation-invocation.observed/v1";
const MAX_READS: usize = 16;
const MAX_BYTES: usize = 4096;

fn slot_bytes(
    state: &WorldlineState,
    node: NodeKey,
) -> Result<Vec<u8>, EchoOperationArtifactErrorV1> {
    let store = state
        .store(&node.warp_id)
        .ok_or_else(|| invalid_structure("observation warp unavailable"))?;
    let node_type = store
        .node(&node.local_id)
        .map_or(CanonicalValueV1::Null, |record| hash_value(record.ty.0));
    let alpha = match store.node_attachment(&node.local_id) {
        None => CanonicalValueV1::Null,
        Some(AttachmentValue::Atom(atom)) => map_value([
            ("type", hash_value(atom.type_id.0)),
            ("bytes", CanonicalValueV1::Bytes(atom.bytes.to_vec())),
        ]),
        Some(_) => {
            return Err(invalid_structure(
                "observation requires atom or absent alpha",
            ))
        }
    };
    encode_canonical_cbor_v1(&map_value([("node_type", node_type), ("alpha", alpha)]))
        .map_err(canonical_error)
}

impl EchoOperationObservationV1 {
    /// Encodes retained observation evidence for the runtime-owned context log.
    pub(crate) fn encode(&self) -> Result<Vec<u8>, EchoOperationArtifactErrorV1> {
        encode_canonical_cbor_v1(&self.to_value()).map_err(canonical_error)
    }

    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, EchoOperationArtifactErrorV1> {
        Self::from_value(decode_canonical_cbor_v1(bytes).map_err(canonical_error)?)
    }

    /// Derives changed support from this exact retained reading.
    pub(crate) fn changed_nodes(
        &self,
        state: &WorldlineState,
    ) -> Result<Vec<NodeKey>, EchoOperationArtifactErrorV1> {
        self.reads
            .iter()
            .filter_map(|(node, expected)| match slot_bytes(state, *node) {
                Ok(actual) if actual == *expected => None,
                Ok(_) => Some(Ok(*node)),
                Err(error) => Some(Err(error)),
            })
            .collect()
    }
    /// Captures the named node/alpha slots at an explicit observation basis.
    pub fn capture(
        state: &WorldlineState,
        basis: EchoOperationEvaluationBasisV1,
        nodes: &[NodeKey],
    ) -> Result<Self, EchoOperationArtifactErrorV1> {
        if state.state_root() != basis.state_root() {
            return Err(invalid_structure(
                "observation basis does not bind supplied state",
            ));
        }
        let nodes = nodes.iter().copied().collect::<BTreeSet<_>>();
        if nodes.is_empty() || nodes.len() > MAX_READS {
            return Err(invalid_structure("observation needs one to sixteen slots"));
        }
        let reads = nodes
            .into_iter()
            .map(|node| Ok((node, slot_bytes(state, node)?)))
            .collect::<Result<Vec<_>, EchoOperationArtifactErrorV1>>()?;
        let observation = Self { basis, reads };
        if observation
            .reads
            .iter()
            .map(|(_, bytes)| bytes.len())
            .sum::<usize>()
            > MAX_BYTES
        {
            return Err(invalid_structure("observation exceeds retained byte bound"));
        }
        Ok(observation)
    }

    /// Exact basis of the supplied reading, independent of a later submission.
    #[must_use]
    pub const fn basis(&self) -> EchoOperationEvaluationBasisV1 {
        self.basis
    }

    /// Canonical node/alpha values supplied by this reading.
    pub fn readings(&self) -> impl Iterator<Item = (NodeKey, &[u8])> {
        self.reads
            .iter()
            .map(|(node, value)| (*node, value.as_slice()))
    }

    pub(super) fn to_value(&self) -> CanonicalValueV1 {
        map_value([
            ("basis", self.basis.to_value()),
            (
                "reads",
                CanonicalValueV1::Array(
                    self.reads
                        .iter()
                        .map(|(node, bytes)| {
                            map_value([
                                ("warp_id", hash_value(node.warp_id.0)),
                                ("node_id", hash_value(node.local_id.0)),
                                ("value", CanonicalValueV1::Bytes(bytes.clone())),
                            ])
                        })
                        .collect(),
                ),
            ),
        ])
    }

    pub(super) fn from_value(
        value: CanonicalValueV1,
    ) -> Result<Self, EchoOperationArtifactErrorV1> {
        let mut fields = exact_text_map(value, &["basis", "reads"])?;
        let basis = EchoOperationEvaluationBasisV1::from_value(take_field(&mut fields, "basis")?)?;
        let CanonicalValueV1::Array(values) = take_field(&mut fields, "reads")? else {
            return Err(invalid_structure("observation reads must be an array"));
        };
        if values.is_empty() || values.len() > MAX_READS {
            return Err(invalid_structure("observation needs one to sixteen slots"));
        }
        let mut reads = Vec::new();
        let mut total = 0;
        for value in values {
            let mut fields = exact_text_map(value, &["warp_id", "node_id", "value"])?;
            let node = NodeKey {
                warp_id: crate::WarpId(take_hash(&mut fields, "warp_id")?),
                local_id: crate::NodeId(take_hash(&mut fields, "node_id")?),
            };
            let bytes = take_bytes(&mut fields, "value")?;
            total += bytes.len();
            if total > MAX_BYTES || reads.last().is_some_and(|(previous, _)| *previous >= node) {
                return Err(invalid_structure(
                    "observation must be bounded and strictly sorted",
                ));
            }
            decode_canonical_cbor_v1(&bytes).map_err(canonical_error)?;
            reads.push((node, bytes));
        }
        Ok(Self { basis, reads })
    }

    pub(super) fn validate_at_execution(
        &self,
        state: &WorldlineState,
        submission: EchoOperationEvaluationBasisV1,
        footprint: &mut Footprint,
        meter: &mut EchoOperationBudgetMeterV1,
    ) -> Result<(), EchoOperationObstructionKindV1> {
        if self.basis.writer_head() != submission.writer_head()
            || self.basis.worldline_tick() > submission.worldline_tick()
        {
            return Err(EchoOperationObstructionKindV1::ObservationChanged);
        }
        for (node, expected) in &self.reads {
            let portals =
                operation_descent_stack_with_portal_reads(state, node.warp_id, |portal| {
                    footprint.a_read.insert(portal);
                    meter.charge(1, 32, 0)
                })?;
            let _ = portals;
            let actual = slot_bytes(state, *node)
                .map_err(|_| EchoOperationObstructionKindV1::ObservationUnavailable)?;
            if !meter.charge(2, 64 + actual.len() as u64, 0) {
                return Err(EchoOperationObstructionKindV1::BudgetExceeded);
            }
            record_node_read(footprint, *node);
            footprint.a_read.insert(AttachmentKey::node_alpha(*node));
            if actual != *expected {
                return Err(EchoOperationObstructionKindV1::ObservationChanged);
            }
        }
        Ok(())
    }
}

impl EchoOperationInvocationV1 {
    pub(crate) fn has_observation(&self, observation: &EchoOperationObservationV1) -> bool {
        self.observation.as_ref() == Some(observation)
    }
    pub(crate) fn observed_semantic_identity(
        &self,
        observation: &EchoOperationObservationV1,
    ) -> Result<Hash, EchoOperationArtifactErrorV1> {
        let mut normalized = self.clone().with_observation(observation.clone());
        // Transport rebasing does not change the originally proposed meaning.
        normalized.evaluation_basis = observation.basis;
        Ok(*blake3::hash(&normalized.to_canonical_bytes()?).as_bytes())
    }
    /// Binds the operation to inputs observed before its submission basis.
    #[must_use]
    pub fn with_observation(mut self, observation: EchoOperationObservationV1) -> Self {
        self.observation = Some(observation);
        self
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn exercise(changed_relevant: bool) -> (EchoOperationPreparationV1, NodeKey) {
        let (installed, mut state, old_basis, policy, mut invocation, _) =
            super::super::tests::projected_create_fixture(1_024);
        let observed = *state.root();
        let observation = EchoOperationObservationV1::capture(&state, old_basis, &[observed])
            .expect("bounded observation");
        let changed = if changed_relevant {
            observed
        } else {
            NodeKey {
                warp_id: observed.warp_id,
                local_id: crate::make_node_id("unrelated"),
            }
        };
        let store = state.warp_state.store_mut(&changed.warp_id).expect("store");
        store.insert_node(
            changed.local_id,
            NodeRecord {
                ty: crate::make_type_id("changed"),
            },
        );
        let basis = EchoOperationEvaluationBasisV1::new(
            old_basis.writer_head(),
            WorldlineTick::from_raw(1),
            None,
            state.state_root(),
            old_basis.commit_id,
            old_basis.application_basis,
        );
        invocation.evaluation_basis = basis;
        invocation.delegated_budget = EchoOperationBudgetV1::new(8, 1_024, 1_024);
        let invocation = invocation.with_observation(observation);
        let authority = EchoOperationEvaluationAuthorityV1::new();
        let admitted = admit_invocation_v1(
            Some(&installed),
            policy,
            &invocation.to_canonical_bytes().expect("encode"),
            basis,
            &state,
            authority.clone(),
        )
        .expect("current submission basis admits");
        (
            prepare_operation_v1(
                Some(&installed),
                admitted,
                basis,
                &state,
                crate::POLICY_ID_NO_POLICY_V0,
                &authority,
            ),
            observed,
        )
    }

    #[test]
    fn retry_identity_survives_submission_occupancy_changes_but_not_input_changes() {
        let (_, state, basis, _, invocation, _) =
            super::super::tests::projected_create_fixture(1_024);
        let observation = EchoOperationObservationV1::capture(&state, basis, &[*state.root()])
            .expect("observation");
        let original = invocation
            .observed_semantic_identity(&observation)
            .expect("identity");
        let mut retry = invocation.clone();
        retry.evaluation_basis.application_basis =
            echo_operation_anchored_node_creation_application_basis_v1(
                invocation.node,
                EchoOperationAnchoredNodeOccupancyV1::NodeAndAttachment,
            );
        retry.evaluation_basis.worldline_tick = WorldlineTick::from_raw(100);
        assert_eq!(
            retry
                .observed_semantic_identity(&observation)
                .expect("retry identity"),
            original
        );
        retry.replacement_bytes.push(1);
        assert_ne!(
            retry
                .observed_semantic_identity(&observation)
                .expect("changed identity"),
            original
        );
    }

    #[test]
    fn observation_cannot_cross_writer_heads_in_one_worldline() {
        let (_, state, basis, _, _, _) = super::super::tests::projected_create_fixture(1024);
        let observation = EchoOperationObservationV1::capture(&state, basis, &[*state.root()])
            .expect("observation");
        let mut other = basis;
        other.writer_head.head_id = crate::make_head_id("another-head");
        let result = observation.validate_at_execution(
            &state,
            other,
            &mut Footprint::default(),
            &mut EchoOperationBudgetMeterV1::new(EchoOperationBudgetV1::new(32, 4096, 1024)),
        );
        assert_eq!(
            result,
            Err(EchoOperationObstructionKindV1::ObservationChanged)
        );
    }

    #[test]
    fn fresh_submission_cannot_erase_changed_observation() {
        let (outcome, _) = exercise(true);
        assert!(
            matches!(outcome, EchoOperationPreparationV1::Obstructed(ref obstruction) if obstruction.kind() == EchoOperationObstructionKindV1::ObservationChanged),
            "a fresh submission basis must not launder an old observation"
        );
    }

    #[test]
    fn unrelated_movement_preserves_support_and_adds_native_reads() {
        let (outcome, observed) = exercise(false);
        let EchoOperationPreparationV1::Prepared(prepared) = outcome else {
            panic!("unrelated movement must not invalidate the observation");
        };
        assert!(prepared
            .actual_footprint()
            .n_read
            .iter()
            .any(|node| *node == observed));
        assert_ne!(
            prepared.actual_footprint().factor_mask & (1_u64 << (observed.local_id.0[0] & 63)),
            0,
            "the observation's partition must be represented in the footprint mask"
        );
        assert!(prepared
            .patch()
            .in_slots()
            .contains(&SlotId::Node(observed)));
    }
}
