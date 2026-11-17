// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Typed function failures (`core.failure/v1`), judged from Core alone.
//!
//! A source function may declare the failures it can end with. Its set is
//! absent for a total function, and otherwise nonempty, strictly ascending
//! and exactly what its body produces: each `fail` and each call to a
//! function that declares failures. Only an intent's `attempt` node may call
//! a failing function outside a function body, and its `failureMap` maps
//! exactly the callee's set to terminal obstruction reasons. A module needs
//! `core.failure/v1` exactly when some function declares failures.
//!
//! A record whose fields can fail is an `orderedRecord`: a nonempty array of
//! uniquely named `{name, value}` entries, evaluated in array order, at least
//! one of which can fail. A failure inside an unordered `record` field, an
//! `orderedRecord` none of whose fields can fail and a failing `fold` step
//! are refused, as Amblicit's Core integrity refuses them.
//!
//! warp-core, the provider lowerer and the provider verifier each hold an
//! identical copy of this file and must reach the same verdicts.
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::{BTreeMap, BTreeSet};

/// The Core capability a module needs once any function declares failures.
pub(crate) const CAPABILITY: &str = "core.failure/v1";
/// Deeper Core is refused rather than walked.
const DEPTH: usize = 256;

type Check<T> = Result<T, ()>;

fn get<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    let Value::Map(fields) = value else {
        return None;
    };
    fields
        .iter()
        .find_map(|(name, value)| matches!(name, Value::Text(name) if name == key).then_some(value))
}
fn required<'a>(value: &'a Value, key: &str) -> Check<&'a Value> {
    get(value, key).ok_or(())
}
fn text(value: &Value) -> Check<&str> {
    match value {
        Value::Text(text) => Ok(text),
        _ => Err(()),
    }
}
fn exact(value: &Value, keys: &[&str]) -> Check<()> {
    match value {
        Value::Map(fields)
            if fields.len() == keys.len() && keys.iter().all(|key| get(value, key).is_some()) =>
        {
            Ok(())
        }
        _ => Err(()),
    }
}

/// A source function's declared failures: none when `failures` is absent,
/// otherwise a nonempty, strictly ascending list of nonempty coordinates.
pub(crate) fn declared(function: &Value) -> Check<Vec<&str>> {
    let Some(failures) = get(function, "failures") else {
        return Ok(Vec::new());
    };
    let Value::Array(items) = failures else {
        return Err(());
    };
    let set = items.iter().map(text).collect::<Check<Vec<_>>>()?;
    if set.is_empty()
        || set.iter().any(|failure| failure.is_empty())
        || !set.windows(2).all(|pair| pair[0] < pair[1])
    {
        return Err(());
    }
    Ok(set)
}

/// The judged failure sets of one module, keyed by full callee coordinate.
pub(crate) struct Sets<'a> {
    declared: BTreeMap<String, Vec<&'a str>>,
}

impl<'a> Sets<'a> {
    /// The failures a call to `callee` can end with; none for a total or
    /// non-source callee.
    pub(crate) fn of(&self, callee: &str) -> &[&'a str] {
        self.declared.get(callee).map_or(&[], Vec::as_slice)
    }

