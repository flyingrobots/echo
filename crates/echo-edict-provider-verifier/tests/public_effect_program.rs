// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! The public lowerer and verifier routes for a flat authored effect program.
//!
//! The package, lawpack, adapter and configuration under
//! `crates/warp-core/tests/fixtures/edict-effect-program` are unchanged
//! public compiler output. The lowering request is rebuilt from them and from
//! the retained source; Core, Target IR and exports come from the package.
//! The compiler does not publish its result projection, which the effect
//! route binds by digest but never reads, so the request carries a stand-in.
//!
//! Oracles: the public compiler's own package bytes (class 4, the components
//! it ran), and the verifier's binding law that a package is accepted only
//! for the request that determines it.
use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, digest_canonical_value_bytes_v1 as digest,
    encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value,
};
use echo_edict_provider_lowerer::{
    lower, Artifact, BoundArtifact, Digest, DigestAlgorithm, LoweringOutputKind,
    LoweringOutputRequest, LoweringRequestV1, ProtocolVersionV1, ResourceRef, ResponseLimitsV1,
    SemanticInput, SemanticInputKind,
};
use echo_edict_provider_verifier as verifier;

macro_rules! retained {
    ($name:literal) => {
        hex::decode(
            include_str!(concat!(
                "../../warp-core/tests/fixtures/edict-effect-program/",
                $name
            ))
            .trim(),
        )
        .expect("retained public bytes")
    };
}

const SOURCE: &str =
    include_str!("../../warp-core/tests/fixtures/edict-effect-program/source/FlatEffect.edict");
const TARGET_PROFILE: &[u8] = include_bytes!("../resources/target-profile.echo-dpo.cbor");
const PACKAGE_DOMAIN: &str = "echo.operation-package/v1";
const MISMATCH: &str = "echo.verifier.effect-package-mismatch";
const REFUSED: &str = "echo.verifier.effect-program-refused";
const UNSELECTED: &str =
    "compiler adapter target configuration reference does not match the bound artifact";

fn bound(coordinate: &str, domain: &str, bytes: Vec<u8>) -> BoundArtifact {
    let value = decode(&bytes).expect("canonical artifact");
    BoundArtifact {
        reference: ResourceRef {
            coordinate: coordinate.to_owned(),
            digest: Digest {
                algorithm: DigestAlgorithm::Sha256,
                bytes: digest(domain, &value).expect("identity").to_vec(),
            },
        },
        artifact: Artifact {
            domain: domain.to_owned(),
            bytes,
        },
    }
}

fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    let Value::Map(entries) = value else {
        panic!("map")
    };
    &entries
        .iter()
        .find(|(key, _)| key == &Value::Text(name.to_owned()))
        .expect("field")
        .1
}

fn field_mut<'a>(value: &'a mut Value, name: &str) -> &'a mut Value {
    let Value::Map(entries) = value else {
        panic!("map")
    };
    &mut entries
        .iter_mut()
        .find(|(key, _)| key == &Value::Text(name.to_owned()))
        .expect("field")
        .1
}

fn bytes(value: &Value) -> &[u8] {
    let Value::Bytes(bytes) = value else {
        panic!("bytes")
    };
    bytes
}

/// Decodes the canonical bytes at `key`, edits them and re-encodes them.
fn edit_bytes(value: &mut Value, key: &str, edit: impl FnOnce(&mut Value)) {
    let slot = field_mut(value, key);
    let mut inner = decode(bytes(slot)).expect("nested canonical bytes");
    edit(&mut inner);
    *slot = Value::Bytes(encode(&inner).expect("re-encoded"));
}

fn public_package() -> Vec<u8> {
    retained!("flat-executable-operation-package.hex")
}

/// The compiler bundle embedded in the public package.
fn bundle() -> Value {
    let package = decode(&public_package()).expect("package");
    let program = decode(bytes(field(&package, "program"))).expect("program");
    decode(bytes(field(&program, "compiled_program"))).expect("bundle")
}

