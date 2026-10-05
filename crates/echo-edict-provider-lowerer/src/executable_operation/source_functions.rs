// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Lowerer-owned forward type, totality, depth and deterministic cost judgment.
//! Applies to the complete source-function module, including unused definitions.
use super::ProviderRefusalV1;
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::{BTreeMap, BTreeSet};

type Check<T> = Result<T, ()>;
const DEPTH: usize = 64;
const WORK: usize = 65_536;
#[derive(Clone, Debug, PartialEq, Eq)]
enum Ty {
    Word(u64),
    Bytes(u64, u64),
    Record(BTreeMap<String, Ty>),
}
#[derive(Clone, Copy, Default)]
struct Cost {
    steps: u64,
    bytes: u64,
}
impl Cost {
    fn plus(self, other: Self) -> Check<Self> {
        Ok(Self {
            steps: self.steps.checked_add(other.steps).ok_or(())?,
            bytes: self.bytes.checked_add(other.bytes).ok_or(())?,
        })
    }
    fn maximum(self, other: Self) -> Self {
        Self {
            steps: self.steps.max(other.steps),
            bytes: self.bytes.max(other.bytes),
        }
    }
}
#[derive(Clone)]
struct Fact {
    ty: Ty,
    cost: Cost,
    depth: usize,
}
#[derive(Clone, Copy)]
struct Function<'a> {
    params: &'a [Value],
    body: &'a Value,
    result: &'a str,
    source: bool,
}
type Scope<'a> = BTreeMap<String, (&'a Value, Ty)>;
struct Judgment<'a> {
    core: &'a Value,
    functions: BTreeMap<String, Function<'a>>,
    completed: BTreeMap<String, Fact>,
    active: BTreeSet<String>,
    work: usize,
}

