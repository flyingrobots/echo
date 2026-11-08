// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Independently charge a statically named invariant-state step from the DAG.
use super::{
    exact, get, sequence, string, unsigned, Charge, Check, Checker, Expression, Frame, Value,
};
impl<'a> Checker<'a, '_> {
    pub(super) fn state_fold(
        &mut self,
        value: &'a Value,
        frame: &Frame<'a>,
        depth: usize,
    ) -> Check<Expression> {
        exact(value, &["kind", "initial", "step", "bound", "stateType"])?;
        if !sequence(get(self.types.module, "requiredCoreCapabilities")?)?
            .iter()
            .any(|c| string(c) == Ok("core.fold/v1"))
        {
            return Err(());
        }
        let name = string(get(value, "step")?)?;
        let state = string(get(value, "stateType")?)?;
        let bound = u128::from(unsigned(get(value, "bound")?)?);
        let definition = self.definitions.get(name).ok_or(())?;
        if !definition.source
            || definition.params.len() != 1
            || string(get(&definition.params[0], "type")?)? != state
        {
            return Err(());
        }
        let prefix = format!("{}.", string(get(self.types.module, "coordinate")?)?);
        let source_name = name.strip_prefix(&prefix).ok_or(())?;
        let source_function = get(get(self.types.module, "functions")?, source_name)?;
        if string(get(source_function, "returnType")?)? != state {
            return Err(());
        }
        let schema = self.types.resolve(state, 0)?;
        if definition.result != schema || definition.parameter_types[0] != schema {
            return Err(());
        }
        let initial = self.expression(get(value, "initial")?, frame, depth + 1)?;
        if !schema.encloses(&initial.schema) {
            return Err(());
        }
        let step = *self.summaries.get(name).ok_or(())?;
        let size = schema.measure()?;
        let mut charge = initial.charge;
        charge.append(Charge {
            ticks: 1 + size.checks,
            storage: 0,
        })?;
        charge.append(Charge {
            ticks: step
                .charge
                .ticks
                .checked_add(size.checks)
                .and_then(|n| n.checked_add(1))
                .and_then(|n| n.checked_mul(bound))
                .ok_or(())?,
            storage: step.charge.storage.checked_mul(bound).ok_or(())?,
        })?;
        let height = initial.height.max(step.height).max(size.height) + 1;
        if depth.checked_add(height).ok_or(())? > 64 {
            return Err(());
        }
        Ok(Expression {
            schema,
            charge,
            height,
        })
    }
}
