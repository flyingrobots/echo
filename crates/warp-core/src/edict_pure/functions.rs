// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Closed source/imported helper authority, lexical scopes, and runtime apertures.
use super::{
    model::{
        Binding, Expr, Helper, Parameter, Predicate, RuntimeType, MAX_DEPTH, MAX_PROGRAM_NODES,
    },
    syntax::Parser,
    values::{array, exact_fields, field, map, number, require_text, text, text_field},
    EvaluationError as Error,
};
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::{BTreeMap, BTreeSet};

struct Definition<'a> {
    params: &'a [Value],
    body: &'a Value,
    result_type: &'a str,
    source_owned: bool,
}

pub(crate) fn decode(
    core: &Value,
    exports: &Value,
    parser: &mut Parser<'_>,
) -> Result<BTreeMap<String, Helper>, Error> {
    let mut definitions = BTreeMap::new();
    for item in array(field(exports, "pureFunctions")?)? {
        require_text(item, "source", "edict")?;
        let implementation = field(item, "body")?;
        let body = field(implementation, "body")?;
        // This publication preserves the previous imported helper subset. It
        // does not upgrade imported authority merely because source fn exists.
        for (value, name) in [
            (item, "parameterTypes"),
            (item, "typeParameters"),
            (implementation, "params"),
            (body, "locals"),
            (body, "bindings"),
        ] {
            if !array(field(value, name)?)?.is_empty() {
                return Err(Error::UnsupportedProgram);
            }
        }
        let name = text_field(item, "coordinate")?;
        if name.starts_with("core.")
            || definitions
                .insert(
                    name.to_owned(),
                    Definition {
                        params: array(field(implementation, "params")?)?,
                        body,
                        result_type: text_field(item, "returnType")?,
                        source_owned: false,
                    },
                )
                .is_some()
        {
            return Err(Error::InvalidArtifact);
        }
    }
    if let Ok(functions) = field(core, "functions") {
        for (name, value) in map(functions)? {
            let name = text(name)?;
            if name.is_empty()
                || name.len() > 1024
                || !name.bytes().enumerate().all(|(i, c)| {
                    c == b'_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit())
                })
            {
                return Err(Error::InvalidArtifact);
            }
            if field(field(core, "intents")?, name).is_ok()
                || field(field(core, "types")?, name).is_ok()
            {
                return Err(Error::InvalidArtifact);
            }
            exact_fields(value, &["params", "returnType", "body"])?;
            let coordinate = format!("{}.{}", parser.coordinate, name);
            if coordinate.starts_with("core.")
                || definitions
                    .insert(
                        coordinate,
                        Definition {
                            params: array(field(value, "params")?)?,
                            body: field(value, "body")?,
                            result_type: text_field(value, "returnType")?,
                            source_owned: true,
                        },
                    )
                    .is_some()
            {
                return Err(Error::InvalidArtifact);
            }
        }
    }
    let mut helpers = BTreeMap::new();
    // Collect all signatures before resolving calls: definition order carries
    // no execution meaning. Every body is checked, including unused functions.
    for (name, definition) in &definitions {
        parser.enter(0)?;
        exact_fields(definition.body, &["locals", "bindings", "result"])?;
        let mut scope = BTreeMap::new();
        let mut params = Vec::new();
        for parameter in definition.params {
            local(parameter)?;
            let id = text_field(parameter, "id")?;
            if scope.insert(id, parameter).is_some() {
                return Err(Error::InvalidArtifact);
            }
            params.push(Parameter {
                id: id.to_owned(),
                ty: parser.ty(text_field(parameter, "type")?, 0)?,
            });
        }
        let mut pending = BTreeMap::new();
        for declaration in array(field(definition.body, "locals")?)? {
            local(declaration)?;
            let id = text_field(declaration, "id")?;
            if scope.contains_key(id) || pending.insert(id, declaration).is_some() {
                return Err(Error::InvalidArtifact);
            }
        }
        let mut bindings = Vec::new();
        for binding in array(field(definition.body, "bindings")?)? {
            exact_fields(binding, &["kind", "binding", "value"])?;
            require_text(binding, "kind", "let")?;
            let declaration = field(binding, "binding")?;
            let id = text_field(declaration, "id")?;
            if pending.remove(id) != Some(declaration) {
                return Err(Error::InvalidArtifact);
            }
            let value = field(binding, "value")?;
            let actual = infer(
                value,
                &scope,
                &definitions,
                definition.source_owned,
                parser,
                0,
            )?;
            let ty = parser.ty(text_field(declaration, "type")?, 0)?;
            if !accepts(&ty, &actual) {
                return Err(Error::InvalidArtifact);
            }
            bindings.push(Binding {
                id: id.to_owned(),
                ty,
                value: parser.expr(value, 0)?,
            });
            scope.insert(id, declaration);
        }
        if !pending.is_empty() {
            return Err(Error::InvalidArtifact);
        }
        let result = field(definition.body, "result")?;
        let actual = infer(
            result,
            &scope,
            &definitions,
            definition.source_owned,
            parser,
            0,
        )?;
        let ty = parser.ty(definition.result_type, 0)?;
        if !accepts(&ty, &actual) {
            return Err(Error::InvalidArtifact);
        }
        helpers.insert(
            name.clone(),
            Helper {
                source_owned: definition.source_owned,
                params,
                bindings,
                result: parser.expr(result, 0)?,
                ty,
            },
        );
    }
    let mut basis_roots = Vec::new();
    if field(core, "functions").is_ok() {
        for (_, intent) in map(field(core, "intents")?)? {
            let Some(basis) = map(intent)?
                .iter()
                .find_map(|(key, value)| (text(key) == Ok("basis")).then_some(value))
            else {
                continue;
            };
            let input = array(field(field(intent, "body")?, "locals")?)?
                .iter()
                .find(|value| text_field(value, "id") == Ok("arg.0"))
                .ok_or(Error::InvalidArtifact)?;
            local(input)?;
            let scope = BTreeMap::from([("arg.0", input)]);
            // A pure basis is retained authority, not executed work. Admit its
            // input-only scope, calls and combined depth without spending Meter.
            infer(basis, &scope, &definitions, true, parser, 0)?;
            basis_roots.push(parser.expr(basis, 0)?);
        }
    }
    check_roots(&helpers, basis_roots.iter(), std::iter::empty())?;
    Ok(helpers)
}

