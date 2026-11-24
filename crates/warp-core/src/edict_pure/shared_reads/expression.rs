// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! This module owns expression and predicate evaluation under `edict.core-cost/v2`.
//!
//! Every node costs one step. Reading a local, field, element or fold state
//! shares the value. Building a record, list or variant pays its shell. A call
//! pays one step per parameter plus two, and a fold pays one step per
//! iteration plus the call. Each charge is admitted before the work it pays
//! for, and none exceeds the schedule's static rule for the same node.
use super::value::{Shared, Size, Tagged, CELL};
use crate::edict_pure::{
    evaluate::Meter,
    integer_operations,
    model::{
        Comparison, Expr, Helper, IdentityIntrinsic, IntegerOperation, MatchArm, OptionOperation,
        Predicate, RuntimeType,
    },
    EvaluationError as Error,
};
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::BTreeMap;
use std::rc::Rc;

/// The values bound in one lexical frame.
pub(crate) type Locals<'p> = BTreeMap<&'p str, Shared<'p>>;

pub(crate) struct Evaluator<'p, 'm> {
    pub(crate) helpers: &'p BTreeMap<String, Helper>,
    pub(crate) meter: &'m mut Meter,
}

impl<'p> Evaluator<'p, '_> {
    /// Admits a value's shell, in cells as steps and in bytes.
    pub(super) fn build(&mut self, size: Size) -> Result<(), Error> {
        self.meter.charge_steps(size.cells)?;
        self.meter.allocate(size.bytes)
    }

    pub(crate) fn expression(
        &mut self,
        expr: &'p Expr,
        locals: &mut Locals<'p>,
        depth: usize,
    ) -> Result<Shared<'p>, Error> {
        self.meter.step(depth)?;
        match expr {
            Expr::Constant(value) => self.constant(value),
            Expr::Local(id) => locals
                .get(id.as_str())
                .cloned()
                .ok_or(Error::InvalidArtifact),
            Expr::Field(base, name) => {
                let Shared::Record(fields) = self.expression(base, locals, depth + 1)? else {
                    return Err(Error::InvalidArtifact);
                };
                fields
                    .iter()
                    .find_map(|(key, value)| (*key == name).then(|| value.clone()))
                    .ok_or(Error::InvalidArtifact)
            }
            Expr::List(items) => {
                let mut values = Vec::new();
                for item in items {
                    values.push(self.expression(item, locals, depth + 1)?);
                }
                self.build(Size::list_shell(values.len())?)?;
                Ok(Shared::List(Rc::from(values)))
            }
            Expr::Record(fields) => {
                let mut values = Vec::new();
                let mut shell = CELL;
                for (key, value) in fields {
                    values.push((key.as_str(), self.expression(value, locals, depth + 1)?));
                    shell = shell.field(key)?;
                }
                self.build(shell)?;
                Ok(Shared::Record(Rc::from(values)))
            }
            Expr::Variant {
                identity,
                case,
                payload,
            } => {
                let payload = match payload {
                    Some(payload) => Some(self.expression(payload, locals, depth + 1)?),
                    None => None,
                };
                self.build(Size::variant(CELL, identity, case)?)?;
                Ok(Shared::Variant(Rc::new(Tagged {
                    identity,
                    case,
                    payload,
                })))
            }
            Expr::Match(scrutinee, arms) => self.dispatch(scrutinee, arms, locals, depth),
            Expr::If(condition, yes, no) => {
                let branch = if self.predicate(condition, locals, depth + 1)? {
                    yes
                } else {
                    no
                };
                self.expression(branch, locals, depth + 1)
            }
            Expr::Fold {
                initial,
                step,
                bound,
                state,
            } => self.fold(initial, step, *bound, state, locals, depth),
            Expr::Call(name, args) => {
                let helper = self.helpers.get(name).ok_or(Error::UnsupportedProgram)?;
                if args.len() != helper.params.len() {
                    return Err(Error::InvalidArtifact);
                }
                let mut values = Vec::new();
                for argument in args {
                    values.push(self.expression(argument, locals, depth + 1)?);
                }
                self.call(helper, values, depth + 1)
            }
            Expr::UnsignedAdd { max, left, right }
            | Expr::UnsignedSubtract { max, left, right } => {
                let (Shared::Integer(left), Shared::Integer(right)) = (
                    self.expression(left, locals, depth + 1)?,
                    self.expression(right, locals, depth + 1)?,
                ) else {
                    return Err(Error::InvalidArtifact);
                };
                // One step per operand cell, and the result is a new value.
                self.meter.charge_steps(2)?;
                self.build(CELL)?;
                let limit = i128::from(*max);
                if !(0..=limit).contains(&left) || !(0..=limit).contains(&right) {
                    return Err(Error::InvalidArtifact);
                }
                let result = if matches!(expr, Expr::UnsignedAdd { .. }) {
                    left.checked_add(right).filter(|value| *value <= limit)
                } else {
                    left.checked_sub(right).filter(|value| *value >= 0)
                };
                result.map(Shared::Integer).ok_or(Error::InvalidArtifact)
            }
            Expr::ListLength { .. }
            | Expr::ListAt { .. }
            | Expr::ListAppend { .. }
            | Expr::ListConcat { .. } => self.list_operation(expr, locals, depth),
            Expr::Integer {
                operation,
                max,
                left,
                right,
            } => {
                let (Shared::Integer(left), Shared::Integer(right)) = (
                    self.expression(left, locals, depth + 1)?,
                    self.expression(right, locals, depth + 1)?,
                ) else {
                    return Err(Error::InvalidArtifact);
                };
                // One step per operand cell; the result, with a checked
                // result's presence cell, is a new value charged in full.
                self.meter.charge_steps(2)?;
                let word = |value: i128| {
                    u64::try_from(value)
                        .ok()
                        .filter(|value| value <= max)
                        .ok_or(Error::InvalidArtifact)
                };
                let result = integer_operations::apply(*operation, *max, word(left)?, word(right)?)
                    .map(|value| Shared::Integer(i128::from(value)));
                let result = if matches!(operation, IntegerOperation::Min | IntegerOperation::Max) {
                    result.ok_or(Error::InvalidArtifact)?
                } else {
                    result.map_or(Shared::Absent, |value| Shared::Present(Rc::new(value)))
                };
                self.build(result.full_size()?)?;
                Ok(result)
            }
            Expr::Option {
                operation, args, ..
            } => self.option_operation(*operation, args, locals, depth),
            Expr::Identity(intrinsic, args) => self.identity(*intrinsic, args, locals, depth),
            // Byte intrinsics are refused before evaluation; see `admit`.
            Expr::ByteUtf8 { .. }
            | Expr::ByteAt { .. }
            | Expr::ByteLength { .. }
            | Expr::ByteCount { .. }
            | Expr::ByteSlice { .. }
            | Expr::ByteConcat { .. }
            | Expr::Fail(_)
            | Expr::OrderedRecord(_) => Err(Error::UnsupportedProgram),
        }
    }

    /// An identity intrinsic runs outside the shared heap: every operand is
    /// copied in and charged at full size, as a target intrinsic is under
    /// Amblicit's schedule, then the call does its copying-schedule work and
    /// returns a new 32-byte value.
    fn identity(
        &mut self,
        intrinsic: IdentityIntrinsic,
        args: &'p [Expr],
        locals: &mut Locals<'p>,
        depth: usize,
    ) -> Result<Shared<'p>, Error> {
        let types = intrinsic.parameter_types();
        if args.len() != types.len() {
            return Err(Error::InvalidArtifact);
        }
        let mut operands = Vec::new();
        operands
            .try_reserve_exact(args.len())
            .map_err(|_| Error::HostAllocationFailed)?;
        for (argument, ty) in args.iter().zip(&types) {
            let shared = self.expression(argument, locals, depth + 1)?;
            self.build(shared.full_size()?)?;
            operands.push(super::to_canonical(&shared, ty, depth + 1)?);
        }
        let identity = crate::edict_pure::identity::apply(
            intrinsic,
            &operands,
            self.meter,
            crate::edict_pure::identity::Scratch::InOperandCopy,
            depth,
        )?;
        let Value::Bytes(bytes) = identity else {
            return Err(Error::InvalidArtifact);
        };
        Ok(Shared::Bytes(Rc::from(bytes)))
    }

    /// An `Option` consumer reads its operands by reference and charges one
    /// step per operand cell scanned. Constructors are refused by `admit`.
    fn option_operation(
        &mut self,
        operation: OptionOperation,
        args: &'p [Expr],
        locals: &mut Locals<'p>,
        depth: usize,
    ) -> Result<Shared<'p>, Error> {
        let mut values = Vec::with_capacity(args.len());
        let mut cells = 0_u64;
        for argument in args {
            let value = self.expression(argument, locals, depth + 1)?;
            cells = cells
                .checked_add(value.full_size()?.cells)
                .ok_or(Error::StepBudgetExceeded)?;
            values.push(value);
        }
        self.meter.charge_steps(cells)?;
        match (operation, values.as_slice()) {
            (OptionOperation::IsSome, [operand]) => {
                let present = match operand {
                    Shared::Present(_) => true,
                    Shared::Absent => false,
                    _ => return Err(Error::InvalidArtifact),
                };
                // The flag is a new value; `unwrap` and `default` share theirs.
                self.build(CELL)?;
                Ok(Shared::Bool(present))
            }
            // A checked artifact proved presence; absence is a forged artifact.
            (OptionOperation::Unwrap, [Shared::Present(item)])
            | (OptionOperation::Default, [Shared::Present(item), _]) => Ok(Shared::clone(item)),
            (OptionOperation::Default, [Shared::Absent, fallback]) => Ok(fallback.clone()),
            _ => Err(Error::InvalidArtifact),
        }
    }

    fn constant(&mut self, value: &Value) -> Result<Shared<'p>, Error> {
        match value {
            Value::Bool(value) => {
                self.build(CELL)?;
                Ok(Shared::Bool(*value))
            }
            Value::Integer(value) => {
                self.build(CELL)?;
                Ok(Shared::Integer(*value))
            }
            // Byte constants are copied in full, as under the copying schedule.
            Value::Bytes(bytes) => {
                self.build(CELL.plus(Size {
                    cells: 0,
                    bytes: super::value::length(bytes.len())?,
                })?)?;
                Ok(Shared::Bytes(Rc::from(bytes.as_slice())))
            }
            _ => Err(Error::InvalidArtifact),
        }
    }

    fn dispatch(
        &mut self,
        scrutinee: &'p Expr,
        arms: &'p [MatchArm],
        locals: &mut Locals<'p>,
        depth: usize,
    ) -> Result<Shared<'p>, Error> {
        let Shared::Variant(tagged) = self.expression(scrutinee, locals, depth + 1)? else {
            return Err(Error::InvalidArtifact);
        };
        // Dispatch scans at most every arm.
        self.meter
            .charge_steps(super::value::length(arms.len())?.saturating_add(1))?;
        let arm = arms
            .iter()
            .find(|arm| arm.case == tagged.case)
            .ok_or(Error::InvalidArtifact)?;
        let Some(binder) = &arm.binder else {
            return self.expression(&arm.body, locals, depth + 1);
        };
        // The payload binder is one shared reference in the enclosing frame,
        // removed again when the arm returns.
        self.meter.charge_steps(1)?;
        let payload = tagged.payload.clone().ok_or(Error::InvalidArtifact)?;
        if locals.insert(binder.as_str(), payload).is_some() {
            return Err(Error::InvalidArtifact);
        }
        let result = self.expression(&arm.body, locals, depth + 1);
        locals.remove(binder.as_str());
        result
    }

    fn fold(
        &mut self,
        initial: &'p Expr,
        step: &str,
        bound: u64,
        state: &RuntimeType,
        locals: &mut Locals<'p>,
        depth: usize,
    ) -> Result<Shared<'p>, Error> {
        let helper = self.helpers.get(step).ok_or(Error::UnsupportedProgram)?;
        if !helper.source_owned
            || !helper.fold_capability
            || helper.params.len() != 1
            || helper.params[0].ty != *state
            || helper.ty != *state
        {
            return Err(Error::InvalidArtifact);
        }
        let mut value = self.expression(initial, locals, depth + 1)?;
        for _ in 0..bound {
            self.meter.step(depth + 1)?;
            value = self.call(helper, vec![value], depth + 1)?;
        }
        Ok(value)
    }

    /// Runs a helper body in a fresh frame. Arguments and the result are passed
    /// by reference: one step per parameter plus two.
    pub(super) fn call(
        &mut self,
        helper: &'p Helper,
        arguments: Vec<Shared<'p>>,
        depth: usize,
    ) -> Result<Shared<'p>, Error> {
        if !helper.source_owned {
            return Err(Error::UnsupportedProgram);
        }
        self.meter
            .charge_steps(super::value::length(helper.params.len())?.saturating_add(2))?;
        let mut frame = Locals::new();
        for (parameter, value) in helper.params.iter().zip(arguments) {
            if frame.insert(parameter.id.as_str(), value).is_some() {
                return Err(Error::InvalidArtifact);
            }
        }
        for binding in &helper.bindings {
            let value = self.expression(&binding.value, &mut frame, depth)?;
            if frame.insert(binding.id.as_str(), value).is_some() {
                return Err(Error::InvalidArtifact);
            }
        }
        self.expression(&helper.result, &mut frame, depth)
    }

    fn list_operation(
        &mut self,
        expr: &'p Expr,
        locals: &mut Locals<'p>,
        depth: usize,
    ) -> Result<Shared<'p>, Error> {
        match expr {
            Expr::ListLength { value, .. } => {
                let Shared::List(items) = self.expression(value, locals, depth + 1)? else {
                    return Err(Error::InvalidArtifact);
                };
                self.build(CELL)?;
                Ok(Shared::Integer(i128::from(super::value::length(
                    items.len(),
                )?)))
            }
            Expr::ListAt { value, index, .. } => {
                let (Shared::List(items), Shared::Integer(index)) = (
                    self.expression(value, locals, depth + 1)?,
                    self.expression(index, locals, depth + 1)?,
                ) else {
                    return Err(Error::InvalidArtifact);
                };
                // An indexed read shares the element; the list is not rescanned.
                let index = usize::try_from(index).map_err(|_| Error::InvalidArtifact)?;
                items.get(index).cloned().ok_or(Error::InvalidArtifact)
            }
            Expr::ListAppend { ty, value, item } => {
                let RuntimeType::List { max, .. } = ty else {
                    return Err(Error::InvalidArtifact);
                };
                let (Shared::List(items), entry) = (
                    self.expression(value, locals, depth + 1)?,
                    self.expression(item, locals, depth + 1)?,
                ) else {
                    return Err(Error::InvalidArtifact);
                };
                if super::value::length(items.len())? >= *max {
                    return Err(Error::InvalidArtifact);
                }
                // A new shell of shared references, one per item.
                self.build(Size::list_shell(items.len().saturating_add(1))?)?;
                let mut values = Vec::new();
                values
                    .try_reserve_exact(items.len().saturating_add(1))
                    .map_err(|_| Error::HostAllocationFailed)?;
                values.extend(items.iter().cloned());
                values.push(entry);
                Ok(Shared::List(Rc::from(values)))
            }
            Expr::ListConcat {
                left_type,
                right_type,
                left,
                right,
            } => {
                let (
                    RuntimeType::List { max: left_max, .. },
                    RuntimeType::List { max: right_max, .. },
                ) = (left_type.as_ref(), right_type.as_ref())
                else {
                    return Err(Error::InvalidArtifact);
                };
                let (Shared::List(left), Shared::List(right)) = (
                    self.expression(left, locals, depth + 1)?,
                    self.expression(right, locals, depth + 1)?,
                ) else {
                    return Err(Error::InvalidArtifact);
                };
                if super::value::length(left.len())? > *left_max
                    || super::value::length(right.len())? > *right_max
                {
                    return Err(Error::InvalidArtifact);
                }
                let total = left.len().saturating_add(right.len());
                self.build(Size::list_shell(total)?)?;
                let mut values = Vec::new();
                values
                    .try_reserve_exact(total)
                    .map_err(|_| Error::HostAllocationFailed)?;
                values.extend(left.iter().cloned());
                values.extend(right.iter().cloned());
                Ok(Shared::List(Rc::from(values)))
            }
            _ => Err(Error::InvalidArtifact),
        }
    }

    pub(crate) fn predicate(
        &mut self,
        predicate: &'p Predicate,
        locals: &mut Locals<'p>,
        depth: usize,
    ) -> Result<bool, Error> {
        self.meter.step(depth)?;
        let (op, left, right) = match predicate {
            Predicate::Compare { op, left, right } => (op, left, right),
            Predicate::Not(value) => return self.predicate(value, locals, depth + 1).map(|v| !v),
            Predicate::Any(values) => {
                for value in values {
                    if self.predicate(value, locals, depth + 1)? {
                        return Ok(true);
                    }
                }
                return Ok(false);
            }
            Predicate::All(values) => {
                for value in values {
                    if !self.predicate(value, locals, depth + 1)? {
                        return Ok(false);
                    }
                }
                return Ok(true);
            }
        };
        let left = self.expression(left, locals, depth + 1)?;
        let right = self.expression(right, locals, depth + 1)?;
        // A comparison scans both operands in full, whatever their content.
        let scanned = left.full_size()?.bytes.max(right.full_size()?.bytes);
        self.meter.charge_steps(scanned)?;
        match (op, left, right) {
            (Comparison::Equal, Shared::Bool(left), Shared::Bool(right)) => Ok(left == right),
            (Comparison::Equal, Shared::Integer(left), Shared::Integer(right)) => Ok(left == right),
            (Comparison::NotEqual, Shared::Integer(left), Shared::Integer(right)) => {
                Ok(left != right)
            }
            (Comparison::Less, Shared::Integer(left), Shared::Integer(right)) => Ok(left < right),
            (Comparison::LessOrEqual, Shared::Integer(left), Shared::Integer(right)) => {
                Ok(left <= right)
            }
            (Comparison::Greater, Shared::Integer(left), Shared::Integer(right)) => {
                Ok(left > right)
            }
            (Comparison::GreaterOrEqual, Shared::Integer(left), Shared::Integer(right)) => {
                Ok(left >= right)
            }
            (Comparison::Equal, Shared::Bytes(left), Shared::Bytes(right)) => Ok(left == right),
            _ => Err(Error::UnsupportedProgram),
        }
    }
}

