// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Command rewrite rules for warp-core.
//!
//! Generic engine-level commands (e.g. system management or GC triggers)
//! belong in this module. Application-specific commands should be defined
//! in application crates and registered with the engine at runtime.

use blake3::Hasher;
use bytes::Bytes;
use echo_wasm_abi::kernel_port as abi;
use echo_wasm_abi::{encode_cbor, unpack_import_suffix_intent_v1};

use crate::attachment::{AtomPayload, AttachmentKey, AttachmentValue};
use crate::footprint::{AttachmentSet, EdgeSet, Footprint, NodeSet, PortSet};
use crate::ident::{make_type_id, EdgeId, NodeId, NodeKey};
use crate::inbox::INTENT_ATTACHMENT_TYPE;
use crate::record::{EdgeRecord, NodeRecord};
use crate::rule::{ConflictPolicy, PatternGraph, RewriteRule};
use crate::tick_patch::WarpOp;
use crate::TickDelta;

/// Human-readable command rule for witnessed suffix import proposals.
pub const IMPORT_SUFFIX_INTENT_RULE_NAME: &str = "cmd/import_suffix_intent";

/// Type identifier label for result nodes created by [`import_suffix_intent_rule`].
pub const IMPORT_SUFFIX_RESULT_NODE_TYPE: &str = "echo/import-suffix-result";

/// Type identifier label for result edges from ingress event to import result.
pub const IMPORT_SUFFIX_RESULT_EDGE_TYPE: &str = "echo/import-suffix-result-edge";

/// Type identifier label for canonical CBOR [`abi::ImportSuffixResult`] atoms.
pub const IMPORT_SUFFIX_RESULT_ATTACHMENT_TYPE: &str = "echo/import-suffix-result/cbor-v1";

/// Constructs the core command rule for Echo-owned witnessed suffix import intents.
///
/// This handler is intentionally conservative. It does not directly mutate a
/// target worldline with remote history. It records a typed `Staged` admission
/// result as causal graph evidence during the admitted tick; later slices can
/// replace the staging evaluator with full basis-aware admission.
#[must_use]
pub fn import_suffix_intent_rule() -> RewriteRule {
    RewriteRule {
        id: make_type_id("rule:cmd/import_suffix_intent").0,
        name: IMPORT_SUFFIX_INTENT_RULE_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: import_suffix_intent_matches,
        executor: import_suffix_intent_executor,
        compute_footprint: import_suffix_intent_footprint,
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

/// Stable result node id for one import-suffix ingress event.
#[must_use]
pub fn import_suffix_result_node_id(event_id: &NodeId) -> NodeId {
    let mut hasher = Hasher::new();
    hasher.update(b"echo.import_suffix.result.node.v1:");
    hasher.update(&event_id.0);
    NodeId(hasher.finalize().into())
}

/// Stable result edge id for one import-suffix ingress event.
#[must_use]
pub fn import_suffix_result_edge_id(event_id: &NodeId, result_id: &NodeId) -> EdgeId {
    let mut hasher = Hasher::new();
    hasher.update(b"echo.import_suffix.result.edge.v1:");
    hasher.update(&event_id.0);
    hasher.update(&result_id.0);
    EdgeId(hasher.finalize().into())
}

fn import_suffix_intent_matches(view: crate::GraphView<'_>, scope: &NodeId) -> bool {
    import_suffix_request_from_scope(view, scope).is_some()
}

fn import_suffix_intent_executor(
    view: crate::GraphView<'_>,
    scope: &NodeId,
    delta: &mut TickDelta,
) {
    let Some(request) = import_suffix_request_from_scope(view, scope) else {
        return;
    };
    let result = staged_import_suffix_result(&request);
    let Ok(result_bytes) = encode_cbor(&result) else {
        return;
    };

    let warp_id = view.warp_id();
    let result_id = import_suffix_result_node_id(scope);
    let result_edge_id = import_suffix_result_edge_id(scope, &result_id);
    let result_key = AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: result_id,
    });

    delta.push(WarpOp::UpsertNode {
        node: NodeKey {
            warp_id,
            local_id: result_id,
        },
        record: NodeRecord {
            ty: make_type_id(IMPORT_SUFFIX_RESULT_NODE_TYPE),
        },
    });
    delta.push(WarpOp::UpsertEdge {
        warp_id,
        record: EdgeRecord {
            id: result_edge_id,
            from: *scope,
            to: result_id,
            ty: make_type_id(IMPORT_SUFFIX_RESULT_EDGE_TYPE),
        },
    });
    delta.push(WarpOp::SetAttachment {
        key: result_key,
        value: Some(AttachmentValue::Atom(AtomPayload::new(
            make_type_id(IMPORT_SUFFIX_RESULT_ATTACHMENT_TYPE),
            Bytes::from(result_bytes),
        ))),
    });
}

fn import_suffix_intent_footprint(view: crate::GraphView<'_>, scope: &NodeId) -> Footprint {
    let warp_id = view.warp_id();
    let result_id = import_suffix_result_node_id(scope);
    let result_edge_id = import_suffix_result_edge_id(scope, &result_id);

    let mut n_read = NodeSet::default();
    let mut n_write = NodeSet::default();
    let mut e_write = EdgeSet::default();
    let mut a_read = AttachmentSet::default();
    let mut a_write = AttachmentSet::default();

    n_read.insert_with_warp(warp_id, *scope);
    n_write.insert_with_warp(warp_id, *scope);
    n_write.insert_with_warp(warp_id, result_id);
    e_write.insert_with_warp(warp_id, result_edge_id);
    a_read.insert(AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: *scope,
    }));
    a_write.insert(AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: result_id,
    }));

    Footprint {
        n_read,
        n_write,
        e_read: EdgeSet::default(),
        e_write,
        a_read,
        a_write,
        b_in: PortSet::default(),
        b_out: PortSet::default(),
        factor_mask: 0,
    }
}

fn import_suffix_request_from_scope(
    view: crate::GraphView<'_>,
    scope: &NodeId,
) -> Option<abi::ImportSuffixRequest> {
    let Some(AttachmentValue::Atom(atom)) = view.node_attachment(scope) else {
        return None;
    };
    if atom.type_id != make_type_id(INTENT_ATTACHMENT_TYPE) {
        return None;
    }
    unpack_import_suffix_intent_v1(atom.bytes.as_ref()).ok()
}

fn staged_import_suffix_result(request: &abi::ImportSuffixRequest) -> abi::ImportSuffixResult {
    let staged_refs = if request.bundle.source_suffix.source_entries.is_empty() {
        vec![request.target_basis.clone()]
    } else {
        request.bundle.source_suffix.source_entries.clone()
    };

    abi::ImportSuffixResult {
        bundle_digest: request.bundle.bundle_digest.clone(),
        admission: abi::WitnessedSuffixAdmissionResponse {
            source_shell_digest: request.bundle.source_suffix.witness_digest.clone(),
            target_basis: request.target_basis.clone(),
            outcome: abi::WitnessedSuffixAdmissionOutcome::Staged {
                staged_refs,
                basis_report: request.basis_report.clone(),
            },
        },
    }
}
