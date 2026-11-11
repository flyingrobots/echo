// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Independent verification of an authored effect package before installation.
//!
//! The verifier judges the compiled effect program itself and requires every
//! identity the package claims to name that exact program. It does not trust
//! the lowerer that produced the package or the runtime that will execute it.
use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, encode_canonical_cbor_v1 as encode,
    CanonicalValueV1 as Value,
};

/// Why an authored effect package was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectPackageRefusal {
    /// The bytes are not a canonical authored effect package.
    InvalidEnvelope,
    /// A package identity, operation or output does not name the program the
    /// package carries.
    InvalidBinding,
    /// The compiled effect program is unsafe or does not match its own
    /// artifacts, intrinsic contracts or declared budget.
    UnsupportedProgram,
}

impl std::fmt::Display for EffectPackageRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidEnvelope => "not a canonical authored effect package",
            Self::InvalidBinding => "the package does not name the program it carries",
            Self::UnsupportedProgram => {
                "the compiled effect program is outside the checked profile"
            }
        })
    }
}
impl std::error::Error for EffectPackageRefusal {}

/// Verifies an authored effect package's program and its binding.
///
/// # Errors
/// Refuses malformed envelopes, packages whose identities name another
/// program, and programs outside the verifier's own judgment.
pub fn verify_effect_package(package: &[u8]) -> Result<(), EffectPackageRefusal> {
    use EffectPackageRefusal::{InvalidBinding as B, InvalidEnvelope as E};
    let package = canonical(package)?;
    exact_fields(&package, &PACKAGE_FIELDS)?;
    let closure = get(&package, "semantic_closure", E)?;
    exact_fields(closure, &CLOSURE_FIELDS)?;
    let ceiling = get(&package, "budget_ceiling", E)?;
    exact_fields(ceiling, &["read_bytes", "steps", "write_bytes"])?;
    for field in ["read_bytes", "write_bytes"] {
        unsigned(get(ceiling, field, E)?)?;
    }
    if text(get(&package, "schema", E)?, E)? != "echo.operation-package/v1"
        || unsigned(get(ceiling, "steps", E)?)? == 0
        || text(get(closure, "application_schema_coordinate", E)?, E)?.is_empty()
    {
        return Err(E);
    }
    // The verifier binds neither the authority profile nor the source
    // identity to a value, but each is the 32-byte hash the runtime decodes.
    for identity in [
        get(&package, "authority_profile_identity", E)?,
        get(closure, "edict_source_identity", E)?,
    ] {
        if bytes(identity, E)?.len() != 32 {
            return Err(E);
        }
    }
    // Every fixed profile and schema identity names the effect profile the
    // runtime supports.
    for (field, label) in PACKAGE_PROFILES {
        if bytes(get(&package, field, E)?, E)? != profile_identity(label) {
            return Err(E);
        }
    }
    let program = canonical(bytes(get(&package, "program", E)?, E)?)?;
    envelope(&program)?;
    let compiled = bytes(get(&program, "compiled_program", E)?, E)?;
    let judged = crate::executable_operation::effect_program::judge(compiled)
        .map_err(|()| EffectPackageRefusal::UnsupportedProgram)?;
    let closure = get(&package, "semantic_closure", E)?;
    // The lowerer names the judged Core as the canonical meaning; the runtime
    // carries that claim into the installed operation and its receipts.
    for (value, expected) in [
        (
            get(closure, "canonical_meaning_identity", E)?,
            &judged.core_identity,
        ),
        (get(closure, "core_identity", E)?, &judged.core_identity),
        (
            get(closure, "target_ir_identity", E)?,
            &judged.target_identity,
        ),
        (
            get(closure, "application_schema_identity", E)?,
            &judged.exports_identity,
        ),
        (
            get(&program, "target_ir_identity", E)?,
            &judged.target_identity,
        ),
    ] {
        if value != expected {
            return Err(B);
        }
    }
    if text(get(&package, "operation_coordinate", E)?, E)? != judged.operation
        || text(get(&program, "output_type", E)?, E)? != judged.output
    {
        return Err(B);
    }
    let mut footprint = blake3::Hasher::new();
    footprint.update(b"echo:operation-footprint-law:v1\0");
    footprint.update(bytes(get(&program, "footprint_law", E)?, E)?);
    if bytes(get(&package, "footprint_contract_identity", E)?, E)?
        != footprint.finalize().as_bytes()
    {
        return Err(B);
    }
    Ok(())
}

