// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! This module owns the typing of Core ordered records (`orderedRecord`).
//!
//! An ordered record is the Core spelling of a record whose fields can fail:
//! its `fields` array keeps the authored evaluation order. A `fail` field
//! returns no value, so it takes its type from the field the enclosing
//! declared type expects: a helper's result or a binding's type. Without
//! that expectation, as in a list item or a call argument, a bare `fail`
//! field is refused as unsupported.
use super::{
    functions::{infer_arm, Definition, Proof, Scope},
    model::RuntimeType,
    syntax::Parser,
    values::{array, exact_fields, field, text_field},
    EvaluationError as Error,
};
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::BTreeMap;

/// The record type of an `orderedRecord`, its fields named in authored order.
/// `expected`, when present, types each `fail` field. The caller has already
/// entered this node.
#[allow(
    clippy::too_many_arguments,
    reason = "The judgment context of `infer`, plus the expected type."
)]
pub(super) fn infer<'v>(
    value: &'v Value,
    expected: Option<&RuntimeType>,
    scope: &Scope<'_>,
    proof: Proof<'_, 'v>,
    definitions: &BTreeMap<String, Definition<'_>>,
    source: bool,
    parser: &mut Parser<'_>,
    depth: usize,
) -> Result<RuntimeType, Error> {
    exact_fields(value, &["kind", "fields"])?;
    let entries = array(field(value, "fields")?)?;
    if entries.is_empty() {
        return Err(Error::InvalidArtifact);
    }
    let expected_fields = match expected.map(RuntimeType::representation) {
        Some(RuntimeType::Record(fields)) => Some(fields),
        _ => None,
    };
    let mut fields: Vec<(String, RuntimeType)> = Vec::new();
    for entry in entries {
        exact_fields(entry, &["name", "value"])?;
        let name = text_field(entry, "name")?;
        if fields.iter().any(|(prior, _)| prior == name) {
            return Err(Error::InvalidArtifact);
        }
        let want = expected_fields
            .and_then(|types| types.iter().find(|(key, _)| key == name))
            .map(|(_, ty)| ty);
        let expression = field(entry, "value")?;
        let ty = if text_field(expression, "kind")? == "orderedRecord" {
            parser.enter(depth + 1)?;
            infer(
                expression,
                want,
                scope,
                proof,
                definitions,
                source,
                parser,
                depth + 1,
            )?
        } else {
            match infer_arm(
                expression,
                scope,
                proof,
                definitions,
                source,
                parser,
                depth + 1,
            )? {
                Some(ty) => ty,
                None => want.cloned().ok_or(Error::UnsupportedProgram)?,
            }
        };
        fields.push((name.to_owned(), ty));
    }
    Ok(RuntimeType::Record(fields))
}
