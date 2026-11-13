// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Verifier-owned typing, proof and charge audit of an authored effect
//! program's body, independent of the lowerer and the runtime.
//!
//! Proofs follow Edict: each input constraint is judged with no facts, then
//! all of them dominate the whole body; an unsigned `let` carries its proven
//! cap to later nodes, earlier conjuncts of one `all` prove later ones, and a
//! `require` proves nothing.
use super::effect_block::Audit;
use super::{
    exact, get, members, prepare, sequence, string, unsigned, Charge, Check, Checker, Frame,
    Schema, Types, Value,
};
use crate::executable_operation::intrinsic_names::operation;
use std::collections::{BTreeMap, BTreeSet};

/// The largest atom an authored effect may read or write, in bytes.
const STORAGE_CEILING: u64 = 16 * 1024 * 1024;
/// The row the runtime reserves for each emitted mutation, in bytes, beside
/// the 64-byte unit value the effect binds.
const EMISSION_RESERVATION: u64 = 280;
/// The most mutations one runtime evaluation emits.
const EFFECT_COUNT_CEILING: u128 = 1024;
/// What one root-warp atom read records beside its payload, in bytes: two
/// 128-byte observations, two 64-byte authorizations and the 64-byte value.
const READ_RECORDS: u128 = 448;

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
    let mut types = Types::new(core);
    let (definitions, summaries) = prepare(core, exports, &mut types, 0)?;
    Checker {
        types: &mut types,
        definitions: &definitions,
        summaries: &summaries,
        proven: Vec::new(),
        branch: Vec::new(),
        path: Vec::new(),
        bound: Vec::new(),
        proven_sum: None,
        edge_scans: 0,
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
        self.branch.clear();
        self.path.clear();
        self.bound.clear();
        let body = get(intent, "body")?;
        exact(body, &["locals", "nodes", "result"])?;
        let mut frame = Frame::new(sequence(get(body, "locals")?)?, self.types)?;
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
            let checked = self.expression(basis, &frame, 0)?;
            // The runtime refuses any evaluation basis but exactly 32 bytes.
            if checked.schema.representation() != &(Schema::Blob { low: 32, high: 32 }) {
                return Err(());
            }
            charge.append(checked.charge)?;
        }
        // Each input constraint is judged with no facts, as the runtime does;
        // only then do they all dominate the body. One constraint never
        // proves another.
        let constraints = sequence(get(intent, "inputConstraints")?)?;
        for constraint in constraints {
            // The runtime names each constraint by its coordinate when it fails.
            exact(constraint, &["coordinate", "predicate"])?;
            string(get(constraint, "coordinate")?)?;
            charge.append(self.predicate(get(constraint, "predicate")?, &frame, 0)?.0)?;
        }
        for constraint in constraints {
            self.proven.push(get(constraint, "predicate")?);
        }
        let mut audit = Audit {
            sites,
            position: 0,
            seen: BTreeSet::new(),
            binders: BTreeSet::new(),
        };
        let tally = self.audit_block(body, &mut frame, &mut audit)?;
        // Every effect site the relation established is an instruction here.
        if sites.keys().any(|index| *index >= audit.position) {
            return Err(());
        }
        charge.append(tally.charge)?;
        let obstruction_output = tally.obstruction_output;
        let mutations = tally.mutations;
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
        let max_output = u128::from(unsigned(get(ceiling, "maxOutputBytes")?)?);
        if charge.ticks > u128::from(unsigned(get(ceiling, "maxSteps")?)?)
            || charge.storage > u128::from(unsigned(get(ceiling, "maxAllocatedBytes")?)?)
            || size.encoded > max_output
            || obstruction_output > max_output
            || mutations > EFFECT_COUNT_CEILING
        {
            return Err(());
        }
        Ok(())
    }
}

/// Audits an effect's input and result against its intrinsic's runtime ABI
/// and returns the charge of the effect itself.
pub(super) fn abi(intrinsic: &str, input: &Schema, output: &Schema) -> Check<Charge> {
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
    // The runtime requires the result itself to be the ABI's type, not a
    // nominal wrapper around it.
    let unit = matches!(output, Schema::Fields(fields) if fields.is_empty());
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
                storage: 64 + u128::from(EMISSION_RESERVATION),
            })
        } else {
            Err(())
        }
    };
    match operation(intrinsic).ok_or(())? {
        "node-atom-read" => {
            let Schema::Blob { low: 0, high } = output else {
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
                // The payload is retained as read evidence and copied into
                // the value, beside the read's fixed records.
                storage: 2 * u128::from(*high) + READ_RECORDS,
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
