// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Generated Echo helper projection for one admitted Edict operation.
//! Final Edict contract-bundle identity is bound explicitly after assembly.

/// Namespaced generated contract and invocation surface for `a.b@1.t`.
pub mod echo_dpo {
    use echo_registry_api::{
        OpKind, ProviderBundleIdentityV1, ProviderDigestIdentityV1,
        ProviderFootprintIdentityV1, ProviderOperationV1, ProviderRegistryV1,
        ProviderSchemaIdentityV1, ProviderSemanticIdentityV1, ProviderValueContractV1,
    };
    use echo_wasm_abi::codec::{
        decode_from_bytes, encode_to_vec, CodecError, Decode, Encode, Reader, Writer,
    };
    use echo_wasm_abi::{pack_intent_v1, EnvelopeError};
    use warp_core::{
        matches_eint_op, propose_provider_contract_package_v1, ContractPackageIdentity,
        GeneratedProviderMutationDispatchV1, GraphView, NodeId,
        ProviderContractPackageProposalV1, ProviderMutationHooksV1,
        ProviderMutationImplementationIdentityV1, ProviderPackageProposalError,
    };

    /// Exact Edict-authored semantic operation coordinate.
    pub const OPERATION_COORDINATE: &str = "a.b@1.t";
    /// Semantic domain owning the operation coordinate.
    pub const OPERATION_DOMAIN: &str = "echo.edict-provider/operation/v1";
    /// Echo-owned law that derives the persisted operation id.
    pub const OPERATION_ID_LAW: &str = "echo.semantic-operation-id.fnv1-32/v1";
    /// Exact persisted operation id carried by the generated-artifact profile.
    pub const OPERATION_ID: u32 = 3_389_142_194;
    /// Exact Echo value codec carried by the generated-artifact profile.
    pub const VALUE_CODEC_ID: &str = "le-binary-v1";
    /// Exact input schema coordinate owned by the generated-artifact profile.
    pub const INPUT_SCHEMA: &str = "a.b@1.Input";
    /// Exact output schema coordinate owned by the generated-artifact profile.
    pub const OUTPUT_SCHEMA: &str = "a.b@1.Output";
    /// Shared semantic domain owning the exact operation type schemas.
    pub const TYPE_SCHEMA_DOMAIN: &str = "echo.edict-provider/value/v1";
    /// Exact typed obstruction coordinate for the reviewed failure mapping.
    pub const OBSTRUCTION_COORDINATE: &str = "domain.WriteRejected";
    /// Semantic domain owning the typed obstruction coordinate.
    pub const OBSTRUCTION_DOMAIN: &str = "echo.edict-provider/obstruction/v1";
    /// Exact target failure payload schema before obstruction mapping.
    pub const EFFECT_FAILURE_SCHEMA: &str = "target.replace.rejected";
    /// Exact domain obstruction payload schema after obstruction mapping.
    pub const OBSTRUCTION_PAYLOAD_SCHEMA: &str = "domain.WriteRejected.Payload";
    /// Semantic coordinate carried by the emitted Target IR artifact.
    pub const TARGET_IR_COORDINATE: &str = "echo.span-ir/v1";
    /// Digest-framing domain for the complete Target IR artifact envelope.
    pub const TARGET_IR_DIGEST_DOMAIN: &str = "edict.target-ir.artifact/v1";
    /// Exact domain-framed identity of the emitted Target IR artifact.
    pub const TARGET_IR_DIGEST: &str = "sha256:01c7ac3e85c61bc3cfae56185353e313998f7bc30fabaca7f8b026db0a7001b3";
    /// Exact target-profile coordinate.
    pub const TARGET_PROFILE_COORDINATE: &str = "echo.dpo@1";
    /// Digest-framing domain for the target-profile artifact.
    pub const TARGET_PROFILE_DIGEST_DOMAIN: &str = "edict.target-profile/v1";
    /// Exact domain-framed identity of the target profile.
    pub const TARGET_PROFILE_DIGEST: &str = "sha256:70a6b0ed2ab67eca5eeba55844a970e201957ab6cd91988c3959915f309c2b06";
    /// Semantic profile for Echo contract bundles; not a bundle occurrence.
    pub const TARGET_BUNDLE_PROFILE_COORDINATE: &str = "echo.dpo.bundle/v1";
    /// Digest-framing domain for the target-bundle profile artifact.
    pub const TARGET_BUNDLE_PROFILE_DIGEST_DOMAIN: &str = "echo.dpo.bundle/v1";
    /// Exact domain-framed identity of the target-bundle profile.
    pub const TARGET_BUNDLE_PROFILE_DIGEST: &str = "sha256:aa0438bcc6ef14ee6cb6d4976622f6080381d731459dcb7b9102595c9bed92c0";
    /// Echo contract ABI targeted by this generated helper.
    pub const ECHO_CONTRACT_ABI_VERSION: u32 = 1;
    /// Contract-host helper API targeted by this generated helper.
    pub const CONTRACT_HOST_HELPER_API_VERSION: u32 = 1;
    /// Exact self-contained provider CDDL coordinate.
    pub const PROVIDER_SCHEMA_COORDINATE: &str = "echo.provider-artifacts.cddl@1";
    /// Raw SHA-256 of the exact self-contained provider CDDL bytes.
    pub const PROVIDER_SCHEMA_SHA256_HEX: &str =
        "9b88d04a8a4fc9c7f1cac77e5465340fbe1d1d14c604007344c14e7732c287cd";
    /// Exact generated-artifact profile coordinate owning operation schemas.
    pub const GENERATED_ARTIFACT_PROFILE: &str = "echo.dpo.registration/v1";
    /// Digest-framing domain for the generated-artifact profile.
    pub const GENERATED_ARTIFACT_PROFILE_DIGEST_DOMAIN: &str =
        "echo.generated-artifact-profile/v1";
    /// Domain-framed identity of the generated-artifact profile.
    pub const GENERATED_ARTIFACT_PROFILE_DIGEST: &str =
        "sha256:ff88be93c26cc533948d8a93601954dc391912d593ca1e96115c846cbf2c5b5d";
    /// Exact semantic operation profile selected by the authored operation.
    pub const OPERATION_PROFILE: &str = "continuum.profile.write/v1";
    /// Semantic domain owning the selected operation profile.
    pub const OPERATION_PROFILE_DOMAIN: &str =
        "echo.edict-provider/operation-profile/v1";
    /// Exact operation-profiles document coordinate.
    pub const OPERATION_PROFILES_COORDINATE: &str = "echo.dpo.operation-profiles/v1";
    /// Digest-framing domain for the operation-profiles document.
    pub const OPERATION_PROFILES_DIGEST_DOMAIN: &str = "echo.dpo.operation-profiles/v1";
    /// Domain-framed identity of the operation-profiles document.
    pub const OPERATION_PROFILES_DIGEST: &str =
        "sha256:53256c51f6c817a77cc8694458bf9d3891abd15b9c94f79ca97d920d3c5f0416";
    /// Abstract footprint obligation carried across lowering.
    pub const FOOTPRINT_OBLIGATION: &str = "target.replace.footprint";
    /// Exact target footprint-algebra coordinate.
    pub const FOOTPRINT_ALGEBRA: &str = "echo.dpo.footprint/v1";
    /// Digest-framing domain for the target footprint algebra.
    pub const FOOTPRINT_ALGEBRA_DIGEST_DOMAIN: &str = "echo.dpo.footprint/v1";
    /// Domain-framed identity of the target footprint algebra.
    pub const FOOTPRINT_ALGEBRA_DIGEST: &str =
        "sha256:f47bb65867e78099ddcfd6ae7af83870df8823f974a496a111ed94e5d785c769";
    /// Edict domain for the semantic contract-bundle digest proposition.
    pub const SEMANTIC_BUNDLE_DIGEST_DOMAIN: &str = "edict.bundle.semantic/v1";
    /// Edict domain for the release contract-bundle digest proposition.
    pub const RELEASE_BUNDLE_DIGEST_DOMAIN: &str = "edict.bundle.release/v1";

