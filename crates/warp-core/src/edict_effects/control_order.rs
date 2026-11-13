// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Exact correspondence between Core blocks and the Target execution order.
//!
//! An ordered (`echo.span-ir/v2`) order is a flat list of instruction ids. A
//! structured (`echo.span-ir/v3`) order is a tree: `for` and `branch` entries
//! copy the Core loop or branch they lower and nest their own orders. Either
//! way every instruction of the Target tables appears exactly once, at the
//! position of a Core node of the same kind, and control ids are nonempty,
//! unique and distinct from instruction ids.
use crate::edict_pure::{
    model::{MAX_DEPTH, MAX_PROGRAM_NODES},
    values::{array, exact_fields, field, require_text, text_field},
    EvaluationError as Error,
};
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::{BTreeMap, BTreeSet};

/// The structured Target IR contract.
pub(super) const STRUCTURED_TARGET_IR: &str = "echo.span-ir/v3";

/// Each Core instruction node paired with its Target instruction, in
/// pre-order: a loop's or branch's nodes follow the control node itself.
pub(super) fn instructions<'a>(
    intent: &'a Value,
    target: &'a Value,
) -> Result<Vec<(&'a Value, &'a Value)>, Error> {
    let mut tables = BTreeMap::new();
    for (collection, kind) in [
        ("steps", "effect"),
        ("requirements", "require"),
        ("pureBindings", "let"),
    ] {
        let values = match field(target, collection) {
            Ok(value) => array(value)?,
            Err(_) if collection == "pureBindings" => continue,
            Err(error) => return Err(error),
        };
        for value in values {
            let id = text_field(value, "id")?;
            if id.is_empty() || tables.insert(id, (kind, value)).is_some() {
                return Err(Error::InvalidArtifact);
            }
        }
    }
    let identities = tables.keys().copied().collect();
    let mut walk = Walk {
        tables,
        identities,
        remaining: MAX_PROGRAM_NODES,
        pairs: Vec::new(),
    };
    walk.block(field(intent, "body")?, field(target, "executionOrder")?, 0)?;
    if !walk.tables.is_empty() {
        return Err(Error::InvalidArtifact);
    }
    Ok(walk.pairs)
}

struct Walk<'a> {
    /// Target instructions not yet placed, by id.
    tables: BTreeMap<&'a str, (&'static str, &'a Value)>,
    /// Every instruction and control id seen so far.
    identities: BTreeSet<&'a str>,
    remaining: usize,
    pairs: Vec<(&'a Value, &'a Value)>,
}
impl<'a> Walk<'a> {
    fn block(&mut self, body: &'a Value, order: &'a Value, depth: usize) -> Result<(), Error> {
        if depth > MAX_DEPTH {
            return Err(Error::UnsupportedProgram);
        }
        exact_fields(body, &["locals", "nodes", "result"])?;
        let nodes = array(field(body, "nodes")?)?;
        let entries = array(order)?;
        if nodes.len() != entries.len() {
            return Err(Error::InvalidArtifact);
        }
        for (node, entry) in nodes.iter().zip(entries) {
            self.remaining = self
                .remaining
                .checked_sub(1)
                .ok_or(Error::UnsupportedProgram)?;
            if let Value::Text(id) = entry {
                let (kind, target) = self
                    .tables
                    .remove(id.as_str())
                    .ok_or(Error::InvalidArtifact)?;
                require_text(node, "kind", kind)?;
                self.pairs.push((node, target));
                continue;
            }
            self.control(node, entry, depth)?;
        }
        Ok(())
    }
    /// A `for` or `branch` entry copies its Core node's binder, iterable,
    /// bound or predicate exactly and nests one order per Core block.
    fn control(&mut self, node: &'a Value, entry: &'a Value, depth: usize) -> Result<(), Error> {
        let id = text_field(entry, "id")?;
        if id.is_empty() || !self.identities.insert(id) {
            return Err(Error::InvalidArtifact);
        }
        let kind = text_field(node, "kind")?;
        require_text(entry, "kind", kind)?;
        match kind {
            "for" => {
                exact_fields(node, &["kind", "binder", "iter", "bound", "body"])?;
                exact_fields(
                    entry,
                    &["kind", "id", "binder", "iterable", "bound", "body"],
                )?;
                for (source, target) in [
                    ("binder", "binder"),
                    ("iter", "iterable"),
                    ("bound", "bound"),
                ] {
                    if field(node, source)? != field(entry, target)? {
                        return Err(Error::InvalidArtifact);
                    }
                }
                self.child(field(node, "body")?, field(entry, "body")?, depth + 1)
            }
            "branch" => {
                // A value-producing branch binds a local; only statement
                // branches lower to Target control entries.
                exact_fields(node, &["kind", "predicate", "then", "else"])?;
                exact_fields(entry, &["kind", "id", "predicate", "then", "else"])?;
                if field(node, "predicate")? != field(entry, "predicate")? {
                    return Err(Error::InvalidArtifact);
                }
                self.child(field(node, "then")?, field(entry, "then")?, depth + 1)?;
                self.child(field(node, "else")?, field(entry, "else")?, depth + 1)
            }
            _ => Err(Error::UnsupportedProgram),
        }
    }
    /// A loop body or branch arm is a statement block: its result is `null`.
    fn child(&mut self, body: &'a Value, order: &'a Value, depth: usize) -> Result<(), Error> {
        let result = field(body, "result")?;
        exact_fields(result, &["kind", "value"])?;
        require_text(result, "kind", "const")?;
        let value = field(result, "value")?;
        exact_fields(value, &["kind"])?;
        require_text(value, "kind", "null")?;
        self.block(body, order, depth)
    }
}
