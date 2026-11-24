// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! Lower the actual compiler request for an effect program whose helper lives
//! in a digest-pinned source module (flyingrobots/echo#1009).
//!
//! The control is unmodified Amblicit output. Each forgery edits only the
//! source closure, re-deriving every digest the envelope itself carries, so
//! the refusal comes from the closure's `use module` pins.

use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, digest_canonical_value_bytes_v1 as digest,
    encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value,
};
use echo_edict_provider_lowerer::{
    lower, Artifact, BoundArtifact, Digest, DigestAlgorithm, LoweringOutputKind,
    LoweringOutputRequest, LoweringRequestV1, ProtocolVersionV1, ProviderRefusalKind, ResourceRef,
    ResponseLimitsV1, SemanticInput, SemanticInputKind,
};

const PACKAGE: &str = "echo.operation-package/v1";
const ROOT: &str = "consumer.identity.program@1";
const LEAF: &str = "jedit.probe.leaf@1";
const SOURCE_ROLE: usize = 3;

fn bound(coordinate: &str, domain: &str, bytes: Vec<u8>) -> BoundArtifact {
    let value = decode(&bytes).expect("compiler artifact is canonical");
    BoundArtifact {
        reference: ResourceRef {
            coordinate: coordinate.to_owned(),
            digest: Digest {
                algorithm: DigestAlgorithm::Sha256,
                bytes: digest(domain, &value).expect("artifact identity").to_vec(),
            },
        },
        artifact: Artifact {
            domain: domain.to_owned(),
            bytes,
        },
    }
}

fn request() -> LoweringRequestV1 {
    macro_rules! raw {
        ($name:literal) => {
            hex::decode(
                include_str!(concat!("fixtures/source-module-effect/", $name, ".hex")).trim(),
            )
            .expect("captured compiler bytes")
        };
    }
    let aux = |kind: &str| SemanticInputKind::Auxiliary(kind.to_owned());
    let inputs = [
        (
            "01-lawpack-adapter",
            aux("lawpack-adapter"),
            "consumer.identity.echo-adapter/v1",
            "edict.lawpack-adapter/v1",
            raw!("01-lawpack-adapter"),
        ),
        (
            "02-lawpack-exports",
            aux("lawpack-exports"),
            "consumer.identity.exports/v1",
            "edict.lawpack-exports/v1",
            raw!("02-lawpack-exports"),
        ),
        (
            "03-lawpack",
            SemanticInputKind::Lawpack,
            "consumer.identity@1",
            "edict.lawpack/v1",
            raw!("03-lawpack"),
        ),
        (
            "04-source",
            aux("edict-source-closure"),
            ROOT,
            "edict.source-closure/v1",
            raw!("04-source"),
        ),
        (
            "05-target-configuration",
            aux("target-configuration"),
            "echo.operation-lowering-configuration/v1",
            "echo.operation-lowering-configuration/v1",
            raw!("05-target-configuration"),
        ),
        (
            "06-target-ir",
            aux("target-ir"),
            "echo.span-ir/v3",
            "edict.target-ir.artifact/v1",
            raw!("06-target-ir"),
        ),
        (
            "07-result-projection",
            aux("result-projection"),
            "consumer.identity.program@1.go",
            "edict.result-projection.artifact/v1",
            raw!("07-result-projection"),
        ),
    ];
    LoweringRequestV1 {
        protocol_version: ProtocolVersionV1 {
            major: 1,
            minor: 0,
            patch: 0,
        },
        core: bound(ROOT, "edict.core.module/v1", raw!("core")),
        target_profile: bound("echo.dpo@2", "edict.target-profile/v1", raw!("profile")),
        semantic_inputs: inputs
            .into_iter()
            .map(|(role, kind, coordinate, domain, bytes)| SemanticInput {
                role: role.to_owned(),
                kind,
                artifact: bound(coordinate, domain, bytes),
            })
            .collect(),
        requested_outputs: vec![LoweringOutputRequest {
            role: "executable-operation-package.echo".to_owned(),
            kind: LoweringOutputKind::GeneratedArtifact,
            domain: PACKAGE.to_owned(),
        }],
        limits: ResponseLimitsV1 {
            max_output_count: 1,
            max_total_response_bytes: 1024 * 1024,
            max_diagnostic_count: 8,
        },
    }
}

fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}

fn entries_mut(value: &mut Value) -> &mut Vec<(Value, Value)> {
    let Value::Map(entries) = value else {
        panic!("map required")
    };
    entries
}

/// One closure module whose envelope digest is its exact source identity.
fn unit(source: &str) -> Value {
    let source = Value::Bytes(source.as_bytes().to_vec());
    let identity = digest("edict.source/v1", &source).expect("source identity");
    Value::Map(vec![
        (
            text("digest"),
            Value::Array(vec![text("sha256"), Value::Bytes(identity.to_vec())]),
        ),
        (text("source"), source),
    ])
}

/// Applies `edit` to the closure's module map and rebinds the source input.
fn with_modules(edit: impl FnOnce(&mut Vec<(Value, Value)>)) -> LoweringRequestV1 {
    let mut request = request();
    let input = &mut request.semantic_inputs[SOURCE_ROLE];
    let mut closure = decode(&input.artifact.artifact.bytes).expect("canonical closure");
    let modules = entries_mut(&mut closure)
        .iter_mut()
        .find(|(key, _)| key == &text("modules"))
        .map(|(_, modules)| entries_mut(modules))
        .expect("module map");
    edit(modules);
    modules.sort_by_key(|(key, _)| encode(key).expect("key"));
    input.artifact = bound(
        ROOT,
        "edict.source-closure/v1",
        encode(&closure).expect("canonical closure"),
    );
    request
}

fn refusal(request: LoweringRequestV1) -> ProviderRefusalKind {
    lower(request).expect_err("the forged closure refuses").kind
}

#[test]
fn an_effect_program_calling_a_source_module_helper_lowers() {
    let output = lower(request()).expect("the compiler's source-module program lowers");
    assert_eq!(output.outputs.len(), 1, "one package");
    assert_eq!(output.outputs[0].artifact.domain, PACKAGE);
}

#[test]
fn a_module_whose_source_differs_from_its_pin_refuses() {
    // The substitute is internally consistent: its envelope digest is its own
    // source identity. Only the root's `use module` pin still names the original.
    let forged = with_modules(|modules| {
        let leaf = modules
            .iter_mut()
            .find(|(key, _)| key == &text(LEAF))
            .expect("leaf module");
        leaf.1 = unit("package jedit.probe.leaf@1;\nfn same(v: U64) -> U64 { return 0u64; }\n");
    });
    assert_eq!(refusal(forged), ProviderRefusalKind::UnsupportedSemantics);
}

#[test]
fn a_module_no_pin_reaches_refuses() {
    let forged = with_modules(|modules| {
        modules.push((
            text("jedit.probe.unpinned@1"),
            unit("package jedit.probe.unpinned@1;\nfn same(v: U64) -> U64 { return v; }\n"),
        ));
    });
    assert_eq!(refusal(forged), ProviderRefusalKind::UnsupportedSemantics);
}

#[test]
fn a_pin_to_a_module_outside_the_closure_refuses() {
    let forged = with_modules(|modules| modules.retain(|(key, _)| key != &text(LEAF)));
    assert_eq!(refusal(forged), ProviderRefusalKind::UnsupportedSemantics);
}
