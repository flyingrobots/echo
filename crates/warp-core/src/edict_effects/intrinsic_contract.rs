// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Concrete declared signature/fault agreement across the pinned catalog and lawpack.
//!
//! For an application intrinsic the catalog and the exported effect name the
//! same types. For a shared runtime intrinsic the catalog names Echo's ABI
//! coordinates instead, and the application's own types are checked for
//! representation conformance by `runtime_abi`.
use crate::edict_pure::{
    values::{array, exact_fields, field, map, require_text, text, text_field},
    EvaluationError as Error,
};
use echo_edict_canonical::CanonicalValueV1 as Value;

pub(super) fn validate(
    intrinsic_name: &str,
    intrinsic: &Value,
    effect: &Value,
    implementation: &Value,
) -> Result<(), Error> {
    let abi = super::runtime_abi::contract(intrinsic_name);
    exact_fields(
        intrinsic,
        &[
            "intrinsicClass",
            "argumentTypes",
            "returnType",
            "typeParameters",
            "effectKind",
            "writeClass",
            "guardSupport",
            "canParticipateInAtomicGuard",
            "costTemplate",
            "footprintTemplate",
            "effectFailures",
        ],
    )?;
    require_text(intrinsic, "intrinsicClass", "effect")?;
    require_text(effect, "executionClass", "runtime")?;
    let [argument] = array(field(intrinsic, "argumentTypes")?)? else {
        return Err(Error::UnsupportedProgram);
    };
    let input = text(argument)?;
    let output = text_field(intrinsic, "returnType")?;
    let (expected_input, expected_output) = match abi {
        Some(abi) => (abi.input, abi.result),
        None => (
            text_field(effect, "inputType")?,
            text_field(effect, "outputType")?,
        ),
    };
    if input != expected_input || output != expected_output {
        return Err(Error::InvalidArtifact);
    }
    if !array(field(intrinsic, "typeParameters")?)?.is_empty()
        || !array(field(effect, "typeParameters")?)?.is_empty()
    {
        return Err(Error::UnsupportedProgram);
    }
    require_text(
        intrinsic,
        "writeClass",
        text_field(implementation, "writeClass")?,
    )?;
    let boolean = |value: &Value| {
        if let Value::Bool(value) = value {
            Ok(*value)
        } else {
            Err(Error::InvalidArtifact)
        }
    };
    if boolean(field(effect, "guardSupport")?)? && !boolean(field(intrinsic, "guardSupport")?)? {
        return Err(Error::InvalidArtifact);
    }
    boolean(field(intrinsic, "guardSupport")?)?;
    boolean(field(intrinsic, "canParticipateInAtomicGuard")?)?;
    for name in ["effectKind", "costTemplate", "footprintTemplate"] {
        if text_field(intrinsic, name)?.is_empty() {
            return Err(Error::InvalidArtifact);
        }
    }
    let failures = field(intrinsic, "effectFailures")?;
    let exported = field(effect, "effectFailures")?;
    let mappings = map(field(implementation, "failureMappings")?)?;
    if map(failures)?.len() != mappings.len() || map(exported)?.len() != mappings.len() {
        return Err(Error::InvalidArtifact);
    }
    for (source, target) in mappings {
        let actual = field(failures, text(target)?)?;
        let expected = field(exported, text(source)?)?;
        exact_fields(actual, &["authorityClass", "payloadType"])?;
        exact_fields(expected, &["authorityClass", "payloadType"])?;
        if text_field(actual, "authorityClass")? != text_field(expected, "authorityClass")? {
            return Err(Error::InvalidArtifact);
        }
        // A shared intrinsic's failure carries its ABI input; the exported
        // failure must carry this effect's own declared input type.
        let (catalog_payload, exported_payload) = match abi {
            Some(abi) => (abi.input, text_field(effect, "inputType")?),
            None => (
                text_field(actual, "payloadType")?,
                text_field(actual, "payloadType")?,
            ),
        };
        if text_field(actual, "payloadType")? != catalog_payload
            || text_field(expected, "payloadType")? != exported_payload
        {
            return Err(Error::InvalidArtifact);
        }
    }
    Ok(())
}
