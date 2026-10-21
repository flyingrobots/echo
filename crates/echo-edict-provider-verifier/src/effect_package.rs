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
    if text(get(&package, "schema", E)?, E)? != "echo.operation-package/v1"
        || map_field(&package, "obstruction_coordinate").is_some()
    {
        return Err(E);
    }
    let program = canonical(bytes(get(&package, "program", E)?, E)?)?;
    if text(get(&program, "schema", E)?, E)? != "echo.operation-program/v1"
        || text(get(&program, "kind", E)?, E)? != "edict-effect-program/v1"
    {
        return Err(E);
    }
    let compiled = bytes(get(&program, "compiled_program", E)?, E)?;
    let judged = crate::executable_operation::effect_program::judge(compiled)
        .map_err(|()| EffectPackageRefusal::UnsupportedProgram)?;
    let closure = get(&package, "semantic_closure", E)?;
    for (value, expected) in [
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
