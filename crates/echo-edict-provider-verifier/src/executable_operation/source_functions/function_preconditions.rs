// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Source-function preconditions over untrusted Core: their admission and
//! the obligation every call creates.
//!
//! Normative source: Amblicit's language specification, "Function
//! Preconditions" and "Core Source Function Shape" (Amblicit `1196396b`,
//! `EDICT-TARGET-FN-PRECONDITION-001`). A function's `preconditions` are
//! never evaluated. Its body may assume them, and every call must prove each
//! one, with the arguments substituted for the parameters, from the facts
//! that dominate the call. This module owns the admission of the field and the
//! call-site re-proof; the judgment that types each precondition and seeds the
//! body with them stays in the calling judgment.
//!
//! The facts at each Core position follow the specification's table. An
//! intent's `basis` and each `inputConstraints` predicate start with none; the
//! intent body starts with every input constraint; a function body starts with
//! its own preconditions. An `if` adds its predicate to its `then` arm and the
//! predicate's negation to its `else` arm; item `k` of an `all` adds the items
//! before it, and item `k` of an `any` the negation of each item before it.
//! Statement-level branches and everything else add no fact.
//!
//! An obligation is never built: comparison walks the callee's precondition
//! and reads `args[N]` wherever it meets `params[N]`, in one simultaneous
//! pass, so no obligation can exceed a depth limit, and an inserted argument
//! is never substituted again.
use echo_edict_canonical::CanonicalValueV1 as Value;

mod call_sites;
pub(crate) use call_sites::check_calls;

/// The most precondition conjuncts a function may hold, counted through
/// nested `all` (`MAX_FUNCTION_PRECONDITION_CONJUNCTS`).
pub(crate) const MAX_CONJUNCTS: usize = 16;
/// The most Core nodes a function's preconditions may hold, one per
/// predicate and one per expression (`MAX_FUNCTION_PRECONDITION_NODES`).
pub(crate) const MAX_NODES: usize = 256;
/// The work one module's call-site re-proof may spend: one unit per Core
/// value visited, per fact read and per pair of values compared. A module
/// that needs more is refused rather than judged without a bound.
pub(crate) const MAX_WORK: usize = 1 << 22;
/// Nesting beyond the canonical decoder's limit cannot come from an admitted
/// artifact; a walk refuses it rather than recursing without a bound.
pub(super) const MAX_NESTING: usize = 2 * echo_edict_canonical::MAX_CANONICAL_NESTING_DEPTH_V1;

/// Why a module's preconditions are refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// The field, a parameter name or a precondition's shape is not Core.
    Malformed,
    /// A call does not establish a precondition, or the re-proof would
    /// exceed its work bound.
    Unproven,
}
pub(super) type Judged<T> = Result<T, Refusal>;

/// One precondition-bearing function: its parameters and its preconditions.
#[derive(Clone, Copy)]
pub(crate) struct Contract<'v> {
    pub(crate) params: &'v [Value],
    pub(crate) preconditions: &'v [Value],
}

pub(super) fn field<'v>(value: &'v Value, name: &str) -> Option<&'v Value> {
    let Value::Map(fields) = value else {
        return None;
    };
    fields
        .iter()
        .find_map(|(key, value)| matches!(key, Value::Text(key) if key == name).then_some(value))
}
pub(super) fn text<'v>(value: &'v Value, name: &str) -> Option<&'v str> {
    match field(value, name) {
        Some(Value::Text(text)) => Some(text),
        _ => None,
    }
}
pub(super) fn items<'v>(value: &'v Value, name: &str) -> Judged<&'v [Value]> {
    match field(value, name) {
        Some(Value::Array(items)) => Ok(items),
        _ => Err(Refusal::Malformed),
    }
}
pub(super) fn has_exactly(value: &Value, names: &[&str]) -> bool {
    matches!(value, Value::Map(fields)
        if fields.len() == names.len()
            && fields
                .iter()
                .all(|(key, _)| matches!(key, Value::Text(key) if names.contains(&key.as_str()))))
}

