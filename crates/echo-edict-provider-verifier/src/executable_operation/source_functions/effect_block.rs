// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Verifier-owned audit of one block of an authored effect body, written
//! independently of the lowerer: instructions in order and, under the
//! structured contract, bounded loops and statement branches, each block in
//! its own lexical frame.
//!
//! A block's charge is the sum of its nodes. A loop charges its iterable
//! once and then, for each of up to `bound` items, one tick, the checks of
//! its binder's schema and its body. A branch charges its predicate and the
//! larger of its two arms. Mutations add and multiply the same way; the
//! obstruction payload ceiling is the largest any path encodes.
use super::effect_intent::{abi, EffectSite};
use super::{
    exact, get, members, sequence, string, unsigned, Charge, Check, Checker, Frame, Schema, Value,
};
use crate::executable_operation::intrinsic_names::operation;
use std::collections::{BTreeMap, BTreeSet};

/// What one block can cost at most.
#[derive(Clone, Copy, Default)]
pub(super) struct Tally {
    pub(super) charge: Charge,
    pub(super) mutations: u128,
    pub(super) obstruction_output: u128,
}
impl Tally {
    fn add(&mut self, other: Self) -> Check<()> {
        self.charge.append(other.charge)?;
        self.mutations = self.mutations.checked_add(other.mutations).ok_or(())?;
        self.obstruction_output = self.obstruction_output.max(other.obstruction_output);
        Ok(())
    }
    fn larger(a: Self, b: Self) -> Self {
        Self {
            charge: Charge::choice(a.charge, b.charge),
            mutations: a.mutations.max(b.mutations),
            obstruction_output: a.obstruction_output.max(b.obstruction_output),
        }
    }
    fn repeated(self, count: u64) -> Check<Self> {
        let count = u128::from(count);
        let mut charge = Charge::default();
        charge.append(Charge {
            ticks: self.charge.ticks.checked_mul(count).ok_or(())?,
            storage: self.charge.storage.checked_mul(count).ok_or(())?,
        })?;
        Ok(Self {
            charge,
            mutations: self.mutations.checked_mul(count).ok_or(())?,
            obstruction_output: self.obstruction_output,
        })
    }
}

/// State shared by every frame of one body.
pub(super) struct Audit<'s, 'a> {
    pub(super) sites: &'s BTreeMap<usize, EffectSite>,
    /// The next instruction's place in the pre-order walk of the body.
    pub(super) position: usize,
    /// Every local any frame has defined or spent, once in the whole body.
    pub(super) seen: BTreeSet<&'a str>,
    /// Failure binders, spent by their own arm and never in scope.
    pub(super) binders: BTreeSet<&'a str>,
}
impl<'a> Audit<'_, 'a> {
    fn first_use(&mut self, local: &'a Value) -> Check<&'a str> {
        let id = string(get(local, "id")?)?;
        if self.seen.insert(id) {
            Ok(id)
        } else {
            Err(())
        }
    }
}

