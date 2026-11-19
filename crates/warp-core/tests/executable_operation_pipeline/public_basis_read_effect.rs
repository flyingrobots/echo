// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! jedit#296's basis-read witness, compiled by Amblicit 738d3da7 (echo#1023).
//!
//! Echo main's provider accepted this package while runtime admission refused
//! it. Its footprint law declares `max_writes = 1024`, above the effect
//! profile's 256 (`docs/architecture/authored-operation-effects.md`), so
//! admission is right to refuse it; the provider now refuses it too. This
//! test pins the runtime's side: admission refuses the package, at both step
//! ceilings the witness was tried under, and names the failing check.
//!
//! The package is unchanged public compiler output retained under
//! `fixtures/edict-effect-basis-read/`; its identity is recorded here
//! independently of the bytes.
use super::current_effect_publication_fixture as publication;
use warp_core::{
    make_head_id, EchoOperationAdmissionErrorKind, EchoOperationAdmissionPolicy,
    EchoOperationBudgetV1, EffectPackageHost, EngineBuilder, GraphStore, InboxPolicy, NodeId,
    NodeRecord, PlaybackMode, TypeId, WarpId, WorldlineId, WorldlineRuntime, WorldlineState,
    WriterHead, WriterHeadKey,
};

const PACKAGE: &str =
    include_str!("../fixtures/edict-effect-basis-read/basis-read-executable-operation-package.hex");
/// `echo:operation-package:v1` identity of the retained package bytes.
const PACKAGE_PIN: &str = "8e99859aa259a4f0ca4dff7c487292efddc4fcb0305a5e26d73a134a17ead865";
const OPERATION: &str = "jedit.text.basis_read@1.basisRead";
const AUTHORITY_LABEL: &[u8] = b"jedit.text.authority/v2";
const ROOT_WARP: WarpId = WarpId([0x71; 32]);
const ROOT: NodeId = NodeId([0x72; 32]);

fn host() -> EffectPackageHost {
    let mut store = GraphStore::new(ROOT_WARP);
    store.insert_node(
        ROOT,
        NodeRecord {
            ty: TypeId([0x73; 32]),
        },
    );
    let state = WorldlineState::from_root_store(store.clone(), ROOT).expect("root state");
    let head = WriterHeadKey {
        worldline_id: WorldlineId::from_bytes([0x74; 32]),
        head_id: make_head_id("public-basis-read-effect"),
    };
    let mut runtime = WorldlineRuntime::new();
    runtime
        .register_worldline(head.worldline_id, state)
        .expect("worldline");
    runtime
        .register_writer_head(WriterHead::with_routing(
            head,
            PlaybackMode::Play,
            InboxPolicy::AcceptAll,
            None,
            true,
        ))
        .expect("head");
    let engine = EngineBuilder::new(store, ROOT).build();
    EffectPackageHost::new(runtime, engine).expect("host")
}

/// Admission refuses the package's footprint law under its owner's exact
/// policy and names the `max_writes` ceiling.
#[test]
fn admission_refuses_the_basis_read_law_above_the_write_ceiling() {
    let bytes = hex::decode(PACKAGE.trim()).expect("retained hex");
    let identity = publication::hash(b"echo:operation-package:v1\0", &bytes);
    assert_eq!(
        hex::encode(identity),
        PACKAGE_PIN,
        "retained package identity"
    );
    let authority = publication::hash(b"echo:operation-profile:v1\0", AUTHORITY_LABEL);
    for steps in [1_048_576, (1_u64 << 53) - 1] {
        let budget = EchoOperationBudgetV1::new(steps, 65_536, 65_536);
        let policy = EchoOperationAdmissionPolicy::exact(identity, OPERATION, authority, budget);
        let refusal = host()
            .admit_echo_operation_package(&policy, bytes.clone())
            .expect_err("a law above the write ceiling is refused");
        assert_eq!(
            refusal.kind(),
            EchoOperationAdmissionErrorKind::ArtifactInvalid,
            "steps {steps}"
        );
        assert_eq!(
            refusal.reason_code(),
            "effect-program.footprint-law.max-writes",
            "steps {steps}: {refusal}"
        );
    }
}
