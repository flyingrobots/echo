// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use std::collections::BTreeMap;

use echo_edict_canonical::CanonicalValueV1 as Value;

use super::model::{Comparison, Expr, Predicate, RuntimeType, MAX_DEPTH};
use super::values::{array, exact_fields, field, map, number, text, text_field};
use super::EvaluationError as Error;

pub(crate) struct Parser<'a> {
    pub types: &'a Value,
    pub coordinate: &'a str,
    pub remaining: usize,
}

impl Parser<'_> {
    pub(crate) fn enter(&mut self, depth: usize) -> Result<(), Error> {
        if depth > MAX_DEPTH {
            return Err(Error::UnsupportedProgram);
        }
        self.remaining = self
            .remaining
            .checked_sub(1)
            .ok_or(Error::UnsupportedProgram)?;
        Ok(())
    }

    pub fn ty(&mut self, name: &str, depth: usize) -> Result<RuntimeType, Error> {
        self.enter(depth)?;
        if let Some(max) = match name {
            "U8" => Some(u64::from(u8::MAX)),
            "U16" => Some(u64::from(u16::MAX)),
            "U32" => Some(u64::from(u32::MAX)),
            "U64" => Some(u64::MAX),
            _ => None,
        } {
            return Ok(RuntimeType::Unsigned(max));
        }
        if name.starts_with("Bytes<") {
            return bounded_bytes(name);
        }
        let name = name
            .strip_prefix(self.coordinate)
            .and_then(|name| name.strip_prefix('.'))
            .unwrap_or(name);
        let definition = field(self.types, name)?;
        match text_field(definition, "kind")? {
            "Int" => self.ty(text_field(definition, "width")?, depth + 1),
            "Nominal" => self.ty(text_field(definition, "representation")?, depth + 1),
            "Bytes" => {
                let min = field(definition, "min").map_or(Ok(0), number)?;
                let max = number(field(definition, "max")?)?;
                if min > max {
                    return Err(Error::InvalidArtifact);
                }
                Ok(RuntimeType::Bytes { min, max })
            }
            "Record" => {
                let mut fields = Vec::new();
                for (key, value) in map(field(definition, "fields")?)? {
                    fields.push((text(key)?.to_owned(), self.ty(text(value)?, depth + 1)?));
                }
                Ok(RuntimeType::Record(fields))
            }
            _ => Err(Error::UnsupportedProgram),
        }
    }

    pub fn expr(&mut self, value: &Value, depth: usize) -> Result<Expr, Error> {
        self.enter(depth)?;
        match text_field(value, "kind")? {
            "local" => {
                exact_fields(value, &["kind", "ref"])?;
                Ok(Expr::Local(
                    text_field(field(value, "ref")?, "id")?.to_owned(),
                ))
            }
            "field" => {
                exact_fields(value, &["kind", "base", "field"])?;
                Ok(Expr::Field(
                    Box::new(self.expr(field(value, "base")?, depth + 1)?),
                    text_field(value, "field")?.to_owned(),
                ))
            }
            "const" => {
                exact_fields(value, &["kind", "value"])?;
                let literal = field(value, "value")?;
                exact_fields(literal, &["kind", "value", "width"])?;
                if text_field(literal, "kind")? != "int" {
                    return Err(Error::UnsupportedProgram);
                }
                let number = number(field(literal, "value")?)?;
                match text_field(literal, "width")? {
                    "U32" if u32::try_from(number).is_ok() => {}
                    "U64" => {}
                    _ => return Err(Error::UnsupportedProgram),
                }
                Ok(Expr::Constant(Value::Integer(number.into())))
            }
            "record" => {
                exact_fields(value, &["kind", "fields"])?;
                let mut fields = Vec::new();
                for (key, expression) in map(field(value, "fields")?)? {
                    fields.push((text(key)?.to_owned(), self.expr(expression, depth + 1)?));
                }
                Ok(Expr::Record(fields))
            }
            "if" => {
                exact_fields(value, &["kind", "predicate", "then", "else"])?;
                Ok(Expr::If(
                    Box::new(self.predicate(field(value, "predicate")?, depth + 1)?),
                    Box::new(self.expr(field(value, "then")?, depth + 1)?),
                    Box::new(self.expr(field(value, "else")?, depth + 1)?),
                ))
            }
            "call" => self.call(value, depth),
            _ => Err(Error::UnsupportedProgram),
        }
    }

    fn call(&mut self, value: &Value, depth: usize) -> Result<Expr, Error> {
        exact_fields(value, &["kind", "callee", "args", "typeArgs"])?;
        let callee = text_field(value, "callee")?;
        let args = array(field(value, "args")?)?;
        let types = array(field(value, "typeArgs")?)?;
        if callee == "core.bytes.length" {
            let ([value], [coordinate]) = (args, types) else {
                return Err(Error::UnsupportedProgram);
            };
            let RuntimeType::Bytes { min, max } = self.ty(text(coordinate)?, depth + 1)? else {
                return Err(Error::UnsupportedProgram);
            };
            return Ok(Expr::ByteLength {
                min,
                max,
                value: Box::new(self.expr(value, depth + 1)?),
            });
        }
        if callee == "core.bytes.slice" {
            let ([value, start, end], [coordinate]) = (args, types) else {
                return Err(Error::UnsupportedProgram);
            };
            let RuntimeType::Bytes { min, max } = self.ty(text(coordinate)?, depth + 1)? else {
                return Err(Error::UnsupportedProgram);
            };
            return Ok(Expr::ByteSlice {
                min,
                max,
                value: Box::new(self.expr(value, depth + 1)?),
                start: Box::new(self.expr(start, depth + 1)?),
                end: Box::new(self.expr(end, depth + 1)?),
            });
        }
        if callee == "core.bytes.concat" {
            let ([left, right], [left_coordinate, right_coordinate]) = (args, types) else {
                return Err(Error::UnsupportedProgram);
            };
            let RuntimeType::Bytes {
                min: left_min,
                max: left_max,
            } = self.ty(text(left_coordinate)?, depth + 1)?
            else {
                return Err(Error::UnsupportedProgram);
            };
            let RuntimeType::Bytes {
                min: right_min,
                max: right_max,
            } = self.ty(text(right_coordinate)?, depth + 1)?
            else {
                return Err(Error::UnsupportedProgram);
            };
            left_max
                .checked_add(right_max)
                .ok_or(Error::InvalidArtifact)?;
            return Ok(Expr::ByteConcat {
                left_min,
                left_max,
                right_min,
                right_max,
                left: Box::new(self.expr(left, depth + 1)?),
                right: Box::new(self.expr(right, depth + 1)?),
            });
        }
        if callee == "core.integer.subtract" {
            let ([left, right], [width]) = (args, types) else {
                return Err(Error::UnsupportedProgram);
            };
            let max = match text(width)? {
                "U32" => u64::from(u32::MAX),
                "U64" => u64::MAX,
                _ => return Err(Error::UnsupportedProgram),
            };
            return Ok(Expr::UnsignedSubtract {
                max,
                left: Box::new(self.expr(left, depth + 1)?),
                right: Box::new(self.expr(right, depth + 1)?),
            });
        }
        if !types.is_empty() {
            return Err(Error::UnsupportedProgram);
        }
        let args = args
            .iter()
            .map(|value| self.expr(value, depth + 1))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Expr::Call(callee.to_owned(), args))
    }

    pub fn predicate(&mut self, value: &Value, depth: usize) -> Result<Predicate, Error> {
        self.enter(depth)?;
        exact_fields(value, &["kind", "op", "left", "right"])?;
        if text_field(value, "kind")? != "compare" {
            return Err(Error::UnsupportedProgram);
        }
        let op = match text_field(value, "op")? {
            "==" => Comparison::Equal,
            "<=" => Comparison::LessOrEqual,
            _ => return Err(Error::UnsupportedProgram),
        };
        Ok(Predicate {
            op,
            left: self.expr(field(value, "left")?, depth + 1)?,
            right: self.expr(field(value, "right")?, depth + 1)?,
        })
    }

    pub fn projection(
        &mut self,
        value: &Value,
        input_id: &str,
        bindings: &BTreeMap<String, String>,
        capabilities: &BTreeMap<String, String>,
        depth: usize,
    ) -> Result<Expr, Error> {
        self.enter(depth)?;
        match text_field(value, "kind")? {
            "record" => {
                exact_fields(value, &["kind", "fields"])?;
                let mut fields = Vec::new();
                for (key, expression) in map(field(value, "fields")?)? {
                    fields.push((
                        text(key)?.to_owned(),
                        self.projection(expression, input_id, bindings, capabilities, depth + 1)?,
                    ));
                }
                Ok(Expr::Record(fields))
            }
            "source" => {
                exact_fields(value, &["kind", "source", "path"])?;
                let source = field(value, "source")?;
                let id = match text_field(source, "kind")? {
                    "applicationInput" => input_id,
                    "pureBinding" => bindings
                        .get(text_field(source, "bindingId")?)
                        .ok_or(Error::InvalidArtifact)?,
                    "capabilityResult" => capabilities
                        .get(text_field(source, "stepId")?)
                        .ok_or(Error::InvalidArtifact)?,
                    _ => return Err(Error::UnsupportedProgram),
                };
                let mut expression = Expr::Local(id.to_owned());
                for (index, segment) in array(field(value, "path")?)?.iter().enumerate() {
                    self.enter(depth + index + 1)?;
                    expression = Expr::Field(Box::new(expression), text(segment)?.to_owned());
                }
                Ok(expression)
            }
            _ => Err(Error::UnsupportedProgram),
        }
    }
}

