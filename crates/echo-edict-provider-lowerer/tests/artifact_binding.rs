// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::panic)]
//! A canonical artifact binding refusal names the artifact and the check.
//!
//! Oracle: the reason-code scheme of flyingrobots/echo#1021 and the binding
//! checks of flyingrobots/echo#1025 (reference, decode, canonical form,
//! digest). Expected codes are written from that specification.

use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, digest_canonical_value_bytes_v1 as digest,
};
use echo_edict_provider_lowerer::{
    lower, Artifact, BoundArtifact, Digest, DigestAlgorithm, LoweringOutputKind,
    LoweringOutputRequest, LoweringRequestV1, ProtocolVersionV1, ProviderRefusalKind,
    ProviderRefusalV1, ResourceRef, ResponseLimitsV1, SemanticInput, SemanticInputKind,
};

const INVALID: &str = "echo.provider.invalid-semantic-artifact";

fn bound(coordinate: &str, domain: &str, bytes: Vec<u8>, digest: Vec<u8>) -> BoundArtifact {
    BoundArtifact {
        reference: ResourceRef {
            coordinate: coordinate.to_owned(),
            digest: Digest {
                algorithm: DigestAlgorithm::Sha256,
                bytes: digest,
            },
        },
        artifact: Artifact {
            domain: domain.to_owned(),
            bytes,
        },
    }
}

/// Bind canonical bytes under their recomputed identity.
fn rebound(coordinate: &str, domain: &str, bytes: Vec<u8>) -> BoundArtifact {
    let value = decode(&bytes).expect("fixture artifact is canonical");
    let identity = digest(domain, &value).expect("artifact identity").to_vec();
    bound(coordinate, domain, bytes, identity)
}

fn request(
    core: BoundArtifact,
    profile: BoundArtifact,
    inputs: Vec<SemanticInput>,
) -> LoweringRequestV1 {
    LoweringRequestV1 {
        protocol_version: ProtocolVersionV1 {
            major: 1,
            minor: 0,
            patch: 0,
        },
        core,
        target_profile: profile,
        semantic_inputs: inputs,
        requested_outputs: vec![LoweringOutputRequest {
            role: "executable-operation-package.echo".to_owned(),
            kind: LoweringOutputKind::GeneratedArtifact,
            domain: "echo.operation-package/v1".to_owned(),
        }],
        limits: ResponseLimitsV1 {
            max_output_count: 1,
            max_total_response_bytes: 1024 * 1024,
            max_diagnostic_count: 8,
        },
    }
}

fn input(role: &str, kind: SemanticInputKind, artifact: BoundArtifact) -> SemanticInput {
    SemanticInput {
        role: role.to_owned(),
        kind,
        artifact,
    }
}

fn auxiliary(name: &str) -> SemanticInputKind {
    SemanticInputKind::Auxiliary(name.to_owned())
}

/// The node-atom-read compiler request (`fixtures/node-atom-read`), whose
/// every artifact passes binding.
fn node_atom_read() -> LoweringRequestV1 {
    macro_rules! raw {
        ($name:literal) => {
            hex::decode(include_str!(concat!("fixtures/node-atom-read/", $name, ".hex")).trim())
                .expect("captured compiler bytes")
        };
    }
    request(
        rebound(
            "jedit.text.replace_range@1",
            "edict.core.module/v1",
            raw!("core"),
        ),
        rebound("echo.dpo@1", "edict.target-profile/v1", raw!("profile")),
        vec![
            input(
                "01-lawpack-adapter",
                auxiliary("lawpack-adapter"),
                rebound(
                    "jedit.text.echo-adapter/v1",
                    "edict.lawpack-adapter/v1",
                    raw!("01-lawpack-adapter"),
                ),
            ),
            input(
                "02-lawpack-exports",
                auxiliary("lawpack-exports"),
                rebound(
                    "jedit.text.exports/v1",
                    "edict.lawpack-exports/v1",
                    raw!("02-lawpack-exports"),
                ),
            ),
            input(
                "03-lawpack",
                SemanticInputKind::Lawpack,
                rebound("jedit.text@1", "edict.lawpack/v1", raw!("03-lawpack")),
            ),
            input(
                "04-source",
                auxiliary("edict-source"),
                rebound(
                    "jedit.text.replace_range@1",
                    "edict.source/v1",
                    raw!("04-source"),
                ),
            ),
            input(
                "05-target-configuration",
                auxiliary("target-configuration"),
                rebound(
                    "echo.operation-lowering-configuration/v1",
                    "echo.operation-lowering-configuration/v1",
                    raw!("05-target-configuration"),
                ),
            ),
            input(
                "06-target-ir",
                auxiliary("target-ir"),
                rebound(
                    "echo.span-ir/v2",
                    "edict.target-ir.artifact/v1",
                    raw!("06-target-ir"),
                ),
            ),
            input(
                "07-result-projection",
                auxiliary("result-projection"),
                rebound(
                    "jedit.text.replace_range@1.replaceRange",
                    "edict.result-projection.artifact/v1",
                    raw!("07-result-projection"),
                ),
            ),
        ],
    )
}

