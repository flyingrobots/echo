// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! This module owns pure evaluation under the shared-read Core cost schedule,
//! `edict.core-cost/v2`, which a module selects by requiring the Core
//! capability `core.cost.shared-reads/v1`.
//!
//! Values are immutable and shared by reference (`value::Shared`), so the
//! schedule's one-step reads are real O(1) work rather than a lowered charge.
//! The intent's input and output are still converted, validated and charged at
//! full size: two steps per cell and their bytes. Comparisons scan both
//! operands in full. Byte and identity intrinsics and imported helpers have no shared-read
//! rule in Echo, so a module that uses them is refused before evaluation.
//!
//! The copying evaluator in `evaluate.rs` is unchanged and still meters every
//! module without the capability.
mod expression;
mod value;

use super::{
    evaluate::Meter,
    model::{Expr, Predicate, Program},
    EvaluationError as Error, EvaluationResult,
};
use echo_edict_canonical::{encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value};
use expression::{Evaluator, Locals};
use value::{from_canonical, to_canonical, Size};

/// Evaluates a pure program whose Core requires the shared-read schedule.
pub(super) fn run(
    program: &Program,
    input: &Value,
    meter: &mut Meter,
) -> Result<EvaluationResult, Error> {
    admit(program)?;
    let input = from_canonical(input, &program.input_type, 0).map_err(|error| {
        if error == Error::InvalidArtifact {
            Error::InvalidInput
        } else {
            error
        }
    })?;
    let mut evaluator = Evaluator {
        helpers: &program.helpers,
        meter,
    };
    evaluator.boundary(input.full_size()?)?;
    let mut locals = Locals::from([(program.input_id.as_str(), input)]);
    for (coordinate, constraint) in &program.constraints {
        if !evaluator.predicate(constraint, &mut locals, 0)? {
            return Err(Error::InputConstraintFailed(coordinate.clone()));
        }
    }
    for binding in &program.bindings {
        // Each intent node costs one step beyond its value.
        evaluator.meter.charge_steps(1)?;
        let value = evaluator.expression(&binding.value, &mut locals, 0)?;
        if locals.insert(binding.id.as_str(), value).is_some() {
            return Err(Error::InvalidArtifact);
        }
    }
    let result = evaluator.expression(&program.result, &mut locals, 0)?;
    evaluator.boundary(result.full_size()?)?;
    let output = encode(&to_canonical(&result, &program.output_type, 0)?)
        .map_err(|_| Error::InvalidArtifact)?;
    if u64::try_from(output.len()).map_err(|_| Error::OutputBudgetExceeded)?
        > program.limits.max_output_bytes
    {
        return Err(Error::OutputBudgetExceeded);
    }
    let (steps, allocated_bytes) = evaluator.meter.usage();
    Ok(EvaluationResult {
        output,
        steps,
        allocated_bytes,
    })
}

impl Evaluator<'_, '_> {
    /// The intent boundary converts and validates a value at full size: two
    /// steps per cell and the value's bytes.
    fn boundary(&mut self, size: Size) -> Result<(), Error> {
        self.build(Size {
            cells: size.cells.checked_mul(2).ok_or(Error::StepBudgetExceeded)?,
            bytes: size.bytes,
        })
    }
}

/// Refuses, before any work, a program that uses an operation without a
/// shared-read rule. Every helper is checked, used or not.
fn admit(program: &Program) -> Result<(), Error> {
    for helper in program.helpers.values() {
        if !helper.source_owned {
            return Err(Error::UnsupportedProgram);
        }
        for binding in &helper.bindings {
            admit_expression(&binding.value)?;
        }
        admit_expression(&helper.result)?;
    }
    for binding in &program.bindings {
        admit_expression(&binding.value)?;
    }
    for (_, constraint) in &program.constraints {
        admit_predicate(constraint)?;
    }
    admit_expression(&program.result)
}

fn admit_expression(expr: &Expr) -> Result<(), Error> {
    match expr {
        Expr::Constant(Value::Bool(_) | Value::Integer(_) | Value::Bytes(_))
        | Expr::Local(_)
        | Expr::Variant { payload: None, .. } => Ok(()),
        Expr::Constant(_)
        | Expr::ByteUtf8 { .. }
        | Expr::ByteAt { .. }
        | Expr::ByteLength { .. }
        | Expr::ByteCount { .. }
        | Expr::ByteSlice { .. }
        | Expr::ByteConcat { .. }
        | Expr::Identity(..)
        | Expr::Option { .. }
        | Expr::Fail(_) => Err(Error::UnsupportedProgram),
        Expr::Variant {
            payload: Some(value),
            ..
        }
        | Expr::Field(value, _)
        | Expr::ListLength { value, .. }
        | Expr::Fold { initial: value, .. } => admit_expression(value),
        Expr::Match(scrutinee, arms) => {
            admit_expression(scrutinee)?;
            arms.iter().try_for_each(|arm| admit_expression(&arm.body))
        }
        Expr::List(items) | Expr::Call(_, items) => items.iter().try_for_each(admit_expression),
        Expr::Record(fields) => fields
            .iter()
            .try_for_each(|(_, value)| admit_expression(value)),
        Expr::ListAt {
            value: left,
            index: right,
            ..
        }
        | Expr::ListAppend {
            value: left,
            item: right,
            ..
        }
        | Expr::ListConcat { left, right, .. }
        | Expr::UnsignedAdd { left, right, .. }
        | Expr::UnsignedSubtract { left, right, .. } => {
            admit_expression(left)?;
            admit_expression(right)
        }
        Expr::If(condition, yes, no) => {
            admit_predicate(condition)?;
            admit_expression(yes)?;
            admit_expression(no)
        }
    }
}

fn admit_predicate(predicate: &Predicate) -> Result<(), Error> {
    match predicate {
        Predicate::Compare { left, right, .. } => {
            admit_expression(left)?;
            admit_expression(right)
        }
        Predicate::All(values) | Predicate::Any(values) => {
            values.iter().try_for_each(admit_predicate)
        }
        Predicate::Not(value) => admit_predicate(value),
    }
}
