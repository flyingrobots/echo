// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Exact correspondence between Core blocks and the Target order tree.
use crate::edict_pure::{
    model::{MAX_DEPTH, MAX_PROGRAM_NODES},
    values::{array, exact_fields, field, require_text, text_field},
    EvaluationError as Error,
};
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::{BTreeMap, BTreeSet};

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
    if !walk.tables.is_empty() || (walk.pairs.is_empty() && !walk.identities.is_empty()) {
        return Err(Error::InvalidArtifact);
    }
    Ok(walk.pairs)
}

struct Walk<'a> {
    tables: BTreeMap<&'a str, (&'static str, &'a Value)>,
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
                    self.child(field(node, "body")?, field(entry, "body")?, depth + 1)?;
                }
                "branch" => {
                    exact_fields(node, &["kind", "predicate", "then", "else"])?;
                    exact_fields(entry, &["kind", "id", "predicate", "then", "else"])?;
                    if field(node, "predicate")? != field(entry, "predicate")? {
                        return Err(Error::InvalidArtifact);
                    }
                    for arm in ["then", "else"] {
                        self.child(field(node, arm)?, field(entry, arm)?, depth + 1)?;
                    }
                }
                _ => return Err(Error::UnsupportedProgram),
            }
        }
        Ok(())
    }
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