// Core's canonical structural Bytes coordinates are self-describing. They do
// never obtain their meaning from a caller-defined type-table entry.
fn bounded_bytes(name: &str) -> Result<RuntimeType, Error> {
    let fields = name
        .strip_prefix("Bytes<")
        .and_then(|name| name.strip_suffix('>'))
        .ok_or(Error::InvalidArtifact)?;
    let (min, max) = if let Some(exact) = fields.strip_prefix("exact=") {
        let exact = byte_bound(exact)?;
        (exact, exact)
    } else if let Some(max) = fields.strip_prefix("max=") {
        (0, byte_bound(max)?)
    } else if let Some(bounds) = fields.strip_prefix("min=") {
        let (min, max) = bounds.split_once(",max=").ok_or(Error::InvalidArtifact)?;
        let (min, max) = (byte_bound(min)?, byte_bound(max)?);
        // Equal endpoints have the canonical exact=N spelling.
        if min >= max {
            return Err(Error::InvalidArtifact);
        }
        (min, max)
    } else {
        return Err(Error::InvalidArtifact);
    };
    Ok(RuntimeType::Bytes { min, max })
}

fn byte_bound(value: &str) -> Result<u64, Error> {
    let bound: u64 = value.parse().map_err(|_| Error::InvalidArtifact)?;
    if bound.to_string() != value {
        return Err(Error::InvalidArtifact);
    }
    Ok(bound)
}
