// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! The provider verifier and warp-core's runtime admission reach the same
//! verdict on every authored effect package in the shared corpus, and on the
//! jedit#296 basis-read witness that once split them (echo#1023).
//!
//! Each package is judged by [`verify_effect_package`] and admitted through
//! the public [`EffectPackageHost`] under an exact policy built from the
//! package's own pins and budget ceiling, so only the package's contents can
//! refuse it. A package the provider accepts must be admitted; a package the
//! runtime refuses must be refused by the provider in the same reason class.
use echo_edict_canonical::{decode_canonical_cbor_v1 as decode, CanonicalValueV1 as V};
use echo_edict_provider_verifier::effect_contract::ContractRefusal;
use echo_edict_provider_verifier::effect_package::{verify_effect_package, EffectPackageRefusal};
use warp_core::{
    echo_operation_package_id_v1, make_head_id, EchoOperationAdmissionError,
    EchoOperationAdmissionPolicy, EchoOperationBudgetV1, EffectPackageHost, EngineBuilder,
    GraphStore, InboxPolicy, NodeId, NodeRecord, PlaybackMode, TypeId, WarpId, WorldlineId,
    WorldlineRuntime, WorldlineState, WriterHead, WriterHeadKey,
};

/// The retained corpus, embedded so the judgment reads no files.
macro_rules! retained {
    ($($name:literal),* $(,)?) => {
        &[$((
            $name,
            include_str!(concat!("../../echo-edict-provider-lowerer/tests/fixtures/effect-programs/", $name, ".hex")),
        )),*]
    };
}
const RETAINED: &[(&str, &str)] = retained![
    "read-control",
    "ensure-node-control",
    "echo-dpo-2-ensure-node-control",
    "read-without-failures",
    "read-catalog-without-failures",
    "slice-proven",
    "slice-unproven",
    "slice-other-operand",
    "slice-require-only",
    "two-let-control",
    "forged-node-order",
    "wrong-intrinsic-input-type",
    "read-exports-control",
    "undeclared-obstruction",
    "read-adapter-control",
    "failure-mapping-tag-spelled",
    "failure-mapping-unknown",
    "failure-mapping-duplicate",
    "constraint-proven-by-constraint",
    "constraint-conjunct-proven",
    "over-budget",
    "mutation-allocation-exact",
    "mutation-allocation-short",
    "require-payload-over-output",
    "mutation-count-over",
    "local-name-empty",
    "constraint-without-coordinate",
    "mutation-input-nominal",
    "mutation-result-nominal",
    "nominal-extra-field",
    "failure-declared-twice",
    "basis-control",
    "basis-not-bytes",
    "type-table-width",
    "type-table-int",
    "require-payload-fits",
    "mutation-count-control",
    "read-and-return-control",
    "read-allocation-short",
    "read-captured-budget",
    "typed-failure-control",
    "typed-failure-undeclared",
    "typed-failure-unmapped",
    "typed-failure-unknown-arm",
    "typed-failure-repeated-set",
    "typed-failure-empty-set",
    "typed-failure-without-capability",
    "typed-failure-arms-differ",
    "imported-helper-total",
    "imported-helper-fails",
    "imported-helper-fails-in-arm",
    "imported-helper-fails-attempted",
    "ordered-record-z-first",
    "ordered-record-a-first",
    "ordered-record-guarded-z-first",
    "ordered-record-guarded-a-first",
    "ordered-record-duplicate-name",
    "ordered-record-empty",
    "ordered-record-total",
    "ordered-record-unordered-fallible",
    "ordered-record-extra-entry-key",
    "structured-loop-control",
    "structured-branch-control",
    "structured-nested-control",
    "structured-echo-dpo-1",
    "structured-bound-below-list",
    "structured-coordinate-bound",
    "structured-leaked-local",
    "structured-control-id-reused",
    "structured-dropped-body",
    "structured-ordered-with-control",
    "structured-substituted-binder",
    "structured-body-with-result",
    "structured-loop-allocation-exact",
    "structured-loop-allocation-short",
    "structured-loop-under-budget",
    "structured-mutation-count-control",
    "structured-mutation-count-over",
    "footprint-writes-at-ceiling",
    "footprint-writes-over-ceiling",
];

/// The jedit#296 basis-read witness: Amblicit 738d3da7 output whose footprint
/// law declares `max_writes = 1024`.
const BASIS_READ: &str = include_str!(
    "../../warp-core/tests/fixtures/edict-effect-basis-read/basis-read-executable-operation-package.hex"
);

/// Entries warp-core's corpus marks `Runtime::Executes`: the provider refuses
/// them statically, while admission admits them and their execution refuses
/// them (`the_runtime_refuses_each_executed_entry_when_it_runs` in warp-core's
/// `provider_corpus.rs`). Their reason class is a run-time limit or input
/// check the runtime meters as it runs, not the package's contents.
const REFUSED_WHEN_RUN: &[&str] = &[
    "over-budget",
    "mutation-allocation-short",
    "require-payload-over-output",
    "mutation-count-over",
    "constraint-without-coordinate",
    "basis-not-bytes",
    "read-allocation-short",
    "read-captured-budget",
    "structured-loop-allocation-short",
    "structured-loop-under-budget",
    "structured-mutation-count-over",
];