    /// Adds every failure `value` can produce to `found`.
    fn produced(&self, value: &Value, found: &mut BTreeSet<String>, depth: usize) -> Check<()> {
        if depth > DEPTH {
            return Err(());
        }
        match value {
            Value::Array(items) => {
                for item in items {
                    self.produced(item, found, depth + 1)?;
                }
            }
            Value::Map(fields) => {
                match get(value, "kind").map(text) {
                    // A literal is data, whatever it spells.
                    Some(Ok("const")) => return Ok(()),
                    Some(Ok("fail")) => {
                        exact(value, &["kind", "failure"])?;
                        let failure = text(required(value, "failure")?)?;
                        if failure.is_empty() {
                            return Err(());
                        }
                        found.insert(failure.to_owned());
                        return Ok(());
                    }
                    Some(Ok("attempt")) => return Err(()),
                    Some(Ok("orderedRecord")) => {
                        exact(value, &["kind", "fields"])?;
                        let Value::Array(entries) = required(value, "fields")? else {
                            return Err(());
                        };
                        let mut names = BTreeSet::new();
                        let mut inner = BTreeSet::new();
                        for entry in entries {
                            exact(entry, &["name", "value"])?;
                            if !names.insert(text(required(entry, "name")?)?) {
                                return Err(());
                            }
                            self.produced(required(entry, "value")?, &mut inner, depth + 1)?;
                        }
                        // An empty or wholly total ordered record has one
                        // canonical spelling: the unordered `record`.
                        if inner.is_empty() {
                            return Err(());
                        }
                        found.extend(inner);
                        return Ok(());
                    }
                    Some(Ok("call")) => {
                        let callee = text(required(value, "callee")?)?;
                        found.extend(self.of(callee).iter().map(|&failure| failure.to_owned()));
                    }
                    Some(Ok("fold")) => {
                        if !self.of(text(required(value, "step")?)?).is_empty() {
                            return Err(());
                        }
                    }
                    Some(Ok("record")) => {
                        let mut inner = BTreeSet::new();
                        for (_, field) in fields {
                            self.produced(field, &mut inner, depth + 1)?;
                        }
                        return if inner.is_empty() { Ok(()) } else { Err(()) };
                    }
                    _ => {}
                }
                for (_, field) in fields {
                    self.produced(field, found, depth + 1)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Refuses a `value` that can fail.
    fn total(&self, value: &Value) -> Check<()> {
        let mut found = BTreeSet::new();
        self.produced(value, &mut found, 0)?;
        if found.is_empty() {
            Ok(())
        } else {
            Err(())
        }
    }

    /// An `attempt` node: a call to a failing function with failure-free
    /// arguments, whose map holds exactly the callee's failures, each to a
    /// terminal reason with failure-free payload fields.
    fn attempt(&self, node: &Value) -> Check<()> {
        exact(node, &["kind", "binding", "value", "failureMap"])?;
        let call = required(node, "value")?;
        exact(call, &["kind", "callee", "args", "typeArgs"])?;
        if text(required(call, "kind")?)? != "call" {
            return Err(());
        }
        let declared = self.of(text(required(call, "callee")?)?);
        if declared.is_empty() {
            return Err(());
        }
        self.total(required(call, "args")?)?;
        let Value::Map(arms) = required(node, "failureMap")? else {
            return Err(());
        };
        let mut mapped = BTreeSet::new();
        for (failure, reason) in arms {
            if !mapped.insert(text(failure)?) {
                return Err(());
            }
            exact(reason, &["reasonKind", "payload"])?;
            if text(required(reason, "reasonKind")?)?.is_empty() {
                return Err(());
            }
            let Value::Map(payload) = required(reason, "payload")? else {
                return Err(());
            };
            for (name, value) in payload {
                text(name)?;
                self.total(value)?;
            }
        }
        if mapped.iter().ne(declared.iter()) {
            return Err(());
        }
        Ok(())
    }
}

/// Judges every declared set, `fail` and `attempt` in `core`, whose source
/// functions are named under `coordinate`, and the `core.failure/v1`
/// capability, and returns the module's sets.
pub(crate) fn judge<'a>(core: &'a Value, coordinate: &str) -> Check<Sets<'a>> {
    let functions = match get(core, "functions") {
        None => &[][..],
        Some(Value::Map(functions)) => functions.as_slice(),
        Some(_) => return Err(()),
    };
    let mut sets = Sets {
        declared: BTreeMap::new(),
    };
    for (name, function) in functions {
        let set = declared(function)?;
        if !set.is_empty() {
            sets.declared
                .insert(format!("{coordinate}.{}", text(name)?), set);
        }
    }
    let capable = match get(core, "requiredCoreCapabilities") {
        Some(Value::Array(capabilities)) => capabilities
            .iter()
            .any(|capability| text(capability) == Ok(CAPABILITY)),
        _ => false,
    };
    if capable == sets.declared.is_empty() {
        return Err(());
    }
    // Shapes are the other judgments' concern: this walks whatever is
    // present and refuses only what can fail where it must not.
    for (name, function) in functions {
        let mut found = BTreeSet::new();
        sets.produced(function, &mut found, 0)?;
        let declared = sets.of(&format!("{coordinate}.{}", text(name)?));
        if found
            .iter()
            .map(String::as_str)
            .ne(declared.iter().copied())
        {
            return Err(());
        }
    }
    let intents = match get(core, "intents") {
        Some(Value::Map(intents)) => intents.as_slice(),
        _ => &[][..],
    };
    for (_, intent) in intents {
        let Value::Map(fields) = intent else {
            sets.total(intent)?;
            continue;
        };
        for (key, value) in fields {
            let Value::Map(body) = value else {
                sets.total(value)?;
                continue;
            };
            if text(key)? != "body" {
                sets.total(value)?;
                continue;
            }
            for (key, value) in body {
                match value {
                    Value::Array(nodes) if text(key)? == "nodes" => {
                        for node in nodes {
                            if get(node, "kind").map(text) == Some(Ok("attempt")) {
                                sets.attempt(node)?;
                            } else {
                                sets.total(node)?;
                            }
                        }
                    }
                    _ => sets.total(value)?,
                }
            }
        }
    }
    Ok(sets)
}
