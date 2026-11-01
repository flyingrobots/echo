// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! First-order bounded iteration; helper bodies retain their own proof frames.
use super::{
    exact, get, includes, list, number, shape, string, Check, Cost, Fact, Judgment, Scope, Value,
    DEPTH,
};
impl<'a> Judgment<'a> {
    pub(super) fn state_fold(
        &mut self,
        value: &'a Value,
        scope: &Scope<'a>,
        source: bool,
        prefix: usize,
    ) -> Check<Fact> {
        exact(value, &["kind", "initial", "step", "bound", "stateType"])?;
        if !source
            || !list(get(self.core, "requiredCoreCapabilities")?)?
                .iter()
                .any(|c| string(c) == Ok("core.fold/v1"))
        {
            return Err(());
        }
        let name = string(get(value, "step")?)?;
        let state = string(get(value, "stateType")?)?;
        let bound = number(get(value, "bound")?)?;
        let function = *self.functions.get(name).ok_or(())?;
        if !function.source
            || function.params.len() != 1
            || function.result != state
            || string(get(&function.params[0], "type")?)? != state
        {
            return Err(());
        }
        let ty = self.ty(state, 0)?;
        let initial = self.expression(get(value, "initial")?, scope, source, prefix + 1)?;
        if !includes(&ty, &initial.ty) {
            return Err(());
        }
        // Check the step even at zero iterations: dead syntax grants no authority
        // to retain recursion, missing helpers, or a malformed state signature.
        let step = self.function(name, prefix + 1)?;
        let (_, checks, _, state_depth) = shape(&ty)?;
        let iteration = step.cost.plus(Cost {
            steps: checks.checked_add(1).ok_or(())?,
            bytes: 0,
        })?;
        let cost = initial
            .cost
            .plus(Cost {
                steps: checks.checked_add(1).ok_or(())?,
                bytes: 0,
            })?
            .plus(Cost {
                steps: iteration.steps.checked_mul(bound).ok_or(())?,
                bytes: iteration.bytes.checked_mul(bound).ok_or(())?,
            })?;
        let depth = initial.depth.max(step.depth).max(state_depth) + 1;
        if prefix.checked_add(depth).ok_or(())? > DEPTH {
            return Err(());
        }
        Ok(Fact { ty, cost, depth })
    }
}
