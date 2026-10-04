// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use std::collections::BTreeMap;

use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, digest_canonical_value_bytes_v1 as digest,
    CanonicalValueV1 as Value,
};

use super::model::{Binding, Helper, Program, MAX_PROGRAM_NODES};
use super::syntax::Parser;
use super::values::{array, bytes, exact_fields, field, number, require_text, text_field};
use super::{EvaluationError as Error, EvaluationLimits};

const PACKAGE_DOMAIN: &str = "echo.operation-package/v1";
const PROGRAM_KIND: &str = "compiler-produced-bounded-pure/v1";

pub(super) fn package(
    value: &Value,
    pin: [u8; 32],
    host: EvaluationLimits,
) -> Result<Program, Error> {
    if digest(PACKAGE_DOMAIN, value).map_err(|_| Error::InvalidArtifact)? != pin {
        return Err(Error::PackageIdentityMismatch);
    }
    exact_fields(
        value,
        &[
            "schema",
            "program",
            "package_kind",
            "budget_ceiling",
            "semantic_closure",
            "operation_coordinate",
            "target_profile_identity",
            "authority_profile_identity",
            "footprint_contract_identity",
            "interpreter_profile_identity",
        ],
    )?;
    require_text(value, "schema", PACKAGE_DOMAIN)?;
    require_text(value, "package_kind", PROGRAM_KIND)?;
    for (key, profile) in [
        (
            "authority_profile_identity",
            "echo.operation.authority.no-effects/v1",
        ),
        (
            "footprint_contract_identity",
            "echo.operation.footprint.empty/v1",
        ),
        (
            "interpreter_profile_identity",
            "echo.operation-interpreter.compiler-produced-bounded-pure/v1",
        ),
    ] {
        let mut hash = blake3::Hasher::new();
        hash.update(b"echo:operation-profile:v1\0");
        hash.update(&(profile.len() as u64).to_le_bytes());
        hash.update(profile.as_bytes());
        if bytes(field(value, key)?)? != hash.finalize().as_bytes() {
            return Err(Error::UnsupportedProgram);
        }
    }
    let program = decode(bytes(field(value, "program")?)?).map_err(|_| Error::InvalidArtifact)?;
    exact_fields(
        &program,
        &[
            "schema",
            "kind",
            "intent",
            "core_artifact",
            "target_ir_artifact",
            "lawpack_exports_artifact",
            "result_projection_artifact",
        ],
    )?;
    require_text(&program, "schema", "echo.compiler-produced-pure-program/v1")?;
    require_text(&program, "kind", PROGRAM_KIND)?;
    let core = embedded(&program, "core_artifact")?;
    let target = embedded(&program, "target_ir_artifact")?;
    let exports = embedded(&program, "lawpack_exports_artifact")?;
    let projection = embedded(&program, "result_projection_artifact")?;
    let closure = field(value, "semantic_closure")?;
    for (key, domain, artifact) in [
        ("core_identity", "edict.core.module/v1", &core),
        ("target_ir_identity", "edict.target-ir.artifact/v1", &target),
        (
            "application_schema_identity",
            "edict.lawpack-exports/v1",
            &exports,
        ),
    ] {
        if bytes(field(closure, key)?)?
            != digest(domain, artifact).map_err(|_| Error::InvalidArtifact)?
        {
            return Err(Error::InvalidArtifact);
        }
    }
    require_text(&core, "apiVersion", "edict.core/v1")?;
    require_text(&target, "domain", "echo.span-ir/v1")?;
    require_text(&projection, "schema", "edict.result-projection/v1")?;
    let intent_name = text_field(&program, "intent")?;
    let coordinate = format!("{}.{}", text_field(&core, "coordinate")?, intent_name);
    require_text(value, "operation_coordinate", &coordinate)?;
    require_text(&projection, "operationCoordinate", &coordinate)?;
    let core_intent = field(field(&core, "intents")?, intent_name)?;
    let target_intent = field(field(&target, "intents")?, intent_name)?;
    for name in ["steps", "requirements"] {
        if !array(field(target_intent, name)?)?.is_empty() {
            return Err(Error::UnsupportedProgram);
        }
    }
    if let Ok(actions) = field(target_intent, "externalActionRequests") {
        if !array(actions)?.is_empty() {
            return Err(Error::UnsupportedProgram);
        }
    }
    let input_local = array(field(field(core_intent, "body")?, "locals")?)?
        .first()
        .ok_or(Error::InvalidArtifact)?;
    let input_id = text_field(input_local, "id")?.to_owned();
    if text_field(input_local, "type")? != text_field(core_intent, "input")? {
        return Err(Error::UnsupportedProgram);
    }
    let mut parser = Parser {
        types: field(&core, "types")?,
        coordinate: text_field(&core, "coordinate")?,
        remaining: MAX_PROGRAM_NODES,
    };
    let input_type = parser.ty(text_field(core_intent, "input")?, 0)?;
    let output_type = parser.ty(text_field(core_intent, "output")?, 0)?;
    let mut bindings = Vec::new();
    let mut projection_bindings = BTreeMap::new();
    for binding in array(field(target_intent, "pureBindings")?)? {
        let local = field(binding, "binding")?;
        let id = text_field(local, "id")?.to_owned();
        if id == input_id || bindings.iter().any(|binding: &Binding| binding.id == id) {
            return Err(Error::InvalidArtifact);
        }
        if projection_bindings
            .insert(text_field(binding, "id")?.to_owned(), id.clone())
            .is_some()
        {
            return Err(Error::InvalidArtifact);
        }
        bindings.push(Binding {
            id,
            ty: parser.ty(text_field(local, "type")?, 0)?,
            value: parser.expr(field(binding, "value")?, 0)?,
        });
    }
    let result = parser.expr(field(target_intent, "result")?, 0)?;
    let projected = parser.projection(
        field(&projection, "expression")?,
        &input_id,
        &projection_bindings,
        0,
    )?;
    if result != projected {
        return Err(Error::InvalidArtifact);
    }
    let mut constraints = Vec::new();
    for constraint in array(field(target_intent, "inputConstraints")?)? {
        constraints.push((
            text_field(constraint, "coordinate")?.to_owned(),
            parser.predicate(field(constraint, "predicate")?, 0)?,
        ));
    }
    let helpers = helpers(&exports, &mut parser)?;
    let budget = field(value, "budget_ceiling")?;
    let core_budget = field(core_intent, "coreEvaluationBudget")?;
    if field(target_intent, "coreEvaluationBudget")? != core_budget {
        return Err(Error::InvalidArtifact);
    }
    for (package_key, core_key) in [
        ("max_steps", "maxSteps"),
        ("max_allocated_bytes", "maxAllocatedBytes"),
        ("max_output_bytes", "maxOutputBytes"),
    ] {
        if field(budget, package_key)? != field(core_budget, core_key)? {
            return Err(Error::InvalidArtifact);
        }
    }
    let limits = EvaluationLimits {
        max_steps: host.max_steps.min(number(field(budget, "max_steps")?)?),
        max_allocated_bytes: host
            .max_allocated_bytes
            .min(number(field(budget, "max_allocated_bytes")?)?),
        max_output_bytes: host
            .max_output_bytes
            .min(number(field(budget, "max_output_bytes")?)?)
            .min(number(field(&projection, "maxOutputBytes")?)?),
        ..host
    };
    Ok(Program {
        input_id,
        input_type,
        output_type,
        constraints,
        bindings,
        helpers,
        result,
        limits,
    })
}