pub(super) fn validate(core: &Value, exports: &Value, read: bool) -> Result<(), ProviderRefusalV1> {
    if super::map_field(core, "functions").is_none() {
        return Ok(());
    }
    run(core, exports, read)
        .map_err(|()| super::super::unsupported_semantics("core.source-functions"))
}
fn run(core: &Value, exports: &Value, read: bool) -> Check<()> {
    let mut functions = BTreeMap::new();
    for item in list(get(exports, "pureFunctions")?)? {
        if string(get(item, "source")?)? != "edict" {
            return Err(());
        }
        let implementation = get(item, "body")?;
        let body = get(implementation, "body")?;
        for (value, key) in [
            (item, "parameterTypes"),
            (item, "typeParameters"),
            (implementation, "params"),
            (body, "locals"),
            (body, "bindings"),
        ] {
            if !list(get(value, key)?)?.is_empty() {
                return Err(());
            }
        }
        let name = string(get(item, "coordinate")?)?;
        if name.starts_with("core.")
            || functions
                .insert(
                    name.to_owned(),
                    Function {
                        params: list(get(implementation, "params")?)?,
                        body,
                        result: string(get(item, "returnType")?)?,
                        source: false,
                    },
                )
                .is_some()
        {
            return Err(());
        }
    }
    // Exported effects own their coordinates even when no source body calls them.
    // This exclusion does not add effects to the pure-call inventory.
    let effect_names = list(get(exports, "effects")?)?
        .iter()
        .map(|effect| string(get(effect, "coordinate")?))
        .collect::<Check<BTreeSet<_>>>()?;
    for (name, function) in members(get(core, "functions")?)? {
        let name = string(name)?;
        if !identifier(name)
            || super::map_field(get(core, "intents")?, name).is_some()
            || super::map_field(get(core, "types")?, name).is_some()
        {
            return Err(());
        }
        exact(function, &["params", "returnType", "body"])?;
        let name = format!("{}.{}", string(get(core, "coordinate")?)?, name);
        if name.starts_with("core.")
            || effect_names.contains(name.as_str())
            || functions
                .insert(
                    name,
                    Function {
                        params: list(get(function, "params")?)?,
                        body: get(function, "body")?,
                        result: string(get(function, "returnType")?)?,
                        source: true,
                    },
                )
                .is_some()
        {
            return Err(());
        }
    }
    let mut proof = Judgment {
        core,
        functions,
        completed: BTreeMap::new(),
        active: BTreeSet::new(),
        work: WORK,
    };
    for name in proof.functions.keys().cloned().collect::<Vec<_>>() {
        proof.function(&name, 1)?;
    }
    for (_, intent) in members(get(core, "intents")?)? {
        proof.intent(intent, read)?;
    }
    Ok(())
}
impl Judgment<'_> {
    fn charge(&mut self, depth: usize) -> Check<()> {
        self.work = self.work.checked_sub(1).ok_or(())?;
        if depth > DEPTH {
            Err(())
        } else {
            Ok(())
        }
    }
    fn ty(&mut self, name: &str, depth: usize) -> Check<Ty> {
        self.charge(depth)?;
        if let Some(max) = match name {
            "U8" => Some(u8::MAX.into()),
            "U16" => Some(u16::MAX.into()),
            "U32" => Some(u32::MAX.into()),
            "U64" => Some(u64::MAX),
            _ => None,
        } {
            return Ok(Ty::Word(max));
        }
        if let Some(bounds) = name
            .strip_prefix("Bytes<")
            .and_then(|value| value.strip_suffix('>'))
        {
            if let Some(n) = bounds.strip_prefix("exact=") {
                let n = decimal(n)?;
                return Ok(Ty::Bytes(n, n));
            }
            if let Some(n) = bounds.strip_prefix("max=") {
                return Ok(Ty::Bytes(0, decimal(n)?));
            }
            if let Some((min, max)) = bounds
                .strip_prefix("min=")
                .and_then(|v| v.split_once(",max="))
            {
                let (min, max) = (decimal(min)?, decimal(max)?);
                if min < max {
                    return Ok(Ty::Bytes(min, max));
                }
            }
            return Err(());
        }
        let prefix = format!("{}.", string(get(self.core, "coordinate")?)?);
        let definition = get(
            get(self.core, "types")?,
            name.strip_prefix(&prefix).unwrap_or(name),
        )?;
        match string(get(definition, "kind")?)? {
            "Nominal" => self.ty(string(get(definition, "representation")?)?, depth + 1),
            "Bytes" => {
                let min = super::map_field(definition, "min")
                    .map(number)
                    .transpose()?
                    .unwrap_or(0);
                let max = number(get(definition, "max")?)?;
                if min > max {
                    return Err(());
                }
                Ok(Ty::Bytes(min, max))
            }
            "Record" => {
                let mut fields = BTreeMap::new();
                for (key, value) in members(get(definition, "fields")?)? {
                    if fields
                        .insert(string(key)?.to_owned(), self.ty(string(value)?, depth + 1)?)
                        .is_some()
                    {
                        return Err(());
                    }
                }
                Ok(Ty::Record(fields))
            }
            _ => Err(()),
        }
    }
    fn declaration<'a>(&mut self, value: &'a Value, scope: &mut Scope<'a>) -> Check<Ty> {
        exact(value, &["id", "type", "alphaName"])?;
        let id = string(get(value, "id")?)?;
        string(get(value, "alphaName")?)?;
        let ty = self.ty(string(get(value, "type")?)?, 0)?;
        if scope.insert(id.to_owned(), (value, ty.clone())).is_some() {
            return Err(());
        }
        Ok(ty)
    }
    fn function(&mut self, name: &str, prefix: usize) -> Check<Fact> {
        self.charge(prefix)?;
        if self.active.contains(name) {
            return Err(());
        }
        if let Some(fact) = self.completed.get(name) {
            if prefix.checked_add(fact.depth).ok_or(())? > DEPTH {
                return Err(());
            }
            self.work = self
                .work
                .checked_sub(usize::try_from(shape(&fact.ty)?.0.steps).map_err(|_| ())?)
                .ok_or(())?;
            return Ok(fact.clone());
        }
        let function = *self.functions.get(name).ok_or(())?;
        self.active.insert(name.to_owned());
        exact(function.body, &["locals", "bindings", "result"])?;
        let mut scope = BTreeMap::new();
        let mut depth = 0;
        for parameter in function.params {
            depth = depth.max(shape(&self.declaration(parameter, &mut scope)?)?.3);
        }
        let mut pending = BTreeMap::new();
        for local in list(get(function.body, "locals")?)? {
            exact(local, &["id", "type", "alphaName"])?;
            let id = string(get(local, "id")?)?;
            if scope.contains_key(id) || pending.insert(id, local).is_some() {
                return Err(());
            }
        }
        let mut cost = Cost::default();
        for binding in list(get(function.body, "bindings")?)? {
            exact(binding, &["kind", "binding", "value"])?;
            if string(get(binding, "kind")?)? != "let" {
                return Err(());
            }
            let local = get(binding, "binding")?;
            if pending.remove(string(get(local, "id")?)?) != Some(local) {
                return Err(());
            }
            let fact = self.expression(get(binding, "value")?, &scope, function.source, prefix)?;
            let expected = self.declaration(local, &mut scope)?;
            if !includes(&expected, &fact.ty) {
                return Err(());
            }
            let (_, validations, _, ty_depth) = shape(&expected)?;
            cost = cost.plus(fact.cost)?.plus(Cost {
                steps: validations,
                bytes: 0,
            })?;
            depth = depth.max(fact.depth).max(ty_depth);
        }
        if !pending.is_empty() {
            return Err(());
        }
        let result = self.expression(
            get(function.body, "result")?,
            &scope,
            function.source,
            prefix,
        )?;
        let ty = self.ty(function.result, 0)?;
        if !includes(&ty, &result.ty) {
            return Err(());
        }
        let (_, validations, _, ty_depth) = shape(&ty)?;
        let fact = Fact {
            ty,
            cost: cost.plus(result.cost)?.plus(Cost {
                steps: validations,
                bytes: 0,
            })?,
            depth: depth.max(result.depth).max(ty_depth),
        };
        if prefix.checked_add(fact.depth).ok_or(())? > DEPTH {
            return Err(());
        }
        self.active.remove(name);
        self.completed.insert(name.to_owned(), fact.clone());
        Ok(fact)
    }
    fn expression(
        &mut self,
        value: &Value,
        scope: &Scope<'_>,
        source: bool,
        prefix: usize,
    ) -> Check<Fact> {
        self.charge(prefix)?;
        let mut cost = Cost { steps: 1, bytes: 0 };
        let mut depth = 0;
        let ty = match string(get(value, "kind")?)? {
            "local" => {
                exact(value, &["kind", "ref"])?;
                let local = get(value, "ref")?;
                let (declared, ty) = scope.get(string(get(local, "id")?)?).ok_or(())?;
                if *declared != local {
                    return Err(());
                }
                let copied = shape(ty)?.0;
                self.work = self
                    .work
                    .checked_sub(usize::try_from(copied.steps).map_err(|_| ())?)
                    .ok_or(())?;
                cost = cost.plus(copied)?;
                ty.clone()
            }
            "const" => {
                exact(value, &["kind", "value"])?;
                let literal = get(value, "value")?;
                exact(literal, &["kind", "value", "width"])?;
                let width = string(get(literal, "width")?)?;
                if string(get(literal, "kind")?)? != "int" || !["U32", "U64"].contains(&width) {
                    return Err(());
                }
                let ty = self.ty(width, 0)?;
                let Ty::Word(max) = ty else {
                    return Err(());
                };
                if number(get(literal, "value")?)? > max {
                    return Err(());
                }
                cost = cost.plus(Cost {
                    steps: 1,
                    bytes: 64,
                })?;
                ty
            }
            "field" => {
                exact(value, &["kind", "base", "field"])?;
                let base = self.expression(get(value, "base")?, scope, source, prefix + 1)?;
                cost = cost.plus(base.cost)?;
                depth = base.depth + 1;
                let Ty::Record(mut fields) = base.ty else {
                    return Err(());
                };
                fields.remove(string(get(value, "field")?)?).ok_or(())?
            }
            "record" => {
                exact(value, &["kind", "fields"])?;
                cost.bytes = 64;
                let mut fields = BTreeMap::new();
                for (key, value) in members(get(value, "fields")?)? {
                    let key = string(key)?;
                    let fact = self.expression(value, scope, source, prefix + 1)?;
                    cost = cost.plus(fact.cost)?.plus(Cost {
                        steps: 0,
                        bytes: 64_u64.checked_add(key.len() as u64).ok_or(())?,
                    })?;
                    depth = depth.max(fact.depth + 1);
                    if fields.insert(key.to_owned(), fact.ty).is_some() {
                        return Err(());
                    }
                }
                Ty::Record(fields)
            }
            "if" => {
                exact(value, &["kind", "predicate", "then", "else"])?;
                let predicate =
                    self.predicate(get(value, "predicate")?, scope, source, prefix + 1)?;
                let yes = self.expression(get(value, "then")?, scope, source, prefix + 1)?;
                let no = self.expression(get(value, "else")?, scope, source, prefix + 1)?;
                cost = cost.plus(predicate.0)?.plus(yes.cost.maximum(no.cost))?;
                depth = predicate.1.max(yes.depth).max(no.depth) + 1;
                union(yes.ty, no.ty)?
            }
            "call" => {
                exact(value, &["kind", "callee", "args", "typeArgs"])?;
                let name = string(get(value, "callee")?)?;
                let arguments = list(get(value, "args")?)?;
                let types = list(get(value, "typeArgs")?)?;
                let mut actual = Vec::new();
                for argument in arguments {
                    let fact = self.expression(argument, scope, source, prefix + 1)?;
                    cost = cost.plus(fact.cost)?;
                    depth = depth.max(fact.depth + 1);
                    actual.push(fact.ty);
                }
                if let Some(function) = self.functions.get(name).copied() {
                    if !types.is_empty()
                        || arguments.len() != function.params.len()
                        || (!source && function.source)
                    {
                        return Err(());
                    }
                    for (actual, parameter) in actual.iter().zip(function.params) {
                        let ty = self.ty(string(get(parameter, "type")?)?, 0)?;
                        if !includes(&ty, actual) {
                            return Err(());
                        }
                        let (_, validations, _, ty_depth) = shape(&ty)?;
                        cost = cost.plus(Cost {
                            steps: validations,
                            bytes: 0,
                        })?;
                        depth = depth.max(ty_depth + 1);
                    }
                    let result = self.function(name, prefix + 1)?;
                    cost = cost.plus(result.cost)?;
                    depth = depth.max(result.depth + 1);
                    result.ty
                } else {
                    let declared = types
                        .iter()
                        .map(|value| self.ty(string(value)?, prefix + 1))
                        .collect::<Check<Vec<_>>>()?;
                    depth = depth.max(1);
                    match (name, actual.as_slice(), declared.as_slice()) {
                        ("core.bytes.length", [actual], [expected @ Ty::Bytes(_, _)])
                            if includes(expected, actual) =>
                        {
                            cost = cost.plus(Cost {
                                steps: 2,
                                bytes: 64,
                            })?;
                            Ty::Word(u64::MAX)
                        }
                        (
                            "core.bytes.concat",
                            [left, right],
                            [a @ Ty::Bytes(a_min, a_max), b @ Ty::Bytes(b_min, b_max)],
                        ) if includes(a, left) && includes(b, right) => {
                            let min = a_min.checked_add(*b_min).ok_or(())?;
                            let max = a_max.checked_add(*b_max).ok_or(())?;
                            cost = cost.plus(Cost {
                                steps: max.checked_add(3).ok_or(())?,
                                bytes: max.checked_add(64).ok_or(())?,
                            })?;
                            Ty::Bytes(min, max)
                        }
                        ("core.integer.subtract", [left, right], [expected @ Ty::Word(_)])
                            if includes(expected, left) && includes(expected, right) =>
                        {
                            literal(&arguments[0])?
                                .checked_sub(literal(&arguments[1])?)
                                .ok_or(())?;
                            cost = cost.plus(Cost {
                                steps: 3,
                                bytes: 64,
                            })?;
                            expected.clone()
                        }
                        (
                            "core.bytes.slice",
                            [actual, Ty::Word(a), Ty::Word(b)],
                            [expected @ Ty::Bytes(min, _)],
                        ) if *a == u64::MAX && *b == u64::MAX && includes(expected, actual) => {
                            let start = literal(&arguments[1])?;
                            let end = literal(&arguments[2])?;
                            if start > end || end > *min {
                                return Err(());
                            }
                            let count = end - start;
                            cost = cost.plus(Cost {
                                steps: count.checked_add(4).ok_or(())?,
                                bytes: count.checked_add(64).ok_or(())?,
                            })?;
                            Ty::Bytes(0, count)
                        }
                        _ => return Err(()),
                    }
                }
            }
            _ => return Err(()),
        };
        if prefix.checked_add(depth).ok_or(())? > DEPTH {
            return Err(());
        }
        Ok(Fact { ty, cost, depth })
    }
    fn predicate(
        &mut self,
        value: &Value,
        scope: &Scope<'_>,
        source: bool,
        prefix: usize,
    ) -> Check<(Cost, usize)> {
        self.charge(prefix)?;
        exact(value, &["kind", "op", "left", "right"])?;
        if string(get(value, "kind")?)? != "compare" {
            return Err(());
        }
        let left = self.expression(get(value, "left")?, scope, source, prefix + 1)?;
        let right = self.expression(get(value, "right")?, scope, source, prefix + 1)?;
        let extra = match (string(get(value, "op")?)?, &left.ty, &right.ty) {
            ("==", Ty::Bytes(_, a), Ty::Bytes(_, b)) => (*a).max(*b),
            ("==" | "<=", Ty::Word(a), Ty::Word(b)) if a == b => 0,
            _ => return Err(()),
        };
        Ok((
            left.cost.plus(right.cost)?.plus(Cost {
                steps: extra.checked_add(1).ok_or(())?,
                bytes: 0,
            })?,
            left.depth.max(right.depth) + 1,
        ))
    }
    fn intent(&mut self, intent: &Value, read: bool) -> Check<()> {
        let body = get(intent, "body")?;
        exact(body, &["locals", "nodes", "result"])?;
        let mut pending = BTreeMap::new();
        for declaration in list(get(body, "locals")?)? {
            exact(declaration, &["id", "type", "alphaName"])?;
            string(get(declaration, "alphaName")?)?;
            self.ty(string(get(declaration, "type")?)?, 0)?;
            if pending
                .insert(string(get(declaration, "id")?)?, declaration)
                .is_some()
            {
                return Err(());
            }
        }
        let input = pending.remove("arg.0").ok_or(())?;
        if get(input, "type")? != get(intent, "input")? {
            return Err(());
        }
        let mut scope = BTreeMap::new();
        let ty = self.declaration(input, &mut scope)?;
        let (copy, validation, _, _) = shape(&ty)?;
        let mut cost = copy.plus(Cost {
            steps: validation,
            bytes: 0,
        })?;
        if let Some(basis) = super::map_field(intent, "basis") {
            let checked = self.expression(basis, &scope, true, 0)?;
            // The pure profile retains basis authority without executing it.
            if read {
                cost = cost.plus(checked.cost)?;
            }
        } else if read {
            return Err(());
        }
        for constraint in list(get(intent, "inputConstraints")?)? {
            cost = cost.plus(
                self.predicate(get(constraint, "predicate")?, &scope, true, 0)?
                    .0,
            )?;
        }
        for node in list(get(body, "nodes")?)? {
            if read {
                cost = cost.plus(Cost { steps: 1, bytes: 0 })?;
            }
            let kind = string(get(node, "kind")?)?;
            if kind == "let" || (read && kind == "effect") {
                let binding = get(node, "binding")?;
                if pending.remove(string(get(binding, "id")?)?) != Some(binding) {
                    return Err(());
                }
            }
            match kind {
                "let" => {
                    exact(node, &["kind", "binding", "value"])?;
                    let fact = self.expression(get(node, "value")?, &scope, true, 0)?;
                    let ty = self.declaration(get(node, "binding")?, &mut scope)?;
                    if !includes(&ty, &fact.ty) {
                        return Err(());
                    }
                    cost = cost.plus(fact.cost)?.plus(Cost {
                        steps: shape(&ty)?.1,
                        bytes: 0,
                    })?;
                }
                "effect" if read => {
                    cost =
                        cost.plus(self.expression(get(node, "input")?, &scope, true, 0)?.cost)?;
                    let Ty::Bytes(_, max) = self.declaration(get(node, "binding")?, &mut scope)?
                    else {
                        return Err(());
                    };
                    cost = cost.plus(Cost {
                        steps: max.checked_add(1).ok_or(())?,
                        bytes: max.checked_add(64).ok_or(())?,
                    })?;
                }
                "require" if read => {
                    cost =
                        cost.plus(self.predicate(get(node, "predicate")?, &scope, true, 0)?.0)?;
                }
                _ => return Err(()),
            }
        }
        // Failure-handler declarations on the read route are checked by its
        // full ordered relation; a pure intent has no other declaration owner.
        if !read && !pending.is_empty() {
            return Err(());
        }
        let result = self.expression(get(body, "result")?, &scope, true, 0)?;
        let expected = self.ty(string(get(intent, "output")?)?, 0)?;
        if !includes(&expected, &result.ty) {
            return Err(());
        }
        let (scratch, validations, output, _) = shape(&expected)?;
        cost = cost.plus(result.cost)?.plus(scratch)?.plus(Cost {
            steps: validations,
            bytes: 0,
        })?;
        let budget = get(intent, "coreEvaluationBudget")?;
        if cost.steps > number(get(budget, "maxSteps")?)?
            || cost.bytes > number(get(budget, "maxAllocatedBytes")?)?
            || output > number(get(budget, "maxOutputBytes")?)?
        {
            return Err(());
        }
        Ok(())
    }
}
fn shape(ty: &Ty) -> Check<(Cost, u64, u64, usize)> {
    match ty {
        Ty::Word(_) => Ok((
            Cost {
                steps: 1,
                bytes: 64,
            },
            1,
            9,
            0,
        )),
        Ty::Bytes(_, max) => Ok((
            Cost {
                steps: 1,
                bytes: max.checked_add(64).ok_or(())?,
            },
            1,
            max.checked_add(9).ok_or(())?,
            0,
        )),
        Ty::Record(fields) => {
            let mut cost = Cost {
                steps: 1,
                bytes: 64,
            };
            let mut validations = 1_u64;
            let mut encoded = 9_u64;
            let mut depth = 0;
            for (name, ty) in fields {
                let (child, validation, bytes, child_depth) = shape(ty)?;
                cost = cost.plus(child)?.plus(Cost {
                    steps: 1,
                    bytes: 64_u64.checked_add(name.len() as u64).ok_or(())?,
                })?;
                validations = validations.checked_add(validation).ok_or(())?;
                encoded = encoded
                    .checked_add(9)
                    .and_then(|v| v.checked_add(name.len() as u64))
                    .and_then(|v| v.checked_add(bytes))
                    .ok_or(())?;
                depth = depth.max(child_depth + 1);
            }
            Ok((cost, validations, encoded, depth))
        }
    }
}
fn includes(expected: &Ty, actual: &Ty) -> bool {
    match (expected, actual) {
        (Ty::Bytes(a, b), Ty::Bytes(c, d)) => a <= c && d <= b,
        (Ty::Record(a), Ty::Record(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(name, ty)| b.get(name).is_some_and(|actual| includes(ty, actual)))
        }
        _ => expected == actual,
    }
}
fn union(left: Ty, right: Ty) -> Check<Ty> {
    match (left, right) {
        (Ty::Bytes(a, b), Ty::Bytes(c, d)) => Ok(Ty::Bytes(a.min(c), b.max(d))),
        (a, b) if a == b => Ok(a),
        _ => Err(()),
    }
}
fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 1024
        && value
            .bytes()
            .enumerate()
            .all(|(i, c)| c == b'_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit()))
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
fn list(value: &Value) -> Check<&[Value]> {
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
fn number(value: &Value) -> Check<u64> {
    if let Value::Integer(value) = value {
        u64::try_from(*value).map_err(|_| ())
    } else {
        Err(())
    }
}
fn decimal(value: &str) -> Check<u64> {
    let parsed: u64 = value.parse().map_err(|_| ())?;
    if parsed.to_string() != value {
        return Err(());
    }
    Ok(parsed)
}
fn literal(value: &Value) -> Check<u64> {
    if string(get(value, "kind")?)? != "const" {
        return Err(());
    }
    number(get(get(value, "value")?, "value")?)
}
fn exact(value: &Value, names: &[&str]) -> Check<()> {
    let fields = members(value)?;
    if fields.len() != names.len()
        || fields
            .iter()
            .any(|(key, _)| !names.contains(&string(key).unwrap_or("")))
    {
        return Err(());
    }
    Ok(())
}
