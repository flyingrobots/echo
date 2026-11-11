// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Retained public compiler output for typed edge and guided closure reads.
//!
//! The packages and reports under `crates/warp-core/tests/fixtures/edict-graph-read`
//! were produced by the unmodified public compiler through the promoted
//! components. Each lowering request here is rebuilt only from the artifacts a
//! package itself embeds; nothing is authored or re-encoded.
use super::{
    assert_rejected, bound, change_package, changed_target_named, field, mutate, request, PACKAGE,
};
use echo_edict_canonical::{decode_canonical_cbor_v1 as decode, CanonicalValueV1 as Value};
use echo_edict_provider_lowerer::{
    lower, LoweringOutputKind, LoweringOutputRequest, LoweringRequestV1, ProtocolVersionV1,
    ResponseLimitsV1, SemanticInput, SemanticInputKind,
};
use echo_edict_provider_verifier as verifier;

const EDGE_PACKAGE: &str = include_str!(
    "../../../warp-core/tests/fixtures/edict-graph-read/edge-executable-operation-package.hex"
);
const EDGE_REPORT: &str =
    include_str!("../../../warp-core/tests/fixtures/edict-graph-read/edge-verification-report.hex");
const CLOSURE_PACKAGE: &str = include_str!(
    "../../../warp-core/tests/fixtures/edict-graph-read/closure-executable-operation-package.hex"
);
const CLOSURE_REPORT: &str = include_str!(
    "../../../warp-core/tests/fixtures/edict-graph-read/closure-verification-report.hex"
);
const TARGET_PROFILE: &[u8] = include_bytes!("../../resources/target-profile.echo-dpo.cbor");

fn text(value: &Value) -> &str {
    let Value::Text(text) = value else {
        panic!("text")
    };
    text
}

fn embedded(program: &Value, name: &str) -> Vec<u8> {
    let Value::Bytes(bytes) = field(program, name) else {
        panic!("embedded artifact")
    };
    bytes.clone()
}

/// The lowering request a public compiler sent, rebuilt from its package.
fn lowering_request(package_hex: &str) -> LoweringRequestV1 {
    let package = decode(&hex::decode(package_hex.trim()).expect("hex")).expect("package");
    let Value::Bytes(program) = field(&package, "program") else {
        panic!("program bytes")
    };
    let program = decode(program).expect("program");
    let core_bytes = embedded(&program, "core_artifact");
    let core = decode(&core_bytes).expect("core");
    let module = text(field(&core, "coordinate")).to_owned();
    let operation = text(field(&package, "operation_coordinate")).to_owned();
    let auxiliary =
        |role: &str, kind: &str, coordinate: &str, domain: &str, name: &str| SemanticInput {
            role: role.to_owned(),
            kind: SemanticInputKind::Auxiliary(kind.to_owned()),
            artifact: bound(coordinate, domain, embedded(&program, name)),
        };
    LoweringRequestV1 {
        protocol_version: ProtocolVersionV1 {
            major: 1,
            minor: 0,
            patch: 0,
        },
        core: bound(&module, "edict.core.module/v1", core_bytes),
        target_profile: bound(
            "echo.dpo@1",
            "edict.target-profile/v1",
            TARGET_PROFILE.to_vec(),
        ),
        semantic_inputs: vec![
            auxiliary(
                "01-lawpack-adapter",
                "lawpack-adapter",
                "consumer.graph.echo-adapter/v1",
                "edict.lawpack-adapter/v1",
                "lawpack_adapter_artifact",
            ),
            auxiliary(
                "02-lawpack-exports",
                "lawpack-exports",
                "consumer.graph.exports/v1",
                "edict.lawpack-exports/v1",
                "lawpack_exports_artifact",
            ),
            SemanticInput {
                role: "03-lawpack".to_owned(),
                kind: SemanticInputKind::Lawpack,
                artifact: bound(
                    "consumer.graph@1",
                    "edict.lawpack/v1",
                    embedded(&program, "lawpack_artifact"),
                ),
            },
            auxiliary(
                "04-source",
                "edict-source",
                &module,
                "edict.source/v1",
                "source_artifact",
            ),
            auxiliary(
                "05-target-configuration",
                "target-configuration",
                "echo.operation-lowering-configuration/v1",
                "echo.operation-lowering-configuration/v1",
                "target_configuration_artifact",
            ),
            auxiliary(
                "06-target-ir",
                "target-ir",
                "echo.span-ir/v2",
                "edict.target-ir.artifact/v1",
                "target_ir_artifact",
            ),
            auxiliary(
                "07-result-projection",
                "result-projection",
                &operation,
                "edict.result-projection.artifact/v1",
                "result_projection_artifact",
            ),
        ],
        requested_outputs: vec![LoweringOutputRequest {
            role: "executable-operation-package.echo".to_owned(),
            kind: LoweringOutputKind::GeneratedArtifact,
            domain: PACKAGE.to_owned(),
        }],
        limits: ResponseLimitsV1 {
            max_output_count: 1,
            max_total_response_bytes: 16 * 1024 * 1024,
            max_diagnostic_count: 8,
        },
    }
}