/// The lowering request the public compiler sent.
fn lowering_request() -> LoweringRequestV1 {
    let bundle = bundle();
    let embedded = |key| bytes(field(&bundle, key)).to_vec();
    let core = embedded("core_artifact");
    let module = "consumer.effect.program@1";
    let auxiliary = |role: &str, kind: &str, coordinate: &str, domain: &str, raw| SemanticInput {
        role: role.to_owned(),
        kind: SemanticInputKind::Auxiliary(kind.to_owned()),
        artifact: bound(coordinate, domain, raw),
    };
    let source = encode(&Value::Bytes(SOURCE.as_bytes().to_vec())).expect("source");
    let projection = encode(&Value::Text("stand-in".to_owned())).expect("stand-in");
    LoweringRequestV1 {
        protocol_version: ProtocolVersionV1 {
            major: 1,
            minor: 0,
            patch: 0,
        },
        core: bound(module, "edict.core.module/v1", core),
        target_profile: bound(
            "echo.dpo@1",
            "edict.target-profile/v1",
            TARGET_PROFILE.to_vec(),
        ),
        semantic_inputs: vec![
            auxiliary(
                "01-lawpack-adapter",
                "lawpack-adapter",
                "consumer.effect.echo-adapter/v1",
                "edict.lawpack-adapter/v1",
                retained!("flat-adapter.hex"),
            ),
            auxiliary(
                "02-lawpack-exports",
                "lawpack-exports",
                "consumer.effect.exports/v1",
                "edict.lawpack-exports/v1",
                embedded("lawpack_exports_artifact"),
            ),
            SemanticInput {
                role: "03-lawpack".to_owned(),
                kind: SemanticInputKind::Lawpack,
                artifact: bound(
                    "consumer.effect@1",
                    "edict.lawpack/v1",
                    retained!("flat-lawpack.hex"),
                ),
            },
            auxiliary(
                "04-source",
                "edict-source",
                module,
                "edict.source/v1",
                source,
            ),
            auxiliary(
                "05-target-configuration",
                "target-configuration",
                "echo.operation-lowering-configuration/v1",
                "echo.operation-lowering-configuration/v1",
                retained!("flat-configuration.hex"),
            ),
            auxiliary(
                "06-target-ir",
                "target-ir",
                "echo.span-ir/v2",
                "edict.target-ir.artifact/v1",
                embedded("target_ir_artifact"),
            ),
            auxiliary(
                "07-result-projection",
                "result-projection",
                "consumer.effect.program@1.settle",
                "edict.result-projection.artifact/v1",
                projection,
            ),
        ],
        requested_outputs: vec![LoweringOutputRequest {
            role: "executable-operation-package.echo".to_owned(),
            kind: LoweringOutputKind::GeneratedArtifact,
            domain: PACKAGE_DOMAIN.to_owned(),
        }],
        limits: ResponseLimitsV1 {
            max_output_count: 1,
            max_total_response_bytes: 16 * 1024 * 1024,
            max_diagnostic_count: 8,
        },
    }
}

fn transport(artifact: BoundArtifact) -> verifier::BoundArtifact {
    verifier::BoundArtifact {
        reference: verifier::ResourceRef {
            coordinate: artifact.reference.coordinate,
            digest: verifier::Digest {
                algorithm: verifier::DigestAlgorithm::Sha256,
                bytes: artifact.reference.digest.bytes,
            },
        },
        artifact: verifier::Artifact {
            domain: artifact.artifact.domain,
            bytes: artifact.artifact.bytes,
        },
    }
}

