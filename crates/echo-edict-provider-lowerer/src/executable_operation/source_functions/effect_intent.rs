// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Lowerer-owned typing, proof and cost judgment of an authored effect
//! program's body, independent of the runtime's own judgment.
//!
//! Proofs follow Edict: each input constraint is judged with no facts, then
//! all of them dominate the whole body; an unsigned `let` carries its proven
//! cap to later nodes, earlier conjuncts of one `all` prove later ones, and a
//! `require` proves nothing.
use super::{
    exact, get, includes, judgment, list, members, number, shape, string, Check, Cost, Judgment,
    Ty, Value,
};
use std::collections::BTreeMap;

/// The largest atom an authored effect may read or write, in bytes.
const STORAGE_CEILING: u64 = 16 * 1024 * 1024;
/// The row the runtime reserves for each emitted mutation, in bytes, beside
/// the 64-byte unit value the effect binds.
const EMISSION_RESERVATION: u64 = 280;
/// The most mutations one runtime evaluation emits.
const EFFECT_COUNT_CEILING: usize = 1024;
/// What one root-warp atom read records beside its payload, in bytes: two
/// 128-byte observations, two 64-byte authorizations and the 64-byte value.
const READ_RECORDS: u64 = 448;

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
    // Core names an unsigned width directly; the runtime has no `Int` kind.
    // A nominal entry has exactly its kind, contract and representation.
    if let Some(types) = super::super::map_field(core, "types") {
        for (_, definition) in members(types)? {
            match super::super::map_field(definition, "kind") {
                Some(Value::Text(kind)) if kind == "Int" => return Err(()),
                Some(Value::Text(kind)) if kind == "Nominal" => {
                    exact(definition, &["kind", "contract", "representation"])?;
                }
                _ => {}
            }
        }
    }
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
        self.branch.clear();
        self.path.clear();
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
            let fact = self.expression(basis, &scope, true, 0)?;
            // The runtime refuses any evaluation basis but exactly 32 bytes.
            if fact.ty.representation() != &Ty::Bytes(32, 32) {
                return Err(());
            }
            cost = cost.plus(fact.cost)?;
        }
        // Each input constraint is judged with no facts, as the runtime does;
        // only then do they all dominate the body. One constraint never
        // proves another.
        let constraints = list(get(intent, "inputConstraints")?)?;
        for constraint in constraints {
            // The runtime names each constraint by its coordinate when it fails.
            exact(constraint, &["coordinate", "predicate"])?;
            string(get(constraint, "coordinate")?)?;
            cost = cost.plus(
                self.predicate(get(constraint, "predicate")?, &scope, true, 0)?
                    .0,
            )?;
        }
        for constraint in constraints {
            self.proven.push(get(constraint, "predicate")?);
        }
        let nodes = list(get(body, "nodes")?)?;
        if sites.keys().any(|index| *index >= nodes.len()) {
            return Err(());
        }
        // The largest obstruction payload a failure path encodes, and the
        // mutations the body emits.
        let mut obstruction_output = 0_u64;
        let mut mutations = 0_usize;
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
                    // On failure the runtime builds this record and encodes it
                    // as the obstruction payload, under the result's output
                    // ceiling.
                    let mut fields = BTreeMap::new();
                    let mut payload = Cost {
                        steps: 1,
                        bytes: 64,
                    };
                    for (name, value) in members(get(reason, "payload")?)? {
                        let name = string(name)?;
                        let fact = self.expression(value, &scope, true, 0)?;
                        payload = payload.plus(fact.cost)?.plus(Cost {
                            steps: 0,
                            bytes: 64_u64.checked_add(name.len() as u64).ok_or(())?,
                        })?;
                        if fields.insert(name.to_owned(), fact.ty).is_some() {
                            return Err(());
                        }
                    }
                    let (scratch, _, encoded, _) = shape(&Ty::Record(fields))?;
                    cost = cost.plus(payload)?.plus(scratch)?;
                    obstruction_output = obstruction_output.max(encoded);
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
                    if site.intrinsic != "echo.dpo@1.node-atom-read" {
                        mutations = mutations.checked_add(1).ok_or(())?;
                    }
                    let arms = members(get(node, "obstructionMap")?)?;
                    if !arms.is_empty() {
                        // A mapped read failure encodes an empty payload.
                        let (scratch, _, encoded, _) = shape(&Ty::Record(BTreeMap::new()))?;
                        cost = cost.plus(scratch)?;
                        obstruction_output = obstruction_output.max(encoded);
                    }
                    for (_, arm) in arms {
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
        let max_output = number(get(budget, "maxOutputBytes")?)?;
        if cost.steps > number(get(budget, "maxSteps")?)?
            || cost.bytes > number(get(budget, "maxAllocatedBytes")?)?
            || output > max_output
            || obstruction_output > max_output
            || mutations > EFFECT_COUNT_CEILING
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
    // The runtime requires the result itself to be the ABI's type, not a
    // nominal wrapper around it.
    let unit = matches!(output, Ty::Record(fields) if fields.is_empty());
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
                bytes: 64 + EMISSION_RESERVATION,
            })
        } else {
            Err(())
        }
    };
    match intrinsic {
        "echo.dpo@1.node-atom-read" => {
            let Ty::Bytes(0, max) = output else {
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
                // The payload is retained as read evidence and copied into
                // the value, beside the read's fixed records.
                bytes: max
                    .checked_mul(2)
                    .and_then(|bytes| bytes.checked_add(READ_RECORDS))
                    .ok_or(())?,
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
