// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Verifier-owned typing, proof and charge audit of an authored effect
//! program's body, independent of the lowerer and the runtime.
//!
//! Proofs follow Edict: the intent's input constraints dominate the whole
//! body, an unsigned `let` carries its proven cap to later nodes, earlier
//! conjuncts of one `all` prove later ones, and a `require` proves nothing.
use super::{
    exact, get, members, prepare, sequence, string, unsigned, Charge, Check, Checker, Frame,
    Schema, Types, Value,
};
use std::collections::{BTreeMap, BTreeSet};

/// The largest atom an authored effect may read or write, in bytes.
const STORAGE_CEILING: u64 = 16 * 1024 * 1024;

/// What the verifier's relation established about one effect node.
pub(in crate::executable_operation) struct EffectSite {
    pub(in crate::executable_operation) intrinsic: String,
    pub(in crate::executable_operation) input: String,
    pub(in crate::executable_operation) output: String,
}

/// Audits the selected intent's body against the module's helpers.
pub(in crate::executable_operation) fn judge(
    core: &Value,
    exports: &Value,
    intent: &Value,
    sites: &BTreeMap<usize, EffectSite>,
) -> Check<()> {
    let mut types = Types::new(core);
    let (definitions, summaries) = prepare(core, exports, &mut types)?;
    Checker {
        types: &mut types,
        definitions: &definitions,
        summaries: &summaries,
        proven: Vec::new(),
        bound: Vec::new(),
        proven_sum: None,
    }
    .effect_intent(intent, sites)
}

impl<'a> Checker<'a, '_> {
    #[allow(
        clippy::too_many_lines,
        reason = "One ordered walk keeps scope, proof and charge in step with the body."
    )]
    fn effect_intent(
        &mut self,
        intent: &'a Value,
        sites: &BTreeMap<usize, EffectSite>,
    ) -> Check<()> {
        self.proven.clear();
        self.bound.clear();
        let body = get(intent, "body")?;
        exact(body, &["locals", "nodes", "result"])?;
        let mut frame = Frame::new(sequence(get(body, "locals")?)?, self.types)?;
        // Failure binders are spent by their own arm and never enter scope.
        let mut binders = BTreeSet::new();
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
        if let Some(basis) = super::super::map_field(intent, "basis") {
            charge.append(self.expression(basis, &frame, 0)?.charge)?;
        }
        for constraint in sequence(get(intent, "inputConstraints")?)? {
            charge.append(self.predicate(get(constraint, "predicate")?, &frame, 0)?.0)?;
            self.proven.push(get(constraint, "predicate")?);
        }
        let nodes = sequence(get(body, "nodes")?)?;
        if sites.keys().any(|index| *index >= nodes.len()) {
            return Err(());
        }
        for (index, node) in nodes.iter().enumerate() {
            charge.append(Charge {
                ticks: 1,
                storage: 0,
            })?;
            let kind = string(get(node, "kind")?)?;
            if (kind == "effect") != sites.contains_key(&index) {
                return Err(());
            }
            match kind {
                "let" => {
                    exact(node, &["kind", "binding", "value"])?;
                    let value = self.expression(get(node, "value")?, &frame, 0)?;
                    let local = get(node, "binding")?;
                    let expected = self.types.local(local)?;
                    if !expected.encloses(&value.schema) {
                        return Err(());
                    }
                    self.record_bound(local, get(node, "value")?, &expected)?;
                    charge.append(value.charge)?;
                    charge.append(Charge {
                        ticks: expected.measure()?.checks,
                        storage: 0,
                    })?;
                    frame.define(local, expected)?;
                }
                "require" => {
                    exact(node, &["kind", "predicate", "onFailure"])?;
                    // Refuses later work; proves nothing for it.
                    charge.append(self.predicate(get(node, "predicate")?, &frame, 0)?.0)?;
                    let failure = get(node, "onFailure")?;
                    exact(failure, &["kind", "reason"])?;
                    if string(get(failure, "kind")?)? != "terminal" {
                        return Err(());
                    }
                    let reason = get(failure, "reason")?;
                    exact(reason, &["reasonKind", "payload"])?;
                    if string(get(reason, "reasonKind")?)?.is_empty() {
                        return Err(());
                    }
                    for (_, value) in members(get(reason, "payload")?)? {
                        charge.append(self.expression(value, &frame, 0)?.charge)?;
                    }
                }
                _ => {
                    exact(
                        node,
                        &["kind", "binding", "effect", "input", "obstructionMap"],
                    )?;
                    let site = sites.get(&index).ok_or(())?;
                    let value = self.expression(get(node, "input")?, &frame, 0)?;
                    let input = self.types.resolve(&site.input, 0)?;
                    if !input.encloses(&value.schema) {
                        return Err(());
                    }
                    let output = self.types.resolve(&site.output, 0)?;
                    let cost = abi(&site.intrinsic, &input, &output)?;
                    for (_, arm) in members(get(node, "obstructionMap")?)? {
                        exact(arm, &["binder", "value"])?;
                        let binder = get(arm, "binder")?;
                        let id = string(get(binder, "id")?)?;
                        if frame.declared.get(id).copied() != Some(binder)
                            || frame.values.contains_key(id)
                            || !binders.insert(id)
                        {
                            return Err(());
                        }
                        let reason = get(arm, "value")?;
                        exact(reason, &["kind", "callee", "args", "typeArgs"])?;
                        if string(get(reason, "kind")?)? != "call"
                            || !sequence(get(reason, "args")?)?.is_empty()
                            || !sequence(get(reason, "typeArgs")?)?.is_empty()
                        {
                            return Err(());
                        }
                    }
                    let local = get(node, "binding")?;
                    let declared = self.types.local(local)?;
                    if declared != output {
                        return Err(());
                    }
                    charge.append(value.charge)?;
                    charge.append(Charge {
                        ticks: input.measure()?.checks,
                        storage: 0,
                    })?;
                    charge.append(cost)?;
                    frame.define(local, declared)?;
                }
            }
        }
        if frame.values.len() + binders.len() != frame.declared.len()
            || binders.iter().any(|id| frame.values.contains_key(id))
        {
            return Err(());
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
        exact(
            ceiling,
            &["maxSteps", "maxAllocatedBytes", "maxOutputBytes"],
        )?;
        if charge.ticks > u128::from(unsigned(get(ceiling, "maxSteps")?)?)
            || charge.storage > u128::from(unsigned(get(ceiling, "maxAllocatedBytes")?)?)
            || size.encoded > u128::from(unsigned(get(ceiling, "maxOutputBytes")?)?)
        {
            return Err(());
        }
        Ok(())
    }
}

