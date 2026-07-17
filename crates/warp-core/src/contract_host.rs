// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Helpers for installed contract handlers hosted by Echo scheduler ticks.
//!
//! Generated or host-installed contract handlers are ordinary `cmd/*`
//! [`RewriteRule`](crate::RewriteRule)s. They must not run from application
//! dispatch. During scheduler-owned execution, Echo materializes each admitted
//! EINT as a runtime ingress event node with the canonical intent bytes attached.
//! This module provides the small, op-id-aware read helpers those handlers need
//! to recognize their operation and decode their own generated vars.

use crate::attachment::{AttachmentKey, AttachmentValue};
use crate::footprint::{AttachmentSet, EdgeSet, Footprint, NodeSet, PortSet};
use crate::graph_view::GraphView;
use crate::ident::{make_type_id, NodeId, NodeKey};
use crate::inbox::INTENT_ATTACHMENT_TYPE;

/// Decodes one canonical EINT envelope at the contract-host serialization boundary.
pub(crate) fn decode_canonical_eint(bytes: &[u8]) -> Option<(u32, &[u8])> {
    echo_wasm_abi::unpack_intent_v1(bytes).ok()
}

/// Encodes one canonical EINT envelope at the contract-host serialization boundary.
#[cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
pub(crate) fn encode_canonical_eint(op_id: u32, vars_bytes: &[u8]) -> Option<Vec<u8>> {
    echo_wasm_abi::pack_intent_v1(op_id, vars_bytes).ok()
}

/// Returns the EINT operation id attached to a scheduler-materialized runtime
/// ingress event.
///
/// This reads only the event node's attachment plane. Malformed or non-EINT
/// ingress returns `None`, leaving installed command rules unmatched.
#[must_use]
pub fn eint_op_id(view: GraphView<'_>, scope: &NodeId) -> Option<u32> {
    let (op_id, _) = runtime_ingress_eint(view, scope)?;
    Some(op_id)
}

/// Returns `true` when the scheduler-materialized runtime ingress event carries
/// the expected EINT operation id.
#[must_use]
pub fn matches_eint_op(view: GraphView<'_>, scope: &NodeId, expected_op_id: u32) -> bool {
    eint_op_id(view, scope) == Some(expected_op_id)
}

/// Returns the canonical EINT vars bytes for the expected operation id.
///
/// The returned slice is borrowed from the runtime ingress event attachment.
/// Generated handlers remain responsible for decoding these bytes with their
/// generated codec. Echo core only verifies the envelope boundary and op id.
#[must_use]
pub fn eint_vars_for_op<'a>(
    view: GraphView<'a>,
    scope: &NodeId,
    expected_op_id: u32,
) -> Option<&'a [u8]> {
    let (op_id, vars) = runtime_ingress_eint(view, scope)?;
    (op_id == expected_op_id).then_some(vars)
}

/// Returns the standard read footprint for a handler that inspects the EINT
/// attached to a runtime ingress event.
///
/// Contract handlers should extend this footprint with any handler-specific
/// graph, edge, attachment, or port writes they emit. This helper deliberately
/// declares no write authority by itself. The scope and attachment reads are
/// unconditional because observing their absence is still a read.
#[must_use]
pub fn runtime_ingress_eint_read_footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
    let warp_id = view.warp_id();
    let mut n_read = NodeSet::default();
    let mut a_read = AttachmentSet::default();
    n_read.insert_with_warp(warp_id, *scope);
    a_read.insert(AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: *scope,
    }));
    Footprint {
        n_read,
        n_write: NodeSet::default(),
        e_read: EdgeSet::default(),
        e_write: EdgeSet::default(),
        a_read,
        a_write: AttachmentSet::default(),
        b_in: PortSet::default(),
        b_out: PortSet::default(),
        factor_mask: 0,
    }
}

fn runtime_ingress_eint<'a>(view: GraphView<'a>, scope: &NodeId) -> Option<(u32, &'a [u8])> {
    let Some(AttachmentValue::Atom(atom)) = view.node_attachment(scope) else {
        return None;
    };
    if atom.type_id != make_type_id(INTENT_ATTACHMENT_TYPE) {
        return None;
    }
    decode_canonical_eint(atom.bytes.as_ref())
}