/// The verification request for `package` against the public compiler request.
fn verification_request(package: Vec<u8>) -> verifier::VerificationRequestV1 {
    let mut source = lowering_request();
    let target = source.semantic_inputs.remove(5).artifact;
    let mut inputs: Vec<_> = source
        .semantic_inputs
        .into_iter()
        .map(|input| verifier::SemanticInput {
            role: input.role,
            kind: match input.kind {
                SemanticInputKind::Lawpack => verifier::SemanticInputKind::Lawpack,
                SemanticInputKind::Auxiliary(kind) => verifier::SemanticInputKind::Auxiliary(kind),
                _ => panic!("unexpected compiler semantic kind"),
            },
            artifact: transport(input.artifact),
        })
        .collect();
    inputs.push(verifier::SemanticInput {
        role: "executable-operation-package.echo".to_owned(),
        kind: verifier::SemanticInputKind::Auxiliary("executable-operation-package".to_owned()),
        artifact: transport(bound(
            "executable-operation-package.echo",
            PACKAGE_DOMAIN,
            package,
        )),
    });
    verifier::VerificationRequestV1 {
        protocol_version: verifier::ProtocolVersionV1 {
            major: 1,
            minor: 0,
            patch: 0,
        },
        core: transport(source.core),
        target_profile: transport(source.target_profile),
        target_ir: transport(target),
        semantic_inputs: inputs,
        requested_outputs: vec![verifier::VerificationOutputRequest {
            role: "verifier-report.echo-operation".to_owned(),
            kind: verifier::VerificationOutputKind::VerifierReport,
            domain: "echo.operation-package-verifier-report/v1".to_owned(),
        }],
        limits: verifier::ResponseLimitsV1 {
            max_output_count: 1,
            max_diagnostic_count: 8,
            max_total_response_bytes: 16 * 1024 * 1024,
        },
    }
}

/// The report's outcome and diagnostic code.
fn verdict(package: Vec<u8>) -> (String, String) {
    let response = verifier::verify(verification_request(package)).expect("bound request");
    let report = decode(&response.outputs[0].artifact.bytes).expect("report");
    let Value::Text(outcome) = field(&report, "outcome") else {
        panic!("outcome")
    };
    let code = String::from_utf8(bytes(field(&report, "diagnosticBytes")).to_vec())
        .expect("diagnostic code");
    (outcome.clone(), code)
}

/// The public package with `edit` applied to its compiled Core, Target IR and
/// outer fields, every identity they carry re-derived so that the package is
/// internally coherent and only its relation to the request differs.
fn rebound(edit: impl Fn(&mut Value, &mut Value)) -> Vec<u8> {
    let mut package = decode(&public_package()).expect("package");
    let (mut core_id, mut target_id) = (Value::Null, Value::Null);
    edit_bytes(&mut package, "program", |program| {
        edit_bytes(program, "compiled_program", |bundle| {
            let mut core = decode(bytes(field(bundle, "core_artifact"))).expect("Core");
            let mut target = decode(bytes(field(bundle, "target_ir_artifact"))).expect("IR");
            edit(&mut core, &mut target);
            core_id = Value::Bytes(digest("edict.core.module/v1", &core).expect("id").to_vec());
            let source_core = field_mut(field_mut(&mut target, "semanticClosure"), "sourceCore");
            *field_mut(source_core, "digest") =
                Value::Array(vec![Value::Text("sha256".to_owned()), core_id.clone()]);
            target_id = Value::Bytes(
                digest("edict.target-ir.artifact/v1", &target)
                    .expect("id")
                    .to_vec(),
            );
            *field_mut(bundle, "core_artifact") = Value::Bytes(encode(&core).expect("Core"));
            *field_mut(bundle, "target_ir_artifact") = Value::Bytes(encode(&target).expect("IR"));
        });
        *field_mut(program, "target_ir_identity") = target_id.clone();
    });
    let closure = field_mut(&mut package, "semantic_closure");
    *field_mut(closure, "core_identity") = core_id.clone();
    *field_mut(closure, "canonical_meaning_identity") = core_id;
    *field_mut(closure, "target_ir_identity") = target_id;
    encode(&package).expect("package")
}

fn intent_mut(artifact: &mut Value) -> &mut Value {
    let Value::Map(intents) = field_mut(artifact, "intents") else {
        panic!("intents")
    };
    &mut intents[0].1
}