/// The flat ReplaceRange request (`fixtures/replace-range-flat`), bound under
/// the digests Amblicit 57b35d26 sent, not digests recomputed here.
fn flat_replace_range() -> LoweringRequestV1 {
    macro_rules! art {
        ($coordinate:literal, $domain:literal, $name:literal, $digest:literal) => {
            bound(
                $coordinate,
                $domain,
                include_bytes!(concat!("fixtures/replace-range-flat/", $name, ".cbor")).to_vec(),
                hex::decode($digest).expect("compiler digest"),
            )
        };
    }
    request(
        art!(
            "jedit.text.replace_range@2",
            "edict.core.module/v1",
            "core",
            "2b554daa9b537d8faf208aa84a8aa4a7fc989ec5cf929c33edf153e637a33693"
        ),
        art!(
            "echo.dpo@2",
            "edict.target-profile/v1",
            "profile",
            "7cadb97e7e2d3f586f3e16ebfe5b357346a33a4e27f3beeb63f0b8889835cc50"
        ),
        vec![
            input(
                "01-lawpack-adapter",
                auxiliary("lawpack-adapter"),
                art!(
                    "jedit.text.echo-adapter/v2",
                    "edict.lawpack-adapter/v1",
                    "01-lawpack-adapter",
                    "cc17812aab2b9b34ac16861ada1fab2697954736a278d5c7b307f5e6682e37dd"
                ),
            ),
            input(
                "02-lawpack-exports",
                auxiliary("lawpack-exports"),
                art!(
                    "jedit.text.exports/v2",
                    "edict.lawpack-exports/v1",
                    "02-lawpack-exports",
                    "b586c7544d1f5fde57df6df7cf449ca5353def3e5507453b9442cf2f74c93266"
                ),
            ),
            input(
                "03-lawpack",
                SemanticInputKind::Lawpack,
                art!(
                    "jedit.text@2",
                    "edict.lawpack/v1",
                    "03-lawpack",
                    "a89daf603f66361421366bba75450fd94e41961b97eefac6b7ad542aa2b3ff22"
                ),
            ),
            input(
                "04-source",
                auxiliary("edict-source-closure"),
                art!(
                    "jedit.text.replace_range@2",
                    "edict.source-closure/v1",
                    "04-source",
                    "b3b885c6a5de6f299bf1469be9a62cf0f58bd81aff129d6f0198994033992330"
                ),
            ),
            input(
                "05-target-configuration",
                auxiliary("target-configuration"),
                art!(
                    "echo.operation-lowering-configuration/v1",
                    "echo.operation-lowering-configuration/v1",
                    "05-target-configuration",
                    "bc8c1c23ff2cc7d030fce69432c2a9ce22926f1fd5460ae69488f764692de7c3"
                ),
            ),
            input(
                "06-target-ir",
                auxiliary("target-ir"),
                art!(
                    "echo.span-ir/v3",
                    "edict.target-ir.artifact/v1",
                    "06-target-ir",
                    "5a5815b2c7f95a61a07c05985958a8b8cfddcb7f5406781bb5e84019457b428e"
                ),
            ),
            input(
                "07-result-projection",
                auxiliary("result-projection"),
                art!(
                    "jedit.text.replace_range@2.replaceRange",
                    "edict.result-projection.artifact/v1",
                    "07-result-projection",
                    "bf3795db78b787a0d00655e0abcd15d8ff999df1ce3f290309631b58b0abaa76"
                ),
            ),
        ],
    )
}

fn refused(request: LoweringRequestV1) -> ProviderRefusalV1 {
    match lower(request) {
        Err(refusal) => refusal,
        Ok(success) => panic!(
            "expected a binding refusal, lowered {} outputs",
            success.outputs.len()
        ),
    }
}

/// The refusal's codes and the named-reason message, for one assertion.
fn named(
    refusal: &ProviderRefusalV1,
) -> (ProviderRefusalKind, Option<&str>, Vec<&str>, Option<&str>) {
    (
        refusal.kind,
        refusal.subject.as_deref(),
        refusal
            .diagnostics
            .iter()
            .map(|d| d.code.as_str())
            .collect(),
        refusal.diagnostics.get(1).map(|d| d.message.as_str()),
    )
}

fn flip_digest(artifact: &mut BoundArtifact) {
    artifact.reference.digest.bytes[0] ^= 0x01;
}