fn local(value: &Value) -> Result<(), Error> {
    exact_fields(value, &["id", "type", "alphaName"])?;
    for name in ["id", "type", "alphaName"] {
        let value = text_field(value, name)?;
        if value.is_empty() || value.len() > 1024 {
            return Err(Error::InvalidArtifact);
        }
    }
    Ok(())
}

fn accepts(expected: &RuntimeType, actual: &RuntimeType) -> bool {
    match (expected, actual) {
        (RuntimeType::Bytes { min, max }, RuntimeType::Bytes { min: a, max: b }) => {
            min <= a && b <= max
        }
        (RuntimeType::Record(left), RuntimeType::Record(right)) => {
            left.len() == right.len()
                && left.iter().all(|(key, ty)| {
                    right
                        .iter()
                        .find(|(name, _)| name == key)
                        .is_some_and(|(_, value)| accepts(ty, value))
                })
        }
        _ => expected == actual,
    }
}

fn join(left: RuntimeType, right: RuntimeType) -> Result<RuntimeType, Error> {
    match (left, right) {
        (RuntimeType::Bytes { min: a, max: b }, RuntimeType::Bytes { min: c, max: d }) => {
            Ok(RuntimeType::Bytes {
                min: a.min(c),
                max: b.max(d),
            })
        }
        (left, right) if left == right => Ok(left),
        _ => Err(Error::UnsupportedProgram),
    }
}

