// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Minimal, generic registry interface for Echo WASM helpers.
//!
//! The registry provider is supplied by the application (generated from the
//! GraphQL/Wesley IR). Echo core and `warp-wasm` depend only on this crate and
//! **must not** embed app-specific registries.

#![no_std]

/// Codec identifier used by the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegistryInfo {
    /// Canonical codec identifier (e.g., "cbor-canon-v1").
    pub codec_id: &'static str,
    /// Registry schema version for breaking changes in layout.
    pub registry_version: u32,
    /// Hex-encoded schema hash (lowercase, 64 chars).
    pub schema_sha256_hex: &'static str,
}

/// Trust posture assigned after a generated contract artifact has been verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractArtifactTrustPosture {
    /// The registry artifact matched the expected schema, codec, registry
    /// version, and footprint certificate set supplied by the host policy.
    CompileTimeCertified,
}

/// One footprint certificate expected by the host for a generated operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedFootprintCertificate<'a> {
    /// Operation identifier that must exist in the generated registry.
    pub op_id: u32,
    /// Expected footprint certificate hash for the operation.
    pub certificate_hash_hex: &'a str,
    /// Optional expected generated artifact hash for the operation.
    pub artifact_hash_hex: Option<&'a str>,
}

/// Host policy for verifying a generated contract artifact before admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContractArtifactVerificationPolicy<'a> {
    /// Expected codec identifier for the generated artifact.
    pub codec_id: &'a str,
    /// Expected registry layout version for the generated artifact.
    pub registry_version: u32,
    /// Expected schema hash for the generated artifact.
    pub schema_sha256_hex: &'a str,
    /// Expected footprint certificates keyed by operation id.
    pub footprint_certificates: &'a [ExpectedFootprintCertificate<'a>],
    /// Require every mutation op to carry a footprint certificate named by this policy.
    pub require_mutation_footprint_certificates: bool,
}

/// Successful generated contract artifact verification result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifiedContractArtifact {
    /// Registry metadata that was verified against host policy.
    pub info: RegistryInfo,
    /// Trust posture assigned to the generated artifact.
    pub posture: ContractArtifactTrustPosture,
}

/// Rejection returned when a generated contract artifact fails verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractArtifactRejection<'a> {
    /// The registry codec id did not match host policy.
    CodecIdMismatch {
        /// Expected codec id.
        expected: &'a str,
        /// Actual codec id from the registry.
        actual: &'static str,
    },
    /// The registry layout version did not match host policy.
    RegistryVersionMismatch {
        /// Expected registry layout version.
        expected: u32,
        /// Actual registry layout version.
        actual: u32,
    },
    /// The registry schema hash did not match host policy.
    SchemaHashMismatch {
        /// Expected schema hash.
        expected: &'a str,
        /// Actual schema hash from the registry.
        actual: &'static str,
    },
    /// Host policy named an operation id that the registry does not contain.
    MissingOperation {
        /// Missing operation identifier.
        op_id: u32,
    },
    /// A required footprint certificate was missing from the operation.
    MissingFootprintCertificate {
        /// Operation identifier missing the certificate.
        op_id: u32,
    },
    /// A footprint certificate names a different operation id than its registry entry.
    FootprintCertificateOpMismatch {
        /// Operation identifier from the registry entry.
        op_id: u32,
        /// Operation identifier from the certificate.
        certificate_op_id: u32,
    },
    /// A footprint certificate names a different operation name than its registry entry.
    FootprintCertificateNameMismatch {
        /// Operation identifier whose certificate mismatched.
        op_id: u32,
        /// Operation name from the registry entry.
        expected: &'static str,
        /// Operation name from the certificate.
        actual: &'static str,
    },
    /// A footprint certificate was created for a different schema hash.
    FootprintCertificateSchemaMismatch {
        /// Operation identifier whose certificate mismatched.
        op_id: u32,
        /// Expected schema hash.
        expected: &'static str,
        /// Schema hash from the certificate.
        actual: &'static str,
    },
    /// A footprint certificate hash did not match host policy.
    FootprintCertificateHashMismatch {
        /// Operation identifier whose certificate mismatched.
        op_id: u32,
        /// Expected certificate hash.
        expected: &'a str,
        /// Certificate hash from the registry artifact.
        actual: &'static str,
    },
    /// A generated artifact hash did not match host policy.
    FootprintArtifactHashMismatch {
        /// Operation identifier whose artifact hash mismatched.
        op_id: u32,
        /// Expected generated artifact hash.
        expected: &'a str,
        /// Generated artifact hash from the registry artifact.
        actual: &'static str,
    },
    /// Host policy requires certified mutations and this mutation was uncertified.
    UncertifiedMutation {
        /// Operation identifier missing the certificate.
        op_id: u32,
        /// Operation name missing the certificate.
        op_name: &'static str,
    },
    /// Host policy requires certified mutations and this mutation certificate
    /// was not named in the expected certificate set.
    UnverifiedMutationFootprintCertificate {
        /// Operation identifier missing from the expected certificate set.
        op_id: u32,
        /// Operation name missing from the expected certificate set.
        op_name: &'static str,
    },
}