fn programs() -> [(&'static str, &'static str, &'static str); 2] {
    [
        ("edge", EDGE_PACKAGE, EDGE_REPORT),
        ("closure", CLOSURE_PACKAGE, CLOSURE_REPORT),
    ]
}

#[test]
fn the_native_lowerer_reproduces_each_public_graph_read_package_exactly() {
    for (name, package, _) in programs() {
        let output = lower(lowering_request(package)).expect("public request lowers natively");
        assert_eq!(
            hex::encode(&output.outputs[0].artifact.bytes),
            package.trim(),
            "{name}"
        );
    }
}

#[test]
fn the_separate_verifier_accepts_each_public_package_with_its_retained_report() {
    for (name, package, report) in programs() {
        let response = verifier::verify(request(lowering_request(package))).expect("verification");
        assert!(response.diagnostics.is_empty(), "{name}");
        assert_eq!(
            hex::encode(&response.outputs[0].artifact.bytes),
            report.trim(),
            "{name}"
        );
    }
}

/// A graph-read package may not claim the explicit node aperture: the
/// material it discovers is not nameable in advance.
#[test]
fn a_graph_package_relabelled_with_the_node_aperture_footprint_is_rejected() {
    for (_, package, _) in programs() {
        let mut substituted = request(lowering_request(package));
        change_package(&mut substituted, |package| {
            mutate(package, "footprint_contract_identity", |identity| {
                let mut hash = blake3::Hasher::new();
                hash.update(b"echo:operation-profile:v1\0");
                let profile = b"echo.operation.footprint.explicit-node-read-aperture/v1";
                hash.update(&(profile.len() as u64).to_le_bytes());
                hash.update(profile);
                *identity = Value::Bytes(hash.finalize().as_bytes().to_vec());
            });
        });
        assert_rejected(substituted);
    }
}

/// Coherently re-binding a graph step to another intrinsic does not make the
/// adapter's exported signature or failure mapping agree with it.
#[test]
fn a_graph_step_rebound_to_another_intrinsic_is_rejected() {
    for (intent, package, other) in [
        ("readEdges", EDGE_PACKAGE, "echo.dpo@1.closure-read"),
        (
            "readClosure",
            CLOSURE_PACKAGE,
            "echo.dpo@1.outgoing-edge-read",
        ),
        ("readClosure", CLOSURE_PACKAGE, "echo.dpo@1.node-atom-read"),
    ] {
        let mut rebound = request(lowering_request(package));
        changed_target_named(&mut rebound, intent, |intent| {
            mutate(intent, "steps", |steps| {
                let Value::Array(steps) = steps else {
                    panic!("steps")
                };
                mutate(&mut steps[0], "targetIntrinsic", |intrinsic| {
                    *intrinsic = Value::Text(other.to_owned());
                });
            });
        });
        assert_rejected(rebound);
    }
}

/// Swapping which authored arm handles the selector's insufficiency and
/// which handles malformed storage collapses two declared outcome classes.
#[test]
fn swapping_the_selector_and_storage_failure_arms_is_rejected() {
    let mut swapped = request(lowering_request(CLOSURE_PACKAGE));
    changed_target_named(&mut swapped, "readClosure", |intent| {
        mutate(intent, "steps", |steps| {
            let Value::Array(steps) = steps else {
                panic!("steps")
            };
            mutate(&mut steps[0], "obstructionArms", |arms| {
                let Value::Map(arms) = arms else {
                    panic!("arms")
                };
                let position = |tag: &str| {
                    arms.iter()
                        .position(|(key, _)| key == &Value::Text(tag.to_owned()))
                        .expect("declared arm")
                };
                let insufficient = position("echo.closure-selector-insufficient/v1");
                let malformed = position("echo.closure-storage-malformed/v1");
                let first = arms[insufficient].1.clone();
                arms[insufficient].1 = arms[malformed].1.clone();
                arms[malformed].1 = first;
            });
        });
    });
    assert_rejected(swapped);
}
