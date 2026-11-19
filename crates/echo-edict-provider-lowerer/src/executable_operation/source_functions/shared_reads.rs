// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! This module owns the lowerer's static cost under the shared-read Core cost
//! schedule, `edict.core-cost/v2`, selected by `core.cost.shared-reads/v1`.
//!
//! The rules are Amblicit's (`target_ir/fold_work/shared_reads.rs` at
//! `26a201e7`), so the lowerer's minimum equals the compiler's. Reading a
//! local, field, element or fold state, `if`, `match`, `fold`, a call and a
//! local reference cost one step. Construction charges its shell. A call adds
//! one step per parameter plus one for the result; a fold charges every
//! iteration of its bound. Comparisons scan both operands at their full
//! static size, and the intent's input and output are charged at full size.
//! Unsigned and checked arithmetic and min/max keep their copying charge, one
//! step per operand cell; an `Option` consumer adds its operand and result
//! bytes. Each of those prelude calls is also charged the value it builds in
//! full (amblicit#340). A list shell is sized by the result's proven length
//! when one exists (amblicit#336; see `reach`). Byte intrinsics, `Option` constructors and imported helpers have no
//! shared-read rule in Echo yet, so the module is refused.
//!
//! The ordinary judgment runs first and alone establishes typing, totality and
//! depth; this pass reads the types it recorded and only computes cost.
use super::{exact, get, list, members, number, shape, string, Check, Cost, Judgment, Ty, Value};
use std::collections::BTreeMap;

/// Core capability selecting the shared-read schedule.
pub(super) const CAPABILITY: &str = "core.cost.shared-reads/v1";

/// A static value size in 64-byte cells and bytes.
#[derive(Clone, Copy)]
struct Size {
    cells: u64,
    bytes: u64,
}

const CELL: Size = Size {
    cells: 1,
    bytes: 64,
};

impl Size {
    fn plus(self, other: Self) -> Check<Self> {
        Ok(Self {
            cells: self.cells.checked_add(other.cells).ok_or(())?,
            bytes: self.bytes.checked_add(other.bytes).ok_or(())?,
        })
    }
    fn maximum(self, other: Self) -> Self {
        Self {
            cells: self.cells.max(other.cells),
            bytes: self.bytes.max(other.bytes),
        }
    }
    /// A record field slot around a child of size `self`.
    fn field(self, key: &str) -> Check<Self> {
        self.plus(Self {
            cells: 1,
            bytes: 64_u64
                .checked_add(u64::try_from(key.len()).map_err(|_| ())?)
                .ok_or(())?,
        })
    }
    /// Evaluating a value of this size: one step per cell plus one.
    fn evaluation(self) -> Check<Cost> {
        Ok(Cost {
            steps: self.cells.checked_add(1).ok_or(())?,
            bytes: self.bytes,
        })
    }
    /// A list shell holding `items` shared references.
    fn list_shell(items: u64) -> Check<Self> {
        CELL.plus(Self {
            cells: items,
            bytes: items.checked_mul(64).ok_or(())?,
        })
    }
}

fn steps(count: u64) -> Cost {
    Cost {
        steps: count,
        bytes: 0,
    }
}

fn length(value: usize) -> Check<u64> {
    u64::try_from(value).map_err(|_| ())
}

/// The full size of a maximal value of `ty`.
fn type_size(ty: &Ty) -> Check<Size> {
    let copy = shape(ty)?.0;
    Ok(Size {
        cells: copy.steps,
        bytes: copy.bytes,
    })
}

/// Upper bounds on the lengths of the lists inside one value, by field path;
/// the empty path is the value itself. A path without an entry has no proven
/// bound and is charged at its declared maximum.
type Reach = BTreeMap<Vec<String>, u64>;

/// The shared-read meter's list-length state (amblicit#336): the bounds of the
/// current frame's locals and each call's cost and result bounds by its
/// argument bounds.
#[derive(Default)]
pub(super) struct Reachable {
    locals: BTreeMap<String, Reach>,
    calls: BTreeMap<(String, Vec<Reach>), (Cost, Reach)>,
    visiting: Vec<String>,
}

/// The bounds two branches share: a path keeps one only if both prove it.
fn join(left: &Reach, right: &Reach) -> Reach {
    left.iter()
        .filter_map(|(path, bound)| {
            right
                .get(path)
                .map(|other| (path.clone(), (*bound).max(*other)))
        })
        .collect()
}