type Scope<'a> = BTreeMap<&'a str, &'a Value>;
fn infer(
    value: &Value,
    scope: &Scope<'_>,
    definitions: &BTreeMap<String, Definition<'_>>,
    source: bool,
    parser: &mut Parser<'_>,
    depth: usize,
) -> Result<RuntimeType, Error> {
    parser.enter(depth)?;
    match text_field(value, "kind")? {
        "local" => {
            exact_fields(value, &["kind", "ref"])?;
            let reference = field(value, "ref")?;
            if scope.get(text_field(reference, "id")?).copied() != Some(reference) {
                return Err(Error::InvalidArtifact);
            }
            parser.ty(text_field(reference, "type")?, 0)
        }
        "const" => {
            exact_fields(value, &["kind", "value"])?;
            let literal = field(value, "value")?;
            exact_fields(literal, &["kind", "value", "width"])?;
            require_text(literal, "kind", "int")?;
            let ty = parser.ty(text_field(literal, "width")?, 0)?;
            if !matches!(ty, RuntimeType::Unsigned(max) if number(field(literal, "value")?)? <= max)
            {
                return Err(Error::UnsupportedProgram);
            }
            Ok(ty)
        }
        "field" => {
            exact_fields(value, &["kind", "base", "field"])?;
            let RuntimeType::Record(fields) = infer(
                field(value, "base")?,
                scope,
                definitions,
                source,
                parser,
                depth + 1,
            )?
            else {
                return Err(Error::InvalidArtifact);
            };
            fields
                .into_iter()
                .find_map(|(name, ty)| {
                    (text_field(value, "field") == Ok(name.as_str())).then_some(ty)
                })
                .ok_or(Error::InvalidArtifact)
        }
        "record" => {
            exact_fields(value, &["kind", "fields"])?;
            let fields = map(field(value, "fields")?)?
                .iter()
                .map(|(name, expression)| {
                    Ok((
                        text(name)?.to_owned(),
                        infer(expression, scope, definitions, source, parser, depth + 1)?,
                    ))
                })
                .collect::<Result<Vec<_>, Error>>()?;
            Ok(RuntimeType::Record(fields))
        }
        "if" => {
            exact_fields(value, &["kind", "predicate", "then", "else"])?;
            check_predicate(
                field(value, "predicate")?,
                scope,
                definitions,
                source,
                parser,
                depth + 1,
            )?;
            let left = infer(
                field(value, "then")?,
                scope,
                definitions,
                source,
                parser,
                depth + 1,
            )?;
            let right = infer(
                field(value, "else")?,
                scope,
                definitions,
                source,
                parser,
                depth + 1,
            )?;
            join(left, right)
        }
        "call" => {
            exact_fields(value, &["kind", "callee", "args", "typeArgs"])?;
            let name = text_field(value, "callee")?;
            let arguments = array(field(value, "args")?)?;
            let types = array(field(value, "typeArgs")?)?;
            if let Some(callee) = definitions.get(name) {
                if (!source && callee.source_owned)
                    || !types.is_empty()
                    || arguments.len() != callee.params.len()
                {
                    return Err(Error::InvalidArtifact);
                }
                for (argument, parameter) in arguments.iter().zip(callee.params) {
                    let actual = infer(argument, scope, definitions, source, parser, depth + 1)?;
                    let expected = parser.ty(text_field(parameter, "type")?, 0)?;
                    if !accepts(&expected, &actual) {
                        return Err(Error::InvalidArtifact);
                    }
                }
                return parser.ty(callee.result_type, 0);
            }
            let actual = arguments
                .iter()
                .map(|argument| infer(argument, scope, definitions, source, parser, depth + 1))
                .collect::<Result<Vec<_>, _>>()?;
            let declared = types
                .iter()
                .map(|coordinate| parser.ty(text(coordinate)?, depth + 1))
                .collect::<Result<Vec<_>, _>>()?;
            match (name, actual.as_slice(), declared.as_slice()) {
                ("core.bytes.length", [actual], [expected @ RuntimeType::Bytes { .. }])
                    if accepts(expected, actual) =>
                {
                    Ok(RuntimeType::Unsigned(u64::MAX))
                }
                (
                    "core.bytes.concat",
                    [left, right],
                    [a @ RuntimeType::Bytes {
                        min: a_min,
                        max: a_max,
                    }, b @ RuntimeType::Bytes {
                        min: b_min,
                        max: b_max,
                    }],
                ) if accepts(a, left) && accepts(b, right) => Ok(RuntimeType::Bytes {
                    min: a_min.checked_add(*b_min).ok_or(Error::InvalidArtifact)?,
                    max: a_max.checked_add(*b_max).ok_or(Error::InvalidArtifact)?,
                }),
                // No caller where proof crosses a fresh frame. This target's
                // first source-function subset proves partial intrinsics only
                // from literal offsets/operands, refusing all other cases.
                ("core.integer.subtract", [left, right], [expected @ RuntimeType::Unsigned(_)])
                    if accepts(expected, left) && accepts(expected, right) =>
                {
                    let left = unsigned_literal(&arguments[0])?;
                    let right = unsigned_literal(&arguments[1])?;
                    left.checked_sub(right).ok_or(Error::UnsupportedProgram)?;
                    Ok(expected.clone())
                }
                (
                    "core.bytes.slice",
                    [actual, RuntimeType::Unsigned(a), RuntimeType::Unsigned(b)],
                    [expected @ RuntimeType::Bytes { min, .. }],
                ) if *a == u64::MAX && *b == u64::MAX && accepts(expected, actual) => {
                    let start = unsigned_literal(&arguments[1])?;
                    let end = unsigned_literal(&arguments[2])?;
                    if start > end || end > *min {
                        return Err(Error::UnsupportedProgram);
                    }
                    Ok(RuntimeType::Bytes {
                        min: 0,
                        max: end - start,
                    })
                }
                _ => Err(Error::UnsupportedProgram),
            }
        }
        _ => Err(Error::UnsupportedProgram),
    }
}

