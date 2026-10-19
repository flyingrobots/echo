// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Bounded wire checks for Edict's distinct source-closure artifact.
//!
//! This verifies envelope shape, source identities and the selected root. It
//! does not parse source, authenticate import pins or prove graph reachability.
use std::collections::BTreeSet;

use crate::{digest_canonical_value_bytes_v1, CanonicalValueV1 as Value};

/// Stable source-closure encoding refusals, separate from compiler semantics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceClosureEncodingError {
    /// The closed artifact or module envelope has the wrong fields or types.
    InvalidShape,
    /// The module map is empty or exceeds the fixed 64-unit ceiling.
    ModuleLimit,
    /// Aggregate original source bytes exceed the fixed 1 MiB ceiling.
    ByteLimit,
    /// A module source is not UTF-8.
    InvalidUtf8,
    /// A module digest is not a typed SHA-256 value of exactly 32 bytes.
    InvalidDigest,
    /// A module's supplied digest differs from its exact source identity.
    DigestMismatch,
    /// Root selection differs from the expected Core coordinate or is absent.
    RootMismatch,
}

impl std::fmt::Display for SourceClosureEncodingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "source-closure encoding: {self:?}")
    }
}
impl std::error::Error for SourceClosureEncodingError {}

/// Check a decoded source-closure envelope and borrow its original root text.
///
/// The caller must separately validate the artifact's canonical encoding and
/// outer domain-framed identity. This function checks every retained module's
/// exact source digest and UTF-8 under fixed aggregate bounds. It performs no
/// source parsing, graph authentication, discovery or runtime admission.
///
/// # Errors
/// Returns a stable refusal for malformed, oversized, rebound or substituted
/// source material. A valid envelope is not a compiler authentication witness.
pub fn source_closure_root_v1<'a>(
    value: &'a Value,
    expected_coordinate: &str,
) -> Result<&'a str, SourceClosureEncodingError> {
    use SourceClosureEncodingError as Error;
    let envelope = object(value, &["apiVersion", "root", "modules"])?;
    if field(envelope, "apiVersion")? != &Value::Text("edict.source-closure/v1".into()) {
        return Err(Error::InvalidShape);
    }
    let Value::Text(root) = field(envelope, "root")? else {
        return Err(Error::InvalidShape);
    };
    if root != expected_coordinate || root.is_empty() {
        return Err(Error::RootMismatch);
    }
    let Value::Map(modules) = field(envelope, "modules")? else {
        return Err(Error::InvalidShape);
    };
    if modules.is_empty() || modules.len() > 64 {
        return Err(Error::ModuleLimit);
    }
    let mut total = 0_usize;
    let mut coordinates = BTreeSet::new();
    let mut root_source = None;
    for (coordinate, unit) in modules {
        let Value::Text(coordinate) = coordinate else {
            return Err(Error::InvalidShape);
        };
        if coordinate.is_empty() || !coordinates.insert(coordinate) {
            return Err(Error::InvalidShape);
        }
        let unit = object(unit, &["source", "digest"])?;
        let source = field(unit, "source")?;
        let Value::Bytes(bytes) = source else {
            return Err(Error::InvalidShape);
        };
        total = total
            .checked_add(bytes.len())
            .filter(|total| *total <= 1024 * 1024)
            .ok_or(Error::ByteLimit)?;
        let source_text = std::str::from_utf8(bytes).map_err(|_| Error::InvalidUtf8)?;
        let Value::Array(digest) = field(unit, "digest")? else {
            return Err(Error::InvalidDigest);
        };
        let [Value::Text(algorithm), Value::Bytes(hash)] = digest.as_slice() else {
            return Err(Error::InvalidDigest);
        };
        if algorithm != "sha256" || hash.len() != 32 {
            return Err(Error::InvalidDigest);
        }
        let expected = digest_canonical_value_bytes_v1("edict.source/v1", source)
            .map_err(|_| Error::InvalidShape)?;
        if hash.as_slice() != expected {
            return Err(Error::DigestMismatch);
        }
        if coordinate == root {
            root_source = Some(source_text);
        }
    }
    root_source.ok_or(Error::RootMismatch)
}

fn object<'a>(
    value: &'a Value,
    names: &[&str],
) -> Result<&'a [(Value, Value)], SourceClosureEncodingError> {
    let Value::Map(fields) = value else {
        return Err(SourceClosureEncodingError::InvalidShape);
    };
    if fields.len() != names.len() {
        return Err(SourceClosureEncodingError::InvalidShape);
    }
    let mut seen = BTreeSet::new();
    for (key, _) in fields {
        let Value::Text(key) = key else {
            return Err(SourceClosureEncodingError::InvalidShape);
        };
        if !names.contains(&key.as_str()) || !seen.insert(key) {
            return Err(SourceClosureEncodingError::InvalidShape);
        }
    }
    Ok(fields)
}
fn field<'a>(
    fields: &'a [(Value, Value)],
    key: &str,
) -> Result<&'a Value, SourceClosureEncodingError> {
    fields
        .iter()
        .find_map(|(name, value)| match name {
            Value::Text(name) if name == key => Some(value),
            _ => None,
        })
        .ok_or(SourceClosureEncodingError::InvalidShape)
}
