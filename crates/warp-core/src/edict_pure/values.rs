// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use echo_edict_canonical::CanonicalValueV1 as Value;

use super::EvaluationError as Error;

pub(super) fn map(value: &Value) -> Result<&[(Value, Value)], Error> {
    if let Value::Map(fields) = value {
        Ok(fields)
    } else {
        Err(Error::InvalidArtifact)
    }
}

pub(super) fn field<'a>(value: &'a Value, name: &str) -> Result<&'a Value, Error> {
    map(value)?
        .iter()
        .find_map(|(key, value)| {
            if matches!(key, Value::Text(key) if key == name) {
                Some(value)
            } else {
                None
            }
        })
        .ok_or(Error::InvalidArtifact)
}

pub(super) fn text(value: &Value) -> Result<&str, Error> {
    if let Value::Text(value) = value {
        Ok(value)
    } else {
        Err(Error::InvalidArtifact)
    }
}

pub(super) fn text_field<'a>(value: &'a Value, name: &str) -> Result<&'a str, Error> {
    text(field(value, name)?)
}

pub(super) fn number(value: &Value) -> Result<u64, Error> {
    if let Value::Integer(value) = value {
        u64::try_from(*value).map_err(|_| Error::InvalidArtifact)
    } else {
        Err(Error::InvalidArtifact)
    }
}

pub(super) fn bytes(value: &Value) -> Result<&[u8], Error> {
    if let Value::Bytes(value) = value {
        Ok(value)
    } else {
        Err(Error::InvalidArtifact)
    }
}

pub(super) fn array(value: &Value) -> Result<&[Value], Error> {
    if let Value::Array(value) = value {
        Ok(value)
    } else {
        Err(Error::InvalidArtifact)
    }
}

pub(super) fn exact_fields(value: &Value, names: &[&str]) -> Result<(), Error> {
    let fields = map(value)?;
    if fields.len() != names.len() || names.iter().any(|name| field(value, name).is_err()) {
        return Err(Error::InvalidArtifact);
    }
    Ok(())
}

pub(super) fn require_text(value: &Value, name: &str, expected: &str) -> Result<(), Error> {
    if text_field(value, name)? == expected {
        Ok(())
    } else {
        Err(Error::UnsupportedProgram)
    }
}