#[cfg(test)]
#[allow(clippy::panic, reason = "test fixtures fail loudly")]
mod tests {
    use super::{Evaluator, Locals, Shared};
    use crate::edict_pure::{
        evaluate::Meter,
        model::{Expr, IntegerOperation},
        EvaluationLimits,
    };
    use std::collections::BTreeMap;
    use std::rc::Rc;

    fn limits() -> EvaluationLimits {
        EvaluationLimits {
            max_package_bytes: 0,
            max_input_bytes: 0,
            max_steps: 1_000,
            max_allocated_bytes: 0,
            max_output_bytes: 0,
        }
    }

    fn integer(value: i128) -> Expr {
        Expr::Constant(echo_edict_canonical::CanonicalValueV1::Integer(value))
    }

    /// Allocation the meter recorded for evaluating `expr` with no locals.
    fn allocated(expr: &Expr) -> u64 {
        let helpers = BTreeMap::new();
        let mut meter = Meter::new(EvaluationLimits {
            max_allocated_bytes: 1_000,
            ..limits()
        });
        let mut evaluator = Evaluator {
            helpers: &helpers,
            meter: &mut meter,
        };
        assert!(
            evaluator.expression(expr, &mut Locals::new(), 0).is_ok(),
            "evaluates"
        );
        meter.usage().1
    }