/// The prelude calls Echo admits under the shared-read schedule that build a
/// new value (amblicit#340): integer operations and the `Option` readers.
fn builds_value(callee: &str) -> bool {
    matches!(
        callee,
        "core.integer.add"
            | "core.integer.subtract"
            | "core.integer.checkedAdd"
            | "core.integer.checkedSub"
            | "core.integer.checkedMul"
            | "core.integer.checkedDiv"
            | "core.integer.checkedRem"
            | "core.integer.min"
            | "core.integer.max"
            | "core.option.isSome"
            | "core.option.unwrap"
            | "core.option.default"
    )
}

/// Whether the module requires the shared-read schedule.
pub(super) fn required(core: &Value) -> Check<bool> {
    let Some(capabilities) = super::super::map_field(core, "requiredCoreCapabilities") else {
        return Ok(false);
    };
    Ok(list(capabilities)?
        .iter()
        .any(|capability| string(capability) == Ok(CAPABILITY)))
}

impl<'a> Judgment<'a> {
    /// Judges one pure intent: the ordinary judgment types it and bounds its
    /// output; the shared-read cost must fit the step and allocation budget.
    pub(super) fn shared_intent(&mut self, intent: &'a Value) -> Check<()> {
        let (_, output) = self.intent_cost(intent, false)?;
        let cost = self.shared_intent_cost(intent)?;
        let budget = get(intent, "coreEvaluationBudget")?;
        if cost.steps > number(get(budget, "maxSteps")?)?
            || cost.bytes > number(get(budget, "maxAllocatedBytes")?)?
            || output > number(get(budget, "maxOutputBytes")?)?
        {
            return Err(());
        }
        Ok(())
    }

    fn shared_intent_cost(&mut self, intent: &'a Value) -> Check<Cost> {
        // Each intent starts with no proven bounds of its own.
        self.shared_costs.locals.clear();
        let input = type_size(&self.ty(string(get(intent, "input")?)?, 0)?)?;
        let output = type_size(&self.ty(string(get(intent, "output")?)?, 0)?)?;
        // Input and output are converted and validated at full size.
        let mut cost = Cost {
            steps: input
                .cells
                .checked_add(output.cells)
                .and_then(|cells| cells.checked_mul(2))
                .ok_or(())?,
            bytes: input.bytes.checked_add(output.bytes).ok_or(())?,
        };
        for constraint in list(get(intent, "inputConstraints")?)? {
            cost = cost.plus(self.shared_predicate(get(constraint, "predicate")?)?)?;
        }
        if let Some(basis) = super::super::map_field(intent, "basis") {
            cost = cost.plus(self.shared_expression(basis)?)?;
        }
        let body = get(intent, "body")?;
        for node in list(get(body, "nodes")?)? {
            if string(get(node, "kind")?)? != "let" {
                return Err(());
            }
            let value = get(node, "value")?;
            cost = cost.plus(self.shared_expression(value)?)?.plus(steps(1))?;
            let reach = self.reach(value)?;
            let id = string(get(get(node, "binding")?, "id")?)?;
            self.shared_costs.locals.insert(id.to_owned(), reach);
        }
        cost.plus(self.shared_expression(get(body, "result")?)?)
    }

    /// The cost and result bounds of one source helper call under its
    /// arguments' bounds. Arguments and the result are passed by reference.
    fn shared_function(&mut self, name: &str, arguments: Vec<Reach>) -> Check<(Cost, Reach)> {
        let key = (name.to_owned(), arguments);
        if let Some(cached) = self.shared_costs.calls.get(&key) {
            return Ok(cached.clone());
        }
        let function = *self.functions.get(name).ok_or(())?;
        if !function.source
            || function.params.len() != key.1.len()
            || self
                .shared_costs
                .visiting
                .iter()
                .any(|active| active == name)
        {
            return Err(());
        }
        let mut frame = BTreeMap::new();
        for (parameter, reach) in function.params.iter().zip(&key.1) {
            frame.insert(string(get(parameter, "id")?)?.to_owned(), reach.clone());
        }
        let saved = std::mem::replace(&mut self.shared_costs.locals, frame);
        self.shared_costs.visiting.push(name.to_owned());
        let evaluated = self.shared_body(function.body, function.params.len());
        self.shared_costs.visiting.pop();
        self.shared_costs.locals = saved;
        let evaluated = evaluated?;
        self.shared_costs.calls.insert(key, evaluated.clone());
        Ok(evaluated)
    }