impl<'a> Checker<'a, '_> {
    /// Audits a block's nodes in `frame`; every local the frame declares is
    /// defined or spent by the end.
    pub(super) fn audit_block(
        &mut self,
        body: &'a Value,
        frame: &mut Frame<'a>,
        audit: &mut Audit<'_, 'a>,
    ) -> Check<Tally> {
        let mut tally = Tally::default();
        for node in sequence(get(body, "nodes")?)? {
            tally.add(Tally {
                charge: Charge {
                    ticks: 1,
                    storage: 0,
                },
                ..Tally::default()
            })?;
            let kind = string(get(node, "kind")?)?;
            let node_tally = match kind {
                "for" => self.audit_loop(node, frame, audit)?,
                "branch" => self.audit_branch(node, frame, audit)?,
                _ => {
                    let position = audit.position;
                    audit.position = position.checked_add(1).ok_or(())?;
                    if (kind == "effect") != audit.sites.contains_key(&position) {
                        return Err(());
                    }
                    self.audit_instruction(kind, node, position, frame, audit)?
                }
            };
            tally.add(node_tally)?;
        }
        for id in frame.declared.keys() {
            let defined = frame.values.contains_key(id);
            if defined == audit.binders.contains(id) {
                return Err(());
            }
        }
        Ok(tally)
    }

    /// A nested frame: the block's own locals over everything visible here.
    fn nested(&mut self, block: &'a Value, frame: &Frame<'a>) -> Check<Frame<'a>> {
        exact(block, &["locals", "nodes", "result"])?;
        let mut nested = Frame::new(sequence(get(block, "locals")?)?, self.types)?;
        // An inner local never shadows an outer one.
        if nested
            .declared
            .keys()
            .any(|id| frame.values.contains_key(id))
        {
            return Err(());
        }
        nested.values = frame.values.clone();
        Ok(nested)
    }

    fn audit_loop(
        &mut self,
        node: &'a Value,
        frame: &Frame<'a>,
        audit: &mut Audit<'_, 'a>,
    ) -> Check<Tally> {
        exact(node, &["kind", "binder", "iter", "bound", "body"])?;
        let iterable = self.expression(get(node, "iter")?, frame, 0)?;
        let Schema::Sequence { element, bound } = iterable.schema.representation().clone() else {
            return Err(());
        };
        let declared = get(node, "bound")?;
        exact(declared, &["kind", "value"])?;
        // Only a literal bound is authenticated, and it covers every list
        // the iterable's schema admits.
        if string(get(declared, "kind")?)? != "literal" {
            return Err(());
        }
        let count = unsigned(get(declared, "value")?)?;
        if bound > count {
            return Err(());
        }
        let block = get(node, "body")?;
        let mut nested = self.nested(block, frame)?;
        let binder = get(node, "binder")?;
        audit.first_use(binder)?;
        let item = self.types.local(binder)?;
        if element.is_some_and(|element| !item.encloses(&element)) {
            return Err(());
        }
        let checks = item.measure()?.checks;
        nested.define(binder, item)?;
        let mut iteration = Tally {
            charge: Charge {
                ticks: checks.checked_add(1).ok_or(())?,
                storage: 0,
            },
            ..Tally::default()
        };
        iteration.add(self.audit_block(block, &mut nested, audit)?)?;
        let mut tally = Tally {
            charge: iterable.charge,
            ..Tally::default()
        };
        tally.add(iteration.repeated(count)?)?;
        Ok(tally)
    }

    fn audit_branch(
        &mut self,
        node: &'a Value,
        frame: &Frame<'a>,
        audit: &mut Audit<'_, 'a>,
    ) -> Check<Tally> {
        exact(node, &["kind", "predicate", "then", "else"])?;
        // The predicate proves nothing for either arm.
        let (charge, _) = self.predicate(get(node, "predicate")?, frame, 0)?;
        let then_block = get(node, "then")?;
        let else_block = get(node, "else")?;
        let mut then_frame = self.nested(then_block, frame)?;
        let then_tally = self.audit_block(then_block, &mut then_frame, audit)?;
        let mut else_frame = self.nested(else_block, frame)?;
        let else_tally = self.audit_block(else_block, &mut else_frame, audit)?;
        let mut tally = Tally {
            charge,
            ..Tally::default()
        };
        tally.add(Tally::larger(then_tally, else_tally))?;
        Ok(tally)
    }

    fn audit_instruction(
        &mut self,
        kind: &str,
        node: &'a Value,
        position: usize,
        frame: &mut Frame<'a>,
        audit: &mut Audit<'_, 'a>,
    ) -> Check<Tally> {
        let mut tally = Tally::default();
        match kind {
            "let" => {
                exact(node, &["kind", "binding", "value"])?;
                let value = self.expression(get(node, "value")?, frame, 0)?;
                let local = get(node, "binding")?;
                let expected = self.types.local(local)?;
                if !expected.encloses(&value.schema) {
                    return Err(());
                }
                self.record_bound(local, get(node, "value")?, &expected)?;
                tally.charge.append(value.charge)?;
                tally.charge.append(Charge {
                    ticks: expected.measure()?.checks,
                    storage: 0,
                })?;
                audit.first_use(local)?;
                frame.define(local, expected)?;
            }
            "require" => {
                exact(node, &["kind", "predicate", "onFailure"])?;
                // Refuses later work; proves nothing for it.
                tally
                    .charge
                    .append(self.predicate(get(node, "predicate")?, frame, 0)?.0)?;
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
                // On failure the runtime builds this record and encodes it as
                // the obstruction payload, under the result's output ceiling.
                let mut fields = BTreeMap::new();
                tally.charge.append(Charge {
                    ticks: 1,
                    storage: 64,
                })?;
                for (name, value) in members(get(reason, "payload")?)? {
                    let name = string(name)?;
                    let checked = self.expression(value, frame, 0)?;
                    tally.charge.append(checked.charge)?;
                    tally.charge.append(Charge {
                        ticks: 0,
                        storage: 64 + name.len() as u128,
                    })?;
                    if fields.insert(name.to_owned(), checked.schema).is_some() {
                        return Err(());
                    }
                }
                let payload = Schema::Fields(fields).measure()?;
                tally.charge.append(payload.copy)?;
                tally.obstruction_output = payload.encoded;
            }
            "effect" => {
                let site = audit.sites.get(&position).ok_or(())?;
                tally = self.audit_effect(node, site, frame, audit)?;
            }
            _ => return Err(()),
        }
        Ok(tally)
    }

    fn audit_effect(
        &mut self,
        node: &'a Value,
        site: &EffectSite,
        frame: &mut Frame<'a>,
        audit: &mut Audit<'_, 'a>,
    ) -> Check<Tally> {
        exact(
            node,
            &["kind", "binding", "effect", "input", "obstructionMap"],
        )?;
        let mut tally = Tally::default();
        let value = self.expression(get(node, "input")?, frame, 0)?;
        let input = self.types.resolve(&site.input, 0)?;
        if !input.encloses(&value.schema) {
            return Err(());
        }
        let output = self.types.resolve(&site.output, 0)?;
        let cost = abi(&site.intrinsic, &input, &output)?;
        if operation(&site.intrinsic) != Some("node-atom-read") {
            tally.mutations = 1;
        }
        let arms = members(get(node, "obstructionMap")?)?;
        if !arms.is_empty() {
            // A mapped read failure encodes an empty payload.
            let payload = Schema::Fields(BTreeMap::new()).measure()?;
            tally.charge.append(payload.copy)?;
            tally.obstruction_output = payload.encoded;
        }
        for (_, arm) in arms {
            exact(arm, &["binder", "value"])?;
            let binder = get(arm, "binder")?;
            let id = string(get(binder, "id")?)?;
            if frame.declared.get(id).copied() != Some(binder) || frame.values.contains_key(id) {
                return Err(());
            }
            audit.first_use(binder)?;
            audit.binders.insert(id);
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
        tally.charge.append(value.charge)?;
        tally.charge.append(Charge {
            ticks: input.measure()?.checks,
            storage: 0,
        })?;
        tally.charge.append(cost)?;
        audit.first_use(local)?;
        frame.define(local, declared)?;
        Ok(tally)
    }
}