fn host() -> EffectPackageHost {
    let warp = WarpId([0x71; 32]);
    let root = NodeId([0x72; 32]);
    let mut store = GraphStore::new(warp);
    store.insert_node(
        root,
        NodeRecord {
            ty: TypeId([0x73; 32]),
        },
    );
    let state = WorldlineState::from_root_store(store.clone(), root).expect("root state");
    let head = WriterHeadKey {
        worldline_id: WorldlineId::from_bytes([0x74; 32]),
        head_id: make_head_id("provider-runtime-agreement"),
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
    EffectPackageHost::new(runtime, EngineBuilder::new(store, root).build()).expect("host")
}

fn get<'a>(value: &'a V, key: &str) -> &'a V {
    let V::Map(fields) = value else { panic!("map") };
    fields
        .iter()
        .find_map(|(k, v)| (k == &V::Text(key.into())).then_some(v))
        .unwrap_or_else(|| panic!("field {key}"))
}
fn unsigned(value: &V) -> u64 {
    let V::Integer(n) = value else {
        panic!("integer")
    };
    u64::try_from(*n).expect("unsigned")
}

/// Admits `bytes` under the exact policy its own pins and ceiling name.
fn admit(bytes: &[u8]) -> Result<(), EchoOperationAdmissionError> {
    let package = decode(bytes).expect("canonical package");
    let V::Text(operation) = get(&package, "operation_coordinate") else {
        panic!("operation")
    };
    let V::Bytes(authority) = get(&package, "authority_profile_identity") else {
        panic!("authority")
    };
    let ceiling = get(&package, "budget_ceiling");
    let budget = EchoOperationBudgetV1::new(
        unsigned(get(ceiling, "steps")),
        unsigned(get(ceiling, "read_bytes")),
        unsigned(get(ceiling, "write_bytes")),
    );
    let policy = EchoOperationAdmissionPolicy::exact(
        echo_operation_package_id_v1(bytes).as_hash(),
        operation.clone(),
        authority.as_slice().try_into().expect("32-byte authority"),
        budget,
    );
    host()
        .admit_echo_operation_package(&policy, bytes.to_vec())
        .map(drop)
}

/// The reason class a refusal belongs to: a footprint-law or graph-schema
/// code by itself, since both judges name those checks identically, and
/// otherwise the stage that refused.
fn provider_class(refusal: EffectPackageRefusal) -> String {
    match refusal {
        EffectPackageRefusal::Contract(contract) => contract.code().to_owned(),
        EffectPackageRefusal::UnsupportedProgram => "effect-program".to_owned(),
        EffectPackageRefusal::InvalidEnvelope | EffectPackageRefusal::InvalidBinding => {
            "package".to_owned()
        }
    }
}
fn runtime_class(refusal: &EchoOperationAdmissionError) -> String {
    let code = refusal.reason_code();
    if code.starts_with("effect-program.footprint-law") || code == "effect-program.graph-schema" {
        code.to_owned()
    } else if code.starts_with("effect-program.") {
        "effect-program".to_owned()
    } else {
        "package".to_owned()
    }
}

#[test]
fn the_provider_and_runtime_admission_agree_on_the_whole_corpus() {
    assert!(RETAINED.len() >= 70, "the whole corpus is judged");
    let mut disagreements = Vec::new();
    for (name, hex) in RETAINED {
        let bytes = hex::decode(hex.trim()).expect("hex");
        let provider = verify_effect_package(&bytes).map_err(provider_class);
        let runtime = admit(&bytes).map_err(|refusal| runtime_class(&refusal));
        let agrees = match (&provider, &runtime) {
            (Ok(()), Ok(())) => true,
            (Err(_), Ok(())) => REFUSED_WHEN_RUN.contains(name),
            (Err(provider), Err(runtime)) => provider == runtime,
            (Ok(()), Err(_)) => false,
        };
        if !agrees {
            disagreements.push(format!(
                "{name}: provider {provider:?}, runtime {runtime:?}"
            ));
        }
    }
    assert_eq!(disagreements, Vec::<String>::new());
}

/// Every entry refused only when run is in the corpus, refused by the
/// provider and admitted by the runtime, so the exemption names only what it
/// must.
#[test]
fn each_entry_refused_when_run_is_admitted_and_provider_refused() {
    for name in REFUSED_WHEN_RUN {
        let (_, hex) = RETAINED
            .iter()
            .find(|(entry, _)| entry == name)
            .unwrap_or_else(|| panic!("{name} is retained"));
        let bytes = hex::decode(hex.trim()).expect("hex");
        assert!(verify_effect_package(&bytes).is_err(), "{name}");
        assert!(admit(&bytes).is_ok(), "{name}");
    }
}

#[test]
fn both_judges_refuse_the_basis_read_law_above_the_write_ceiling() {
    let bytes = hex::decode(BASIS_READ.trim()).expect("hex");
    assert_eq!(
        verify_effect_package(&bytes),
        Err(EffectPackageRefusal::Contract(
            ContractRefusal::FootprintMaxWrites
        ))
    );
    let runtime = admit(&bytes).expect_err("admission refuses the law");
    assert_eq!(
        runtime.reason_code(),
        ContractRefusal::FootprintMaxWrites.code()
    );
}