fn embedded(program: &Value, key: &str) -> Result<Value, Error> {
    decode(bytes(field(program, key)?)?).map_err(|_| Error::InvalidArtifact)
}

fn helpers(exports: &Value, parser: &mut Parser<'_>) -> Result<BTreeMap<String, Helper>, Error> {
    let mut result = BTreeMap::new();
    for helper in array(field(exports, "pureFunctions")?)? {
        require_text(helper, "source", "edict")?;
        let implementation = field(helper, "body")?;
        let body = field(implementation, "body")?;
        for (value, name) in [
            (helper, "parameterTypes"),
            (helper, "typeParameters"),
            (implementation, "params"),
            (body, "bindings"),
            (body, "locals"),
        ] {
            if !array(field(value, name)?)?.is_empty() {
                return Err(Error::UnsupportedProgram);
            }
        }
        let definition = Helper {
            result: parser.expr(field(body, "result")?, 0)?,
            ty: parser.ty(text_field(helper, "returnType")?, 0)?,
        };
        if result
            .insert(text_field(helper, "coordinate")?.to_owned(), definition)
            .is_some()
        {
            return Err(Error::InvalidArtifact);
        }
    }
    // Reject effects even when a malformed package claims an empty authority profile.
    if !array(field(exports, "effects")?)?.is_empty() {
        return Err(Error::UnsupportedProgram);
    }
    Ok(result)
}
