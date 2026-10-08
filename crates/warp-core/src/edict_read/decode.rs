// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use super::{
    instructions,
    model::{Program, MAX_ARTIFACT_BYTES},
    ReadLimits,
};
use crate::edict_pure::{
    model::MAX_PROGRAM_NODES,
    syntax::Parser,
    values::{array, bytes, exact_fields, field, map, number, require_text, text_field},
    EvaluationError as Error, EvaluationLimits,
};
use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, digest_canonical_value_bytes_v1 as digest,
    CanonicalValueV1 as Value,
};

const KIND: &str = "compiler-produced-bounded-read/v1";
const PACKAGE_DOMAIN: &str = "echo.operation-package/v1";

pub(super) fn package(raw: &[u8], pin: [u8; 32], host: ReadLimits) -> Result<Program, Error> {
    if raw.len() > host.evaluation.max_package_bytes.min(MAX_ARTIFACT_BYTES) {
        return Err(Error::PackageTooLarge);
    }
    let package = decode(raw).map_err(|_| Error::InvalidArtifact)?;
    if digest(PACKAGE_DOMAIN, &package).map_err(|_| Error::InvalidArtifact)? != pin {
        return Err(Error::PackageIdentityMismatch);
    }
    envelope(&package)?;
    let program = embedded(&package, "program")?;
    exact_fields(
        &program,
        &[
            "schema",
            "kind",
            "intent",
            "core_artifact",
            "target_ir_artifact",
            "lawpack_exports_artifact",
            "lawpack_artifact",
            "lawpack_adapter_artifact",
            "target_configuration_artifact",
            "result_projection_artifact",
            "source_artifact",
        ],
    )?;
    require_text(&program, "schema", "echo.compiler-produced-read-program/v1")?;
    require_text(&program, "kind", KIND)?;
    let core = embedded(&program, "core_artifact")?;
    let target = embedded(&program, "target_ir_artifact")?;
    let exports = embedded(&program, "lawpack_exports_artifact")?;
    let lawpack = embedded(&program, "lawpack_artifact")?;
    let source = embedded(&program, "source_artifact")?;
    let projection = embedded(&program, "result_projection_artifact")?;
    let configuration = embedded(&program, "target_configuration_artifact")?;
    let adapter = embedded(&program, "lawpack_adapter_artifact")?;
    closure(&package, &core, &target, &exports, &lawpack, &source)?;
    require_text(&core, "apiVersion", "edict.core/v1")?;
    require_text(&target, "domain", "echo.span-ir/v2")?;
    require_text(&target, "kind", "orderedTargetIrArtifact")?;
    require_text(&projection, "schema", "edict.result-projection/v1")?;
    exact_fields(
        &configuration,
        &["apiVersion", "programKind", "maxReads", "maxReadBytes"],
    )?;
    require_text(
        &configuration,
        "apiVersion",
        "echo.operation-lowering-configuration/v1",
    )?;
    require_text(&configuration, "programKind", KIND)?;
    let reads = number(field(&configuration, "maxReads")?)?;
    let read_bytes = number(field(&configuration, "maxReadBytes")?)?;
    if !(1..=65_536).contains(&reads) || !(1..=67_108_864).contains(&read_bytes) {
        return Err(Error::InvalidArtifact);
    }
    let name = text_field(&program, "intent")?;
    if map(field(&core, "intents")?)?.len() != 1 || map(field(&target, "intents")?)?.len() != 1 {
        return Err(Error::UnsupportedProgram);
    }
    let intent = field(field(&core, "intents")?, name)?;
    let target_intent = field(field(&target, "intents")?, name)?;
    require_text(
        intent,
        "requiredOperationProfile",
        "continuum.profile.read-only/v1",
    )?;
    require_text(
        target_intent,
        "operationProfile",
        "continuum.profile.read-only/v1",
    )?;
    if field(target_intent, "externalActionRequests").is_ok() {
        return Err(Error::UnsupportedProgram);
    }
    let coordinate = format!("{}.{}", text_field(&core, "coordinate")?, name);
    require_text(&package, "operation_coordinate", &coordinate)?;
    require_text(&projection, "operationCoordinate", &coordinate)?;
    let body = field(intent, "body")?;
    // Provider scopes identify the input by identity, not declaration order.
    let input_local = array(field(body, "locals")?)?
        .iter()
        .find(|local| text_field(local, "id") == Ok("arg.0"))
        .ok_or(Error::InvalidArtifact)?;
    let input_id = text_field(input_local, "id")?.to_owned();
    if text_field(input_local, "type")? != text_field(intent, "input")? {
        return Err(Error::InvalidArtifact);
    }
    let mut parser = Parser {
        types: field(&core, "types")?,
        coordinate: text_field(&core, "coordinate")?,
        remaining: MAX_PROGRAM_NODES,
    };
    let input_type = parser.ty(text_field(intent, "input")?, 0)?;
    let output_type = parser.ty(text_field(intent, "output")?, 0)?;
    let ordered = instructions::parse(target_intent, &mut parser, &input_id)?;
    if ordered.read_producers.len() as u64 > reads {
        return Err(Error::InvalidArtifact);
    }
    let result = parser.expr(field(target_intent, "result")?, 0)?;
    if result != parser.expr(field(body, "result")?, 0)?
        || result
            != parser.projection(
                field(&projection, "expression")?,
                &input_id,
                &ordered.pure_producers,
                &ordered.read_producers,
                0,
            )?
    {
        return Err(Error::InvalidArtifact);
    }
    if field(target_intent, "basis")? != field(intent, "basis")?
        || field(target_intent, "inputConstraints")? != field(intent, "inputConstraints")?
    {
        return Err(Error::InvalidArtifact);
    }
    let basis = parser.expr(field(intent, "basis")?, 0)?;
    let mut constraints = Vec::new();
    for constraint in array(field(target_intent, "inputConstraints")?)? {
        constraints.push((
            text_field(constraint, "coordinate")?.to_owned(),
            parser.predicate(field(constraint, "predicate")?, 0)?,
        ));
    }
    let evaluation = budget(
        &package,
        intent,
        target_intent,
        &adapter,
        &projection,
        host.evaluation,
    )?;
    let helpers = crate::edict_pure::functions::decode(&core, &exports, &mut parser)?;
    let mut expressions = vec![&basis, &result];
    let mut predicates: Vec<_> = constraints.iter().map(|(_, predicate)| predicate).collect();
    for instruction in &ordered.ordered {
        match instruction {
            super::model::Instruction::Read(read) => expressions.push(&read.input),
            super::model::Instruction::Let { value, .. } => expressions.push(value),
            super::model::Instruction::Require { predicate, .. } => predicates.push(predicate),
        }
    }
    crate::edict_pure::functions::check_roots(&helpers, expressions, predicates)?;
    Ok(Program {
        input_id,
        input_type,
        output_type,
        basis,
        constraints,
        instructions: ordered.ordered,
        result,
        helpers,
        limits: ReadLimits {
            evaluation,
            max_reads: host.max_reads.min(reads),
            max_read_bytes: host.max_read_bytes.min(read_bytes),
        },
    })
}