    const MUTATION_RULE_NAME: &str = concat!(
        "cmd/contract/",
        "9b88d04a8a4fc9c7f1cac77e5465340fbe1d1d14c604007344c14e7732c287cd",
        "/3389142194/a.b@1.t"
    );
    const PROVIDER_OPERATIONS: [ProviderOperationV1<'static>; 1] = [ProviderOperationV1 {
        coordinate: OPERATION_COORDINATE,
        semantic_domain: OPERATION_DOMAIN,
        kind: OpKind::Mutation,
        operation_id_law: OPERATION_ID_LAW,
        operation_id: OPERATION_ID,
        input: ProviderValueContractV1 {
            schema_coordinate: INPUT_SCHEMA,
            schema_domain: TYPE_SCHEMA_DOMAIN,
            codec_id: VALUE_CODEC_ID,
        },
        output: ProviderValueContractV1 {
            schema_coordinate: OUTPUT_SCHEMA,
            schema_domain: TYPE_SCHEMA_DOMAIN,
            codec_id: VALUE_CODEC_ID,
        },
        target_failure_schema: EFFECT_FAILURE_SCHEMA,
        obstruction: ProviderSemanticIdentityV1 {
            coordinate: OBSTRUCTION_COORDINATE,
            semantic_domain: OBSTRUCTION_DOMAIN,
        },
        obstruction_payload_schema: OBSTRUCTION_PAYLOAD_SCHEMA,
        target_ir: ProviderDigestIdentityV1 {
            coordinate: TARGET_IR_COORDINATE,
            digest_domain: TARGET_IR_DIGEST_DOMAIN,
            digest: TARGET_IR_DIGEST,
        },
        target_profile: ProviderDigestIdentityV1 {
            coordinate: TARGET_PROFILE_COORDINATE,
            digest_domain: TARGET_PROFILE_DIGEST_DOMAIN,
            digest: TARGET_PROFILE_DIGEST,
        },
        generated_artifact_profile: ProviderDigestIdentityV1 {
            coordinate: GENERATED_ARTIFACT_PROFILE,
            digest_domain: GENERATED_ARTIFACT_PROFILE_DIGEST_DOMAIN,
            digest: GENERATED_ARTIFACT_PROFILE_DIGEST,
        },
        operation_profile: ProviderSemanticIdentityV1 {
            coordinate: OPERATION_PROFILE,
            semantic_domain: OPERATION_PROFILE_DOMAIN,
        },
        operation_profiles: ProviderDigestIdentityV1 {
            coordinate: OPERATION_PROFILES_COORDINATE,
            digest_domain: OPERATION_PROFILES_DIGEST_DOMAIN,
            digest: OPERATION_PROFILES_DIGEST,
        },
        footprint: ProviderFootprintIdentityV1 {
            obligation: FOOTPRINT_OBLIGATION,
            algebra_coordinate: FOOTPRINT_ALGEBRA,
            algebra_digest_domain: FOOTPRINT_ALGEBRA_DIGEST_DOMAIN,
            algebra_digest: FOOTPRINT_ALGEBRA_DIGEST,
        },
    }];

    const ID_MAX_SCALAR_VALUES: usize = 16;
    const ID_MAX_UTF8_BYTES: usize = ID_MAX_SCALAR_VALUES * 4;

    /// Exact bounded value of semantic type `a.b@1.Id`.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct Id(String);

    impl Id {
        /// Construct an id after enforcing the authored Unicode-scalar bound.
        ///
        /// # Errors
        ///
        /// Returns [`CodecError::StringTooLong`] when `value` contains more
        /// than sixteen Unicode scalar values.
        pub fn new(value: impl Into<String>) -> Result<Self, CodecError> {
            let value = value.into();
            if value.chars().count() > ID_MAX_SCALAR_VALUES {
                return Err(CodecError::StringTooLong);
            }
            Ok(Self(value))
        }

        /// Borrow the exact raw UTF-8 value without normalization.
        pub fn as_str(&self) -> &str {
            &self.0
        }

        /// Consume this value and return its exact raw UTF-8 string.
        pub fn into_string(self) -> String {
            self.0
        }
    }

    impl Encode for Id {
        fn encode(&self, writer: &mut Writer) -> Result<(), CodecError> {
            if self.0.chars().count() > ID_MAX_SCALAR_VALUES {
                return Err(CodecError::StringTooLong);
            }
            writer.write_len_prefixed_bytes(self.0.as_bytes())
        }
    }

    impl Decode for Id {
        fn decode(reader: &mut Reader<'_>) -> Result<Self, CodecError> {
            let bytes = reader.read_len_prefixed_bytes(ID_MAX_UTF8_BYTES)?;
            let value = core::str::from_utf8(bytes).map_err(|_| CodecError::InvalidUtf8)?;
            Self::new(String::from(value))
        }
    }

    /// Exact typed input for semantic operation `a.b@1.t`.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct Input {
        id: Id,
    }

    impl Input {
        /// Construct a validated operation input.
        ///
        /// # Errors
        ///
        /// Returns [`CodecError::StringTooLong`] when `id` exceeds its
        /// authored Unicode-scalar bound.
        pub fn new(id: impl Into<String>) -> Result<Self, CodecError> {
            Ok(Self { id: Id::new(id)? })
        }

        /// Borrow the exact raw UTF-8 id.
        pub fn id(&self) -> &str {
            self.id.as_str()
        }
    }

    impl Encode for Input {
        fn encode(&self, writer: &mut Writer) -> Result<(), CodecError> {
            self.id.encode(writer)
        }
    }

    impl Decode for Input {
        fn decode(reader: &mut Reader<'_>) -> Result<Self, CodecError> {
            Ok(Self {
                id: Id::decode(reader)?,
            })
        }
    }

    /// Exact typed output for semantic operation `a.b@1.t`.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct Output {
        id: Id,
    }

    impl Output {
        /// Construct a validated operation output.
        ///
        /// # Errors
        ///
        /// Returns [`CodecError::StringTooLong`] when `id` exceeds its
        /// authored Unicode-scalar bound.
        pub fn new(id: impl Into<String>) -> Result<Self, CodecError> {
            Ok(Self { id: Id::new(id)? })
        }

        /// Borrow the exact raw UTF-8 id.
        pub fn id(&self) -> &str {
            self.id.as_str()
        }
    }

    impl Encode for Output {
        fn encode(&self, writer: &mut Writer) -> Result<(), CodecError> {
            self.id.encode(writer)
        }
    }

    impl Decode for Output {
        fn decode(reader: &mut Reader<'_>) -> Result<Self, CodecError> {
            Ok(Self {
                id: Id::decode(reader)?,
            })
        }
    }

    /// Stable failure produced while constructing one canonical invocation.
    #[derive(Debug, Eq, PartialEq)]
    pub enum GeneratedInvocationError {
        /// The typed input violates its generated codec contract.
        Codec(CodecError),
        /// The canonical Echo intent envelope could not be constructed.
        Envelope(EnvelopeError),
    }

    /// Independent host pin for the final assembled bundle identity.
    ///
    /// This value is explicit expected evidence, not an admission token.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct ExpectedContractBundleIdentityV1<'a> {
        /// Digest proposition domain for `semantic_digest`.
        pub semantic_digest_domain: &'a str,
        /// Exact semantic-layer bundle digest expected by the host.
        pub semantic_digest: &'a str,
        /// Digest proposition domain for `release_digest`.
        pub release_digest_domain: &'a str,
        /// Exact release-layer bundle digest expected by the host.
        pub release_digest: &'a str,
    }

    /// Untrusted identity and semantic claims read from one assembled bundle.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct ContractBundleIdentityV1<'a> {
        /// Digest proposition domain for `semantic_digest`.
        pub semantic_digest_domain: &'a str,
        /// Claimed semantic-layer bundle digest computed by Edict.
        pub semantic_digest: &'a str,
        /// Digest proposition domain for `release_digest`.
        pub release_digest_domain: &'a str,
        /// Claimed release-layer bundle digest computed by Edict.
        pub release_digest: &'a str,
        /// Semantic operation coordinate carried by the bundle.
        pub operation_coordinate: &'a str,
        /// Semantic domain owning the operation coordinate.
        pub operation_domain: &'a str,
        /// Operation-id law claimed by the generated-artifact profile.
        pub operation_id_law: &'a str,
        /// Persisted operation id claimed by the generated-artifact profile.
        pub operation_id: u32,
        /// Echo value codec claimed by the generated-artifact profile.
        pub value_codec: &'a str,
        /// Semantic coordinate carried by the Target IR artifact.
        pub target_ir_coordinate: &'a str,
        /// Digest-framing domain for the Target IR artifact.
        pub target_ir_digest_domain: &'a str,
        /// Target IR digest carried by the bundle.
        pub target_ir_digest: &'a str,
        /// Target-profile coordinate carried by the bundle.
        pub target_profile_coordinate: &'a str,
        /// Digest-framing domain for the target-profile artifact.
        pub target_profile_digest_domain: &'a str,
        /// Target-profile digest carried by the bundle.
        pub target_profile_digest: &'a str,
        /// Target-bundle-profile coordinate carried by the bundle.
        pub target_bundle_profile_coordinate: &'a str,
        /// Digest-framing domain for the target-bundle-profile artifact.
        pub target_bundle_profile_digest_domain: &'a str,
        /// Target-bundle-profile digest carried by the bundle.
        pub target_bundle_profile_digest: &'a str,
        /// Echo contract ABI claimed by the generated registry.
        pub echo_contract_abi_version: u32,
        /// Contract-host helper API claimed by the generated registry.
        pub helper_api_version: u32,
        /// Provider CDDL coordinate claimed by the assembled bundle.
        pub provider_schema_coordinate: &'a str,
        /// Raw provider CDDL SHA-256 claimed by the assembled bundle.
        pub provider_schema_sha256_hex: &'a str,
        /// Input schema coordinate claimed for this operation.
        pub input_schema: &'a str,
        /// Output schema coordinate claimed for this operation.
        pub output_schema: &'a str,
        /// Semantic domain owning all claimed operation type schemas.
        pub type_schema_domain: &'a str,
        /// Typed obstruction coordinate claimed for the reviewed failure.
        pub obstruction_coordinate: &'a str,
        /// Semantic domain owning the typed obstruction coordinate.
        pub obstruction_domain: &'a str,
        /// Target failure payload schema claimed before obstruction mapping.
        pub effect_failure_schema: &'a str,
        /// Domain obstruction payload schema claimed after obstruction mapping.
        pub obstruction_payload_schema: &'a str,
        /// Generated-artifact profile coordinate claimed by the bundle.
        pub generated_artifact_profile: &'a str,
        /// Digest-framing domain for the generated-artifact profile.
        pub generated_artifact_profile_digest_domain: &'a str,
        /// Generated-artifact profile digest claimed by the bundle.
        pub generated_artifact_profile_digest: &'a str,
        /// Semantic operation profile claimed by the bundle.
        pub operation_profile: &'a str,
        /// Semantic domain owning the operation profile.
        pub operation_profile_domain: &'a str,
        /// Operation-profiles document coordinate claimed by the bundle.
        pub operation_profiles_coordinate: &'a str,
        /// Digest-framing domain for the operation-profiles document.
        pub operation_profiles_digest_domain: &'a str,
        /// Operation-profiles document digest claimed by the bundle.
        pub operation_profiles_digest: &'a str,
        /// Abstract footprint obligation claimed for this operation.
        pub footprint_obligation: &'a str,
        /// Footprint-algebra coordinate claimed by the bundle.
        pub footprint_algebra: &'a str,
        /// Digest-framing domain for the footprint algebra.
        pub footprint_algebra_digest_domain: &'a str,
        /// Footprint-algebra digest claimed by the bundle.
        pub footprint_algebra_digest: &'a str,
    }

    /// Stable reason an assembled bundle cannot bind this generated helper.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum BindingMismatchKind {
        /// A bundle digest is framed under the wrong proposition domain.
        BundleDigestDomain,
        /// A semantic or release bundle digest is not a typed SHA-256 review value.
        BundleDigest,
        /// The assembled semantic-bundle digest differs from the host pin.
        SemanticBundleDigest,
        /// The assembled release-bundle digest differs from the host pin.
        ReleaseBundleDigest,
        /// The bundle names a different semantic operation.
        Operation,
        /// The bundle names a different persisted operation-id proposition.
        OperationId,
        /// The bundle names a different Echo value codec.
        Codec,
        /// The bundle names a different Target IR artifact.
        TargetIr,
        /// The bundle names a different target profile.
        TargetProfile,
        /// The bundle names a different target-bundle profile.
        TargetBundleProfile,
        /// The generated registry targets a different Echo contract ABI.
        EchoAbi,
        /// The generated registry targets a different contract-host helper API.
        HelperApi,
        /// The bundle names different provider or operation schema identities.
        Schema,
        /// The bundle names a different generated-artifact profile.
        GeneratedArtifactProfile,
        /// The bundle names a different semantic operation profile.
        OperationProfile,
        /// The bundle names a different footprint obligation or algebra.
        Footprint,
    }

    /// Exact generated registration binding, still without runtime authority.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct RegistrationDescriptorV1<'a> {
        contract_bundle: ContractBundleIdentityV1<'a>,
    }

    impl<'a> RegistrationDescriptorV1<'a> {
        /// Return the exact bundle claims that matched the explicit host pin.
        pub const fn contract_bundle(&self) -> &ContractBundleIdentityV1<'a> {
            &self.contract_bundle
        }

        /// Return the exact persisted operation id matched by this descriptor.
        pub const fn operation_id(&self) -> u32 {
            self.contract_bundle.operation_id
        }

        /// Return the exact provider-generic registry claims retained by this
        /// already matched descriptor.
        ///
        /// This constructs descriptive evidence only. It does not admit or
        /// install a registry.
        pub const fn provider_registry(&self) -> ProviderRegistryV1<'a> {
            ProviderRegistryV1 {
                echo_contract_abi_version: ECHO_CONTRACT_ABI_VERSION,
                helper_api_version: CONTRACT_HOST_HELPER_API_VERSION,
                provider_schema: ProviderSchemaIdentityV1 {
                    coordinate: PROVIDER_SCHEMA_COORDINATE,
                    raw_sha256_hex: PROVIDER_SCHEMA_SHA256_HEX,
                },
                target_bundle_profile: ProviderDigestIdentityV1 {
                    coordinate: TARGET_BUNDLE_PROFILE_COORDINATE,
                    digest_domain: TARGET_BUNDLE_PROFILE_DIGEST_DOMAIN,
                    digest: TARGET_BUNDLE_PROFILE_DIGEST,
                },
                bundle: ProviderBundleIdentityV1 {
                    semantic_digest_domain: self.contract_bundle.semantic_digest_domain,
                    semantic_digest: self.contract_bundle.semantic_digest,
                    release_digest_domain: self.contract_bundle.release_digest_domain,
                    release_digest: self.contract_bundle.release_digest,
                },
                operations: &PROVIDER_OPERATIONS,
            }
        }

        /// Return the exact identity a host implementation must independently
        /// claim before its callbacks can be proposed for this operation.
        pub const fn mutation_implementation_identity(
            &self,
        ) -> ProviderMutationImplementationIdentityV1<'a> {
            let registry = self.provider_registry();
            ProviderMutationImplementationIdentityV1 {
                echo_contract_abi_version: registry.echo_contract_abi_version,
                helper_api_version: registry.helper_api_version,
                provider_schema: registry.provider_schema,
                target_bundle_profile: registry.target_bundle_profile,
                bundle: registry.bundle,
                operation: PROVIDER_OPERATIONS[0],
            }
        }

        /// Construct one opaque package proposal from exact matched claims and
        /// an explicit host executor/footprint binding.
        ///
        /// The result grants no runtime admission, registration, installation,
        /// scheduling, execution, durability, or receipt authority.
        ///
        /// # Errors
        ///
        /// Returns [`ProviderPackageProposalError`] when occurrence metadata,
        /// generated dispatch identity, or any host implementation claim does
        /// not exactly match this descriptor.
        pub fn propose_contract_package<'proposal>(
            &'proposal self,
            occurrence: ContractPackageIdentity<'proposal>,
            hooks: ProviderMutationHooksV1<'proposal>,
        ) -> Result<ProviderContractPackageProposalV1<'proposal>, ProviderPackageProposalError>
        where
            'a: 'proposal,
        {
            let registry: ProviderRegistryV1<'proposal> = self.provider_registry();
            propose_provider_contract_package_v1(
                occurrence,
                registry,
                GeneratedProviderMutationDispatchV1::new(
                    OPERATION_ID,
                    MUTATION_RULE_NAME,
                    matches_operation,
                ),
                hooks,
            )
        }

        /// Encode one exact typed input under the matched value-codec claim.
        ///
        /// # Errors
        ///
        /// Returns [`CodecError`] when the typed value violates its generated
        /// bound or cannot be represented by the selected codec.
        #[allow(clippy::unused_self)]
        pub fn encode_input(&self, input: &Input) -> Result<Vec<u8>, CodecError> {
            encode_to_vec(input)
        }

        /// Decode one exact typed input under the matched value-codec claim.
        ///
        /// # Errors
        ///
        /// Returns [`CodecError`] for malformed, over-bound, truncated, or
        /// trailing bytes.
        #[allow(clippy::unused_self)]
        pub fn decode_input(&self, bytes: &[u8]) -> Result<Input, CodecError> {
            decode_from_bytes(bytes)
        }

        /// Encode one exact typed output under the matched value-codec claim.
        ///
        /// # Errors
        ///
        /// Returns [`CodecError`] when the typed value violates its generated
        /// bound or cannot be represented by the selected codec.
        #[allow(clippy::unused_self)]
        pub fn encode_output(&self, output: &Output) -> Result<Vec<u8>, CodecError> {
            encode_to_vec(output)
        }

        /// Decode one exact typed output under the matched value-codec claim.
        ///
        /// # Errors
        ///
        /// Returns [`CodecError`] for malformed, over-bound, truncated, or
        /// trailing bytes.
        #[allow(clippy::unused_self)]
        pub fn decode_output(&self, bytes: &[u8]) -> Result<Output, CodecError> {
            decode_from_bytes(bytes)
        }

        /// Encode a typed input and wrap it in the canonical Echo EINT v1
        /// envelope for this matched operation id.
        ///
        /// # Errors
        ///
        /// Returns [`GeneratedInvocationError::Codec`] when the input violates
        /// its generated value contract, or
        /// [`GeneratedInvocationError::Envelope`] when Echo refuses envelope
        /// construction.
        pub fn pack_intent(&self, input: &Input) -> Result<Vec<u8>, GeneratedInvocationError> {
            let vars = self
                .encode_input(input)
                .map_err(GeneratedInvocationError::Codec)?;
            pack_intent_v1(self.operation_id(), &vars)
                .map_err(GeneratedInvocationError::Envelope)
        }
    }

    fn matches_operation(view: GraphView<'_>, scope: &NodeId) -> bool {
        matches_eint_op(view, scope, OPERATION_ID)
    }

    /// Compare assembled bundle claims to an independent exact host pin and to
    /// this generated helper's semantic identities.
    ///
    /// This pure equality/consistency preflight neither authenticates the pin,
    /// admits the bundle, nor installs a package. Those remain separate trusted
    /// host and Echo runtime crossings.
    pub fn bind_contract_bundle<'a>(
        expected: ExpectedContractBundleIdentityV1<'a>,
        identity: &ContractBundleIdentityV1<'a>,
    ) -> Result<RegistrationDescriptorV1<'a>, BindingMismatchKind> {
        if expected.semantic_digest_domain != SEMANTIC_BUNDLE_DIGEST_DOMAIN
            || identity.semantic_digest_domain != SEMANTIC_BUNDLE_DIGEST_DOMAIN
            || expected.release_digest_domain != RELEASE_BUNDLE_DIGEST_DOMAIN
            || identity.release_digest_domain != RELEASE_BUNDLE_DIGEST_DOMAIN
        {
            return Err(BindingMismatchKind::BundleDigestDomain);
        }
        if !is_sha256_review(expected.semantic_digest)
            || !is_sha256_review(expected.release_digest)
            || !is_sha256_review(identity.semantic_digest)
            || !is_sha256_review(identity.release_digest)
        {
            return Err(BindingMismatchKind::BundleDigest);
        }
        if identity.semantic_digest != expected.semantic_digest {
            return Err(BindingMismatchKind::SemanticBundleDigest);
        }
        if identity.release_digest != expected.release_digest {
            return Err(BindingMismatchKind::ReleaseBundleDigest);
        }
        if identity.operation_coordinate != OPERATION_COORDINATE
            || identity.operation_domain != OPERATION_DOMAIN
        {
            return Err(BindingMismatchKind::Operation);
        }
        if identity.operation_id_law != OPERATION_ID_LAW || identity.operation_id != OPERATION_ID {
            return Err(BindingMismatchKind::OperationId);
        }
        if identity.value_codec != VALUE_CODEC_ID {
            return Err(BindingMismatchKind::Codec);
        }
        if identity.target_ir_coordinate != TARGET_IR_COORDINATE
            || identity.target_ir_digest_domain != TARGET_IR_DIGEST_DOMAIN
            || identity.target_ir_digest != TARGET_IR_DIGEST
        {
            return Err(BindingMismatchKind::TargetIr);
        }
        if identity.target_profile_coordinate != TARGET_PROFILE_COORDINATE
            || identity.target_profile_digest_domain != TARGET_PROFILE_DIGEST_DOMAIN
            || identity.target_profile_digest != TARGET_PROFILE_DIGEST
        {
            return Err(BindingMismatchKind::TargetProfile);
        }
        if identity.target_bundle_profile_coordinate != TARGET_BUNDLE_PROFILE_COORDINATE
            || identity.target_bundle_profile_digest_domain
                != TARGET_BUNDLE_PROFILE_DIGEST_DOMAIN
            || identity.target_bundle_profile_digest != TARGET_BUNDLE_PROFILE_DIGEST
        {
            return Err(BindingMismatchKind::TargetBundleProfile);
        }
        if identity.echo_contract_abi_version != ECHO_CONTRACT_ABI_VERSION {
            return Err(BindingMismatchKind::EchoAbi);
        }
        if identity.helper_api_version != CONTRACT_HOST_HELPER_API_VERSION {
            return Err(BindingMismatchKind::HelperApi);
        }
        if identity.provider_schema_coordinate != PROVIDER_SCHEMA_COORDINATE
            || identity.provider_schema_sha256_hex != PROVIDER_SCHEMA_SHA256_HEX
            || identity.input_schema != INPUT_SCHEMA
            || identity.output_schema != OUTPUT_SCHEMA
            || identity.type_schema_domain != TYPE_SCHEMA_DOMAIN
            || identity.obstruction_coordinate != OBSTRUCTION_COORDINATE
            || identity.obstruction_domain != OBSTRUCTION_DOMAIN
            || identity.effect_failure_schema != EFFECT_FAILURE_SCHEMA
            || identity.obstruction_payload_schema != OBSTRUCTION_PAYLOAD_SCHEMA
        {
            return Err(BindingMismatchKind::Schema);
        }
        if identity.generated_artifact_profile != GENERATED_ARTIFACT_PROFILE
            || identity.generated_artifact_profile_digest_domain
                != GENERATED_ARTIFACT_PROFILE_DIGEST_DOMAIN
            || identity.generated_artifact_profile_digest
                != GENERATED_ARTIFACT_PROFILE_DIGEST
        {
            return Err(BindingMismatchKind::GeneratedArtifactProfile);
        }
        if identity.operation_profile != OPERATION_PROFILE
            || identity.operation_profile_domain != OPERATION_PROFILE_DOMAIN
            || identity.operation_profiles_coordinate != OPERATION_PROFILES_COORDINATE
            || identity.operation_profiles_digest_domain != OPERATION_PROFILES_DIGEST_DOMAIN
            || identity.operation_profiles_digest != OPERATION_PROFILES_DIGEST
        {
            return Err(BindingMismatchKind::OperationProfile);
        }
        if identity.footprint_obligation != FOOTPRINT_OBLIGATION
            || identity.footprint_algebra != FOOTPRINT_ALGEBRA
            || identity.footprint_algebra_digest_domain != FOOTPRINT_ALGEBRA_DIGEST_DOMAIN
            || identity.footprint_algebra_digest != FOOTPRINT_ALGEBRA_DIGEST
        {
            return Err(BindingMismatchKind::Footprint);
        }
        Ok(RegistrationDescriptorV1 {
            contract_bundle: *identity,
        })
    }

    fn is_sha256_review(value: &str) -> bool {
        let Some(hex) = value.strip_prefix("sha256:") else {
            return false;
        };
        hex.len() == 64
            && hex
                .as_bytes()
                .iter()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    }
}
