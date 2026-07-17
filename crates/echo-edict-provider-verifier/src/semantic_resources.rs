// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Closed-world admission for the exact generated semantic resources reviewed
//! by the first Echo provider verifier.

use std::fmt::Write as _;

use echo_edict_canonical::{decode_canonical_cbor_v1, digest_canonical_value_v1, CanonicalValueV1};

use super::DIAGNOSTIC_ABI;

const PROFILE: ResourceSpec = ResourceSpec {
    coordinate: "echo.dpo@1",
    domain: "edict.target-profile/v1",
    bytes: include_bytes!("../resources/target-profile.echo-dpo.cbor"),
    framed_sha256: "ad7f10e1843f4b3d2c08b11d69df103f9c0b1b7388ae26bb364cc87106cd419e",
};
const LAWPACK: ResourceSpec = ResourceSpec {
    coordinate: "echo.dpo-lawpack@1",
    domain: "edict.lawpack/v1",
    bytes: include_bytes!("../resources/lawpack.echo-dpo.cbor"),
    framed_sha256: "2a1631ae5fe2e11d09ccdfb94e3498f3236a3d615ee217e6cd5843efcd284987",
};
const GENERATED_PROFILE: ResourceSpec = ResourceSpec {
    coordinate: "echo.dpo.registration/v1",
    domain: "echo.generated-artifact-profile/v1",
    bytes: include_bytes!("../resources/generated-artifact-profile.echo-dpo-registration.cbor"),
    framed_sha256: "ff88be93c26cc533948d8a93601954dc391912d593ca1e96115c846cbf2c5b5d",
};
const LAWPACK_EXPORTS: ResourceSpec = ResourceSpec {
    coordinate: "echo.dpo-lawpack.exports@1",
    domain: "echo.dpo-lawpack.exports@1",
    bytes: include_bytes!("../resources/resource.lawpack-exports.cbor"),
    framed_sha256: "b08f2118b018e50bd7f72d77a820424e2a1b46a118c847875da2df973e17ab19",
};
const LAWPACK_ADAPTER: ResourceSpec = ResourceSpec {
    coordinate: "echo.dpo-lawpack.adapter.echo-dpo@1",
    domain: "echo.dpo-lawpack.adapter.echo-dpo@1",
    bytes: include_bytes!("../resources/resource.lawpack-target-adapter.cbor"),
    framed_sha256: "4d4206b6dffb4abb531b44e59345f5f1ac6825253e131b851e6430eeb3480283",
};
const LAWPACK_VERIFIER: ResourceSpec = ResourceSpec {
    coordinate: "echo.dpo-lawpack.verifier@1",
    domain: "echo.dpo-lawpack.verifier@1",
    bytes: include_bytes!("../resources/resource.lawpack-verifier.cbor"),
    framed_sha256: "bd01c4d86a5216cfc60cc69531e7640652d6c2404810e250462c2eae9ef35298",
};
const INTRINSICS: ResourceSpec = ResourceSpec {
    coordinate: "echo.dpo.intrinsics/v1",
    domain: "echo.dpo.intrinsics/v1",
    bytes: include_bytes!("../resources/resource.target-intrinsics.cbor"),
    framed_sha256: "fcf0500f386f4f8ca0003a93f4e63cb98a3480d1cf0b4557b5c726970532c2f5",
};
const FOOTPRINT: ResourceSpec = ResourceSpec {
    coordinate: "echo.dpo.footprint/v1",
    domain: "echo.dpo.footprint/v1",
    bytes: include_bytes!("../resources/resource.target-footprint-algebra.cbor"),
    framed_sha256: "f47bb65867e78099ddcfd6ae7af83870df8823f974a496a111ed94e5d785c769",
};
const COST: ResourceSpec = ResourceSpec {
    coordinate: "echo.dpo.cost/v1",
    domain: "echo.dpo.cost/v1",
    bytes: include_bytes!("../resources/resource.target-cost-algebra.cbor"),
    framed_sha256: "486940acd1d4bc15cea40db29347619a07202d9c0f1e63c4016c26bccf0b52a8",
};
const OBSTRUCTIONS: ResourceSpec = ResourceSpec {
    coordinate: "echo.dpo.obstructions/v1",
    domain: "echo.dpo.obstructions/v1",
    bytes: include_bytes!("../resources/resource.target-obstruction-taxonomy.cbor"),
    framed_sha256: "4f5a139d606053c3807f50710076ea9abeb0e611c12a530f5e5b4b31911c3e64",
};
const OPERATION_PROFILES: ResourceSpec = ResourceSpec {
    coordinate: "echo.dpo.operation-profiles/v1",
    domain: "echo.dpo.operation-profiles/v1",
    bytes: include_bytes!("../resources/resource.target-operation-profiles.cbor"),
    framed_sha256: "53256c51f6c817a77cc8694458bf9d3891abd15b9c94f79ca97d920d3c5f0416",
};
const VERIFIER: ResourceSpec = ResourceSpec {
    coordinate: "echo.dpo.verifier/v1",
    domain: "echo.dpo.verifier/v1",
    bytes: include_bytes!("../resources/resource.target-verifier-contract.cbor"),
    framed_sha256: "ca96b190728de3d668072ec1bd37d24e5197e7bd9bd54f70966d8c566b9b67f2",
};
const TARGET_IR: ResourceSpec = ResourceSpec {
    coordinate: "echo.span-ir/v1",
    domain: "echo.span-ir/v1",
    bytes: include_bytes!("../resources/resource.target-ir.cbor"),
    framed_sha256: "0057167e68f50c99dcce087b3e1cd677d17c5d1dc238bdb52d89469e1472fc2f",
};

const OPERATION: &str = "a.b@1.t";
const EFFECT: &str = "target.replace";
const CAPABILITY: &str = "echo.dpo@1.replace";
const FOOTPRINT_OBLIGATION: &str = "target.replace.footprint";
const COST_OBLIGATION: &str = "target.replace.cost";
const FAILURE: &str = "rejected";
const FAILURE_PAYLOAD: &str = "target.replace.rejected";
const DOMAIN_OBSTRUCTION: &str = "domain.WriteRejected";
const OPERATION_PROFILE: &str = "continuum.profile.write/v1";
const OPTIC_CONTRACT: &str = "replace-point";