fn envelope(package: &Value) -> Result<(), Error> {
    exact_fields(
        package,
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
    require_text(package, "schema", PACKAGE_DOMAIN)?;
    require_text(package, "package_kind", KIND)?;
    for (key, profile) in [
        (
            "authority_profile_identity",
            "echo.operation.authority.explicit-read-view/v1",
        ),
        (
            "footprint_contract_identity",
            "echo.operation.footprint.explicit-node-read-aperture/v1",
        ),
        (
            "interpreter_profile_identity",
            "echo.operation-interpreter.compiler-produced-bounded-read/v1",
        ),
    ] {
        let mut hash = blake3::Hasher::new();
        hash.update(b"echo:operation-profile:v1\0");
        hash.update(&(profile.len() as u64).to_le_bytes());
        hash.update(profile.as_bytes());
        if bytes(field(package, key)?)? != hash.finalize().as_bytes() {
            return Err(Error::UnsupportedProgram);
        }
    }
    Ok(())
}

fn closure(
    package: &Value,
    core: &Value,
    target: &Value,
    exports: &Value,
    lawpack: &Value,
    source: &Value,
) -> Result<(), Error> {
    let closure = field(package, "semantic_closure")?;
    for (key, domain, value) in [
        ("core_identity", "edict.core.module/v1", core),
        ("target_ir_identity", "edict.target-ir.artifact/v1", target),
        (
            "application_schema_identity",
            "edict.lawpack-exports/v1",
            exports,
        ),
        ("lawpack_identity", "edict.lawpack/v1", lawpack),
        ("edict_source_identity", "edict.source/v1", source),
    ] {
        if bytes(field(closure, key)?)?
            != digest(domain, value).map_err(|_| Error::InvalidArtifact)?
        {
            return Err(Error::InvalidArtifact);
        }
    }
    Ok(())
}

fn embedded(program: &Value, key: &str) -> Result<Value, Error> {
    decode(bytes(field(program, key)?)?).map_err(|_| Error::InvalidArtifact)
}

fn budget(
    package: &Value,
    intent: &Value,
    target: &Value,
    adapter: &Value,
    projection: &Value,
    host: EvaluationLimits,
) -> Result<EvaluationLimits, Error> {
    let budget = field(package, "budget_ceiling")?;
    let core = field(intent, "coreEvaluationBudget")?;
    if field(target, "coreEvaluationBudget")? != core {
        return Err(Error::InvalidArtifact);
    }
    let profiles = map(field(adapter, "operationProfiles")?)?;
    let [(_, profile)] = profiles else {
        return Err(Error::UnsupportedProgram);
    };
    if field(
        field(adapter, "budgets")?,
        text_field(profile, "budgetObligation")?,
    )? != core
    {
        return Err(Error::InvalidArtifact);
    }
    for (key, name) in [
        ("max_steps", "maxSteps"),
        ("max_allocated_bytes", "maxAllocatedBytes"),
        ("max_output_bytes", "maxOutputBytes"),
    ] {
        if field(budget, key)? != field(core, name)? {
            return Err(Error::InvalidArtifact);
        }
    }
    Ok(EvaluationLimits {
        max_steps: host.max_steps.min(number(field(budget, "max_steps")?)?),
        max_allocated_bytes: host
            .max_allocated_bytes
            .min(number(field(budget, "max_allocated_bytes")?)?),
        max_output_bytes: host
            .max_output_bytes
            .min(number(field(budget, "max_output_bytes")?)?)
            .min(number(field(projection, "maxOutputBytes")?)?),
        ..host
    })
}