/// Error codes for wasm helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelperError {
    /// No registry provider installed.
    NoRegistry,
    /// Unknown operation ID.
    UnknownOp,
    /// Input did not match schema (unknown key, missing required, wrong type, enum mismatch).
    InvalidInput,
    /// Internal failure (encoding).
    Internal,
}

/// Operation kind (query or mutation/command).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpKind {
    /// Read-only operation.
    Query,
    /// State-mutating operation.
    Mutation,
}

/// Descriptor for a single operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpDef {
    /// Operation kind.
    pub kind: OpKind,
    /// Operation name (GraphQL name).
    pub name: &'static str,
    /// Persisted operation identifier.
    pub op_id: u32,
    /// Argument descriptors.
    pub args: &'static [ArgDef],
    /// Result type name (GraphQL return type).
    pub result_ty: &'static str,
    /// Preserved operation directive metadata as JSON.
    ///
    /// Echo-specific admission tooling can interpret entries such as
    /// `wes_footprint`; the generic registry API only carries the authored
    /// directive data.
    pub directives_json: &'static str,
    /// Optional compile-time footprint certificate emitted by Wesley tooling.
    ///
    /// Hosts can compare the certificate hash during registry load and treat a
    /// match as the proof that this generated artifact is carrying the declared
    /// footprint it was compiled with. Echo core still treats the footprint as
    /// data; domain-specific meaning belongs to the generated application/module.
    pub footprint_certificate: Option<&'static FootprintCertificate>,
}

/// Compile-time footprint certificate for one generated operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FootprintCertificate {
    /// Operation identifier covered by this certificate.
    pub op_id: u32,
    /// Operation name covered by this certificate.
    pub op_name: &'static str,
    /// Hex-encoded schema hash used as the certificate basis.
    pub schema_sha256_hex: &'static str,
    /// Lowercase hex BLAKE3 hash of the generated artifact footprint preimage.
    pub artifact_hash_hex: &'static str,
    /// Lowercase hex BLAKE3 hash of the full footprint certificate preimage.
    pub certificate_hash_hex: &'static str,
    /// Declared read resources, sorted and deduplicated by the generator.
    pub reads: &'static [&'static str],
    /// Declared write resources, sorted and deduplicated by the generator.
    pub writes: &'static [&'static str],
}

impl OpDef {
    /// Return true when this operation carries a footprint certificate matching
    /// the expected certificate hash and the registry schema hash.
    ///
    /// Hosts call this once while loading a generated registry artifact. A
    /// successful match means the operation's declared footprint was certified
    /// against the same schema hash the registry reports.
    pub fn footprint_certificate_matches(
        &self,
        schema_sha256_hex: &str,
        expected_certificate_hash_hex: &str,
    ) -> bool {
        let Some(certificate) = self.footprint_certificate else {
            return false;
        };

        if certificate.op_id != self.op_id {
            return false;
        }
        if certificate.op_name != self.name {
            return false;
        }
        if certificate.schema_sha256_hex != schema_sha256_hex {
            return false;
        }

        certificate.certificate_hash_hex == expected_certificate_hash_hex
    }
}

