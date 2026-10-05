// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Independent verifier judgment: inventory authority, eliminate dependency leaves,
//! then audit ordered lexical frames and conservative whole-operation charges.
mod types;
use super::ProviderRefusalV1;
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::{BTreeMap, BTreeSet};
use types::{Charge, Schema, Types};
type Check<T> = Result<T, ()>;

struct Definition<'a> {
    source: bool,
    params: &'a [Value],
    body: &'a Value,
    parameter_types: Vec<Schema>,
    result: Schema,
}
#[derive(Clone, Copy)]
struct Summary {
    charge: Charge,
    height: usize,
}
struct Expression {
    schema: Schema,
    charge: Charge,
    height: usize,
}
struct Frame<'a> {
    declared: BTreeMap<&'a str, &'a Value>,
    values: BTreeMap<&'a str, (&'a Value, Schema)>,
}
impl<'a> Frame<'a> {
    fn new(
        declarations: impl IntoIterator<Item = &'a Value>,
        types: &mut Types<'_>,
    ) -> Check<Self> {
        let mut declared = BTreeMap::new();
        for local in declarations {
            types.local(local)?;
            if declared.insert(string(get(local, "id")?)?, local).is_some() {
                return Err(());
            }
        }
        Ok(Self {
            declared,
            values: BTreeMap::new(),
        })
    }
    fn define(&mut self, local: &'a Value, schema: Schema) -> Check<()> {
        let id = string(get(local, "id")?)?;
        if self.declared.get(id).copied() != Some(local)
            || self.values.insert(id, (local, schema)).is_some()
        {
            return Err(());
        }
        Ok(())
    }
    fn complete(&self) -> Check<()> {
        if self.values.len() == self.declared.len() {
            Ok(())
        } else {
            Err(())
        }
    }
}

