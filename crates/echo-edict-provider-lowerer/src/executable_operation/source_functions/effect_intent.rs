// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Lowerer-owned typing, proof and cost judgment of an authored effect
//! program's body, independent of the runtime's own judgment.
//!
//! Proofs follow Edict: the intent's input constraints dominate the whole
//! body, an unsigned `let` carries its proven cap to later nodes, earlier
//! conjuncts of one `all` prove later ones, and a `require` proves nothing.
use super::{
    exact, get, includes, judgment, list, members, number, shape, string, Check, Cost, Judgment,
    Ty, Value,
};
use std::collections::BTreeMap;

/// The largest atom an authored effect may read or write, in bytes.
const STORAGE_CEILING: u64 = 16 * 1024 * 1024;

/// What the lowering relation established about one effect node.
pub(in crate::executable_operation) struct EffectSite {
    pub(in crate::executable_operation) intrinsic: String,
    pub(in crate::executable_operation) input: String,
    pub(in crate::executable_operation) output: String,
}

/// Judges the selected intent's body with the module's helpers.
pub(in crate::executable_operation) fn judge(
    core: &Value,
    exports: &Value,
    intent: &Value,
    sites: &BTreeMap<usize, EffectSite>,
) -> Check<()> {
    judgment(core, exports)?.effect_intent(intent, sites)
}

impl<'a> Judgment<'a> {
    #[allow(
        clippy::too_many_lines,
        reason = "One ordered walk keeps scope, proof and cost in step with the body."
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
        if let Some(basis) = super::super::map_field(intent, "basis") {
            cost = cost.plus(self.expression(basis, &scope, true, 0)?.cost)?;
        }
        for constraint in list(get(intent, "inputConstraints")?)? {
            cost = cost.plus(
                self.predicate(get(constraint, "predicate")?, &scope, true, 0)?
                    .0,
            )?;
            self.proven.push(get(constraint, "predicate")?);
        }
        let nodes = list(get(body, "nodes")?)?;
        if sites.keys().any(|index| *index >= nodes.len()) {
            return Err(());
        }
        for (index, node) in nodes.iter().enumerate() {
            cost = cost.plus(Cost { steps: 1, bytes: 0 })?;
            let mut claim = |local: &'a Value| -> Check<()> {
                if pending.remove(string(get(local, "id")?)?) == Some(local) {
                    Ok(())
                } else {
                    Err(())
                }
            };
            match string(get(node, "kind")?)? {
                "let" => {
                    exact(node, &["kind", "binding", "value"])?;
                    if sites.contains_key(&index) {
                        return Err(());
                    }
                    claim(get(node, "binding")?)?;
                    let fact = self.expression(get(node, "value")?, &scope, true, 0)?;
                    let ty = self.declaration(get(node, "binding")?, &mut scope)?;
                    if !includes(&ty, &fact.ty) {
                        return Err(());
                    }
                    self.record_bound(get(node, "binding")?, get(node, "value")?, &ty)?;
                    cost = cost.plus(fact.cost)?.plus(Cost {
                        steps: shape(&ty)?.1,
                        bytes: 0,
                    })?;
                }
                "require" => {
                    exact(node, &["kind", "predicate", "onFailure"])?;
                    if sites.contains_key(&index) {
                        return Err(());
                    }
                    // A require refuses later work; it is not a proof for it.
                    cost =
                        cost.plus(self.predicate(get(node, "predicate")?, &scope, true, 0)?.0)?;
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
                        cost = cost.plus(self.expression(value, &scope, true, 0)?.cost)?;
                    }
                }
                "effect" => {
                    exact(
                        node,
                        &["kind", "binding", "effect", "input", "obstructionMap"],
                    )?;
                    let site = sites.get(&index).ok_or(())?;
                    let fact = self.expression(get(node, "input")?, &scope, true, 0)?;
                    let expected = self.ty(&site.input, 0)?;
                    if !includes(&expected, &fact.ty) {
                        return Err(());
                    }
                    let output = self.ty(&site.output, 0)?;
                    let charge = abi(&site.intrinsic, &expected, &output)?;
                    for (_, arm) in members(get(node, "obstructionMap")?)? {
                        exact(arm, &["binder", "value"])?;
                        claim(get(arm, "binder")?)?;
                        let reason = get(arm, "value")?;
                        exact(reason, &["kind", "callee", "args", "typeArgs"])?;
                        if string(get(reason, "kind")?)? != "call"
                            || !list(get(reason, "args")?)?.is_empty()
                            || !list(get(reason, "typeArgs")?)?.is_empty()
                        {
                            return Err(());
                        }
                    }
                    claim(get(node, "binding")?)?;
                    let declared = self.declaration(get(node, "binding")?, &mut scope)?;
                    if declared != output {
                        return Err(());
                    }
                    cost = cost
                        .plus(fact.cost)?
                        .plus(Cost {
                            steps: shape(&expected)?.1,
                            bytes: 0,
                        })?
                        .plus(charge)?;
                }
                _ => return Err(()),
            }
        }
        if !pending.is_empty() {
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
        exact(budget, &["maxSteps", "maxAllocatedBytes", "maxOutputBytes"])?;
        if cost.steps > number(get(budget, "maxSteps")?)?
            || cost.bytes > number(get(budget, "maxAllocatedBytes")?)?
            || output > number(get(budget, "maxOutputBytes")?)?
        {
            return Err(());
        }
        Ok(())
    }
}

/// Checks an effect's input and result against its intrinsic's runtime ABI
/// and returns the work the effect itself costs.
fn abi(intrinsic: &str, input: &Ty, output: &Ty) -> Check<Cost> {
    let Ty::Record(fields) = input.representation() else {
        return Err(());
    };
    let id = |name: &str| fields.get(name).map(Ty::representation) == Some(&Ty::Bytes(32, 32));
    let atom = |name: &str| {
        matches!(
            fields.get(name).map(Ty::representation),
            Some(Ty::Bytes(0, max)) if *max > 0 && *max <= STORAGE_CEILING
        )
    };
    let unit = matches!(output.representation(), Ty::Record(fields) if fields.is_empty());
    let mutation = |names: &[&str]| -> Check<Cost> {
        let conforming = fields.len() == names.len()
            && names.iter().all(|name| {
                if *name == "atomBytes" {
                    atom(name)
                } else {
                    id(name)
                }
            });
        if conforming && unit {
            Ok(Cost {
                steps: 1,
                bytes: 64,
            })
        } else {
            Err(())
        }
    };
    match intrinsic {
        "echo.dpo@1.node-atom-read" => {
            let Ty::Bytes(0, max) = output.representation() else {
                return Err(());
            };
            if fields.len() != 3
                || !["nodeId", "typeId", "warpId"].iter().all(|name| id(name))
                || *max == 0
                || *max > STORAGE_CEILING
            {
                return Err(());
            }
            Ok(Cost {
                steps: max.checked_add(1).ok_or(())?,
                bytes: max.checked_add(64).ok_or(())?,
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
