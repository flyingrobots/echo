// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Runtime-owned ABI for implemented recording primitives; declarations cannot redefine it.
use crate::edict_pure::{
    model::{RuntimeType, MAX_PROGRAM_NODES},
    syntax::Parser,
    values::{field, map, require_text, text_field},
    EvaluationError as Error,
};
use echo_edict_canonical::CanonicalValueV1 as Value;

pub(super) fn validate(
    name: &str,
    module: &Value,
    effect: &Value,
    intrinsic: &Value,
) -> Result<(), Error> {
    // Mutation intrinsics require their own actual handlers and contracts. A
    // catalog declaration alone never registers executable code.
    if name != "echo.dpo@1.node-atom-read" {
        return Err(Error::UnsupportedProgram);
    }
    require_text(intrinsic, "effectKind", "read")?;
    require_text(intrinsic, "writeClass", "read")?;
    let mut parser = Parser {
        types: field(module, "types")?,
        coordinate: text_field(module, "coordinate")?,
        remaining: MAX_PROGRAM_NODES,
    };
    let input_name = text_field(effect, "inputType")?;
    let input = parser.ty(input_name, 0)?;
    let RuntimeType::Record(fields) = input.representation() else {
        return Err(Error::InvalidArtifact);
    };
    if fields.len() != 3
        || !["nodeId", "typeId", "warpId"].iter().all(|key| {
            fields.iter().any(|(name, ty)| {
                name == key && ty.representation() == &RuntimeType::Bytes { min: 32, max: 32 }
            })
        })
    {
        return Err(Error::InvalidArtifact);
    }
    let output = parser.ty(text_field(effect, "outputType")?, 0)?;
    if !matches!(output, RuntimeType::Bytes { min: 0, max } if max > 0 && max <= super::EFFECT_READ_STORAGE_CEILING_V1)
    {
        return Err(Error::UnsupportedProgram);
    }
    let failures = field(intrinsic, "effectFailures")?;
    if map(failures)?.len() != 4 {
        return Err(Error::InvalidArtifact);
    }
    for (name, authority) in [
        ("echo.atom-missing/v1", "resourceFault"),
        ("echo.atom-required/v1", "integrityFault"),
        ("echo.atom-type-mismatch/v1", "integrityFault"),
        ("echo.atom-byte-bound-exceeded/v1", "integrityFault"),
    ] {
        let failure = field(failures, name)?;
        require_text(failure, "authorityClass", authority)?;
        require_text(failure, "payloadType", input_name)?;
    }
    Ok(())
}