    fn shared_body(&mut self, body: &'a Value, params: usize) -> Check<(Cost, Reach)> {
        let mut cost = steps(length(params)?.checked_add(2).ok_or(())?);
        for binding in list(get(body, "bindings")?)? {
            let value = get(binding, "value")?;
            cost = cost.plus(self.shared_expression(value)?)?;
            let reach = self.reach(value)?;
            let id = string(get(get(binding, "binding")?, "id")?)?;
            self.shared_costs.locals.insert(id.to_owned(), reach);
        }
        let result = get(body, "result")?;
        cost = cost.plus(self.shared_expression(result)?)?;
        Ok((cost, self.reach(result)?))
    }

    /// A fold's rounds, each one step plus the step call, charged under the
    /// bounds the previous round produced; once a round's bounds stop
    /// changing, every remaining round costs the same.
    fn fold_rounds(&mut self, mut state: Reach, step: &str, bound: u64) -> Check<(Cost, Reach)> {
        let mut total = Cost::default();
        let mut round = 0_u64;
        while round < bound {
            let (cost, next) = self.shared_function(step, vec![state.clone()])?;
            let per_round = cost.plus(steps(1))?;
            if next == state {
                let remaining = bound - round;
                return Ok((
                    total.plus(Cost {
                        steps: per_round.steps.checked_mul(remaining).ok_or(())?,
                        bytes: per_round.bytes.checked_mul(remaining).ok_or(())?,
                    })?,
                    state,
                ));
            }
            total = total.plus(per_round)?;
            state = next;
            round += 1;
        }
        Ok((total, state))
    }

    /// The list-length bounds of the value `value` evaluates to.
    fn reach(&mut self, value: &'a Value) -> Check<Reach> {
        self.charge(0)?;
        Ok(match string(get(value, "kind")?)? {
            "local" => {
                let id = string(get(get(value, "ref")?, "id")?)?;
                self.shared_costs
                    .locals
                    .get(id)
                    .cloned()
                    .unwrap_or_default()
            }
            "field" => {
                let name = string(get(value, "field")?)?;
                self.reach(get(value, "base")?)?
                    .into_iter()
                    .filter_map(|(path, bound)| match path.split_first() {
                        Some((head, rest)) if head == name => Some((rest.to_vec(), bound)),
                        _ => None,
                    })
                    .collect()
            }
            "record" => {
                let mut reach = Reach::new();
                for (key, field) in members(get(value, "fields")?)? {
                    let key = string(key)?;
                    for (mut path, bound) in self.reach(field)? {
                        path.insert(0, key.to_owned());
                        reach.insert(path, bound);
                    }
                }
                reach
            }
            "list" => Reach::from([(Vec::new(), length(list(get(value, "values")?)?.len())?)]),
            "if" => {
                let yes = self.reach(get(value, "then")?)?;
                join(&yes, &self.reach(get(value, "else")?)?)
            }
            "match" => {
                let mut joined: Option<Reach> = None;
                for arm in list(get(value, "arms")?)? {
                    let arm = self.reach(get(arm, "body")?)?;
                    joined = Some(match joined {
                        Some(previous) => join(&previous, &arm),
                        None => arm,
                    });
                }
                joined.unwrap_or_default()
            }
            "fold" => {
                let start = self.reach(get(value, "initial")?)?;
                let step = string(get(value, "step")?)?;
                self.fold_rounds(start, step, number(get(value, "bound")?)?)?
                    .1
            }
            "call" => self.call_reach(value)?,
            _ => Reach::new(),
        })
    }