/// Argument descriptor (flat; sufficient for strict object validation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArgDef {
    /// Field name.
    pub name: &'static str,
    /// GraphQL base type name.
    pub ty: &'static str,
    /// Whether the field is required.
    pub required: bool,
    /// Whether the field is a list.
    pub list: bool,
}

/// Enum descriptor (for validating enum string values).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnumDef {
    /// Enum name.
    pub name: &'static str,
    /// Allowed values (uppercase GraphQL names).
    pub values: &'static [&'static str],
}

/// Object descriptor for result validation (optional; fields may be empty for now).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectDef {
    /// Object name.
    pub name: &'static str,
    /// Fields on the object.
    pub fields: &'static [ArgDef],
}

/// Application-supplied registry provider.
///
/// Implemented by a generated crate in the application build. `warp-wasm`
/// should link against that provider to validate op IDs, expose registry
/// metadata, and (eventually) drive schema-typed encoding/decoding.
pub trait RegistryProvider: Sync {
    /// Return registry metadata (codec, version, schema hash).
    fn info(&self) -> RegistryInfo;

    /// Look up an operation by ID.
    fn op_by_id(&self, op_id: u32) -> Option<&'static OpDef>;

    /// Return all operations (sorted by op_id for deterministic iteration).
    fn all_ops(&self) -> &'static [OpDef];

    /// Return all enums (for validating enum values).
    fn all_enums(&self) -> &'static [EnumDef];

    /// Return all objects (for result validation).
    fn all_objects(&self) -> &'static [ObjectDef];
}

/// Verify a generated contract registry against host artifact policy.
///
/// This check is intentionally application-neutral. It proves only that the
/// loaded generated registry matches the host's expected schema, codec, registry
/// layout, and footprint certificate identities. Domain payload validation still
/// belongs to the generated application adapter for this slice.
pub fn verify_contract_artifact<'a>(
    registry: &dyn RegistryProvider,
    policy: &ContractArtifactVerificationPolicy<'a>,
) -> Result<VerifiedContractArtifact, ContractArtifactRejection<'a>> {
    let info = registry.info();
    if info.codec_id != policy.codec_id {
        return Err(ContractArtifactRejection::CodecIdMismatch {
            expected: policy.codec_id,
            actual: info.codec_id,
        });
    }
    if info.registry_version != policy.registry_version {
        return Err(ContractArtifactRejection::RegistryVersionMismatch {
            expected: policy.registry_version,
            actual: info.registry_version,
        });
    }
    if info.schema_sha256_hex != policy.schema_sha256_hex {
        return Err(ContractArtifactRejection::SchemaHashMismatch {
            expected: policy.schema_sha256_hex,
            actual: info.schema_sha256_hex,
        });
    }

    for expected in policy.footprint_certificates {
        let op = registry.op_by_id(expected.op_id).ok_or(
            ContractArtifactRejection::MissingOperation {
                op_id: expected.op_id,
            },
        )?;
        verify_expected_footprint_certificate(op, info.schema_sha256_hex, expected)?;
    }

    if policy.require_mutation_footprint_certificates {
        for op in registry.all_ops() {
            if op.kind == OpKind::Mutation {
                if op.footprint_certificate.is_none() {
                    return Err(ContractArtifactRejection::UncertifiedMutation {
                        op_id: op.op_id,
                        op_name: op.name,
                    });
                }
                if !policy
                    .footprint_certificates
                    .iter()
                    .any(|expected| expected.op_id == op.op_id)
                {
                    return Err(
                        ContractArtifactRejection::UnverifiedMutationFootprintCertificate {
                            op_id: op.op_id,
                            op_name: op.name,
                        },
                    );
                }
            }
        }
    }

    Ok(VerifiedContractArtifact {
        info,
        posture: ContractArtifactTrustPosture::CompileTimeCertified,
    })
}

