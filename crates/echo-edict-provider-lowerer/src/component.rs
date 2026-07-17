// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! `wasm32` guest adapter for the frozen Edict target-provider lowerer world.

// The generated canonical-ABI trampoline necessarily contains unsafe exports;
// all authored conversion and lowering code remains safe Rust.
#![allow(unsafe_code)]

wit_bindgen::generate!({
    path: "wit",
    world: "lowerer",
});

use edict::target_provider::protocol as wit;

struct Component;

impl Guest for Component {
    fn lower(request: wit::LoweringRequestV1) -> wit::LoweringResultV1 {
        from_model_result(super::lower(into_model_request(request)))
    }
}

fn into_model_request(request: wit::LoweringRequestV1) -> super::LoweringRequestV1 {
    super::LoweringRequestV1 {
        protocol_version: super::ProtocolVersionV1 {
            major: request.protocol_version.major,
            minor: request.protocol_version.minor,
            patch: request.protocol_version.patch,
        },
        core: into_model_bound_artifact(request.core),
        target_profile: into_model_bound_artifact(request.target_profile),
        semantic_inputs: request
            .semantic_inputs
            .into_iter()
            .map(into_model_semantic_input)
            .collect(),
        requested_outputs: request
            .requested_outputs
            .into_iter()
            .map(into_model_output_request)
            .collect(),
        limits: super::ResponseLimitsV1 {
            max_output_count: request.limits.max_output_count,
            max_diagnostic_count: request.limits.max_diagnostic_count,
            max_total_response_bytes: request.limits.max_total_response_bytes,
        },
    }
}

fn into_model_bound_artifact(artifact: wit::BoundArtifact) -> super::BoundArtifact {
    super::BoundArtifact {
        reference: super::ResourceRef {
            coordinate: artifact.reference.coordinate,
            digest: super::Digest {
                algorithm: match artifact.reference.digest.algorithm {
                    wit::DigestAlgorithm::Sha256 => super::DigestAlgorithm::Sha256,
                },
                bytes: artifact.reference.digest.bytes,
            },
        },
        artifact: super::Artifact {
            domain: artifact.artifact.domain,
            bytes: artifact.artifact.bytes,
        },
    }
}

fn into_model_semantic_input(input: wit::SemanticInput) -> super::SemanticInput {
    super::SemanticInput {
        role: input.role,
        kind: match input.kind {
            wit::SemanticInputKind::Lawpack => super::SemanticInputKind::Lawpack,
            wit::SemanticInputKind::AuthorityFacts => super::SemanticInputKind::AuthorityFacts,
            wit::SemanticInputKind::LowerabilityFacts => {
                super::SemanticInputKind::LowerabilityFacts
            }
            wit::SemanticInputKind::Auxiliary(label) => super::SemanticInputKind::Auxiliary(label),
        },
        artifact: into_model_bound_artifact(input.artifact),
    }
}

fn into_model_output_request(request: wit::LoweringOutputRequest) -> super::LoweringOutputRequest {
    super::LoweringOutputRequest {
        role: request.role,
        kind: into_model_output_kind(request.kind),
        domain: request.domain,
    }
}

const fn into_model_output_kind(kind: wit::LoweringOutputKind) -> super::LoweringOutputKind {
    match kind {
        wit::LoweringOutputKind::TargetIr => super::LoweringOutputKind::TargetIr,
        wit::LoweringOutputKind::GeneratedArtifact => super::LoweringOutputKind::GeneratedArtifact,
        wit::LoweringOutputKind::ReviewPayload => super::LoweringOutputKind::ReviewPayload,
    }
}

fn from_model_result(result: super::LoweringResultV1) -> wit::LoweringResultV1 {
    result.map(from_model_success).map_err(from_model_refusal)
}

fn from_model_success(success: super::LoweringSuccessV1) -> wit::LoweringSuccessV1 {
    wit::LoweringSuccessV1 {
        outputs: success.outputs.into_iter().map(from_model_output).collect(),
        diagnostics: success
            .diagnostics
            .into_iter()
            .map(from_model_diagnostic)
            .collect(),
    }
}

fn from_model_output(output: super::LoweringOutputArtifact) -> wit::LoweringOutputArtifact {
    wit::LoweringOutputArtifact {
        role: output.role,
        kind: from_model_output_kind(output.kind),
        artifact: wit::Artifact {
            domain: output.artifact.domain,
            bytes: output.artifact.bytes,
        },
        logical_path: output.logical_path,
    }
}

const fn from_model_output_kind(kind: super::LoweringOutputKind) -> wit::LoweringOutputKind {
    match kind {
        super::LoweringOutputKind::TargetIr => wit::LoweringOutputKind::TargetIr,
        super::LoweringOutputKind::GeneratedArtifact => wit::LoweringOutputKind::GeneratedArtifact,
        super::LoweringOutputKind::ReviewPayload => wit::LoweringOutputKind::ReviewPayload,
    }
}

fn from_model_refusal(refusal: super::ProviderRefusalV1) -> wit::ProviderRefusalV1 {
    wit::ProviderRefusalV1 {
        kind: match refusal.kind {
            super::ProviderRefusalKind::UnsupportedCoreAbi => {
                wit::ProviderRefusalKind::UnsupportedCoreAbi
            }
            super::ProviderRefusalKind::UnsupportedTargetProfile => {
                wit::ProviderRefusalKind::UnsupportedTargetProfile
            }
            super::ProviderRefusalKind::UnsupportedSemantics => {
                wit::ProviderRefusalKind::UnsupportedSemantics
            }
            super::ProviderRefusalKind::UnsupportedOutputRole => {
                wit::ProviderRefusalKind::UnsupportedOutputRole
            }
            super::ProviderRefusalKind::InvalidSemanticArtifact => {
                wit::ProviderRefusalKind::InvalidSemanticArtifact
            }
        },
        subject: refusal.subject,
        diagnostics: refusal
            .diagnostics
            .into_iter()
            .map(from_model_diagnostic)
            .collect(),
    }
}

fn from_model_diagnostic(diagnostic: super::Diagnostic) -> wit::Diagnostic {
    wit::Diagnostic {
        code: diagnostic.code,
        severity: match diagnostic.severity {
            super::DiagnosticSeverity::Error => wit::DiagnosticSeverity::Error,
            super::DiagnosticSeverity::Warning => wit::DiagnosticSeverity::Warning,
            super::DiagnosticSeverity::Info => wit::DiagnosticSeverity::Info,
        },
        message: diagnostic.message,
        repair: diagnostic.repair,
    }
}

export!(Component);