/// Audits an effect's input and result against its intrinsic's runtime ABI
/// and returns the charge of the effect itself.
fn abi(intrinsic: &str, input: &Schema, output: &Schema) -> Check<Charge> {
    let Schema::Fields(fields) = input.representation() else {
        return Err(());
    };
    let identity = |name: &str| {
        fields.get(name).map(Schema::representation) == Some(&Schema::Blob { low: 32, high: 32 })
    };
    let atom = |name: &str| {
        matches!(
            fields.get(name).map(Schema::representation),
            Some(Schema::Blob { low: 0, high }) if *high > 0 && *high <= STORAGE_CEILING
        )
    };
    let unit = matches!(output.representation(), Schema::Fields(fields) if fields.is_empty());
    let mutation = |names: &[&str]| -> Check<Charge> {
        if fields.len() == names.len()
            && names.iter().all(|name| {
                if *name == "atomBytes" {
                    atom(name)
                } else {
                    identity(name)
                }
            })
            && unit
        {
            Ok(Charge {
                ticks: 1,
                storage: 64,
            })
        } else {
            Err(())
        }
    };
    match intrinsic {
        "echo.dpo@1.node-atom-read" => {
            let Schema::Blob { low: 0, high } = output.representation() else {
                return Err(());
            };
            if fields.len() != 3
                || !["nodeId", "typeId", "warpId"]
                    .iter()
                    .all(|name| identity(name))
                || *high == 0
                || *high > STORAGE_CEILING
            {
                return Err(());
            }
            Ok(Charge {
                ticks: u128::from(*high) + 1,
                storage: u128::from(*high) + 64,
            })
        }
        "echo.dpo@1.ensure-node" | "echo.dpo@1.create-node" => {
            mutation(&["warpId", "nodeId", "nodeType", "atomType", "atomBytes"])
        }
        "echo.dpo@1.replace-atom" => mutation(&[
            "warpId",
            "nodeId",
            "expectedAtomDigest",
            "atomType",
            "atomBytes",
        ]),
        "echo.dpo@1.ensure-edge" => mutation(&["warpId", "edgeId", "fromId", "toId", "edgeType"]),
        "echo.dpo@1.replace-edge" => mutation(&["warpId", "edgeId", "expectedToId", "toId"]),
        _ => Err(()),
    }
}
