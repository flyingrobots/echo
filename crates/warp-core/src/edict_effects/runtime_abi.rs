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
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AbiContract {
    /// Coordinate the catalog must give as the single argument type.
    pub(super) input: String,
    /// Coordinate the catalog must give as the return type.
    pub(super) result: String,
}

/// The ABI contract of an exact runtime intrinsic identity, if it is one.
///
/// Its coordinates are `<intrinsic>.input` and `<intrinsic>.result` under the
/// intrinsic's own namespace, so `echo.dpo@2.ensure-node` has
/// `echo.dpo@2.ensure-node.input`.
pub(super) fn contract(intrinsic: &str) -> Option<AbiContract> {
    match super::intrinsic_names::operation(intrinsic)? {
        "node-atom-read" | "ensure-node" | "create-node" | "replace-atom" | "ensure-edge"
        | "replace-edge" => Some(AbiContract {
            input: format!("{intrinsic}.input"),
            result: format!("{intrinsic}.result"),
        }),
        _ => None,
    }
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
        require_text(intrinsic, "effectKind", kind.effect_kind())?;
        require_text(intrinsic, "writeClass", kind.effect_kind())?;
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
    // The catalog names each failure by the identifier Echo pins for this
    // intrinsic, and carries the call's ABI input; the contract checker binds
    // the exported failure to this effect's own input type.
    let failures = field(intrinsic, "effectFailures")?;
    let declared = super::failure_names::failures(name).ok_or(Error::InvalidArtifact)?;
    if map(failures)?.len() != declared.len() {
        return Err(Error::InvalidArtifact);
    }
    for expected in declared {
        let failure = field(failures, expected.catalog)?;
        require_text(failure, "authorityClass", expected.authority)?;
        require_text(failure, "payloadType", &abi.input)?;
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn graph_reads_have_echo_owned_abi_coordinates_under_their_own_namespace() {
        for namespace in ["echo.dpo@1", "echo.dpo@2"] {
            for operation in ["outgoing-edge-read", "closure-read"] {
                let name = format!("{namespace}.{operation}");
                assert_eq!(
                    contract(&name),
                    Some(AbiContract {
                        input: format!("{name}.input"),
                        result: format!("{name}.result"),
                    }),
                    "{name}"
                );
            }
        }
        assert_eq!(contract("echo.dpo@2.edge-read"), None);
    }
}