#[test]
fn the_native_lowerer_reproduces_the_public_effect_package() {
    let mut response = lower(lowering_request()).expect("the public route lowers the program");
    let output = response.outputs.remove(0);
    assert_eq!(output.artifact.domain, PACKAGE_DOMAIN);
    assert_eq!(output.artifact.bytes, public_package());
}

#[test]
fn the_separate_verifier_accepts_the_public_effect_package() {
    assert_eq!(
        verdict(public_package()),
        ("accepted".to_owned(), String::new())
    );
}

#[test]
fn a_package_for_another_program_is_rejected_at_verification() {
    // The same request, a different result: a coherent package the effect
    // judgment accepts on its own, but not the one this request determines.
    let substituted = rebound(|core, target| {
        let constant = Value::Map(vec![
            (
                Value::Text("kind".to_owned()),
                Value::Text("const".to_owned()),
            ),
            (
                Value::Text("value".to_owned()),
                Value::Map(vec![
                    (
                        Value::Text("kind".to_owned()),
                        Value::Text("int".to_owned()),
                    ),
                    (
                        Value::Text("width".to_owned()),
                        Value::Text("U64".to_owned()),
                    ),
                    (Value::Text("value".to_owned()), Value::Integer(9)),
                ]),
            ),
        ]);
        *field_mut(field_mut(intent_mut(core), "body"), "result") = constant.clone();
        *field_mut(intent_mut(target), "result") = constant;
    });
    assert_eq!(
        verifier::effect_package::verify_effect_package(&substituted),
        Ok(())
    );
    assert_eq!(
        verdict(substituted),
        ("rejected".to_owned(), MISMATCH.to_owned())
    );
}

#[test]
fn a_package_re_bound_to_another_budget_is_rejected_at_verification() {
    let mut package = decode(&public_package()).expect("package");
    *field_mut(field_mut(&mut package, "budget_ceiling"), "steps") = Value::Integer(4097);
    let rebound = encode(&package).expect("package");
    assert_eq!(
        verifier::effect_package::verify_effect_package(&rebound),
        Ok(())
    );
    assert_eq!(
        verdict(rebound),
        ("rejected".to_owned(), MISMATCH.to_owned())
    );
}

#[test]
fn a_package_whose_program_the_judgment_refuses_is_rejected() {
    // One declared step cannot cover the binding and the result.
    let starved = rebound(|core, target| {
        let budget = Value::Integer(1);
        *field_mut(
            field_mut(intent_mut(core), "coreEvaluationBudget"),
            "maxSteps",
        ) = budget.clone();
        *field_mut(
            field_mut(intent_mut(target), "coreEvaluationBudget"),
            "maxSteps",
        ) = budget;
    });
    assert_eq!(
        verdict(starved),
        ("rejected".to_owned(), REFUSED.to_owned())
    );
}

#[test]
fn the_lowerer_refuses_a_configuration_the_adapter_does_not_select() {
    let mut request = lowering_request();
    let mut configuration = decode(&retained!("flat-configuration.hex")).expect("configuration");
    *field_mut(&mut configuration, "authorityProfile") =
        Value::Text("consumer.effect.other-authority/v1".to_owned());
    request.semantic_inputs[4].artifact = bound(
        "echo.operation-lowering-configuration/v1",
        "echo.operation-lowering-configuration/v1",
        encode(&configuration).expect("configuration"),
    );
    assert!(lowerer_refusal(request).contains(UNSELECTED));
}

/// Replaces one verification input, rebinding it under its own coordinate.
fn replace_input(request: &mut verifier::VerificationRequestV1, role: &str, bytes: Vec<u8>) {
    let input = request
        .semantic_inputs
        .iter_mut()
        .find(|input| input.role == role)
        .expect("input role");
    input.artifact = transport(bound(
        &input.artifact.reference.coordinate,
        &input.artifact.artifact.domain,
        bytes,
    ));
}

/// The refusal message of a verification request the verifier must refuse.
fn verifier_refusal(request: verifier::VerificationRequestV1) -> String {
    let refusal = verifier::verify(request).expect_err("refused before any report");
    refusal.diagnostics[0].message.clone()
}

