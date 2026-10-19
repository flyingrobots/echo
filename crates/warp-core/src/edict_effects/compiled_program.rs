// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Current compiler-artifact binding and ordered instruction correspondence.
use crate::edict_pure::{
    values::{array, exact_fields, field, map, number, require_text, text, text_field},
    EvaluationError as Error,
};
use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, digest_canonical_value_bytes_v1 as digest,
    CanonicalValueV1 as Value,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CompiledProgram {
    module: Value,
    identity: [u8; 32],
    target: Value,
    target_identity: [u8; 32],
    exports_identity: [u8; 32],
    intent: String,
    exports: Value,
    lowering: Value,
}
impl CompiledProgram {
    pub(super) fn parse(bytes: &[u8]) -> Result<Self, Error> {
        let bundle = decode(bytes).map_err(|_| Error::InvalidArtifact)?;
        exact_fields(
            &bundle,
            &[
                "schema",
                "intent",
                "core_artifact",
                "target_ir_artifact",
                "lawpack_exports_artifact",
                "lowering_artifacts",
            ],
        )?;
        require_text(
            &bundle,
            "schema",
            "echo.compiler-produced-effect-program/v1",
        )?;
        let intent = text_field(&bundle, "intent")?.to_owned();
        if intent.is_empty() {
            return Err(Error::InvalidArtifact);
        }
        let embedded = |key| {
            decode(crate::edict_pure::values::bytes(field(&bundle, key)?)?)
                .map_err(|_| Error::InvalidArtifact)
        };
        let module = embedded("core_artifact")?;
        let target = embedded("target_ir_artifact")?;
        let exports = embedded("lawpack_exports_artifact")?;
        exact_fields(
            &target,
            &[
                "domain",
                "kind",
                "intents",
                "targetProfile",
                "semanticClosure",
                "sourceCoreCoordinate",
            ],
        )?;
        require_text(&target, "domain", "echo.span-ir/v2")?;
        require_text(&target, "kind", "orderedTargetIrArtifact")?;
        exact_fields(
            &exports,
            &[
                "types",
                "effects",
                "constants",
                "obstructions",
                "pureFunctions",
                "operationProfiles",
            ],
        )?;
        array(field(&exports, "types")?)?;
        map(field(&exports, "operationProfiles")?)?;
        for key in ["effects", "constants", "obstructions", "pureFunctions"] {
            array(field(&exports, key)?)?;
        }
        let target_identity =
            digest("edict.target-ir.artifact/v1", &target).map_err(|_| Error::InvalidArtifact)?;
        let exports_identity =
            digest("edict.lawpack-exports/v1", &exports).map_err(|_| Error::InvalidArtifact)?;
        let mut fields = vec![
            "apiVersion",
            "coordinate",
            "imports",
            "types",
            "intents",
            "requiredCoreCapabilities",
        ];
        if field(&module, "functions").is_ok() {
            fields.push("functions");
        }
        exact_fields(&module, &fields)?;
        require_text(&module, "apiVersion", "edict.core/v1")?;
        if text_field(&module, "coordinate")?.is_empty()
            || map(field(&module, "intents")?)?.is_empty()
        {
            return Err(Error::InvalidArtifact);
        }
        array(field(&module, "imports")?)?;
        map(field(&module, "types")?)?;
        if let Ok(functions) = field(&module, "functions") {
            if map(functions)?.is_empty() {
                return Err(Error::InvalidArtifact);
            }
        }
        for capability in array(field(&module, "requiredCoreCapabilities")?)? {
            text(capability)?;
        }
        let identity =
            digest("edict.core.module/v1", &module).map_err(|_| Error::InvalidArtifact)?;
        require_text(
            &target,
            "sourceCoreCoordinate",
            text_field(&module, "coordinate")?,
        )?;
        let source = field(field(&target, "semanticClosure")?, "sourceCore")?;
        exact_fields(source, &["id", "digest"])?;
        require_text(source, "id", text_field(&module, "coordinate")?)?;
        let [algorithm, claimed] = array(field(source, "digest")?)? else {
            return Err(Error::InvalidArtifact);
        };
        if text(algorithm)? != "sha256" || crate::edict_pure::values::bytes(claimed)? != identity {
            return Err(Error::InvalidArtifact);
        }
        Ok(Self {
            module,
            identity,
            target,
            target_identity,
            exports_identity,
            intent,
            exports,
            lowering: field(&bundle, "lowering_artifacts")?.clone(),
        })
    }
    pub(super) fn validate_binding(
        &self,
        operation: &str,
        expected_core: [u8; 32],
        expected_target: [u8; 32],
        expected_exports: [u8; 32],
        output_type: &str,
    ) -> Result<(), Error> {
        if self.identity != expected_core
            || self.target_identity != expected_target
            || self.exports_identity != expected_exports
        {
            return Err(Error::InvalidArtifact);
        }
        let module = text_field(&self.module, "coordinate")?;
        let name = operation
            .strip_prefix(module)
            .and_then(|suffix| suffix.strip_prefix('.'))
            .filter(|name| !name.is_empty())
            .ok_or(Error::InvalidArtifact)?;
        if name != self.intent {
            return Err(Error::InvalidArtifact);
        }
        let intent = field(field(&self.module, "intents")?, name)?;
        let target = field(field(&self.target, "intents")?, name)?;
        for key in ["inputConstraints", "coreEvaluationBudget"] {
            if field(target, key)? != field(intent, key)? {
                return Err(Error::InvalidArtifact);
            }
        }
        require_text(
            target,
            "operationProfile",
            text_field(intent, "requiredOperationProfile")?,
        )?;
        match (field(target, "basis"), field(intent, "basis")) {
            (Ok(a), Ok(b)) if a == b => {}
            (Err(_), Err(_)) => {}
            _ => return Err(Error::InvalidArtifact),
        }
        let mut fields = vec![
            "input",
            "output",
            "requiredOperationProfile",
            "inputConstraints",
            "coreEvaluationBudget",
            "body",
        ];
        if field(intent, "basis").is_ok() {
            fields.push("basis");
        }
        if field(intent, "optic").is_ok() {
            fields.push("optic");
        }
        exact_fields(intent, &fields)?;
        if text_field(intent, "input")?.is_empty()
            || text_field(intent, "requiredOperationProfile")?.is_empty()
            || text_field(intent, "output")? != output_type
        {
            return Err(Error::InvalidArtifact);
        }
        array(field(intent, "inputConstraints")?)?;
        let budget = field(intent, "coreEvaluationBudget")?;
        exact_fields(budget, &["maxSteps", "maxAllocatedBytes", "maxOutputBytes"])?;
        if number(field(budget, "maxSteps")?)? == 0 {
            return Err(Error::InvalidArtifact);
        }
        number(field(budget, "maxAllocatedBytes")?)?;
        number(field(budget, "maxOutputBytes")?)?;
        let body = field(intent, "body")?;
        exact_fields(body, &["locals", "nodes", "result"])?;
        array(field(body, "locals")?)?;
        array(field(body, "nodes")?)?;
        map(field(body, "result")?)?;
        if field(target, "result")? != field(body, "result")? {
            return Err(Error::InvalidArtifact);
        }
        super::instruction_relation::validate(&self.module, intent, target)?;
        super::lowering_relation::validate(
            &self.module,
            intent,
            &self.target,
            target,
            &self.exports,
            &self.lowering,
        )?;
        Ok(())
    }
}
