// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! This module owns the lowerer's static cost under the shared-read Core cost
//! schedule, `edict.core-cost/v2`, selected by `core.cost.shared-reads/v1`.
//!
//! The rules are Amblicit's (`target_ir/fold_work/shared_reads.rs` at
//! `da1574f1`), so the lowerer's minimum equals the compiler's. Reading a
//! local, field, element or fold state, `if`, `match`, `fold`, a call and a
//! local reference cost one step. Construction charges its shell. A call adds
//! one step per parameter plus one for the result; a fold charges every
//! iteration of its bound. Comparisons scan both operands at their full
//! static size, and the intent's input and output are charged at full size.
//! Unsigned arithmetic keeps its copying charge, one step per operand cell.
//! Byte intrinsics and imported helpers have no shared-read rule in Echo, so
//! the module is refused.
//!
//! The ordinary judgment runs first and alone establishes typing, totality and
//! depth; this pass reads the types it recorded and only computes cost.
use super::{exact, get, list, members, number, shape, string, Check, Cost, Judgment, Ty, Value};

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
            cost = cost
                .plus(self.shared_expression(get(node, "value")?)?)?
                .plus(steps(1))?;
        }
        cost.plus(self.shared_expression(get(body, "result")?)?)
    }

    fn shared_function(&mut self, name: &str) -> Check<Cost> {
        if let Some(cost) = self.shared_costs.get(name) {
            return Ok(*cost);
        }
        let function = *self.functions.get(name).ok_or(())?;
        if !function.source {
            return Err(());
        }
        // Arguments and the result are passed by reference.
        let mut cost = steps(length(function.params.len())?.checked_add(2).ok_or(())?);
        for binding in list(get(function.body, "bindings")?)? {
            cost = cost.plus(self.shared_expression(get(binding, "value")?)?)?;
        }
        cost = cost.plus(self.shared_expression(get(function.body, "result")?)?)?;
        self.shared_costs.insert(name.to_owned(), cost);
        Ok(cost)
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
                let initial = self.shared_expression(get(value, "initial")?)?;
                let iteration = self
                    .shared_function(string(get(value, "step")?)?)?
                    .plus(steps(1))?;
                let bound = number(get(value, "bound")?)?;
                initial.plus(Cost {
                    steps: iteration.steps.checked_mul(bound).ok_or(())?,
                    bytes: iteration.bytes.checked_mul(bound).ok_or(())?,
                })?
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
            // A new list shell holds one reference per item, up to capacity.
            "core.list.append" | "core.list.concat" => {
                let Ty::List(_, max) = self.recorded(value)?.representation() else {
                    return Err(());
                };
                Size::list_shell(*max)?.evaluation()
            }
            // Unsigned arithmetic keeps its copying charge: one step per
            // operand cell.
            "core.integer.add" | "core.integer.subtract" => {
                let mut cells = 0_u64;
                for argument in list(get(value, "args")?)? {
                    cells = cells.checked_add(self.size(argument)?.cells).ok_or(())?;
                }
                Ok(steps(cells))
            }
            _ if callee.starts_with("core.") => Err(()),
            _ => self.shared_function(callee),
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