pub(super) fn validate(core: &Value, exports: &Value, read: bool) -> Result<(), ProviderRefusalV1> {
    if super::map_field(core, "functions").is_none() {
        return Ok(());
    }
    audit(core, exports, read)
        .map_err(|()| super::super::unsupported_semantics("core.source-functions-verification"))
}
fn audit(core: &Value, exports: &Value, read: bool) -> Check<()> {
    let mut types = Types::new(core);
    let definitions = inventory(core, exports, &mut types)?;
    let mut pending = BTreeMap::new();
    let mut callers: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (name, definition) in &definitions {
        let mut edges = BTreeSet::new();
        let mut expressions = Vec::new();
        for binding in sequence(get(definition.body, "bindings")?)? {
            expressions.push((get(binding, "value")?, 0));
        }
        expressions.push((get(definition.body, "result")?, 0));
        while let Some((expression, depth)) = expressions.pop() {
            types.step(depth)?;
            match string(get(expression, "kind")?)? {
                "const" | "local" => {}
                "field" => expressions.push((get(expression, "base")?, depth + 1)),
                "record" => {
                    for (_, value) in members(get(expression, "fields")?)? {
                        expressions.push((value, depth + 1));
                    }
                }
                "if" => {
                    let predicate = get(expression, "predicate")?;
                    expressions.push((get(predicate, "left")?, depth + 2));
                    expressions.push((get(predicate, "right")?, depth + 2));
                    expressions.push((get(expression, "then")?, depth + 1));
                    expressions.push((get(expression, "else")?, depth + 1));
                }
                "call" => {
                    let callee = string(get(expression, "callee")?)?;
                    if let Some(target) = definitions.get(callee) {
                        if !definition.source && target.source {
                            return Err(());
                        }
                        edges.insert(callee);
                    } else if ![
                        "core.bytes.length",
                        "core.bytes.concat",
                        "core.bytes.slice",
                        "core.integer.subtract",
                    ]
                    .contains(&callee)
                    {
                        return Err(());
                    }
                    for argument in sequence(get(expression, "args")?)? {
                        expressions.push((argument, depth + 1));
                    }
                }
                _ => return Err(()),
            }
        }
        pending.insert(name.as_str(), edges.len());
        for edge in edges {
            callers.entry(edge).or_default().push(name);
        }
    }
    let mut ready: BTreeSet<&str> = pending
        .iter()
        .filter_map(|(name, degree)| (*degree == 0).then_some(*name))
        .collect();
    let mut summaries = BTreeMap::new();
    // No native recursion crosses a function edge. Dependencies receive their
    // own complete summary before any caller can use it, in either name order.
    while let Some(name) = ready.pop_first() {
        types.step(0)?;
        let definition = definitions.get(name).ok_or(())?;
        let summary = Checker {
            types: &mut types,
            definitions: &definitions,
            summaries: &summaries,
        }
        .function(definition)?;
        if summary.height + 1 > 64 {
            return Err(());
        }
        summaries.insert(name.to_owned(), summary);
        if let Some(users) = callers.get(name) {
            for user in users {
                let degree = pending.get_mut(user).ok_or(())?;
                *degree = degree.checked_sub(1).ok_or(())?;
                if *degree == 0 {
                    ready.insert(user);
                }
            }
        }
    }
    if summaries.len() != definitions.len() {
        return Err(());
    }
    let mut checker = Checker {
        types: &mut types,
        definitions: &definitions,
        summaries: &summaries,
    };
    for (_, intent) in members(get(core, "intents")?)? {
        checker.intent(intent, read)?;
    }
    Ok(())
}
fn inventory<'a>(
    core: &'a Value,
    exports: &'a Value,
    types: &mut Types<'_>,
) -> Check<BTreeMap<String, Definition<'a>>> {
    let mut definitions = BTreeMap::new();
    for export in sequence(get(exports, "pureFunctions")?)? {
        if string(get(export, "source")?)? != "edict" {
            return Err(());
        }
        let implementation = get(export, "body")?;
        let body = get(implementation, "body")?;
        for (value, key) in [
            (export, "parameterTypes"),
            (export, "typeParameters"),
            (implementation, "params"),
            (body, "locals"),
            (body, "bindings"),
        ] {
            if !sequence(get(value, key)?)?.is_empty() {
                return Err(());
            }
        }
        exact(body, &["locals", "bindings", "result"])?;
        let name = string(get(export, "coordinate")?)?;
        if name.starts_with("core.")
            || definitions
                .insert(
                    name.to_owned(),
                    Definition {
                        source: false,
                        params: sequence(get(implementation, "params")?)?,
                        body,
                        parameter_types: vec![],
                        result: types.resolve(string(get(export, "returnType")?)?, 0)?,
                    },
                )
                .is_some()
        {
            return Err(());
        }
    }
    for (name, definition) in members(get(core, "functions")?)? {
        types.step(0)?;
        let name = string(name)?;
        if !name.bytes().enumerate().all(|(index, byte)| {
            byte == b'_' || byte.is_ascii_alphabetic() || (index > 0 && byte.is_ascii_digit())
        }) || super::map_field(get(core, "types")?, name).is_some()
            || super::map_field(get(core, "intents")?, name).is_some()
        {
            return Err(());
        }
        exact(definition, &["params", "returnType", "body"])?;
        let body = get(definition, "body")?;
        exact(body, &["locals", "bindings", "result"])?;
        let params = sequence(get(definition, "params")?)?;
        let mut parameter_types = Vec::new();
        for parameter in params {
            parameter_types.push(types.local(parameter)?);
        }
        let name = format!("{}.{}", string(get(core, "coordinate")?)?, name);
        if name.starts_with("core.")
            || definitions
                .insert(
                    name,
                    Definition {
                        source: true,
                        params,
                        body,
                        parameter_types,
                        result: types.resolve(string(get(definition, "returnType")?)?, 0)?,
                    },
                )
                .is_some()
        {
            return Err(());
        }
    }
    Ok(definitions)
}