    fn call_reach(&mut self, value: &'a Value) -> Check<Reach> {
        let callee = string(get(value, "callee")?)?;
        let args = list(get(value, "args")?)?;
        let extra = match (callee, args) {
            ("core.list.append", [_, _]) => Some(1),
            ("core.list.concat", [_, right]) => self.proven_length(right)?,
            _ if self
                .functions
                .get(callee)
                .is_some_and(|function| function.source) =>
            {
                let mut arguments = Vec::with_capacity(args.len());
                for argument in args {
                    arguments.push(self.reach(argument)?);
                }
                return Ok(self.shared_function(callee, arguments)?.1);
            }
            _ => return Ok(Reach::new()),
        };
        let Ty::List(_, max) = self.recorded(value)?.representation() else {
            return Err(());
        };
        let max = *max;
        Ok(match (self.proven_length(&args[0])?, extra) {
            (Some(base), Some(extra)) => {
                Reach::from([(Vec::new(), base.saturating_add(extra).min(max))])
            }
            _ => Reach::new(),
        })
    }

    /// The proven length bound of a list-valued expression, if any.
    fn proven_length(&mut self, value: &'a Value) -> Check<Option<u64>> {
        Ok(self.reach(value)?.get(&Vec::new()).copied())
    }

    /// The recorded type of an expression the ordinary judgment admitted.
    fn recorded(&self, value: &Value) -> Check<&Ty> {
        self.types.get(&std::ptr::from_ref(value).addr()).ok_or(())
    }

    fn shared_expression(&mut self, value: &'a Value) -> Check<Cost> {
        self.charge(0)?;
        let own = self.shared_own(value)?;
        let children = match string(get(value, "kind")?)? {
            "local" | "const" => Cost::default(),
            "field" => self.shared_expression(get(value, "base")?)?,
            "variant" => match super::super::map_field(value, "payload") {
                Some(payload) => self.shared_expression(payload)?,
                None => Cost::default(),
            },
            "record" => {
                let mut sum = Cost::default();
                for (_, field) in members(get(value, "fields")?)? {
                    sum = sum.plus(self.shared_expression(field)?)?;
                }
                sum
            }
            "list" => {
                let mut sum = Cost::default();
                for item in list(get(value, "values")?)? {
                    sum = sum.plus(self.shared_expression(item)?)?;
                }
                sum
            }
            "fold" => {
                let initial = get(value, "initial")?;
                let cost = self.shared_expression(initial)?;
                let start = self.reach(initial)?;
                let step = string(get(value, "step")?)?;
                cost.plus(
                    self.fold_rounds(start, step, number(get(value, "bound")?)?)?
                        .0,
                )?
            }
            "call" => {
                let mut sum = Cost::default();
                for argument in list(get(value, "args")?)? {
                    sum = sum.plus(self.shared_expression(argument)?)?;
                }
                sum.plus(self.shared_call(value)?)?
            }
            "if" => {
                let predicate = self.shared_predicate(get(value, "predicate")?)?;
                let yes = self.shared_expression(get(value, "then")?)?;
                let no = self.shared_expression(get(value, "else")?)?;
                predicate.plus(yes.maximum(no))?
            }
            "match" => {
                let scrutinee = self.shared_expression(get(value, "scrutinee")?)?;
                let arms = list(get(value, "arms")?)?;
                let mut selected = Cost::default();
                for arm in arms {
                    // A payload binder is one shared reference.
                    let binder = steps(u64::from(super::super::map_field(arm, "binder").is_some()));
                    selected =
                        selected.maximum(binder.plus(self.shared_expression(get(arm, "body")?)?)?);
                }
                scrutinee
                    .plus(selected)?
                    .plus(steps(length(arms.len())?.checked_add(1).ok_or(())?))?
            }
            _ => return Err(()),
        };
        own.plus(children)
    }

    /// The work of evaluating the node itself, excluding its children.
    fn shared_own(&self, value: &Value) -> Check<Cost> {
        match string(get(value, "kind")?)? {
            "const" => self.size(value)?.evaluation(),
            "record" => {
                let mut shell = CELL;
                for (key, _) in members(get(value, "fields")?)? {
                    shell = shell.plus(CELL.field(string(key)?)?)?;
                }
                shell.evaluation()
            }
            "list" => Size::list_shell(length(list(get(value, "values")?)?.len())?)?.evaluation(),
            "variant" => {
                let identity = length(string(get(value, "type")?)?.len())?;
                let case = length(string(get(value, "case")?)?.len())?;
                CELL.plus(Size {
                    cells: 6,
                    bytes: 399_u64
                        .checked_add(identity)
                        .and_then(|bytes| bytes.checked_add(case))
                        .ok_or(())?,
                })?
                .evaluation()
            }
            // A prelude call without a shared-read rule builds a new value
            // and is charged it in full, as under the copying schedule.
            "call" if builds_value(string(get(value, "callee")?)?) => {
                self.size(value)?.evaluation()
            }
            _ => Ok(steps(1)),
        }
    }