fn verify_expected_footprint_certificate<'a>(
    op: &OpDef,
    schema_sha256_hex: &'static str,
    expected: &ExpectedFootprintCertificate<'a>,
) -> Result<(), ContractArtifactRejection<'a>> {
    let certificate = op
        .footprint_certificate
        .ok_or(ContractArtifactRejection::MissingFootprintCertificate { op_id: op.op_id })?;

    if certificate.op_id != op.op_id {
        return Err(ContractArtifactRejection::FootprintCertificateOpMismatch {
            op_id: op.op_id,
            certificate_op_id: certificate.op_id,
        });
    }
    if certificate.op_name != op.name {
        return Err(
            ContractArtifactRejection::FootprintCertificateNameMismatch {
                op_id: op.op_id,
                expected: op.name,
                actual: certificate.op_name,
            },
        );
    }
    if certificate.schema_sha256_hex != schema_sha256_hex {
        return Err(
            ContractArtifactRejection::FootprintCertificateSchemaMismatch {
                op_id: op.op_id,
                expected: schema_sha256_hex,
                actual: certificate.schema_sha256_hex,
            },
        );
    }
    if certificate.certificate_hash_hex != expected.certificate_hash_hex {
        return Err(
            ContractArtifactRejection::FootprintCertificateHashMismatch {
                op_id: op.op_id,
                expected: expected.certificate_hash_hex,
                actual: certificate.certificate_hash_hex,
            },
        );
    }
    if let Some(expected_artifact_hash_hex) = expected.artifact_hash_hex {
        if certificate.artifact_hash_hex != expected_artifact_hash_hex {
            return Err(ContractArtifactRejection::FootprintArtifactHashMismatch {
                op_id: op.op_id,
                expected: expected_artifact_hash_hex,
                actual: certificate.artifact_hash_hex,
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        verify_contract_artifact, ArgDef, ContractArtifactRejection, ContractArtifactTrustPosture,
        ContractArtifactVerificationPolicy, ExpectedFootprintCertificate, FootprintCertificate,
        ObjectDef, OpDef, OpKind, RegistryInfo, RegistryProvider, VerifiedContractArtifact,
    };

    const SCHEMA_SHA256_HEX: &str =
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const CERTIFICATE_HASH_HEX: &str =
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const ARTIFACT_HASH_HEX: &str =
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    static READS: &[&str] = &["CounterValue"];
    static WRITES: &[&str] = &["CounterValue"];
    static FOOTPRINT_CERTIFICATE: FootprintCertificate = FootprintCertificate {
        op_id: 1001,
        op_name: "increment",
        schema_sha256_hex: SCHEMA_SHA256_HEX,
        artifact_hash_hex: ARTIFACT_HASH_HEX,
        certificate_hash_hex: CERTIFICATE_HASH_HEX,
        reads: READS,
        writes: WRITES,
    };
    static INCREMENT_ARGS: &[ArgDef] = &[ArgDef {
        name: "input",
        ty: "IncrementInput",
        required: true,
        list: false,
    }];
    static OPS_WITH_CERTIFICATE: &[OpDef] = &[
        OpDef {
            kind: OpKind::Mutation,
            name: "increment",
            op_id: 1001,
            args: INCREMENT_ARGS,
            result_ty: "CounterValue",
            directives_json: "{}",
            footprint_certificate: Some(&FOOTPRINT_CERTIFICATE),
        },
        OpDef {
            kind: OpKind::Query,
            name: "counterValue",
            op_id: 1002,
            args: &[],
            result_ty: "CounterValue",
            directives_json: "{}",
            footprint_certificate: None,
        },
    ];
    static OPS_WITHOUT_CERTIFICATE: &[OpDef] = &[OpDef {
        kind: OpKind::Mutation,
        name: "increment",
        op_id: 1001,
        args: INCREMENT_ARGS,
        result_ty: "CounterValue",
        directives_json: "{}",
        footprint_certificate: None,
    }];

    struct StaticRegistry {
        ops: &'static [OpDef],
    }

    impl RegistryProvider for StaticRegistry {
        fn info(&self) -> RegistryInfo {
            RegistryInfo {
                codec_id: "cbor-canon-v1",
                registry_version: 1,
                schema_sha256_hex: SCHEMA_SHA256_HEX,
            }
        }

        fn op_by_id(&self, op_id: u32) -> Option<&'static OpDef> {
            self.ops.iter().find(|op| op.op_id == op_id)
        }

        fn all_ops(&self) -> &'static [OpDef] {
            self.ops
        }

        fn all_enums(&self) -> &'static [super::EnumDef] {
            &[]
        }

        fn all_objects(&self) -> &'static [ObjectDef] {
            &[]
        }
    }

    #[test]
    fn verifier_accepts_matching_registry_and_expected_certificate() {
        let registry = StaticRegistry {
            ops: OPS_WITH_CERTIFICATE,
        };
        let expected_certificates = [ExpectedFootprintCertificate {
            op_id: 1001,
            certificate_hash_hex: CERTIFICATE_HASH_HEX,
            artifact_hash_hex: Some(ARTIFACT_HASH_HEX),
        }];
        let policy = ContractArtifactVerificationPolicy {
            codec_id: "cbor-canon-v1",
            registry_version: 1,
            schema_sha256_hex: SCHEMA_SHA256_HEX,
            footprint_certificates: &expected_certificates,
            require_mutation_footprint_certificates: true,
        };

        let result = verify_contract_artifact(&registry, &policy);

        assert_eq!(
            result,
            Ok(VerifiedContractArtifact {
                info: registry.info(),
                posture: ContractArtifactTrustPosture::CompileTimeCertified,
            })
        );
    }

    #[test]
    fn verifier_rejects_certificate_hash_mismatch() {
        let registry = StaticRegistry {
            ops: OPS_WITH_CERTIFICATE,
        };
        let expected_certificates = [ExpectedFootprintCertificate {
            op_id: 1001,
            certificate_hash_hex: "wrong",
            artifact_hash_hex: Some(ARTIFACT_HASH_HEX),
        }];
        let policy = ContractArtifactVerificationPolicy {
            codec_id: "cbor-canon-v1",
            registry_version: 1,
            schema_sha256_hex: SCHEMA_SHA256_HEX,
            footprint_certificates: &expected_certificates,
            require_mutation_footprint_certificates: true,
        };

        let result = verify_contract_artifact(&registry, &policy);

        assert_eq!(
            result,
            Err(
                ContractArtifactRejection::FootprintCertificateHashMismatch {
                    op_id: 1001,
                    expected: "wrong",
                    actual: CERTIFICATE_HASH_HEX,
                }
            )
        );
    }

    #[test]
    fn verifier_rejects_uncertified_mutation_when_policy_requires_it() {
        let registry = StaticRegistry {
            ops: OPS_WITHOUT_CERTIFICATE,
        };
        let policy = ContractArtifactVerificationPolicy {
            codec_id: "cbor-canon-v1",
            registry_version: 1,
            schema_sha256_hex: SCHEMA_SHA256_HEX,
            footprint_certificates: &[],
            require_mutation_footprint_certificates: true,
        };

        let result = verify_contract_artifact(&registry, &policy);

        assert_eq!(
            result,
            Err(ContractArtifactRejection::UncertifiedMutation {
                op_id: 1001,
                op_name: "increment",
            })
        );
    }

    #[test]
    fn verifier_rejects_mutation_certificate_not_named_by_policy() {
        let registry = StaticRegistry {
            ops: OPS_WITH_CERTIFICATE,
        };
        let policy = ContractArtifactVerificationPolicy {
            codec_id: "cbor-canon-v1",
            registry_version: 1,
            schema_sha256_hex: SCHEMA_SHA256_HEX,
            footprint_certificates: &[],
            require_mutation_footprint_certificates: true,
        };

        let result = verify_contract_artifact(&registry, &policy);

        assert_eq!(
            result,
            Err(
                ContractArtifactRejection::UnverifiedMutationFootprintCertificate {
                    op_id: 1001,
                    op_name: "increment",
                }
            )
        );
    }
}