#[derive(Clone, Copy)]
struct ResourceSpec {
    coordinate: &'static str,
    domain: &'static str,
    bytes: &'static [u8],
    framed_sha256: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ReviewedSemanticClosure;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SemanticResourceErrorKind {
    InvalidArtifact,
    ReferenceMismatch,
    SemanticMismatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct SemanticResourceError {
    kind: SemanticResourceErrorKind,
    subject: &'static str,
}

impl SemanticResourceError {
    pub(super) const fn kind(self) -> SemanticResourceErrorKind {
        self.kind
    }

    pub(super) const fn subject(self) -> &'static str {
        self.subject
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ResourceSet {
    profile: CanonicalValueV1,
    lawpack: CanonicalValueV1,
    generated_profile: CanonicalValueV1,
    lawpack_exports: CanonicalValueV1,
    lawpack_adapter: CanonicalValueV1,
    lawpack_verifier: CanonicalValueV1,
    intrinsics: CanonicalValueV1,
    footprint: CanonicalValueV1,
    cost: CanonicalValueV1,
    obstructions: CanonicalValueV1,
    operation_profiles: CanonicalValueV1,
    verifier: CanonicalValueV1,
    target_ir: CanonicalValueV1,
}

pub(super) fn admit_packaged_semantic_resources(
) -> Result<ReviewedSemanticClosure, SemanticResourceError> {
    let resources = ResourceSet::decode_packaged()?;
    resources.validate_references()?;
    resources.validate_semantics()?;
    Ok(ReviewedSemanticClosure)
}

impl ResourceSet {
    fn decode_packaged() -> Result<Self, SemanticResourceError> {
        Ok(Self {
            profile: decode_pinned(PROFILE)?,
            lawpack: decode_pinned(LAWPACK)?,
            generated_profile: decode_pinned(GENERATED_PROFILE)?,
            lawpack_exports: decode_pinned(LAWPACK_EXPORTS)?,
            lawpack_adapter: decode_pinned(LAWPACK_ADAPTER)?,
            lawpack_verifier: decode_pinned(LAWPACK_VERIFIER)?,
            intrinsics: decode_pinned(INTRINSICS)?,
            footprint: decode_pinned(FOOTPRINT)?,
            cost: decode_pinned(COST)?,
            obstructions: decode_pinned(OBSTRUCTIONS)?,
            operation_profiles: decode_pinned(OPERATION_PROFILES)?,
            verifier: decode_pinned(VERIFIER)?,
            target_ir: decode_pinned(TARGET_IR)?,
        })
    }

    fn validate_references(&self) -> Result<(), SemanticResourceError> {
        assert_ref(
            field(&self.profile, "targetIr", "target-profile.targetIr")?,
            TARGET_IR,
            &self.target_ir,
            "target-profile.targetIr",
        )?;
        assert_external_ref(
            field(
                &self.profile,
                "diagnosticAbi",
                "target-profile.diagnosticAbi",
            )?,
            DIAGNOSTIC_ABI.coordinate,
            &DIAGNOSTIC_ABI.framed_sha256,
            "target-profile.diagnosticAbi",
        )?;
        assert_ref(
            field(&self.profile, "intrinsics", "target-profile.intrinsics")?,
            INTRINSICS,
            &self.intrinsics,
            "target-profile.intrinsics",
        )?;
        assert_ref(
            field(
                &self.profile,
                "footprintAlgebra",
                "target-profile.footprintAlgebra",
            )?,
            FOOTPRINT,
            &self.footprint,
            "target-profile.footprintAlgebra",
        )?;
        assert_ref(
            field(&self.profile, "costAlgebra", "target-profile.costAlgebra")?,
            COST,
            &self.cost,
            "target-profile.costAlgebra",
        )?;
        assert_ref(
            field(
                &self.profile,
                "obstructionTaxonomy",
                "target-profile.obstructionTaxonomy",
            )?,
            OBSTRUCTIONS,
            &self.obstructions,
            "target-profile.obstructionTaxonomy",
        )?;
        assert_ref(
            field(
                &self.profile,
                "operationProfiles",
                "target-profile.operationProfiles",
            )?,
            OPERATION_PROFILES,
            &self.operation_profiles,
            "target-profile.operationProfiles",
        )?;
        assert_ref(
            field(&self.profile, "verifier", "target-profile.verifier")?,
            VERIFIER,
            &self.verifier,
            "target-profile.verifier",
        )?;
        let generated = exact_array(
            field(
                &self.profile,
                "generatedArtifactProfiles",
                "target-profile.generatedArtifactProfiles",
            )?,
            1,
            "target-profile.generatedArtifactProfiles",
        )?;
        assert_ref(
            &generated[0],
            GENERATED_PROFILE,
            &self.generated_profile,
            "target-profile.generatedArtifactProfiles[0]",
        )?;

        assert_ref(
            field(&self.lawpack, "exports", "lawpack.exports")?,
            LAWPACK_EXPORTS,
            &self.lawpack_exports,
            "lawpack.exports",
        )?;
        assert_ref(
            field(
                field(&self.lawpack, "verifier", "lawpack.verifier")?,
                "ruleset",
                "lawpack.verifier.ruleset",
            )?,
            LAWPACK_VERIFIER,
            &self.lawpack_verifier,
            "lawpack.verifier.ruleset",
        )?;
        let adapters = exact_array(
            field(&self.lawpack, "targetAdapters", "lawpack.targetAdapters")?,
            1,
            "lawpack.targetAdapters",
        )?;
        let adapter = &adapters[0];
        assert_ref(
            field(adapter, "adapter", "lawpack.targetAdapters[0].adapter")?,
            LAWPACK_ADAPTER,
            &self.lawpack_adapter,
            "lawpack.targetAdapters[0].adapter",
        )?;
        assert_ref(
            field(
                adapter,
                "acceptedTargetProfile",
                "lawpack.targetAdapters[0].acceptedTargetProfile",
            )?,
            PROFILE,
            &self.profile,
            "lawpack.targetAdapters[0].acceptedTargetProfile",
        )?;
        assert_ref(
            field(
                adapter,
                "acceptedTargetIr",
                "lawpack.targetAdapters[0].acceptedTargetIr",
            )?,
            TARGET_IR,
            &self.target_ir,
            "lawpack.targetAdapters[0].acceptedTargetIr",
        )?;
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn validate_semantics(&self) -> Result<(), SemanticResourceError> {
        expect_text(
            field(&self.profile, "apiVersion", "target-profile.apiVersion")?,
            PROFILE.domain,
            "target-profile.apiVersion",
        )?;
        expect_text(
            field(&self.profile, "id", "target-profile.id")?,
            "echo.dpo",
            "target-profile.id",
        )?;
        expect_text(
            field(
                &self.profile,
                "intrinsicNamespace",
                "target-profile.intrinsicNamespace",
            )?,
            PROFILE.coordinate,
            "target-profile.intrinsicNamespace",
        )?;
        expect_text(
            field(
                &self.profile,
                "applicationModel",
                "target-profile.applicationModel",
            )?,
            "atomic",
            "target-profile.applicationModel",
        )?;
        expect_text(
            field(
                &self.profile,
                "readConsistency",
                "target-profile.readConsistency",
            )?,
            "application-snapshot",
            "target-profile.readConsistency",
        )?;
        expect_text(
            field(
                &self.profile,
                "guardEvaluation",
                "target-profile.guardEvaluation",
            )?,
            "precommit-atomic",
            "target-profile.guardEvaluation",
        )?;
        expect_text(
            field(
                &self.profile,
                "obstructionRollback",
                "target-profile.obstructionRollback",
            )?,
            "no-visible-effects",
            "target-profile.obstructionRollback",
        )?;
        expect_bool(
            field(&self.profile, "multiTarget", "target-profile.multiTarget")?,
            false,
            "target-profile.multiTarget",
        )?;
        expect_bool(
            field(
                &self.profile,
                "postconditionSupport",
                "target-profile.postconditionSupport",
            )?,
            true,
            "target-profile.postconditionSupport",
        )?;
        let accepted_core = exact_array(
            field(
                &self.profile,
                "acceptedCoreAbi",
                "target-profile.acceptedCoreAbi",
            )?,
            1,
            "target-profile.acceptedCoreAbi",
        )?;
        expect_text(
            &accepted_core[0],
            "edict.core/v1",
            "target-profile.acceptedCoreAbi[0]",
        )?;

        let lawpack_core = exact_array(
            field(&self.lawpack, "acceptedCoreAbi", "lawpack.acceptedCoreAbi")?,
            1,
            "lawpack.acceptedCoreAbi",
        )?;
        expect_text(
            &lawpack_core[0],
            "edict.core/v1",
            "lawpack.acceptedCoreAbi[0]",
        )?;
        exact_array(
            field(&self.lawpack, "dependencies", "lawpack.dependencies")?,
            0,
            "lawpack.dependencies",
        )?;

        expect_text(
            field(
                &self.generated_profile,
                "apiVersion",
                "generated.apiVersion",
            )?,
            GENERATED_PROFILE.domain,
            "generated.apiVersion",
        )?;
        expect_text(
            field(
                &self.generated_profile,
                "targetProfile",
                "generated.targetProfile",
            )?,
            PROFILE.coordinate,
            "generated.targetProfile",
        )?;
        validate_generated_types(&self.generated_profile)?;
        let operations = exact_map(
            field(
                &self.generated_profile,
                "operations",
                "generated.operations",
            )?,
            1,
            "generated.operations",
        )?;
        let operation = entry(operations, OPERATION, "generated.operations")?;
        expect_text(
            field(operation, "inputType", "generated.operation.inputType")?,
            "a.b@1.Input",
            "generated.operation.inputType",
        )?;
        expect_text(
            field(operation, "outputType", "generated.operation.outputType")?,
            "a.b@1.Output",
            "generated.operation.outputType",
        )?;
        expect_text(
            field(operation, "budget", "generated.operation.budget")?,
            "p.tiny",
            "generated.operation.budget",
        )?;
        expect_text(
            field(
                operation,
                "invocationKind",
                "generated.operation.invocationKind",
            )?,
            "mutation",
            "generated.operation.invocationKind",
        )?;
        expect_text(
            field(operation, "effect", "generated.operation.effect")?,
            EFFECT,
            "generated.operation.effect",
        )?;
        expect_text(
            field(
                field(
                    operation,
                    "implementation",
                    "generated.operation.implementation",
                )?,
                "coordinate",
                "generated.operation.implementation.coordinate",
            )?,
            CAPABILITY,
            "generated.operation.implementation.coordinate",
        )?;
        expect_text(
            field(
                field(
                    operation,
                    "implementation",
                    "generated.operation.implementation",
                )?,
                "kind",
                "generated.operation.implementation.kind",
            )?,
            "native",
            "generated.operation.implementation.kind",
        )?;
        expect_text(
            field(
                operation,
                "operationProfile",
                "generated.operation.operationProfile",
            )?,
            OPERATION_PROFILE,
            "generated.operation.operationProfile",
        )?;
        expect_text(
            field(
                operation,
                "opticContract",
                "generated.operation.opticContract",
            )?,
            OPTIC_CONTRACT,
            "generated.operation.opticContract",
        )?;
        let generated_mappings = exact_map(
            field(
                operation,
                "obstructionMappings",
                "generated.operation.obstructionMappings",
            )?,
            1,
            "generated.operation.obstructionMappings",
        )?;
        expect_text(
            entry(
                generated_mappings,
                FAILURE,
                "generated.operation.obstructionMappings",
            )?,
            DOMAIN_OBSTRUCTION,
            "generated.operation.obstructionMappings.rejected",
        )?;

        let exported_types = exact_array(
            field(&self.lawpack_exports, "types", "lawpack-exports.types")?,
            1,
            "lawpack-exports.types",
        )?;
        expect_text(
            field(
                &exported_types[0],
                "coordinate",
                "lawpack-exports.type.coordinate",
            )?,
            "a.b@1.Id",
            "lawpack-exports.type.coordinate",
        )?;
        expect_text(
            field(
                &exported_types[0],
                "definition",
                "lawpack-exports.type.definition",
            )?,
            "String<max=16,canonical=raw-utf8>",
            "lawpack-exports.type.definition",
        )?;
        exact_array(
            field(
                &self.lawpack_exports,
                "constants",
                "lawpack-exports.constants",
            )?,
            0,
            "lawpack-exports.constants",
        )?;
        exact_array(
            field(
                &self.lawpack_exports,
                "pureFunctions",
                "lawpack-exports.pureFunctions",
            )?,
            0,
            "lawpack-exports.pureFunctions",
        )?;
        exact_map(
            field(
                &self.lawpack_exports,
                "operationProfiles",
                "lawpack-exports.operationProfiles",
            )?,
            0,
            "lawpack-exports.operationProfiles",
        )?;
        let effects = exact_array(
            field(&self.lawpack_exports, "effects", "lawpack-exports.effects")?,
            1,
            "lawpack-exports.effects",
        )?;
        let exported_effect = &effects[0];
        expect_text(
            field(
                exported_effect,
                "coordinate",
                "lawpack-exports.effect.coordinate",
            )?,
            EFFECT,
            "lawpack-exports.effect.coordinate",
        )?;
        expect_text(
            field(
                exported_effect,
                "inputType",
                "lawpack-exports.effect.inputType",
            )?,
            "a.b@1.Id",
            "lawpack-exports.effect.inputType",
        )?;
        expect_text(
            field(
                exported_effect,
                "outputType",
                "lawpack-exports.effect.outputType",
            )?,
            "a.b@1.Receipt",
            "lawpack-exports.effect.outputType",
        )?;
        expect_text(
            field(
                exported_effect,
                "executionClass",
                "lawpack-exports.effect.executionClass",
            )?,
            "runtime",
            "lawpack-exports.effect.executionClass",
        )?;
        expect_text(
            field(
                exported_effect,
                "effectKindHint",
                "lawpack-exports.effect.effectKindHint",
            )?,
            "replace",
            "lawpack-exports.effect.effectKindHint",
        )?;
        expect_bool(
            field(
                exported_effect,
                "guardSupport",
                "lawpack-exports.effect.guardSupport",
            )?,
            true,
            "lawpack-exports.effect.guardSupport",
        )?;
        exact_array(
            field(
                exported_effect,
                "typeParameters",
                "lawpack-exports.effect.typeParameters",
            )?,
            0,
            "lawpack-exports.effect.typeParameters",
        )?;
        expect_text(
            field(
                exported_effect,
                "footprintObligation",
                "lawpack-exports.effect.footprintObligation",
            )?,
            FOOTPRINT_OBLIGATION,
            "lawpack-exports.effect.footprintObligation",
        )?;
        expect_text(
            field(
                exported_effect,
                "costObligation",
                "lawpack-exports.effect.costObligation",
            )?,
            COST_OBLIGATION,
            "lawpack-exports.effect.costObligation",
        )?;
        let exported_failures = exact_map(
            field(
                exported_effect,
                "effectFailures",
                "lawpack-exports.effect.effectFailures",
            )?,
            1,
            "lawpack-exports.effect.effectFailures",
        )?;
        let exported_failure = entry(
            exported_failures,
            FAILURE,
            "lawpack-exports.effect.effectFailures",
        )?;
        expect_text(
            field(
                exported_failure,
                "payloadType",
                "lawpack-exports.effect.failure.payloadType",
            )?,
            FAILURE_PAYLOAD,
            "lawpack-exports.effect.failure.payloadType",
        )?;
        expect_text(
            field(
                exported_failure,
                "authorityClass",
                "lawpack-exports.effect.failure.authorityClass",
            )?,
            "domainMappable",
            "lawpack-exports.effect.failure.authorityClass",
        )?;
        let exported_obstructions = exact_array(
            field(
                &self.lawpack_exports,
                "obstructions",
                "lawpack-exports.obstructions",
            )?,
            1,
            "lawpack-exports.obstructions",
        )?;
        expect_text(
            field(
                &exported_obstructions[0],
                "coordinate",
                "lawpack-exports.obstruction.coordinate",
            )?,
            DOMAIN_OBSTRUCTION,
            "lawpack-exports.obstruction.coordinate",
        )?;
        expect_text(
            field(
                &exported_obstructions[0],
                "payloadSchema",
                "lawpack-exports.obstruction.payloadSchema",
            )?,
            "domain.WriteRejected.Payload",
            "lawpack-exports.obstruction.payloadSchema",
        )?;
        expect_text(
            field(
                &exported_obstructions[0],
                "authorityClass",
                "lawpack-exports.obstruction.authorityClass",
            )?,
            "domainMappable",
            "lawpack-exports.obstruction.authorityClass",
        )?;

        expect_text(
            field(
                &self.lawpack_adapter,
                "targetProfile",
                "lawpack-adapter.targetProfile",
            )?,
            PROFILE.coordinate,
            "lawpack-adapter.targetProfile",
        )?;
        expect_text(
            field(
                &self.lawpack_adapter,
                "targetIrDomain",
                "lawpack-adapter.targetIrDomain",
            )?,
            TARGET_IR.domain,
            "lawpack-adapter.targetIrDomain",
        )?;
        let implementations = exact_map(
            field(
                &self.lawpack_adapter,
                "effectImplementations",
                "lawpack-adapter.effectImplementations",
            )?,
            1,
            "lawpack-adapter.effectImplementations",
        )?;
        let implementation = entry(
            implementations,
            EFFECT,
            "lawpack-adapter.effectImplementations",
        )?;
        expect_text(
            field(
                implementation,
                "capability",
                "lawpack-adapter.effect.capability",
            )?,
            CAPABILITY,
            "lawpack-adapter.effect.capability",
        )?;
        expect_text(
            field(implementation, "kind", "lawpack-adapter.effect.kind")?,
            "native",
            "lawpack-adapter.effect.kind",
        )?;
        expect_text(
            field(
                implementation,
                "writeClass",
                "lawpack-adapter.effect.writeClass",
            )?,
            "replace",
            "lawpack-adapter.effect.writeClass",
        )?;

        expect_text(
            field(
                &self.intrinsics,
                "apiVersion",
                "target-intrinsics.apiVersion",
            )?,
            "edict.target-profile.intrinsics/v1",
            "target-intrinsics.apiVersion",
        )?;
        let intrinsic_map = exact_map(
            field(
                &self.intrinsics,
                "intrinsics",
                "target-intrinsics.intrinsics",
            )?,
            1,
            "target-intrinsics.intrinsics",
        )?;
        let intrinsic = entry(intrinsic_map, CAPABILITY, "target-intrinsics.intrinsics")?;
        expect_text(
            field(
                intrinsic,
                "intrinsicClass",
                "target-intrinsics.intrinsic.intrinsicClass",
            )?,
            "effect",
            "target-intrinsics.intrinsic.intrinsicClass",
        )?;
        let argument_types = exact_array(
            field(
                intrinsic,
                "argumentTypes",
                "target-intrinsics.intrinsic.argumentTypes",
            )?,
            1,
            "target-intrinsics.intrinsic.argumentTypes",
        )?;
        expect_text(
            &argument_types[0],
            "a.b@1.Id",
            "target-intrinsics.intrinsic.argumentTypes[0]",
        )?;
        expect_text(
            field(
                intrinsic,
                "returnType",
                "target-intrinsics.intrinsic.returnType",
            )?,
            "a.b@1.Receipt",
            "target-intrinsics.intrinsic.returnType",
        )?;
        expect_text(
            field(
                intrinsic,
                "effectKind",
                "target-intrinsics.intrinsic.effectKind",
            )?,
            "replace",
            "target-intrinsics.intrinsic.effectKind",
        )?;
        expect_text(
            field(
                intrinsic,
                "writeClass",
                "target-intrinsics.intrinsic.writeClass",
            )?,
            "replace",
            "target-intrinsics.intrinsic.writeClass",
        )?;
        expect_bool(
            field(
                intrinsic,
                "guardSupport",
                "target-intrinsics.intrinsic.guardSupport",
            )?,
            true,
            "target-intrinsics.intrinsic.guardSupport",
        )?;
        expect_bool(
            field(
                intrinsic,
                "canParticipateInAtomicGuard",
                "target-intrinsics.intrinsic.canParticipateInAtomicGuard",
            )?,
            true,
            "target-intrinsics.intrinsic.canParticipateInAtomicGuard",
        )?;
        exact_array(
            field(
                intrinsic,
                "typeParameters",
                "target-intrinsics.intrinsic.typeParameters",
            )?,
            0,
            "target-intrinsics.intrinsic.typeParameters",
        )?;
        expect_text(
            field(
                intrinsic,
                "footprintTemplate",
                "target-intrinsics.intrinsic.footprintTemplate",
            )?,
            FOOTPRINT_OBLIGATION,
            "target-intrinsics.intrinsic.footprintTemplate",
        )?;
        expect_text(
            field(
                intrinsic,
                "costTemplate",
                "target-intrinsics.intrinsic.costTemplate",
            )?,
            COST_OBLIGATION,
            "target-intrinsics.intrinsic.costTemplate",
        )?;
        let intrinsic_failures = exact_map(
            field(
                intrinsic,
                "effectFailures",
                "target-intrinsics.intrinsic.effectFailures",
            )?,
            1,
            "target-intrinsics.intrinsic.effectFailures",
        )?;
        let intrinsic_failure = entry(
            intrinsic_failures,
            FAILURE,
            "target-intrinsics.intrinsic.effectFailures",
        )?;
        expect_text(
            field(
                intrinsic_failure,
                "payloadType",
                "target-intrinsics.intrinsic.failure.payloadType",
            )?,
            FAILURE_PAYLOAD,
            "target-intrinsics.intrinsic.failure.payloadType",
        )?;
        expect_text(
            field(
                intrinsic_failure,
                "authorityClass",
                "target-intrinsics.intrinsic.failure.authorityClass",
            )?,
            "domainMappable",
            "target-intrinsics.intrinsic.failure.authorityClass",
        )?;

        validate_algebra(
            &self.footprint,
            "footprintTemplate",
            FOOTPRINT_OBLIGATION,
            Some("replace"),
            FOOTPRINT.domain,
            "target-footprint",
        )?;
        validate_algebra(
            &self.cost,
            "costTemplate",
            COST_OBLIGATION,
            None,
            COST.domain,
            "target-cost",
        )?;

        let profiles = exact_map(
            field(
                &self.operation_profiles,
                "profiles",
                "target-operation-profiles.profiles",
            )?,
            1,
            "target-operation-profiles.profiles",
        )?;
        expect_text(
            field(
                &self.operation_profiles,
                "apiVersion",
                "target-operation-profiles.apiVersion",
            )?,
            "edict.target-profile.operation-profiles/v1",
            "target-operation-profiles.apiVersion",
        )?;
        let profile = entry(
            profiles,
            OPERATION_PROFILE,
            "target-operation-profiles.profiles",
        )?;
        expect_text(
            field(
                profile,
                "effectPredicate",
                "target-operation-profile.effectPredicate",
            )?,
            "echo.dpo.operation-mode.replace-only/v1",
            "target-operation-profile.effectPredicate",
        )?;
        let optic_template = field(
            profile,
            "opticTemplate",
            "target-operation-profile.opticTemplate",
        )?;
        expect_text(
            field(
                optic_template,
                "opticKind",
                "target-operation-profile.opticTemplate.opticKind",
            )?,
            "affectReintegration",
            "target-operation-profile.opticTemplate.opticKind",
        )?;
        expect_text(
            field(
                optic_template,
                "boundaryKind",
                "target-operation-profile.opticTemplate.boundaryKind",
            )?,
            "affect",
            "target-operation-profile.opticTemplate.boundaryKind",
        )?;
        expect_text(
            field(
                optic_template,
                "supportPolicy",
                "target-operation-profile.opticTemplate.supportPolicy",
            )?,
            "continuum.support.carry-or-obstruct/v1",
            "target-operation-profile.opticTemplate.supportPolicy",
        )?;
        expect_text(
            field(
                optic_template,
                "lossDisposition",
                "target-operation-profile.opticTemplate.lossDisposition",
            )?,
            "continuum.support.reject-on-loss/v1",
            "target-operation-profile.opticTemplate.lossDisposition",
        )?;
        let aperture = field(
            optic_template,
            "apertureRequirement",
            "target-operation-profile.apertureRequirement",
        )?;
        expect_text(
            field(aperture, "kind", "target-operation-profile.aperture.kind")?,
            "abstractFootprintObligation",
            "target-operation-profile.aperture.kind",
        )?;
        expect_text(
            field(aperture, "ref", "target-operation-profile.aperture.ref")?,
            FOOTPRINT_OBLIGATION,
            "target-operation-profile.aperture.ref",
        )?;

        expect_text(
            field(&self.verifier, "class", "target-verifier.class")?,
            "declarative",
            "target-verifier.class",
        )?;
        expect_text(
            field(&self.verifier, "apiVersion", "target-verifier.apiVersion")?,
            VERIFIER.domain,
            "target-verifier.apiVersion",
        )?;
        expect_text(
            field(
                &self.verifier,
                "targetProfile",
                "target-verifier.targetProfile",
            )?,
            PROFILE.coordinate,
            "target-verifier.targetProfile",
        )?;
        expect_text(
            field(
                &self.verifier,
                "targetIrDomain",
                "target-verifier.targetIrDomain",
            )?,
            TARGET_IR.domain,
            "target-verifier.targetIrDomain",
        )?;
        assert_null_set(
            field(
                &self.verifier,
                "capabilities",
                "target-verifier.capabilities",
            )?,
            CAPABILITY,
            "target-verifier.capabilities",
        )?;
        assert_null_set(
            field(
                &self.verifier,
                "operationProfiles",
                "target-verifier.operationProfiles",
            )?,
            OPERATION_PROFILE,
            "target-verifier.operationProfiles",
        )?;
        let optic_contracts = exact_map(
            field(
                &self.verifier,
                "opticContracts",
                "target-verifier.opticContracts",
            )?,
            1,
            "target-verifier.opticContracts",
        )?;
        expect_text(
            entry(
                optic_contracts,
                OPERATION_PROFILE,
                "target-verifier.opticContracts",
            )?,
            OPTIC_CONTRACT,
            "target-verifier.opticContracts.write",
        )?;

        expect_text(
            field(&self.target_ir, "class", "target-ir.class")?,
            "declarative",
            "target-ir.class",
        )?;
        expect_text(
            field(&self.target_ir, "apiVersion", "target-ir.apiVersion")?,
            TARGET_IR.domain,
            "target-ir.apiVersion",
        )?;
        assert_null_set(
            field(&self.target_ir, "capabilities", "target-ir.capabilities")?,
            CAPABILITY,
            "target-ir.capabilities",
        )?;
        expect_text(
            field(&self.target_ir, "targetProfile", "target-ir.targetProfile")?,
            PROFILE.coordinate,
            "target-ir.targetProfile",
        )?;
        expect_text(
            field(&self.target_ir, "domain", "target-ir.domain")?,
            TARGET_IR.domain,
            "target-ir.domain",
        )?;

        expect_text(
            field(&self.lawpack_verifier, "class", "lawpack-verifier.class")?,
            "declarative",
            "lawpack-verifier.class",
        )?;
        expect_text(
            field(
                &self.lawpack_verifier,
                "apiVersion",
                "lawpack-verifier.apiVersion",
            )?,
            "echo.edict-provider.lawpack-verifier/v1",
            "lawpack-verifier.apiVersion",
        )?;
        let operation_obstructions = exact_map(
            field(
                &self.lawpack_verifier,
                "operationObstructions",
                "lawpack-verifier.operationObstructions",
            )?,
            1,
            "lawpack-verifier.operationObstructions",
        )?;
        let obstruction_rule = entry(
            operation_obstructions,
            OPERATION,
            "lawpack-verifier.operationObstructions",
        )?;
        expect_text(
            field(
                obstruction_rule,
                "effect",
                "lawpack-verifier.operation.effect",
            )?,
            EFFECT,
            "lawpack-verifier.operation.effect",
        )?;
        let failure_mappings = exact_map(
            field(
                obstruction_rule,
                "failureMappings",
                "lawpack-verifier.operation.failureMappings",
            )?,
            1,
            "lawpack-verifier.operation.failureMappings",
        )?;
        expect_text(
            entry(
                failure_mappings,
                FAILURE,
                "lawpack-verifier.operation.failureMappings",
            )?,
            DOMAIN_OBSTRUCTION,
            "lawpack-verifier.operation.failureMappings.rejected",
        )?;

        expect_text(
            field(&self.obstructions, "class", "target-obstructions.class")?,
            "declarative",
            "target-obstructions.class",
        )?;
        expect_text(
            field(
                &self.obstructions,
                "apiVersion",
                "target-obstructions.apiVersion",
            )?,
            OBSTRUCTIONS.domain,
            "target-obstructions.apiVersion",
        )?;
        let taxonomy_failures = exact_map(
            field(
                &self.obstructions,
                "effectFailures",
                "target-obstructions.effectFailures",
            )?,
            1,
            "target-obstructions.effectFailures",
        )?;
        let taxonomy_failure = entry(
            taxonomy_failures,
            FAILURE_PAYLOAD,
            "target-obstructions.effectFailures",
        )?;
        expect_text(
            field(
                taxonomy_failure,
                "payloadType",
                "target-obstructions.effectFailure.payloadType",
            )?,
            FAILURE_PAYLOAD,
            "target-obstructions.effectFailure.payloadType",
        )?;
        expect_text(
            field(
                taxonomy_failure,
                "authorityClass",
                "target-obstructions.effectFailure.authorityClass",
            )?,
            "domainMappable",
            "target-obstructions.effectFailure.authorityClass",
        )?;
        let domain_obstructions = exact_map(
            field(
                &self.obstructions,
                "domainObstructions",
                "target-obstructions.domainObstructions",
            )?,
            1,
            "target-obstructions.domainObstructions",
        )?;
        let domain_obstruction = entry(
            domain_obstructions,
            DOMAIN_OBSTRUCTION,
            "target-obstructions.domainObstructions",
        )?;
        expect_text(
            field(
                domain_obstruction,
                "payloadSchema",
                "target-obstructions.domainObstruction.payloadSchema",
            )?,
            "domain.WriteRejected.Payload",
            "target-obstructions.domainObstruction.payloadSchema",
        )?;
        expect_text(
            field(
                domain_obstruction,
                "authorityClass",
                "target-obstructions.domainObstruction.authorityClass",
            )?,
            "domainMappable",
            "target-obstructions.domainObstruction.authorityClass",
        )?;
        Ok(())
    }
}

fn decode_pinned(spec: ResourceSpec) -> Result<CanonicalValueV1, SemanticResourceError> {
    let value = decode_canonical_cbor_v1(spec.bytes).map_err(|_| invalid(spec.coordinate))?;
    let digest =
        digest_canonical_value_v1(spec.domain, &value).map_err(|_| invalid(spec.coordinate))?;
    if digest != format!("sha256:{}", spec.framed_sha256) {
        return Err(invalid(spec.coordinate));
    }
    Ok(value)
}

fn assert_ref(
    reference: &CanonicalValueV1,
    spec: ResourceSpec,
    resource: &CanonicalValueV1,
    subject: &'static str,
) -> Result<(), SemanticResourceError> {
    let map = super::as_map(reference).ok_or_else(|| reference_mismatch(subject))?;
    if map.len() != 2 {
        return Err(reference_mismatch(subject));
    }
    let id = super::map_field(reference, "id")
        .and_then(super::as_text)
        .ok_or_else(|| reference_mismatch(subject))?;
    if id != spec.coordinate {
        return Err(reference_mismatch(subject));
    }
    let digest = match super::map_field(reference, "digest") {
        Some(CanonicalValueV1::Array(digest)) if digest.len() == 2 => digest,
        _ => return Err(reference_mismatch(subject)),
    };
    if super::as_text(&digest[0]) != Some("sha256") {
        return Err(reference_mismatch(subject));
    }
    let CanonicalValueV1::Bytes(bytes) = &digest[1] else {
        return Err(reference_mismatch(subject));
    };
    if bytes.len() != 32 {
        return Err(reference_mismatch(subject));
    }
    let expected =
        digest_canonical_value_v1(spec.domain, resource).map_err(|_| invalid(subject))?;
    let mut actual = String::with_capacity(71);
    actual.push_str("sha256:");
    for byte in bytes {
        write!(&mut actual, "{byte:02x}").map_err(|_| invalid(subject))?;
    }
    if actual != expected {
        return Err(reference_mismatch(subject));
    }
    Ok(())
}

fn assert_external_ref(
    reference: &CanonicalValueV1,
    coordinate: &str,
    framed_sha256: &[u8; 32],
    subject: &'static str,
) -> Result<(), SemanticResourceError> {
    let map = super::as_map(reference).ok_or_else(|| reference_mismatch(subject))?;
    if map.len() != 2
        || super::map_field(reference, "id").and_then(super::as_text) != Some(coordinate)
    {
        return Err(reference_mismatch(subject));
    }
    let digest = match super::map_field(reference, "digest") {
        Some(CanonicalValueV1::Array(digest)) if digest.len() == 2 => digest,
        _ => return Err(reference_mismatch(subject)),
    };
    if super::as_text(&digest[0]) != Some("sha256")
        || !matches!(&digest[1], CanonicalValueV1::Bytes(bytes) if bytes == framed_sha256)
    {
        return Err(reference_mismatch(subject));
    }
    Ok(())
}

fn validate_algebra(
    resource: &CanonicalValueV1,
    template_field: &'static str,
    obligation: &'static str,
    write_class: Option<&'static str>,
    api_version: &'static str,
    subject: &'static str,
) -> Result<(), SemanticResourceError> {
    expect_text(field(resource, "class", subject)?, "declarative", subject)?;
    expect_text(
        field(resource, "apiVersion", subject)?,
        api_version,
        subject,
    )?;
    let capabilities = exact_map(field(resource, "capabilities", subject)?, 1, subject)?;
    let capability = entry(capabilities, CAPABILITY, subject)?;
    expect_text(field(capability, "effect", subject)?, EFFECT, subject)?;
    expect_text(
        field(capability, template_field, subject)?,
        obligation,
        subject,
    )?;
    expect_text(
        field(capability, "semanticObligation", subject)?,
        obligation,
        subject,
    )?;
    if let Some(write_class) = write_class {
        expect_text(
            field(capability, "writeClass", subject)?,
            write_class,
            subject,
        )?;
    }
    Ok(())
}

fn validate_generated_types(
    generated_profile: &CanonicalValueV1,
) -> Result<(), SemanticResourceError> {
    let types = exact_map(
        field(generated_profile, "types", "generated.types")?,
        6,
        "generated.types",
    )?;
    let id = exact_map(
        entry(types, "a.b@1.Id", "generated.types")?,
        3,
        "generated.types.Id",
    )?;
    expect_text(
        entry(id, "kind", "generated.types.Id")?,
        "coreStringAlias",
        "generated.types.Id.kind",
    )?;
    expect_text(
        entry(id, "canonical", "generated.types.Id")?,
        "raw-utf8",
        "generated.types.Id.canonical",
    )?;
    expect_integer(
        entry(id, "maxScalarValues", "generated.types.Id")?,
        16,
        "generated.types.Id.maxScalarValues",
    )?;
    validate_generated_record(types, "a.b@1.Input", Some("a.b@1.Id"))?;
    validate_generated_record(types, "a.b@1.Output", Some("a.b@1.Id"))?;
    validate_generated_record(types, "a.b@1.Receipt", Some("a.b@1.Id"))?;
    validate_generated_record(types, FAILURE_PAYLOAD, None)?;
    validate_generated_record(types, "domain.WriteRejected.Payload", None)
}

fn validate_generated_record(
    types: &[(CanonicalValueV1, CanonicalValueV1)],
    coordinate: &'static str,
    field_type: Option<&'static str>,
) -> Result<(), SemanticResourceError> {
    let record = exact_map(entry(types, coordinate, coordinate)?, 2, coordinate)?;
    expect_text(entry(record, "kind", coordinate)?, "record", coordinate)?;
    let fields = exact_array(
        entry(record, "fields", coordinate)?,
        usize::from(field_type.is_some()),
        coordinate,
    )?;
    if let Some(field_type) = field_type {
        let field = exact_map(&fields[0], 2, coordinate)?;
        expect_text(entry(field, "name", coordinate)?, "id", coordinate)?;
        expect_text(entry(field, "type", coordinate)?, field_type, coordinate)?;
    }
    Ok(())
}

fn field<'a>(
    value: &'a CanonicalValueV1,
    name: &str,
    subject: &'static str,
) -> Result<&'a CanonicalValueV1, SemanticResourceError> {
    super::map_field(value, name).ok_or_else(|| invalid(subject))
}

fn exact_map<'a>(
    value: &'a CanonicalValueV1,
    len: usize,
    subject: &'static str,
) -> Result<&'a [(CanonicalValueV1, CanonicalValueV1)], SemanticResourceError> {
    let map = super::as_map(value).ok_or_else(|| invalid(subject))?;
    if map.len() != len {
        return Err(semantic_mismatch(subject));
    }
    Ok(map)
}

