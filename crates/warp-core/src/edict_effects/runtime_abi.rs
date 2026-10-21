// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Runtime-owned recording/emitter ABIs; declarations cannot redefine them.
//!
//! The provider's pinned catalog names each shared intrinsic's input and result
//! by Echo-owned ABI coordinate (`<intrinsic>.input`, `<intrinsic>.result`).
//! An application keeps its own nominal types; the effect its adapter
//! explicitly binds to one of these intrinsics must declare representations
//! that conform to that intrinsic's ABI. Conformance applies only to the exact
//! intrinsic identities below and never selects an intrinsic by shape.
use crate::edict_pure::{
    model::{RuntimeType, MAX_PROGRAM_NODES},
    syntax::Parser,
    values::{field, map, require_text, text_field},
    EvaluationError as Error,
};
use echo_edict_canonical::CanonicalValueV1 as Value;

/// The Echo-owned ABI coordinates of one shared runtime intrinsic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct AbiContract {
    /// Coordinate the catalog must give as the single argument type.
    pub(super) input: &'static str,
    /// Coordinate the catalog must give as the return type.
    pub(super) result: &'static str,
}

/// The ABI contract of an exact runtime intrinsic identity, if it is one.
pub(super) fn contract(intrinsic: &str) -> Option<AbiContract> {
    Some(match intrinsic {
        "echo.dpo@1.node-atom-read" => AbiContract {
            input: "echo.dpo@1.node-atom-read.input",
            result: "echo.dpo@1.node-atom-read.result",
        },
        "echo.dpo@1.ensure-node" => AbiContract {
            input: "echo.dpo@1.ensure-node.input",
            result: "echo.dpo@1.ensure-node.result",
        },
        "echo.dpo@1.create-node" => AbiContract {
            input: "echo.dpo@1.create-node.input",
            result: "echo.dpo@1.create-node.result",
        },
        "echo.dpo@1.replace-atom" => AbiContract {
            input: "echo.dpo@1.replace-atom.input",
            result: "echo.dpo@1.replace-atom.result",
        },
        "echo.dpo@1.ensure-edge" => AbiContract {
            input: "echo.dpo@1.ensure-edge.input",
            result: "echo.dpo@1.ensure-edge.result",
        },
        "echo.dpo@1.replace-edge" => AbiContract {
            input: "echo.dpo@1.replace-edge.input",
            result: "echo.dpo@1.replace-edge.result",
        },
        _ => return None,
    })
}

pub(super) fn validate(
    name: &str,
    module: &Value,
    effect: &Value,
    intrinsic: &Value,
) -> Result<(), Error> {
    let mutation = super::mutations::MutationKind::from_intrinsic(name);
    let Some(abi) = contract(name) else {
        return Err(Error::UnsupportedProgram);
    };

    let mut parser = Parser {
        types: field(module, "types")?,
        coordinate: text_field(module, "coordinate")?,
        remaining: MAX_PROGRAM_NODES,
    };
    let input_name = text_field(effect, "inputType")?;
    let input = parser.ty(input_name, 0)?;
    let output = parser.ty(text_field(effect, "outputType")?, 0)?;
    if let Some(kind) = mutation {
        require_text(intrinsic, "effectKind", "write")?;
        require_text(intrinsic, "writeClass", "write")?;
        if !map(field(intrinsic, "effectFailures")?)?.is_empty() {
            return Err(Error::InvalidArtifact);
        }
        return kind.validate_type(&input, &output);
    }
    require_text(intrinsic, "effectKind", "read")?;
    require_text(intrinsic, "writeClass", "read")?;
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
        // The catalog says each read failure carries the call's ABI input; the
        // contract checker binds the exported failure to this effect's own
        // input type, keeping that nominal relationship.
        let failure = field(failures, name)?;
        require_text(failure, "authorityClass", authority)?;
        require_text(failure, "payloadType", abi.input)?;
    }
    Ok(())
}
