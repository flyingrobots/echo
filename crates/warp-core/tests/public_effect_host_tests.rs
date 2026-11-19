// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Echo's public authored effect packages, run through the public
//! `trusted_runtime` route, [`warp_core::EffectPackageHost`] (echo#1017).
//!
//! Each package is admitted under its owner's exact policy, installed,
//! invoked, evaluated in a recording frame and derived by the same host a
//! downstream consumer uses. The downstream consumer itself, built without
//! `native_rule_bootstrap`, lives in `tests/public-effect-host-consumer/`.
#![cfg(feature = "trusted_runtime")]
#![allow(clippy::expect_used, clippy::panic)]

use bytes::Bytes;
use warp_core::{
    make_head_id, make_node_id, make_type_id, make_warp_id, AtomPayload, AttachmentValue,
    EffectPackageHost, EngineBuilder, GraphStore, InboxPolicy, NodeKey, NodeRecord, PlaybackMode,
    SchedulerKind, WorldlineId, WorldlineRuntime, WorldlineState, WriterHead, WriterHeadKey,
};

#[path = "common/edict_effect_publication.rs"]
mod current_effect_publication_fixture;

#[path = "executable_operation_pipeline/public_flat_effect.rs"]
mod public_flat_effect;

#[path = "executable_operation_pipeline/public_read_effect.rs"]
mod public_read_effect;

#[path = "executable_operation_pipeline/public_mutation_effect.rs"]
mod public_mutation_effect;

#[path = "executable_operation_pipeline/public_structured_effect.rs"]
mod public_structured_effect;

#[path = "executable_operation_pipeline/public_graph_read_effect.rs"]
mod public_graph_read_effect;

#[path = "executable_operation_pipeline/public_basis_read_effect.rs"]
mod public_basis_read_effect;

#[path = "executable_operation_pipeline/public_typed_failure_effect.rs"]
mod public_typed_failure_effect;

#[path = "executable_operation_pipeline/public_ordered_record_effect.rs"]
mod public_ordered_record_effect;

#[path = "executable_operation_pipeline/public_content_address_effect.rs"]
mod public_content_address_effect;

#[path = "executable_operation_pipeline/public_attachment_digest_effect.rs"]
mod public_attachment_digest_effect;

#[path = "executable_operation_pipeline/public_checked_integer.rs"]
mod public_checked_integer;

#[path = "executable_operation_pipeline/public_option_value.rs"]
mod public_option_value;

#[path = "executable_operation_pipeline/public_precondition_effect.rs"]
mod public_precondition_effect;

fn digest(label: &str) -> [u8; 32] {
    *blake3::hash(label.as_bytes()).as_bytes()
}

/// One writer head over a one-node graph whose root holds the atom
/// `before`, in a fresh public effect-package host.
fn fixture_host() -> (EffectPackageHost, WriterHeadKey, NodeKey) {
    let warp_id = make_warp_id("operation-fixture");
    let root = NodeKey {
        warp_id,
        local_id: make_node_id("operation-fixture-root"),
    };
    let mut store = GraphStore::new(warp_id);
    store.insert_node(
        root.local_id,
        NodeRecord {
            ty: make_type_id("operation-fixture-node"),
        },
    );
    store.set_node_attachment(
        root.local_id,
        Some(AttachmentValue::Atom(AtomPayload::new(
            make_type_id("operation-fixture-atom"),
            Bytes::from_static(b"before"),
        ))),
    );
    let state = WorldlineState::from_root_store(store, root.local_id)
        .expect("the fixture state has one lawful root");
    let head = WriterHeadKey {
        worldline_id: WorldlineId::from_bytes(digest("operation-fixture-worldline")),
        head_id: make_head_id("operation-fixture-writer"),
    };
    let mut runtime = WorldlineRuntime::new();
    runtime
        .register_worldline(head.worldline_id, state)
        .expect("the fixture worldline registers");
    runtime
        .register_writer_head(WriterHead::with_routing(
            head,
            PlaybackMode::Play,
            InboxPolicy::AcceptAll,
            None,
            true,
        ))
        .expect("the fixture writer registers");
    let mut engine_store = GraphStore::default();
    let engine_root = make_node_id("root");
    engine_store.insert_node(
        engine_root,
        NodeRecord {
            ty: make_type_id("world"),
        },
    );
    let engine = EngineBuilder::new(engine_store, engine_root)
        .scheduler(SchedulerKind::Radix)
        .workers(1)
        .build();
    let host = EffectPackageHost::new(runtime, engine).expect("the public host initializes");
    (host, head, root)
}