/// The refusal message of a lowering request the lowerer must refuse.
fn lowerer_refusal(request: LoweringRequestV1) -> String {
    let refusal = lower(request).expect_err("refused");
    refusal.diagnostics[0].message.clone()
}

fn configuration_with(edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut configuration = decode(&retained!("flat-configuration.hex")).expect("configuration");
    edit(&mut configuration);
    encode(&configuration).expect("configuration")
}

#[test]
fn the_verifier_refuses_a_configuration_the_adapter_does_not_select() {
    let mut request = verification_request(public_package());
    replace_input(
        &mut request,
        "05-target-configuration",
        configuration_with(|configuration| {
            *field_mut(configuration, "authorityProfile") =
                Value::Text("consumer.effect.other-authority/v1".to_owned());
        }),
    );
    assert!(verifier_refusal(request).contains(UNSELECTED));
}

#[test]
fn the_verifier_refuses_a_catalog_the_target_profile_does_not_pin() {
    let mut request = verification_request(public_package());
    let empty = encode(&Value::Map(vec![
        (
            Value::Text("apiVersion".to_owned()),
            Value::Text("edict.target-profile.intrinsics/v1".to_owned()),
        ),
        (Value::Text("intrinsics".to_owned()), Value::Map(vec![])),
    ]))
    .expect("catalog");
    replace_input(
        &mut request,
        "05-target-configuration",
        configuration_with(|configuration| {
            *field_mut(configuration, "intrinsicCatalog") = Value::Bytes(empty);
        }),
    );
    assert!(verifier_refusal(request).contains("catalog is not the one the target profile pins"));
}

/// Target IR whose semantic closure names a lawpack other than the bound one.
fn target_naming_another_lawpack() -> Vec<u8> {
    let mut target = decode(bytes(field(&bundle(), "target_ir_artifact"))).expect("IR");
    let Value::Array(lawpacks) = field_mut(field_mut(&mut target, "semanticClosure"), "lawpacks")
    else {
        panic!("lawpacks")
    };
    *field_mut(&mut lawpacks[0], "digest") = Value::Array(vec![
        Value::Text("sha256".to_owned()),
        Value::Bytes(vec![7; 32]),
    ]);
    encode(&target).expect("IR")
}

#[test]
fn both_routes_refuse_a_target_ir_naming_another_lawpack() {
    let expected = "effect Target IR lawpack reference does not match the bound artifact";
    let mut lowering = lowering_request();
    lowering.semantic_inputs[5].artifact = bound(
        "echo.span-ir/v2",
        "edict.target-ir.artifact/v1",
        target_naming_another_lawpack(),
    );
    assert!(lowerer_refusal(lowering).contains(expected));
    let mut verification = verification_request(public_package());
    verification.target_ir = transport(bound(
        "echo.span-ir/v2",
        "edict.target-ir.artifact/v1",
        target_naming_another_lawpack(),
    ));
    assert!(verifier_refusal(verification).contains(expected));
}

/// Core importing the lawpack twice, with the Target IR re-pinned to it.
fn core_importing_twice() -> (Vec<u8>, Vec<u8>) {
    let bundle = bundle();
    let mut core = decode(bytes(field(&bundle, "core_artifact"))).expect("Core");
    let Value::Array(imports) = field_mut(&mut core, "imports") else {
        panic!("imports")
    };
    let lawpack = imports
        .iter()
        .find(|import| field(import, "kind") == &Value::Text("lawpack".to_owned()))
        .expect("lawpack import")
        .clone();
    imports.push(lawpack);
    let mut target = decode(bytes(field(&bundle, "target_ir_artifact"))).expect("IR");
    let source_core = field_mut(field_mut(&mut target, "semanticClosure"), "sourceCore");
    *field_mut(source_core, "digest") = Value::Array(vec![
        Value::Text("sha256".to_owned()),
        Value::Bytes(digest("edict.core.module/v1", &core).expect("id").to_vec()),
    ]);
    (encode(&core).expect("Core"), encode(&target).expect("IR"))
}