    /// Oracle: amblicit#340. A prelude call that builds a new value is
    /// charged that value in full: one cell for a word, and a presence cell
    /// plus its item for a present checked result. The two constant operands
    /// are one cell each.
    #[test]
    fn integer_results_are_charged_as_new_values() {
        let add = Expr::UnsignedAdd {
            max: 9,
            left: Box::new(integer(4)),
            right: Box::new(integer(5)),
        };
        let checked = |operation| Expr::Integer {
            operation,
            max: u64::MAX,
            left: Box::new(integer(4)),
            right: Box::new(integer(5)),
        };
        assert_eq!(allocated(&add), 3 * 64, "unsigned add");
        assert_eq!(allocated(&checked(IntegerOperation::Min)), 3 * 64, "min");
        assert_eq!(
            allocated(&checked(IntegerOperation::CheckedAdd)),
            4 * 64,
            "present checked result"
        );
        assert_eq!(
            allocated(&checked(IntegerOperation::CheckedSub)),
            3 * 64,
            "absent checked result"
        );
    }

    /// Reads hand back the stored value itself, never a copy: the property
    /// that makes the schedule's one-step, zero-byte read charge true.
    #[test]
    fn local_field_and_element_reads_share_storage_and_allocate_nothing() {
        let list: Rc<[Shared<'_>]> = Rc::from(vec![Shared::Integer(1), Shared::Integer(2)]);
        let record = Shared::Record(Rc::from(vec![("items", Shared::List(Rc::clone(&list)))]));
        let element = Shared::List(Rc::from(vec![Shared::List(Rc::clone(&list))]));
        let helpers = BTreeMap::new();
        let mut meter = Meter::new(limits());
        let mut evaluator = Evaluator {
            helpers: &helpers,
            meter: &mut meter,
        };
        let mut locals = Locals::from([("r", record), ("e", element)]);
        let local = Expr::Local("r".into());
        let field = Expr::Field(Box::new(Expr::Local("r".into())), "items".into());
        let at = Expr::ListAt {
            ty: crate::edict_pure::model::RuntimeType::List { item: None, max: 1 },
            value: Box::new(Expr::Local("e".into())),
            index: Box::new(Expr::Constant(
                echo_edict_canonical::CanonicalValueV1::Integer(0),
            )),
        };
        let Ok(Shared::Record(read)) = evaluator.expression(&local, &mut locals, 0) else {
            panic!("local read")
        };
        let Some(Shared::Record(stored)) = locals.get("r") else {
            panic!("stored record")
        };
        assert!(Rc::ptr_eq(&read, stored), "local read copied the record");
        let mut meter = Meter::new(EvaluationLimits {
            max_allocated_bytes: 64,
            ..limits()
        });
        let mut evaluator = Evaluator {
            helpers: &helpers,
            meter: &mut meter,
        };
        let Ok(Shared::List(projected)) = evaluator.expression(&field, &mut locals, 0) else {
            panic!("field read")
        };
        let Ok(Shared::List(indexed)) = evaluator.expression(&at, &mut locals, 0) else {
            panic!("element read")
        };
        assert!(Rc::ptr_eq(&projected, &list), "field read copied the list");
        assert!(Rc::ptr_eq(&indexed, &list), "element read copied the list");
        // Only the index constant allocated: one 64-byte cell.
        assert_eq!(meter.usage(), (6, 64));
    }
}