/// The admitted `preconditions` of the source function `function`, empty when
/// the field is absent. `module` is the Core module coordinate and `functions`
/// its function table, so a call to a function of the same table is found.
///
/// A present list is nonempty, the parameters are `arg.N` / `$argN` in order,
/// every precondition uses only `true`, `false`, `not`, `all`, `any` and
/// comparisons (also inside a conditional expression in an operand), no
/// expression is a `fold`, a `match` or a call to a function of `functions`,
/// and the list holds at most [`MAX_CONJUNCTS`] conjuncts and [`MAX_NODES`]
/// Core nodes. Typing, totality and the parameters-only scope are judged by
/// the caller, which knows the types.
pub(crate) fn declared<'v>(
    function: &'v Value,
    module: &str,
    functions: &Value,
) -> Judged<&'v [Value]> {
    if has_exactly(function, &["params", "returnType", "body"]) {
        return Ok(&[]);
    }
    if !has_exactly(function, &["params", "returnType", "preconditions", "body"]) {
        return Err(Refusal::Malformed);
    }
    let preconditions = items(function, "preconditions")?;
    if preconditions.is_empty() {
        return Err(Refusal::Malformed);
    }
    for (index, param) in items(function, "params")?.iter().enumerate() {
        if text(param, "id") != Some(format!("arg.{index}").as_str())
            || text(param, "alphaName") != Some(format!("${}", format_args!("arg{index}")).as_str())
        {
            return Err(Refusal::Malformed);
        }
    }
    let mut shape = Shape {
        module,
        functions,
        nodes: 0,
    };
    let mut conjuncts = 0;
    for precondition in preconditions {
        shape.predicate(precondition, 0)?;
        conjuncts += conjunct_count(precondition, 0)?;
        if conjuncts > MAX_CONJUNCTS {
            return Err(Refusal::Malformed);
        }
    }
    Ok(preconditions)
}

fn conjunct_count(predicate: &Value, depth: usize) -> Judged<usize> {
    if depth > MAX_NESTING {
        return Err(Refusal::Malformed);
    }
    if text(predicate, "kind") != Some("all") {
        return Ok(1);
    }
    let mut count = 0usize;
    for item in items(predicate, "values")? {
        count = count.saturating_add(conjunct_count(item, depth + 1)?);
    }
    Ok(count)
}

/// The closed precondition shape and its Core node count.
struct Shape<'m> {
    module: &'m str,
    functions: &'m Value,
    nodes: usize,
}
impl Shape<'_> {
    fn node(&mut self, depth: usize) -> Judged<()> {
        self.nodes += 1;
        if self.nodes > MAX_NODES || depth > MAX_NESTING {
            return Err(Refusal::Malformed);
        }
        Ok(())
    }
    fn predicate(&mut self, value: &Value, depth: usize) -> Judged<()> {
        self.node(depth)?;
        match text(value, "kind") {
            Some("true" | "false") if has_exactly(value, &["kind"]) => Ok(()),
            Some("not") if has_exactly(value, &["kind", "value"]) => {
                self.predicate(field(value, "value").ok_or(Refusal::Malformed)?, depth + 1)
            }
            Some("all" | "any") if has_exactly(value, &["kind", "values"]) => {
                let values = items(value, "values")?;
                if values.is_empty() {
                    return Err(Refusal::Malformed);
                }
                values
                    .iter()
                    .try_for_each(|item| self.predicate(item, depth + 1))
            }
            Some("compare") if has_exactly(value, &["kind", "op", "left", "right"]) => {
                for side in ["left", "right"] {
                    self.expression(field(value, side).ok_or(Refusal::Malformed)?, depth + 1)?;
                }
                Ok(())
            }
            _ => Err(Refusal::Malformed),
        }
    }
    fn expression(&mut self, value: &Value, depth: usize) -> Judged<()> {
        self.node(depth)?;
        match text(value, "kind") {
            Some("local" | "const") => Ok(()),
            Some("field") => {
                self.expression(field(value, "base").ok_or(Refusal::Malformed)?, depth + 1)
            }
            Some("variant") => match field(value, "payload") {
                Some(payload) => self.expression(payload, depth + 1),
                None => Ok(()),
            },
            Some("record") => match field(value, "fields") {
                Some(Value::Map(fields)) => fields
                    .iter()
                    .try_for_each(|(_, item)| self.expression(item, depth + 1)),
                _ => Err(Refusal::Malformed),
            },
            Some("list") => items(value, "values")?
                .iter()
                .try_for_each(|item| self.expression(item, depth + 1)),
            Some("call") => {
                let callee = text(value, "callee").ok_or(Refusal::Malformed)?;
                let own = callee
                    .strip_prefix(self.module)
                    .and_then(|rest| rest.strip_prefix('.'))
                    .is_some_and(|name| field(self.functions, name).is_some());
                if own {
                    return Err(Refusal::Malformed);
                }
                items(value, "args")?
                    .iter()
                    .try_for_each(|item| self.expression(item, depth + 1))
            }
            Some("if") => {
                self.predicate(
                    field(value, "predicate").ok_or(Refusal::Malformed)?,
                    depth + 1,
                )?;
                for arm in ["then", "else"] {
                    self.expression(field(value, arm).ok_or(Refusal::Malformed)?, depth + 1)?;
                }
                Ok(())
            }
            // A fold, a match and every other form cannot be instantiated by
            // substitution alone.
            _ => Err(Refusal::Malformed),
        }
    }
}