/// The fields Echo's runtime admits in an authored effect package. An effect
/// package names no obstruction coordinate and projects no result.
const PACKAGE_FIELDS: [&str; 17] = [
    "application_basis_schema_identity",
    "authority_profile_identity",
    "budget_ceiling",
    "evaluation_basis_schema_identity",
    "footprint_contract_identity",
    "input_schema_identity",
    "interpreter_profile_identity",
    "intrinsic_profile_identity",
    "obstruction_interpretation_identity",
    "obstruction_schema_identity",
    "operation_coordinate",
    "program",
    "result_interpretation_identity",
    "result_schema_identity",
    "schema",
    "semantic_closure",
    "target_profile_identity",
];
/// An effect package's semantic closure binds no lawpack.
const CLOSURE_FIELDS: [&str; 6] = [
    "application_schema_coordinate",
    "application_schema_identity",
    "canonical_meaning_identity",
    "core_identity",
    "edict_source_identity",
    "target_ir_identity",
];
const PROGRAM_FIELDS: [&str; 13] = [
    "compiled_program",
    "footprint_law",
    "graph_schema",
    "identity_law",
    "interpreter_profile_identity",
    "intrinsic_profile_identity",
    "kind",
    "max_allocated_bytes",
    "max_input_bytes",
    "max_result_bytes",
    "output_type",
    "schema",
    "target_ir_identity",
];
/// The package's fixed profile and schema identities, each the runtime's
/// identity for the effect profile it names.
const PACKAGE_PROFILES: [(&str, &str); 10] = [
    (
        "target_profile_identity",
        "echo.operation-target.edict-effect/v1",
    ),
    (
        "interpreter_profile_identity",
        "echo.operation-interpreter.edict-effect/v1",
    ),
    (
        "intrinsic_profile_identity",
        "echo.operation-intrinsics.edict-effect/v1",
    ),
    (
        "input_schema_identity",
        "echo.operation.input.edict-effect/v1",
    ),
    (
        "result_schema_identity",
        "echo.operation.result.edict-effect/v1",
    ),
    (
        "obstruction_schema_identity",
        "echo.operation.obstruction.edict-effect/v1",
    ),
    (
        "result_interpretation_identity",
        "echo.operation.result-interpretation.edict-effect/v1",
    ),
    (
        "obstruction_interpretation_identity",
        "echo.operation.obstruction-interpretation.edict-effect/v1",
    ),
    (
        "application_basis_schema_identity",
        "echo.operation.basis.edict-effect/v1",
    ),
    (
        "evaluation_basis_schema_identity",
        "echo.operation.evaluation-basis/v1",
    ),
];
/// The runtime's fixed ceilings on an effect program's declared limits.
const MAX_INPUT_BYTES: u64 = 65_536;
const MAX_RESULT_BYTES: u64 = 65_536;
const MAX_ALLOCATED_BYTES: u64 = 16 * 1024 * 1024;
const MAX_OUTPUT_TYPE_BYTES: usize = 1024;

/// Checks the effect program's envelope as the runtime admits it: its exact
/// fields, schema, identity law, profiles and declared limits.
fn envelope(program: &Value) -> Result<(), EffectPackageRefusal> {
    use EffectPackageRefusal::InvalidEnvelope as E;
    exact_fields(program, &PROGRAM_FIELDS)?;
    if text(get(program, "schema", E)?, E)? != "echo.operation-program/v1"
        || text(get(program, "kind", E)?, E)? != "edict-effect-program/v1"
        || unsigned(get(program, "identity_law", E)?)? != 1
        || unsigned(get(program, "max_input_bytes", E)?)? > MAX_INPUT_BYTES
        || unsigned(get(program, "max_result_bytes", E)?)? > MAX_RESULT_BYTES
        || unsigned(get(program, "max_allocated_bytes", E)?)? > MAX_ALLOCATED_BYTES
        || text(get(program, "output_type", E)?, E)?.len() > MAX_OUTPUT_TYPE_BYTES
    {
        return Err(E);
    }
    for (field, label) in [
        (
            "interpreter_profile_identity",
            "echo.operation-interpreter.edict-effect/v1",
        ),
        (
            "intrinsic_profile_identity",
            "echo.operation-intrinsics.edict-effect/v1",
        ),
    ] {
        if bytes(get(program, field, E)?, E)? != profile_identity(label) {
            return Err(E);
        }
    }
    // The runtime parses both; the verifier requires each to be one canonical
    // value. Their contents are the runtime's to admit.
    for field in ["graph_schema", "footprint_law"] {
        canonical(bytes(get(program, field, E)?, E)?)?;
    }
    Ok(())
}
/// The runtime's identity for a named operation profile.
fn profile_identity(label: &str) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"echo:operation-profile:v1\0");
    hasher.update(&(label.len() as u64).to_le_bytes());
    hasher.update(label.as_bytes());
    hasher.finalize().into()
}
fn exact_fields(value: &Value, expected: &[&str]) -> Result<(), EffectPackageRefusal> {
    let Value::Map(fields) = value else {
        return Err(EffectPackageRefusal::InvalidEnvelope);
    };
    if fields.len() != expected.len()
        || fields
            .iter()
            .any(|(key, _)| !matches!(key, Value::Text(name) if expected.contains(&name.as_str())))
    {
        return Err(EffectPackageRefusal::InvalidEnvelope);
    }
    Ok(())
}
fn unsigned(value: &Value) -> Result<u64, EffectPackageRefusal> {
    match value {
        Value::Integer(number) => {
            u64::try_from(*number).map_err(|_| EffectPackageRefusal::InvalidEnvelope)
        }
        _ => Err(EffectPackageRefusal::InvalidEnvelope),
    }
}

fn canonical(bytes: &[u8]) -> Result<Value, EffectPackageRefusal> {
    let value = decode(bytes).map_err(|_| EffectPackageRefusal::InvalidEnvelope)?;
    // Only the exact canonical spelling of a package is verifiable.
    if encode(&value).map_err(|_| EffectPackageRefusal::InvalidEnvelope)? != bytes {
        return Err(EffectPackageRefusal::InvalidEnvelope);
    }
    Ok(value)
}
fn map_field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    let Value::Map(fields) = value else {
        return None;
    };
    fields
        .iter()
        .find_map(|(k, v)| matches!(k, Value::Text(name) if name == key).then_some(v))
}
fn get<'a>(
    value: &'a Value,
    key: &str,
    refusal: EffectPackageRefusal,
) -> Result<&'a Value, EffectPackageRefusal> {
    map_field(value, key).ok_or(refusal)
}
fn text(value: &Value, refusal: EffectPackageRefusal) -> Result<&str, EffectPackageRefusal> {
    match value {
        Value::Text(text) => Ok(text),
        _ => Err(refusal),
    }
}
fn bytes(value: &Value, refusal: EffectPackageRefusal) -> Result<&[u8], EffectPackageRefusal> {
    match value {
        Value::Bytes(bytes) => Ok(bytes),
        _ => Err(refusal),
    }
}