fn exact_array<'a>(
    value: &'a CanonicalValueV1,
    len: usize,
    subject: &'static str,
) -> Result<&'a Vec<CanonicalValueV1>, SemanticResourceError> {
    let CanonicalValueV1::Array(array) = value else {
        return Err(invalid(subject));
    };
    if array.len() != len {
        return Err(semantic_mismatch(subject));
    }
    Ok(array)
}

fn entry<'a>(
    map: &'a [(CanonicalValueV1, CanonicalValueV1)],
    key: &str,
    subject: &'static str,
) -> Result<&'a CanonicalValueV1, SemanticResourceError> {
    map.iter()
        .find_map(|(candidate, value)| (super::as_text(candidate) == Some(key)).then_some(value))
        .ok_or_else(|| semantic_mismatch(subject))
}

fn expect_text(
    value: &CanonicalValueV1,
    expected: &str,
    subject: &'static str,
) -> Result<(), SemanticResourceError> {
    let actual = super::as_text(value).ok_or_else(|| invalid(subject))?;
    if actual != expected {
        return Err(semantic_mismatch(subject));
    }
    Ok(())
}

fn expect_bool(
    value: &CanonicalValueV1,
    expected: bool,
    subject: &'static str,
) -> Result<(), SemanticResourceError> {
    let CanonicalValueV1::Bool(actual) = value else {
        return Err(invalid(subject));
    };
    if *actual != expected {
        return Err(semantic_mismatch(subject));
    }
    Ok(())
}