fn unsigned_literal(value: &Value) -> Result<u64, Error> {
    if text_field(value, "kind")? != "const" {
        return Err(Error::UnsupportedProgram);
    }
    number(field(field(value, "value")?, "value")?)
}
fn check_predicate(
    value: &Value,
    scope: &Scope<'_>,
    definitions: &BTreeMap<String, Definition<'_>>,
    source: bool,
    parser: &mut Parser<'_>,
    depth: usize,
) -> Result<(), Error> {
    parser.enter(depth)?;
    exact_fields(value, &["kind", "op", "left", "right"])?;
    require_text(value, "kind", "compare")?;
    let left = infer(
        field(value, "left")?,
        scope,
        definitions,
        source,
        parser,
        depth + 1,
    )?;
    let right = infer(
        field(value, "right")?,
        scope,
        definitions,
        source,
        parser,
        depth + 1,
    )?;
    match (text_field(value, "op")?, left, right) {
        ("==", RuntimeType::Bytes { .. }, RuntimeType::Bytes { .. }) => Ok(()),
        ("==" | "<=", RuntimeType::Unsigned(left), RuntimeType::Unsigned(right))
            if left == right =>
        {
            Ok(())
        }
        _ => Err(Error::UnsupportedProgram),
    }
}

/// Prove the actual combined expression/predicate/call aperture before running
/// any instruction. Suffix height is intrinsic, independent of traversal order.
pub(crate) fn check_roots<'a>(
    helpers: &BTreeMap<String, Helper>,
    expressions: impl IntoIterator<Item = &'a Expr>,
    predicates: impl IntoIterator<Item = &'a Predicate>,
) -> Result<(), Error> {
    let mut heights = Heights {
        helpers,
        completed: BTreeMap::new(),
        visiting: BTreeSet::new(),
        remaining: MAX_PROGRAM_NODES,
    };
    for name in helpers.keys() {
        if heights.helper(name, 1)? + 1 > MAX_DEPTH {
            return Err(Error::UnsupportedProgram);
        }
    }
    for expression in expressions {
        heights.expr(expression, 0)?;
    }
    for predicate in predicates {
        heights.predicate(predicate, 0)?;
    }
    Ok(())
}
struct Heights<'a> {
    helpers: &'a BTreeMap<String, Helper>,
    completed: BTreeMap<String, usize>,
    visiting: BTreeSet<String>,
    remaining: usize,
}
impl Heights<'_> {
    fn enter(&mut self, depth: usize) -> Result<(), Error> {
        if depth > MAX_DEPTH {
            return Err(Error::UnsupportedProgram);
        }
        self.remaining = self
            .remaining
            .checked_sub(1)
            .ok_or(Error::UnsupportedProgram)?;
        Ok(())
    }
    fn helper(&mut self, name: &str, prefix: usize) -> Result<usize, Error> {
        self.enter(prefix)?;
        if self.visiting.contains(name) {
            return Err(Error::InvalidArtifact);
        }
        if let Some(height) = self.completed.get(name) {
            if prefix + height > MAX_DEPTH {
                return Err(Error::UnsupportedProgram);
            }
            return Ok(*height);
        }
        let helper = self.helpers.get(name).ok_or(Error::UnsupportedProgram)?;
        self.visiting.insert(name.to_owned());
        let mut height = type_height(&helper.ty);
        for parameter in &helper.params {
            height = height.max(type_height(&parameter.ty));
        }
        for binding in &helper.bindings {
            height = height
                .max(self.expr(&binding.value, prefix)?)
                .max(type_height(&binding.ty));
        }
        height = height.max(self.expr(&helper.result, prefix)?);
        if prefix + height > MAX_DEPTH {
            return Err(Error::UnsupportedProgram);
        }
        self.visiting.remove(name);
        self.completed.insert(name.to_owned(), height);
        Ok(height)
    }
    fn expr(&mut self, expression: &Expr, depth: usize) -> Result<usize, Error> {
        self.enter(depth)?;
        let height = match expression {
            Expr::Constant(_) | Expr::Local(_) => 0,
            Expr::Field(base, _) => self.expr(base, depth + 1)? + 1,
            Expr::Record(fields) => {
                let mut maximum = 0;
                for (_, value) in fields {
                    maximum = maximum.max(self.expr(value, depth + 1)? + 1);
                }
                maximum
            }
            Expr::If(condition, yes, no) => {
                self.predicate(condition, depth + 1)?
                    .max(self.expr(yes, depth + 1)?)
                    .max(self.expr(no, depth + 1)?)
                    + 1
            }
            Expr::Call(name, args) => {
                let helper = self.helpers.get(name).ok_or(Error::UnsupportedProgram)?;
                if helper.params.len() != args.len() {
                    return Err(Error::InvalidArtifact);
                }
                if helper.source_owned
                    && self
                        .visiting
                        .iter()
                        .any(|owner| !self.helpers[owner].source_owned)
                {
                    return Err(Error::InvalidArtifact);
                }
                let mut maximum = self.helper(name, depth + 1)?;
                for argument in args {
                    maximum = maximum.max(self.expr(argument, depth + 1)?);
                }
                maximum + 1
            }
            Expr::ByteLength { value, .. } => self.expr(value, depth + 1)? + 1,
            Expr::ByteSlice {
                value, start, end, ..
            } => {
                self.expr(value, depth + 1)?
                    .max(self.expr(start, depth + 1)?)
                    .max(self.expr(end, depth + 1)?)
                    + 1
            }
            Expr::ByteConcat { left, right, .. } | Expr::UnsignedSubtract { left, right, .. } => {
                self.expr(left, depth + 1)?
                    .max(self.expr(right, depth + 1)?)
                    + 1
            }
        };
        if depth + height > MAX_DEPTH {
            return Err(Error::UnsupportedProgram);
        }
        Ok(height)
    }
    fn predicate(&mut self, predicate: &Predicate, depth: usize) -> Result<usize, Error> {
        self.enter(depth)?;
        let height = self
            .expr(&predicate.left, depth + 1)?
            .max(self.expr(&predicate.right, depth + 1)?)
            + 1;
        if depth + height > MAX_DEPTH {
            return Err(Error::UnsupportedProgram);
        }
        Ok(height)
    }
}
fn type_height(ty: &RuntimeType) -> usize {
    match ty {
        RuntimeType::Record(fields) => fields
            .iter()
            .map(|(_, ty)| type_height(ty) + 1)
            .max()
            .unwrap_or(0),
        _ => 0,
    }
}