#[test]
fn both_routes_refuse_a_core_importing_the_lawpack_twice() {
    let expected = "Core imports exactly one lawpack";
    let (core, target) = core_importing_twice();
    let module = "consumer.effect.program@1";
    let mut lowering = lowering_request();
    lowering.core = bound(module, "edict.core.module/v1", core.clone());
    lowering.semantic_inputs[5].artifact = bound(
        "echo.span-ir/v2",
        "edict.target-ir.artifact/v1",
        target.clone(),
    );
    assert!(lowerer_refusal(lowering).contains(expected));
    let mut verification = verification_request(public_package());
    verification.core = transport(bound(module, "edict.core.module/v1", core));
    verification.target_ir = transport(bound(
        "echo.span-ir/v2",
        "edict.target-ir.artifact/v1",
        target,
    ));
    assert!(verifier_refusal(verification).contains(expected));
}

/// Runs one closure mutation through both routes and requires each to refuse
/// it with `expected`. `core` and `target` default to the public artifacts;
/// `coordinate` is the Target IR's bound coordinate.
fn both_refuse(core: Option<Vec<u8>>, target: Vec<u8>, coordinate: &str, expected: &str) {
    let module = "consumer.effect.program@1";
    let mut lowering = lowering_request();
    let mut verification = verification_request(public_package());
    if let Some(core) = core {
        lowering.core = bound(module, "edict.core.module/v1", core.clone());
        verification.core = transport(bound(module, "edict.core.module/v1", core));
    }
    lowering.semantic_inputs[5].artifact =
        bound(coordinate, "edict.target-ir.artifact/v1", target.clone());
    verification.target_ir = transport(bound(coordinate, "edict.target-ir.artifact/v1", target));
    assert!(lowerer_refusal(lowering).contains(expected));
    assert!(verifier_refusal(verification).contains(expected));
}

fn public_target() -> Vec<u8> {
    bytes(field(&bundle(), "target_ir_artifact")).to_vec()
}

#[test]
fn both_routes_refuse_a_target_ir_bound_under_another_coordinate() {
    both_refuse(
        None,
        public_target(),
        "echo.span-ir/v1",
        "Target IR is not bound as the ordered echo.span-ir/v2",
    );
}

#[test]
fn both_routes_refuse_a_target_ir_naming_another_profile() {
    let mut target = decode(&public_target()).expect("IR");
    *field_mut(field_mut(&mut target, "targetProfile"), "digest") = Value::Array(vec![
        Value::Text("sha256".to_owned()),
        Value::Bytes(vec![9; 32]),
    ]);
    both_refuse(
        None,
        encode(&target).expect("IR"),
        "echo.span-ir/v2",
        "effect Target IR target profile reference does not match the bound artifact",
    );
}

#[test]
fn both_routes_refuse_a_core_importing_another_lawpack() {
    let bundle = bundle();
    let mut core = decode(bytes(field(&bundle, "core_artifact"))).expect("Core");
    let Value::Array(imports) = field_mut(&mut core, "imports") else {
        panic!("imports")
    };
    let import = imports
        .iter_mut()
        .find(|import| field(import, "kind") == &Value::Text("lawpack".to_owned()))
        .expect("lawpack import");
    *field_mut(field_mut(import, "ref"), "digest") = Value::Array(vec![
        Value::Text("sha256".to_owned()),
        Value::Bytes(vec![5; 32]),
    ]);
    let mut target = decode(&public_target()).expect("IR");
    let source_core = field_mut(field_mut(&mut target, "semanticClosure"), "sourceCore");
    *field_mut(source_core, "digest") = Value::Array(vec![
        Value::Text("sha256".to_owned()),
        Value::Bytes(digest("edict.core.module/v1", &core).expect("id").to_vec()),
    ]);
    both_refuse(
        Some(encode(&core).expect("Core")),
        encode(&target).expect("IR"),
        "echo.span-ir/v2",
        "effect Core lawpack import reference does not match the bound artifact",
    );
}