#[test]
fn flat_replace_range_core_refusal_names_core_and_the_decode_node_budget() {
    let refusal = refused(flat_replace_range());

    assert_eq!(
        named(&refusal),
        (
            ProviderRefusalKind::InvalidSemanticArtifact,
            Some("jedit.text.replace_range@2"),
            vec![
                INVALID,
                "echo.provider.invalid-semantic-artifact.artifact-binding.decode"
            ],
            Some(
                "edict.core.module/v1 artifact failed artifact-binding.decode: \
                 unsupported-value: canonical CBOR decoded node budget exceeded"
            ),
        )
    );
}

#[test]
fn every_bound_closure_artifact_names_itself_when_its_digest_differs() {
    let base = node_atom_read();
    let mut witnesses = 0;
    for index in 0..=base.semantic_inputs.len() {
        let mut request = base.clone();
        let artifact = match index {
            0 => &mut request.core,
            n => &mut request.semantic_inputs[n - 1].artifact,
        };
        flip_digest(artifact);
        let (coordinate, domain) = (
            artifact.reference.coordinate.clone(),
            artifact.artifact.domain.clone(),
        );

        let refusal = refused(request);

        let (kind, subject, codes, message) = named(&refusal);
        assert_eq!(
            (kind, subject, codes),
            (
                ProviderRefusalKind::InvalidSemanticArtifact,
                Some(coordinate.as_str()),
                vec![
                    INVALID,
                    "echo.provider.invalid-semantic-artifact.artifact-binding.digest"
                ],
            ),
            "artifact {index} ({domain})"
        );
        let message = message.expect("named reason message");
        assert!(
            message.starts_with(&format!(
                "{domain} artifact failed artifact-binding.digest: "
            )),
            "artifact {index}: {message}"
        );
        witnesses += 1;
    }
    // Core plus seven closure inputs.
    assert_eq!(witnesses, 8);
}

#[test]
fn truncated_artifact_names_the_decode_check() {
    let mut request = node_atom_read();
    let target_ir = &mut request.semantic_inputs[5].artifact;
    target_ir.artifact.bytes.pop();

    let refusal = refused(request);

    assert_eq!(
        named(&refusal),
        (
            ProviderRefusalKind::InvalidSemanticArtifact,
            Some("echo.span-ir/v2"),
            vec![
                INVALID,
                "echo.provider.invalid-semantic-artifact.artifact-binding.decode"
            ],
            Some(
                "edict.target-ir.artifact/v1 artifact failed artifact-binding.decode: \
                 unexpected-eof: canonical CBOR declared length exceeds the remaining bytes"
            ),
        )
    );
}

#[test]
fn noncanonical_artifact_names_the_canonical_form_check() {
    let mut request = node_atom_read();
    let lawpack = &mut request.semantic_inputs[2].artifact.artifact.bytes;
    // Spell the root map's short length with a one-byte argument: the same
    // value, under the same digest, in a non-minimal head.
    let head = lawpack[0];
    assert_eq!(head >> 5, 5, "the lawpack root is a map");
    assert!(head & 0x1f < 24, "the lawpack root length is short");
    lawpack.splice(0..1, [0xb8, head & 0x1f]);

    let refusal = refused(request);

    assert_eq!(
        named(&refusal),
        (
            ProviderRefusalKind::InvalidSemanticArtifact,
            Some("jedit.text@1"),
            vec![
                INVALID,
                "echo.provider.invalid-semantic-artifact.artifact-binding.canonical-form"
            ],
            Some(
                "edict.lawpack/v1 artifact failed artifact-binding.canonical-form: \
                 noncanonical: decoded value does not re-encode to the supplied bytes"
            ),
        )
    );
}

#[test]
fn short_reference_digest_names_the_reference_check() {
    let mut request = node_atom_read();
    request.semantic_inputs[0]
        .artifact
        .reference
        .digest
        .bytes
        .pop();

    let refusal = refused(request);

    assert_eq!(
        named(&refusal),
        (
            ProviderRefusalKind::InvalidSemanticArtifact,
            Some("jedit.text.echo-adapter/v1"),
            vec![
                INVALID,
                "echo.provider.invalid-semantic-artifact.artifact-binding.reference"
            ],
            Some(
                "edict.lawpack-adapter/v1 artifact failed artifact-binding.reference: \
                 the reference digest is not 32 SHA-256 bytes"
            ),
        )
    );
}

#[test]
fn target_profile_refusal_names_the_digest_check() {
    let mut request = node_atom_read();
    flip_digest(&mut request.target_profile);

    let refusal = refused(request);

    let (kind, subject, codes, _) = named(&refusal);
    assert_eq!(
        (kind, subject, codes),
        (
            ProviderRefusalKind::InvalidSemanticArtifact,
            Some("target-profile.echo-dpo"),
            vec![
                "echo.provider.invalid-target-profile-artifact",
                "echo.provider.invalid-target-profile-artifact.artifact-binding.digest"
            ],
        )
    );
}