fn expect_integer(
    value: &CanonicalValueV1,
    expected: i128,
    subject: &'static str,
) -> Result<(), SemanticResourceError> {
    let CanonicalValueV1::Integer(actual) = value else {
        return Err(invalid(subject));
    };
    if *actual != expected {
        return Err(semantic_mismatch(subject));
    }
    Ok(())
}

fn assert_null_set(
    value: &CanonicalValueV1,
    expected: &str,
    subject: &'static str,
) -> Result<(), SemanticResourceError> {
    let map = exact_map(value, 1, subject)?;
    if !matches!(entry(map, expected, subject)?, CanonicalValueV1::Null) {
        return Err(invalid(subject));
    }
    Ok(())
}

const fn invalid(subject: &'static str) -> SemanticResourceError {
    SemanticResourceError {
        kind: SemanticResourceErrorKind::InvalidArtifact,
        subject,
    }
}

const fn reference_mismatch(subject: &'static str) -> SemanticResourceError {
    SemanticResourceError {
        kind: SemanticResourceErrorKind::ReferenceMismatch,
        subject,
    }
}

const fn semantic_mismatch(subject: &'static str) -> SemanticResourceError {
    SemanticResourceError {
        kind: SemanticResourceErrorKind::SemanticMismatch,
        subject,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use echo_edict_canonical::{digest_canonical_value_v1, encode_canonical_cbor_v1};
    use sha2::{Digest as _, Sha256};

    use super::*;

    #[test]
    fn packaged_semantic_resource_closure_is_admitted_deterministically() {
        let first = admit_packaged_semantic_resources()
            .expect("the exact packaged semantic resource closure is admitted");
        let second = admit_packaged_semantic_resources()
            .expect("repeated semantic resource admission succeeds");
        assert_eq!(first, second);
    }

    #[test]
    fn packaged_raw_and_domain_framed_identities_are_distinct_and_pinned() {
        let expected_raw = [
            (
                PROFILE,
                "cb5104802031e06d2e2802efe14ad23877dba2756684a5509c06a8de7bb9ec85",
            ),
            (
                LAWPACK,
                "679b090774088b46413a4697a646f10c3627d9f7f698380237db72e0b11739a1",
            ),
            (
                GENERATED_PROFILE,
                "4ef3aaad0d2131ba7129a3e2ae178d10c44f7e9b062af2c1fe211de781462f34",
            ),
            (
                LAWPACK_EXPORTS,
                "e80f3597830f30079bae7428cc6fe1ca27e9f2626614284338c5f5c7bfd9f8b8",
            ),
            (
                LAWPACK_ADAPTER,
                "d0cf6a5310c7aa3d5bf291e40221a50a9be676d0e7f9947b71de0c52f03f9a41",
            ),
            (
                LAWPACK_VERIFIER,
                "a0ca53486f2e1181027cab255792150735a68fc01a93669ab90b58e7c32bd36e",
            ),
            (
                INTRINSICS,
                "d4080b044ad95d88763f605d25d678dad0d44bbfdc32c212334a962eaf456588",
            ),
            (
                FOOTPRINT,
                "c08b853b22fa1920cb8971b60a6622e4d8f400c68b8027a9c4f246c5cb4a222c",
            ),
            (
                COST,
                "29cc941cc96ffdb420a24a377146f0e9d5409a89f84cdbb2ad597e78dfde3503",
            ),
            (
                OBSTRUCTIONS,
                "5eb85dc3fbb53890f4f5accf6a7a3c03889bd25bf00e7c2a23c43e497652328e",
            ),
            (
                OPERATION_PROFILES,
                "3d560e8df1bfb38e8ba9b70b0a4bc843c848a7900e3af76d8f798db51b6b1077",
            ),
            (
                VERIFIER,
                "9cef12fac8190156e152e7c8ab5adeed077bf2bdadf3add2225ce90ab4be8710",
            ),
            (
                TARGET_IR,
                "f0ead7a5dcb41532d5df9067a9735d7078456b59cc267b3a9f6bb6437431000c",
            ),
        ];
        for (spec, raw_sha256) in expected_raw {
            assert_eq!(hex::encode(Sha256::digest(spec.bytes)), raw_sha256);
            let value = decode_canonical_cbor_v1(spec.bytes).expect("resource is canonical");
            assert_eq!(
                digest_canonical_value_v1(spec.domain, &value)
                    .expect("resource has a framed digest"),
                format!("sha256:{}", spec.framed_sha256)
            );
            assert_ne!(raw_sha256, spec.framed_sha256);
        }
    }

    #[test]
    fn malformed_shape_is_distinct_from_semantic_and_reference_disagreement() {
        let malformed = expect_text(&CanonicalValueV1::Integer(1), "expected", "ordinary-field")
            .expect_err("wrong CBOR type is malformed");
        assert_eq!(malformed.kind(), SemanticResourceErrorKind::InvalidArtifact);

        let disagreement = expect_text(&text("other"), "expected", "ordinary-field")
            .expect_err("well-typed contradiction is semantic");
        assert_eq!(
            disagreement.kind(),
            SemanticResourceErrorKind::SemanticMismatch
        );

        let mut entries = vec![
            (text("id"), text(INTRINSICS.coordinate)),
            (
                text("digest"),
                CanonicalValueV1::Array(vec![text("sha512"), CanonicalValueV1::Bytes(vec![0; 32])]),
            ),
        ];
        entries.sort_by(|(left, _), (right, _)| left.cmp(right));
        let resource = decode_canonical_cbor_v1(INTRINSICS.bytes).expect("resource decodes");
        let reference = assert_ref(
            &CanonicalValueV1::Map(entries),
            INTRINSICS,
            &resource,
            "resource-reference",
        )
        .expect_err("wrong digest algorithm is a reference mismatch");
        assert_eq!(
            reference.kind(),
            SemanticResourceErrorKind::ReferenceMismatch
        );
    }

    #[test]
    fn diagnostic_abi_reference_cannot_diverge_from_the_report_identity() {
        let mut resources = ResourceSet::decode_packaged().expect("resources decode");
        set_ref_digest(
            field_mut(&mut resources.profile, "diagnosticAbi"),
            vec![0; 32],
        );
        let profile_digest = digest_bytes(PROFILE.domain, &resources.profile);
        let adapters = array_mut(field_mut(&mut resources.lawpack, "targetAdapters"));
        set_ref_digest(
            field_mut(&mut adapters[0], "acceptedTargetProfile"),
            profile_digest,
        );

        let error = resources
            .validate_references()
            .expect_err("a rebound diagnostic ABI must fail closed");
        assert_eq!(error.kind(), SemanticResourceErrorKind::ReferenceMismatch);
        assert_eq!(error.subject(), "target-profile.diagnosticAbi");
    }

    #[test]
    fn rebound_intrinsic_capability_cannot_cross_the_adapter_boundary() {
        let mut resources = ResourceSet::decode_packaged().expect("resources decode");
        rename_map_key(
            field_mut(&mut resources.intrinsics, "intrinsics"),
            CAPABILITY,
            "echo.dpo@1.other",
        );
        rebind_profile_field(&mut resources, "intrinsics", INTRINSICS.domain);
        let error = resources
            .validate_references()
            .and_then(|()| resources.validate_semantics())
            .expect_err("rebound but contradictory intrinsic must fail");
        assert_eq!(error.kind(), SemanticResourceErrorKind::SemanticMismatch);
        assert_eq!(error.subject(), "target-intrinsics.intrinsics");
    }

    #[test]
    fn rebound_footprint_template_cannot_cross_the_algebra_boundary() {
        let mut resources = ResourceSet::decode_packaged().expect("resources decode");
        let intrinsics = field_mut(&mut resources.intrinsics, "intrinsics");
        let intrinsic = field_mut(intrinsics, CAPABILITY);
        *field_mut(intrinsic, "footprintTemplate") = text("target.replace.other-footprint");
        rebind_profile_field(&mut resources, "intrinsics", INTRINSICS.domain);
        let error = resources
            .validate_references()
            .and_then(|()| resources.validate_semantics())
            .expect_err("rebound but contradictory footprint must fail");
        assert_eq!(error.kind(), SemanticResourceErrorKind::SemanticMismatch);
        assert_eq!(
            error.subject(),
            "target-intrinsics.intrinsic.footprintTemplate"
        );
    }

    #[test]
    fn rebound_obstruction_mapping_cannot_cross_the_taxonomy_boundary() {
        let mut resources = ResourceSet::decode_packaged().expect("resources decode");
        let operations = field_mut(&mut resources.lawpack_verifier, "operationObstructions");
        let operation = field_mut(operations, OPERATION);
        let mappings = field_mut(operation, "failureMappings");
        *field_mut(mappings, FAILURE) = text("domain.Other");
        rebind_lawpack_ruleset(&mut resources);
        let error = resources
            .validate_references()
            .and_then(|()| resources.validate_semantics())
            .expect_err("rebound but contradictory obstruction must fail");
        assert_eq!(error.kind(), SemanticResourceErrorKind::SemanticMismatch);
        assert_eq!(
            error.subject(),
            "lawpack-verifier.operation.failureMappings.rejected"
        );
    }

    fn rebind_profile_field(resources: &mut ResourceSet, name: &str, domain: &str) {
        let canonical = canonical_round_trip(resources.intrinsics.clone());
        resources.intrinsics = canonical;
        let digest = digest_bytes(domain, &resources.intrinsics);
        set_ref_digest(field_mut(&mut resources.profile, name), digest);
        let profile_digest = digest_bytes(PROFILE.domain, &resources.profile);
        let adapters = array_mut(field_mut(&mut resources.lawpack, "targetAdapters"));
        set_ref_digest(
            field_mut(&mut adapters[0], "acceptedTargetProfile"),
            profile_digest,
        );
    }

    fn rebind_lawpack_ruleset(resources: &mut ResourceSet) {
        resources.lawpack_verifier = canonical_round_trip(resources.lawpack_verifier.clone());
        let digest = digest_bytes(LAWPACK_VERIFIER.domain, &resources.lawpack_verifier);
        let verifier = field_mut(&mut resources.lawpack, "verifier");
        set_ref_digest(field_mut(verifier, "ruleset"), digest);
    }

    fn canonical_round_trip(value: CanonicalValueV1) -> CanonicalValueV1 {
        let bytes = encode_canonical_cbor_v1(&value).expect("mutated resource encodes");
        decode_canonical_cbor_v1(&bytes).expect("mutated resource remains canonical")
    }

    fn digest_bytes(domain: &str, value: &CanonicalValueV1) -> Vec<u8> {
        let digest = digest_canonical_value_v1(domain, value).expect("mutated resource digests");
        hex::decode(digest.strip_prefix("sha256:").expect("digest is SHA-256"))
            .expect("digest hexadecimal decodes")
    }

    fn set_ref_digest(reference: &mut CanonicalValueV1, bytes: Vec<u8>) {
        *field_mut(reference, "digest") =
            CanonicalValueV1::Array(vec![text("sha256"), CanonicalValueV1::Bytes(bytes)]);
    }

    fn rename_map_key(value: &mut CanonicalValueV1, old: &str, new: &str) {
        let CanonicalValueV1::Map(entries) = value else {
            panic!("expected map");
        };
        let (key, _) = entries
            .iter_mut()
            .find(|(key, _)| super::super::as_text(key) == Some(old))
            .expect("map key exists");
        *key = text(new);
        entries.sort_by(|(left, _), (right, _)| left.cmp(right));
    }

    fn field_mut<'a>(value: &'a mut CanonicalValueV1, name: &str) -> &'a mut CanonicalValueV1 {
        let CanonicalValueV1::Map(entries) = value else {
            panic!("expected map");
        };
        entries
            .iter_mut()
            .find_map(|(key, value)| (super::super::as_text(key) == Some(name)).then_some(value))
            .expect("map field exists")
    }

    fn array_mut(value: &mut CanonicalValueV1) -> &mut Vec<CanonicalValueV1> {
        let CanonicalValueV1::Array(values) = value else {
            panic!("expected array");
        };
        values
    }

    fn text(value: &str) -> CanonicalValueV1 {
        CanonicalValueV1::Text(value.to_owned())
    }
}