struct Checker<'a, 'b> {
    types: &'b mut Types<'a>,
    definitions: &'b BTreeMap<String, Definition<'a>>,
    summaries: &'b BTreeMap<String, Summary>,
}
impl Checker<'_, '_> {
    fn function(&mut self, definition: &Definition<'_>) -> Check<Summary> {
        let declarations = sequence(get(definition.body, "locals")?)?;
        let mut frame = Frame::new(definition.params.iter().chain(declarations), self.types)?;
        let mut height = definition.result.measure()?.height;
        for (local, schema) in definition.params.iter().zip(&definition.parameter_types) {
            height = height.max(schema.measure()?.height);
            frame.define(local, self.types.copy_type(schema)?)?;
        }
        let mut charge = Charge::default();
        for binding in sequence(get(definition.body, "bindings")?)? {
            exact(binding, &["kind", "binding", "value"])?;
            if string(get(binding, "kind")?)? != "let" {
                return Err(());
            }
            let local = get(binding, "binding")?;
            let value = self.expression(get(binding, "value")?, &frame, 0)?;
            let declared = self.types.local(local)?;
            if !declared.encloses(&value.schema) {
                return Err(());
            }
            let size = declared.measure()?;
            charge.append(value.charge)?;
            charge.append(Charge {
                ticks: size.checks,
                storage: 0,
            })?;
            height = height.max(value.height).max(size.height);
            frame.define(local, declared)?;
        }
        frame.complete()?;
        let value = self.expression(get(definition.body, "result")?, &frame, 0)?;
        if !definition.result.encloses(&value.schema) {
            return Err(());
        }
        charge.append(value.charge)?;
        charge.append(Charge {
            ticks: definition.result.measure()?.checks,
            storage: 0,
        })?;
        Ok(Summary {
            charge,
            height: height.max(value.height),
        })
    }
    fn expression(&mut self, value: &Value, frame: &Frame<'_>, depth: usize) -> Check<Expression> {
        self.types.step(depth)?;
        let mut charge = Charge {
            ticks: 1,
            storage: 0,
        };
        let mut height = 0;
        let schema = match string(get(value, "kind")?)? {
            "const" => {
                exact(value, &["kind", "value"])?;
                let literal = get(value, "value")?;
                exact(literal, &["kind", "value", "width"])?;
                let width = string(get(literal, "width")?)?;
                if string(get(literal, "kind")?)? != "int" || !["U32", "U64"].contains(&width) {
                    return Err(());
                }
                let Schema::Word(max) = self.types.resolve(width, 0)? else {
                    return Err(());
                };
                if unsigned(get(literal, "value")?)? > max {
                    return Err(());
                }
                charge.append(Charge {
                    ticks: 1,
                    storage: 64,
                })?;
                Schema::Word(max)
            }
            "local" => {
                exact(value, &["kind", "ref"])?;
                let local = get(value, "ref")?;
                let (declared, schema) = frame.values.get(string(get(local, "id")?)?).ok_or(())?;
                if *declared != local {
                    return Err(());
                }
                charge.append(schema.measure()?.copy)?;
                self.types.copy_type(schema)?
            }
            "field" => {
                exact(value, &["kind", "base", "field"])?;
                let base = self.expression(get(value, "base")?, frame, depth + 1)?;
                charge.append(base.charge)?;
                height = base.height + 1;
                let Schema::Fields(mut fields) = base.schema else {
                    return Err(());
                };
                fields.remove(string(get(value, "field")?)?).ok_or(())?
            }
            "record" => {
                exact(value, &["kind", "fields"])?;
                charge.storage = 64;
                let mut fields = BTreeMap::new();
                for (name, child) in members(get(value, "fields")?)? {
                    let name = string(name)?;
                    let child = self.expression(child, frame, depth + 1)?;
                    height = height.max(child.height + 1);
                    charge.append(child.charge)?;
                    charge.append(Charge {
                        ticks: 0,
                        storage: 64 + name.len() as u128,
                    })?;
                    if fields.insert(name.to_owned(), child.schema).is_some() {
                        return Err(());
                    }
                }
                Schema::Fields(fields)
            }
            "if" => {
                exact(value, &["kind", "predicate", "then", "else"])?;
                let (predicate, predicate_height) =
                    self.predicate(get(value, "predicate")?, frame, depth + 1)?;
                let yes = self.expression(get(value, "then")?, frame, depth + 1)?;
                let no = self.expression(get(value, "else")?, frame, depth + 1)?;
                height = predicate_height.max(yes.height).max(no.height) + 1;
                charge.append(predicate)?;
                charge.append(Charge::choice(yes.charge, no.charge))?;
                yes.schema.choice(no.schema)?
            }
            "call" => {
                exact(value, &["kind", "callee", "args", "typeArgs"])?;
                let name = string(get(value, "callee")?)?;
                let arguments = sequence(get(value, "args")?)?;
                let types = sequence(get(value, "typeArgs")?)?;
                let mut argument_schemas = Vec::new();
                for argument in arguments {
                    let argument = self.expression(argument, frame, depth + 1)?;
                    charge.append(argument.charge)?;
                    height = height.max(argument.height + 1);
                    argument_schemas.push(argument.schema);
                }
                if let Some(definition) = self.definitions.get(name) {
                    if !types.is_empty() || arguments.len() != definition.parameter_types.len() {
                        return Err(());
                    }
                    for (expected, actual) in
                        definition.parameter_types.iter().zip(&argument_schemas)
                    {
                        if !expected.encloses(actual) {
                            return Err(());
                        }
                        let size = expected.measure()?;
                        charge.append(Charge {
                            ticks: size.checks,
                            storage: 0,
                        })?;
                        height = height.max(size.height + 1);
                    }
                    let summary = self.summaries.get(name).ok_or(())?;
                    charge.append(summary.charge)?;
                    height = height.max(summary.height + 1);
                    self.types.copy_type(&definition.result)?
                } else {
                    let mut declared = Vec::new();
                    for coordinate in types {
                        declared.push(self.types.resolve(string(coordinate)?, depth + 1)?);
                    }
                    height = height.max(1);
                    let (schema, intrinsic) =
                        intrinsic(name, arguments, &argument_schemas, &declared)?;
                    charge.append(intrinsic)?;
                    schema
                }
            }
            _ => return Err(()),
        };
        if depth.checked_add(height).ok_or(())? > 64 {
            return Err(());
        }
        Ok(Expression {
            schema,
            charge,
            height,
        })
    }
    fn predicate(
        &mut self,
        predicate: &Value,
        frame: &Frame<'_>,
        depth: usize,
    ) -> Check<(Charge, usize)> {
        self.types.step(depth)?;
        exact(predicate, &["kind", "op", "left", "right"])?;
        if string(get(predicate, "kind")?)? != "compare" {
            return Err(());
        }
        let left = self.expression(get(predicate, "left")?, frame, depth + 1)?;
        let right = self.expression(get(predicate, "right")?, frame, depth + 1)?;
        let mut charge = left.charge;
        charge.append(right.charge)?;
        let compare = match (string(get(predicate, "op")?)?, &left.schema, &right.schema) {
            ("==", Schema::Blob { high: a, .. }, Schema::Blob { high: b, .. }) => {
                u128::from((*a).max(*b))
            }
            ("==" | "<=", Schema::Word(a), Schema::Word(b)) if a == b => 0,
            _ => return Err(()),
        };
        charge.append(Charge {
            ticks: compare + 1,
            storage: 0,
        })?;
        let height = left.height.max(right.height) + 1;
        if depth.checked_add(height).ok_or(())? > 64 {
            return Err(());
        }
        Ok((charge, height))
    }
    fn intent(&mut self, intent: &Value, read: bool) -> Check<()> {
        let body = get(intent, "body")?;
        exact(body, &["locals", "nodes", "result"])?;
        let mut frame = Frame::new(sequence(get(body, "locals")?)?, self.types)?;
        let input = *frame.declared.get("arg.0").ok_or(())?;
        if get(input, "type")? != get(intent, "input")? {
            return Err(());
        }
        let input_type = self.types.local(input)?;
        let size = input_type.measure()?;
        let mut charge = size.copy;
        charge.append(Charge {
            ticks: size.checks,
            storage: 0,
        })?;
        frame.define(input, input_type)?;
        if let Some(basis) = super::map_field(intent, "basis") {
            let checked = self.expression(basis, &frame, 0)?;
            if read {
                charge.append(checked.charge)?;
            }
        } else if read {
            return Err(());
        }
        for constraint in sequence(get(intent, "inputConstraints")?)? {
            charge.append(self.predicate(get(constraint, "predicate")?, &frame, 0)?.0)?;
        }
        for node in sequence(get(body, "nodes")?)? {
            if read {
                charge.append(Charge {
                    ticks: 1,
                    storage: 0,
                })?;
            }
            match string(get(node, "kind")?)? {
                "let" => {
                    let value = self.expression(get(node, "value")?, &frame, 0)?;
                    let local = get(node, "binding")?;
                    let expected = self.types.local(local)?;
                    if !expected.encloses(&value.schema) {
                        return Err(());
                    }
                    charge.append(value.charge)?;
                    charge.append(Charge {
                        ticks: expected.measure()?.checks,
                        storage: 0,
                    })?;
                    frame.define(local, expected)?;
                }
                "require" if read => {
                    charge.append(self.predicate(get(node, "predicate")?, &frame, 0)?.0)?;
                }
                "effect" if read => {
                    charge.append(self.expression(get(node, "input")?, &frame, 0)?.charge)?;
                    let local = get(node, "binding")?;
                    let expected = self.types.local(local)?;
                    let Schema::Blob { high, .. } = expected else {
                        return Err(());
                    };
                    charge.append(Charge {
                        ticks: u128::from(high) + 1,
                        storage: u128::from(high) + 64,
                    })?;
                    frame.define(local, expected)?;
                }
                _ => return Err(()),
            }
        }
        if !read {
            frame.complete()?;
        }
        let output = self.expression(get(body, "result")?, &frame, 0)?;
        let expected = self.types.resolve(string(get(intent, "output")?)?, 0)?;
        if !expected.encloses(&output.schema) {
            return Err(());
        }
        let size = expected.measure()?;
        charge.append(output.charge)?;
        charge.append(size.copy)?;
        charge.append(Charge {
            ticks: size.checks,
            storage: 0,
        })?;
        let ceiling = get(intent, "coreEvaluationBudget")?;
        if charge.ticks > u128::from(unsigned(get(ceiling, "maxSteps")?)?)
            || charge.storage > u128::from(unsigned(get(ceiling, "maxAllocatedBytes")?)?)
            || size.encoded > u128::from(unsigned(get(ceiling, "maxOutputBytes")?)?)
        {
            return Err(());
        }
        Ok(())
    }
}
fn intrinsic(
    name: &str,
    arguments: &[Value],
    actual: &[Schema],
    declared: &[Schema],
) -> Check<(Schema, Charge)> {
    let literal = |value: &Value| -> Check<u64> {
        if string(get(value, "kind")?)? != "const" {
            return Err(());
        }
        unsigned(get(get(value, "value")?, "value")?)
    };
    match (name, actual, declared) {
        ("core.bytes.length", [actual], [expected @ Schema::Blob { .. }])
            if expected.encloses(actual) =>
        {
            Ok((
                Schema::Word(u64::MAX),
                Charge {
                    ticks: 2,
                    storage: 64,
                },
            ))
        }
        (
            "core.bytes.concat",
            [left, right],
            [a @ Schema::Blob {
                low: a_low,
                high: a_high,
            }, b @ Schema::Blob {
                low: b_low,
                high: b_high,
            }],
        ) if a.encloses(left) && b.encloses(right) => {
            let low = a_low.checked_add(*b_low).ok_or(())?;
            let high = a_high.checked_add(*b_high).ok_or(())?;
            Ok((
                Schema::Blob { low, high },
                Charge {
                    ticks: u128::from(high) + 3,
                    storage: u128::from(high) + 64,
                },
            ))
        }
        ("core.integer.subtract", [left, right], [expected @ Schema::Word(_)])
            if expected.encloses(left) && expected.encloses(right) =>
        {
            literal(&arguments[0])?
                .checked_sub(literal(&arguments[1])?)
                .ok_or(())?;
            Ok((
                expected.clone(),
                Charge {
                    ticks: 3,
                    storage: 64,
                },
            ))
        }
        (
            "core.bytes.slice",
            [actual, Schema::Word(a), Schema::Word(b)],
            [expected @ Schema::Blob { low, .. }],
        ) if *a == u64::MAX && *b == u64::MAX && expected.encloses(actual) => {
            let start = literal(&arguments[1])?;
            let end = literal(&arguments[2])?;
            if start > end || end > *low {
                return Err(());
            }
            let high = end - start;
            Ok((
                Schema::Blob { low: 0, high },
                Charge {
                    ticks: u128::from(high) + 4,
                    storage: u128::from(high) + 64,
                },
            ))
        }
        _ => Err(()),
    }
}
fn get<'a>(value: &'a Value, name: &str) -> Check<&'a Value> {
    super::map_field(value, name).ok_or(())
}
fn members(value: &Value) -> Check<&[(Value, Value)]> {
    if let Value::Map(values) = value {
        Ok(values)
    } else {
        Err(())
    }
}
fn sequence(value: &Value) -> Check<&[Value]> {
    if let Value::Array(values) = value {
        Ok(values)
    } else {
        Err(())
    }
}
fn string(value: &Value) -> Check<&str> {
    if let Value::Text(value) = value {
        if !value.is_empty() && value.len() <= 1024 {
            return Ok(value);
        }
    }
    Err(())
}
fn unsigned(value: &Value) -> Check<u64> {
    if let Value::Integer(value) = value {
        u64::try_from(*value).map_err(|_| ())
    } else {
        Err(())
    }
}
fn exact(value: &Value, fields: &[&str]) -> Check<()> {
    let values = members(value)?;
    if values.len() != fields.len() {
        return Err(());
    }
    for (name, _) in values {
        if !fields.contains(&string(name)?) {
            return Err(());
        }
    }
    Ok(())
}
