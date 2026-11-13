// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Lowerer-owned typing, proof and cost judgment of an authored effect
//! program's body, independent of the runtime's own judgment.
//!
//! Proofs follow Edict: each input constraint is judged with no facts, then
//! all of them dominate the whole body; an unsigned `let` carries its proven
//! cap to later nodes, earlier conjuncts of one `all` prove later ones, and a
//! `require` proves nothing.
use super::effect_block::{claim, Walk};
use super::{
    exact, get, includes, judged, list, members, number, shape, string, Check, Cost, Judgment, Ty,
    Value,
};
use crate::executable_operation::intrinsic_names::operation;
use std::collections::{BTreeMap, BTreeSet};

/// The largest atom an authored effect may read or write, in bytes.
const STORAGE_CEILING: u64 = 16 * 1024 * 1024;
/// The row the runtime reserves for each emitted mutation, in bytes, beside
/// the 64-byte unit value the effect binds.
const EMISSION_RESERVATION: u64 = 280;
/// The most mutations one runtime evaluation emits.
const EFFECT_COUNT_CEILING: u64 = 1024;
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
    judged(core, exports, 0)?.effect_intent(intent, sites)
}

impl<'a> Judgment<'a> {
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
        let mut pending = self.block_locals(body)?;
        let mut walk = Walk {
            sites,
            index: 0,
            claimed: BTreeSet::new(),
        };
        let input = *pending.get("arg.0").ok_or(())?;
        if get(input, "type")? != get(intent, "input")? {
            return Err(());
        }
        claim(input, &mut pending, &mut walk)?;
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
        let totals = self.effect_block(body, &mut scope, pending, &mut walk)?;
        // Every effect site the relation established is an instruction of
        // this body.
        if sites.keys().any(|index| *index >= walk.index) {
            return Err(());
        }
        cost = cost.plus(totals.cost)?;
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
            || totals.obstruction_output > max_output
            || totals.mutations > EFFECT_COUNT_CEILING
        {
            return Err(());
        }
        Ok(())
    }
}
/// Checks an effect's input and result against its intrinsic's runtime ABI
/// and returns the work the effect itself costs.
pub(super) fn abi(intrinsic: &str, input: &Ty, output: &Ty) -> Check<Cost> {
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
    match operation(intrinsic).ok_or(())? {
        "node-atom-read" => {
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
        "ensure-node" | "create-node" => {
            mutation(&["warpId", "nodeId", "nodeType", "atomType", "atomBytes"])
        }
        "replace-atom" => mutation(&[
            "warpId",
            "nodeId",
            "expectedAtomDigest",
            "atomType",
            "atomBytes",
        ]),
        "ensure-edge" => mutation(&["warpId", "edgeId", "fromId", "toId", "edgeType"]),
        "replace-edge" => mutation(&["warpId", "edgeId", "expectedToId", "toId"]),
        _ => Err(()),
    }
}
