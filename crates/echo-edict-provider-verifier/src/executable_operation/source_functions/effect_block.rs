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
use crate::executable_operation::effect_program::GraphReads;
use crate::executable_operation::intrinsic_names::operation;
use crate::refusal_reason::{RefusalReason as R, Trail};
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
    /// The package's graph-read declaration (echo#982).
    pub(super) graph: Option<GraphReads<'s>>,
    /// The running check and the node it audits.
    pub(super) trail: &'s mut Trail,
}
impl<'a> Audit<'_, 'a> {
    /// Audits `check` inside the child `segment`, which is left once the
    /// child is accepted.
    fn within<T>(
        &mut self,
        segment: &str,
        reason: R,
        check: impl FnOnce(&mut Self) -> Check<T>,
    ) -> Check<T> {
        self.trail.enter(segment);
        self.trail.mark(reason);
        let value = check(self)?;
        self.trail.leave();
        Ok(value)
    }

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
        for (place, node) in sequence(get(body, "nodes")?)?.iter().enumerate() {
            audit.trail.enter(format!("nodes/{place}"));
            audit.trail.mark(R::NodeKind);
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
            audit.trail.mark(R::CostOverflow);
            tally.add(node_tally)?;
            audit.trail.leave();
        }
        // Every local the frame declares is defined or spent by a node.
        audit.trail.mark(R::LocalClaim);
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
        let iter = get(node, "iter")?;
        let iterable = audit.within("iter", R::Expression, |_| self.expression(iter, frame, 0))?;
        let iterable_charge = self.effect_value(iter, iterable.charge)?;
        audit.trail.mark(R::LoopBound);
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
        audit.trail.enter("body");
        audit.trail.mark(R::BlockLocals);
        let mut nested = self.nested(block, frame)?;
        let binder = get(node, "binder")?;
        audit.trail.mark(R::LocalClaim);
        audit.first_use(binder)?;
        audit.trail.mark(R::LoopBinder);
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
        audit.trail.leave();
        audit.trail.mark(R::CostOverflow);
        let mut tally = Tally {
            charge: iterable_charge,
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
        let condition = get(node, "predicate")?;
        let (copying, _) = audit.within("predicate", R::Expression, |_| {
            self.predicate(condition, frame, 0)
        })?;
        let charge = self.effect_condition(condition, copying)?;
        let then_block = get(node, "then")?;
        let else_block = get(node, "else")?;
        audit.trail.enter("then");
        audit.trail.mark(R::BlockLocals);
        let mut then_frame = self.nested(then_block, frame)?;
        let then_tally = self.audit_block(then_block, &mut then_frame, audit)?;
        audit.trail.leave();
        audit.trail.enter("else");
        audit.trail.mark(R::BlockLocals);
        let mut else_frame = self.nested(else_block, frame)?;
        let else_tally = self.audit_block(else_block, &mut else_frame, audit)?;
        audit.trail.leave();
        audit.trail.mark(R::CostOverflow);
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
                let node_value = get(node, "value")?;
                let value = audit.within("value", R::Expression, |_| {
                    self.expression(node_value, frame, 0)
                })?;
                let local = get(node, "binding")?;
                audit.trail.mark(R::BindingType);
                let expected = self.types.local(local)?;
                if !expected.encloses(&value.schema) {
                    return Err(());
                }
                audit.trail.mark(R::Expression);
                self.record_bound(local, node_value, &expected)?;
                audit.trail.mark(R::CostOverflow);
                tally
                    .charge
                    .append(self.effect_value(node_value, value.charge)?)?;
                self.effect_bound(local, node_value)?;
                tally.charge.append(Charge {
                    ticks: expected.measure()?.checks,
                    storage: 0,
                })?;
                audit.trail.mark(R::LocalClaim);
                audit.first_use(local)?;
                frame.define(local, expected)?;
            }
            "attempt" => {
                exact(node, &["kind", "binding", "value", "failureMap"])?;
                // Each arm's payload reads only what precedes the attempt.
                let mut worst = Charge::default();
                audit.trail.enter("failureMap");
                audit.trail.mark(R::Obstruction);
                for (_, reason) in members(get(node, "failureMap")?)? {
                    let (payload, encoded) = self.obstruction(reason, frame)?;
                    worst = Charge::choice(worst, payload);
                    tally.obstruction_output = tally.obstruction_output.max(encoded);
                }
                audit.trail.leave();
                let node_value = get(node, "value")?;
                let value = audit.within("value", R::Expression, |_| {
                    self.expression(node_value, frame, 0)
                })?;
                let local = get(node, "binding")?;
                audit.trail.mark(R::BindingType);
                let expected = self.types.local(local)?;
                if !expected.encloses(&value.schema) {
                    return Err(());
                }
                audit.trail.mark(R::CostOverflow);
                // The call runs, then either its result is checked or exactly
                // one arm's payload is built: charge the larger. The shared
                // executor binds the result unchecked: success adds nothing.
                tally
                    .charge
                    .append(self.effect_value(node_value, value.charge)?)?;
                self.effect_bound(local, node_value)?;
                tally.charge.append(Charge::choice(
                    worst,
                    Charge {
                        ticks: if self.types.shared {
                            0
                        } else {
                            expected.measure()?.checks
                        },
                        storage: 0,
                    },
                ))?;
                audit.trail.mark(R::LocalClaim);
                audit.first_use(local)?;
                frame.define(local, expected)?;
            }
            "require" => {
                exact(node, &["kind", "predicate", "onFailure"])?;
                // Refuses later work; proves nothing for it.
                let condition = get(node, "predicate")?;
                let copying = audit
                    .within("predicate", R::Expression, |_| {
                        self.predicate(condition, frame, 0)
                    })?
                    .0;
                tally
                    .charge
                    .append(self.effect_condition(condition, copying)?)?;
                audit.trail.enter("onFailure");
                audit.trail.mark(R::Require);
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
                let mut building = Charge {
                    ticks: 1,
                    storage: 64,
                };
                let entries = members(get(reason, "payload")?)?;
                for (name, value) in entries {
                    let name = string(name)?;
                    let checked = self.expression(value, frame, 0)?;
                    building.append(checked.charge)?;
                    building.append(Charge {
                        ticks: 0,
                        storage: 64 + name.len() as u128,
                    })?;
                    if fields.insert(name.to_owned(), checked.schema).is_some() {
                        return Err(());
                    }
                }
                if let Some(shared) = self.effect_payload(entries)? {
                    building = shared;
                }
                tally.charge.append(building)?;
                let payload = Schema::Fields(fields).measure()?;
                tally.charge.append(payload.copy)?;
                tally.obstruction_output = payload.encoded;
                audit.trail.leave();
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
        let input_value = get(node, "input")?;
        let value = audit.within("input", R::Expression, |_| {
            self.expression(input_value, frame, 0)
        })?;
        audit.trail.mark(R::EffectInput);
        let input = self.types.resolve(&site.input, 0)?;
        if !input.encloses(&value.schema) {
            return Err(());
        }
        audit.trail.mark(R::EffectAbi);
        let output = self.types.resolve(&site.output, 0)?;
        let cost = match site.graph_read() {
            Some(read) => self.graph_read_charge(read, &input, &output, audit.graph)?,
            None => abi(&site.intrinsic, &input, &output)?,
        };
        if operation(&site.intrinsic) != Some("node-atom-read") && site.graph_read().is_none() {
            tally.mutations = 1;
        }
        audit.trail.mark(R::LocalClaim);
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
        audit.trail.mark(R::EffectOutput);
        let declared = self.types.local(local)?;
        if declared != output {
            return Err(());
        }
        audit.trail.mark(R::CostOverflow);
        tally
            .charge
            .append(self.effect_value(input_value, value.charge)?)?;
        tally.charge.append(Charge {
            ticks: input.measure()?.checks,
            storage: 0,
        })?;
        tally.charge.append(cost)?;
        audit.trail.mark(R::LocalClaim);
        audit.first_use(local)?;
        frame.define(local, declared)?;
        Ok(tally)
    }
}