    /// The call work beyond the arguments.
    fn shared_call(&mut self, value: &'a Value) -> Check<Cost> {
        exact(value, &["kind", "callee", "args", "typeArgs"])?;
        let callee = string(get(value, "callee")?)?;
        match callee {
            // An index read returns a shared element.
            "core.list.at" => Ok(Cost::default()),
            // The length is a fresh integer.
            "core.list.length" => CELL.evaluation(),
            // A new list shell holds one reference per item, up to capacity,
            // sized by the result's proven length when one exists.
            "core.list.append" | "core.list.concat" => {
                let Ty::List(_, max) = self.recorded(value)?.representation() else {
                    return Err(());
                };
                let max = *max;
                let items = self
                    .proven_length(value)?
                    .map_or(max, |bound| bound.min(max));
                Size::list_shell(items)?.evaluation()
            }
            // Unsigned arithmetic, checked integers and min/max keep their
            // copying charge: one step per operand cell.
            "core.integer.add"
            | "core.integer.subtract"
            | "core.integer.checkedAdd"
            | "core.integer.checkedSub"
            | "core.integer.checkedMul"
            | "core.integer.checkedDiv"
            | "core.integer.checkedRem"
            | "core.integer.min"
            | "core.integer.max" => {
                let mut cells = 0_u64;
                for argument in list(get(value, "args")?)? {
                    cells = cells.checked_add(self.size(argument)?.cells).ok_or(())?;
                }
                Ok(steps(cells))
            }
            // An Option consumer keeps its copying charge: one step per
            // operand cell plus the bytes of its operands and result.
            "core.option.isSome" | "core.option.unwrap" | "core.option.default" => {
                let mut cost = self.size(value)?.bytes;
                for argument in list(get(value, "args")?)? {
                    let size = self.size(argument)?;
                    cost = cost
                        .checked_add(size.cells)
                        .and_then(|cost| cost.checked_add(size.bytes))
                        .ok_or(())?;
                }
                Ok(steps(cost))
            }
            _ if callee.starts_with("core.") => Err(()),
            _ => {
                let mut arguments = Vec::new();
                for argument in list(get(value, "args")?)? {
                    arguments.push(self.reach(argument)?);
                }
                Ok(self.shared_function(callee, arguments)?.0)
            }
        }
    }

    /// The static size of the value `value` evaluates to.
    fn size(&self, value: &Value) -> Check<Size> {
        match string(get(value, "kind")?)? {
            "record" => {
                let mut total = CELL;
                for (key, field) in members(get(value, "fields")?)? {
                    total = total.plus(self.size(field)?.field(string(key)?)?)?;
                }
                Ok(total)
            }
            "list" => {
                let mut total = CELL;
                for item in list(get(value, "values")?)? {
                    total = total.plus(self.size(item)?)?;
                }
                Ok(total)
            }
            "if" => Ok(self
                .size(get(value, "then")?)?
                .maximum(self.size(get(value, "else")?)?)),
            "match" => {
                let mut result = CELL;
                for arm in list(get(value, "arms")?)? {
                    result = result.maximum(self.size(get(arm, "body")?)?);
                }
                Ok(result)
            }
            _ => type_size(self.recorded(value)?),
        }
    }

    fn shared_predicate(&mut self, value: &'a Value) -> Check<Cost> {
        self.charge(0)?;
        let nested = match string(get(value, "kind")?)? {
            "not" => self.shared_predicate(get(value, "value")?)?,
            "all" | "any" => {
                let mut sum = Cost::default();
                for item in list(get(value, "values")?)? {
                    sum = sum.plus(self.shared_predicate(item)?)?;
                }
                sum
            }
            "compare" => {
                let left = get(value, "left")?;
                let right = get(value, "right")?;
                // Both operands are scanned in full.
                let scan = self.size(left)?.bytes.max(self.size(right)?.bytes);
                self.shared_expression(left)?
                    .plus(self.shared_expression(right)?)?
                    .plus(steps(scan))?
            }
            _ => return Err(()),
        };
        nested.plus(steps(1))
    }
}
