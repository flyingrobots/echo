// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Authenticate imported Core declarations against pinned canonical export definitions.
use crate::edict_pure::{
    model::{MAX_DEPTH, MAX_PROGRAM_NODES},
    values::{array, exact_fields, field, map, number, text, text_field},
    EvaluationError as Error,
};
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn validate(module: &Value, exports: &Value, lawpack: &str) -> Result<(), Error> {
    let mut definitions = BTreeMap::new();
    let mut remaining = MAX_PROGRAM_NODES;
    for item in array(field(exports, "types")?)? {
        spend(&mut remaining)?;
        exact_fields(item, &["coordinate", "definition"])?;
        let name = text_field(item, "coordinate")?;
        let definition = text_field(item, "definition")?;
        if name.is_empty()
            || definition.is_empty()
            || definitions.insert(name, definition).is_some()
        {
            return Err(Error::InvalidArtifact);
        }
    }
    let prefix = format!("{lawpack}.");
    for (name, value) in map(field(module, "types")?)? {
        spend(&mut remaining)?;
        let name = text(name)?;
        if !name.starts_with(&prefix) {
            continue;
        }
        let mut expected = *definitions.get(name).ok_or(Error::InvalidArtifact)?;
        let mut seen = BTreeSet::new();
        while let Some(next) = definitions.get(expected) {
            spend(&mut remaining)?;
            if seen.len() >= MAX_DEPTH || !seen.insert(expected) {
                return Err(Error::InvalidArtifact);
            }
            expected = next;
        }
        if render(name, value, &mut remaining)? != expected {
            return Err(Error::InvalidArtifact);
        }
    }
    Ok(())
}
fn spend(remaining: &mut usize) -> Result<(), Error> {
    *remaining = remaining.checked_sub(1).ok_or(Error::UnsupportedProgram)?;
    Ok(())
}
fn reference<'a>(value: &'a Value, remaining: &mut usize) -> Result<&'a str, Error> {
    spend(remaining)?;
    let value = text(value)?;
    if value.is_empty() || value.len() > 1024 {
        return Err(Error::InvalidArtifact);
    }
    let mut depth = 0usize;
    for byte in value.bytes() {
        match byte {
            b'<' => {
                depth += 1;
                if depth > MAX_DEPTH {
                    return Err(Error::UnsupportedProgram);
                }
            }
            b'>' => {
                depth = depth.checked_sub(1).ok_or(Error::InvalidArtifact)?;
            }
            _ => {}
        }
    }
    if depth != 0 {
        return Err(Error::InvalidArtifact);
    }
    Ok(value)
}
fn render(name: &str, value: &Value, remaining: &mut usize) -> Result<String, Error> {
    spend(remaining)?;
    let kind = text_field(value, "kind")?;
    match kind {
        "Bool" | "Unit" | "I8" | "I16" | "I32" | "I64" | "U8" | "U16" | "U32" | "U64" => {
            exact_fields(value, &["kind"])?;
            Ok(kind.to_owned())
        }
        "Bytes" => {
            let max = number(field(value, "max")?)?;
            if let Ok(min) = field(value, "min") {
                exact_fields(value, &["kind", "min", "max"])?;
                let min = number(min)?;
                if min > max {
                    return Err(Error::InvalidArtifact);
                }
                if min == max {
                    Ok(format!("Bytes<exact={max}>"))
                } else {
                    Ok(format!("Bytes<min={min},max={max}>"))
                }
            } else {
                exact_fields(value, &["kind", "max"])?;
                Ok(format!("Bytes<max={max}>"))
            }
        }
        "String" => {
            exact_fields(value, &["kind", "max", "canonical"])?;
            let max = number(field(value, "max")?)?;
            let canonical = text_field(value, "canonical")?;
            if !["raw-utf8", "unicode-scalar-nfc"].contains(&canonical) {
                return Err(Error::InvalidArtifact);
            }
            Ok(format!("String<max={max},canonical={canonical}>"))
        }
        "Record" => {
            exact_fields(value, &["kind", "fields"])?;
            let mut fields = BTreeMap::new();
            for (key, ty) in map(field(value, "fields")?)? {
                let key = text(key)?;
                if key.is_empty()
                    || !key.bytes().enumerate().all(|(i, c)| {
                        c == b'_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit())
                    })
                {
                    return Err(Error::InvalidArtifact);
                }
                if fields.insert(key, reference(ty, remaining)?).is_some() {
                    return Err(Error::InvalidArtifact);
                }
            }
            let fields = fields
                .into_iter()
                .map(|(name, ty)| format!("{name}:{ty}"))
                .collect::<Vec<_>>()
                .join(",");
            Ok(format!("Record<{fields}>"))
        }
        "Nominal" => {
            exact_fields(value, &["kind", "contract", "representation"])?;
            if text_field(value, "contract")? != name {
                return Err(Error::InvalidArtifact);
            }
            let representation = reference(field(value, "representation")?, remaining)?;
            Ok(format!("Nominal<{representation}>"))
        }
        "Option" | "CapabilityRef" => {
            exact_fields(value, &["kind", "item"])?;
            let item = reference(field(value, "item")?, remaining)?;
            Ok(format!("{kind}<{item}>"))
        }
        "List" => {
            exact_fields(value, &["kind", "item", "max"])?;
            let item = reference(field(value, "item")?, remaining)?;
            let max = number(field(value, "max")?)?;
            Ok(format!("List<{item},max={max}>"))
        }
        "Map" => {
            exact_fields(value, &["kind", "key", "value", "max"])?;
            let key = reference(field(value, "key")?, remaining)?;
            let item = reference(field(value, "value")?, remaining)?;
            let max = number(field(value, "max")?)?;
            Ok(format!("Map<{key},{item},max={max}>"))
        }
        "ExternalActionRequest" => {
            exact_fields(value, &["kind", "settlement"])?;
            let settlement = reference(field(value, "settlement")?, remaining)?;
            Ok(format!("edict.external-action.request/v1<{settlement}>"))
        }
        _ => Err(Error::UnsupportedProgram),
    }
}
