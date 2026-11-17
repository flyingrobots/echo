// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Lowerer-owned judgment of one block of an authored effect body: its
//! instructions in order and, under the structured contract, bounded loops
//! and statement branches with their own scopes.
//!
//! The worst case of a block is the sum of its nodes. A loop costs its
//! iterable once and then, `bound` times, one step, the validation of its
//! binder's type and its body; a branch costs its predicate and the larger of
//! its arms. Mutations multiply and add the same way, and the largest
//! obstruction payload is the largest of any path.
use super::effect_intent::{abi, EffectSite};
use super::{
    exact, get, includes, list, members, number, shape, string, Check, Cost, Judgment, Scope, Ty,
    Value,
};
use crate::executable_operation::intrinsic_names::operation;
use std::collections::{BTreeMap, BTreeSet};

/// The worst case of one block.
#[derive(Clone, Copy, Default)]
pub(super) struct Totals {
    pub(super) cost: Cost,
    /// The most mutations the block emits.
    pub(super) mutations: u64,
    /// The largest obstruction payload any failure inside the block encodes.
    pub(super) obstruction_output: u64,
}
impl Totals {
    fn then(self, other: Self) -> Check<Self> {
        Ok(Self {
            cost: self.cost.plus(other.cost)?,
            mutations: self.mutations.checked_add(other.mutations).ok_or(())?,
            obstruction_output: self.obstruction_output.max(other.obstruction_output),
        })
    }
    fn either(self, other: Self) -> Self {
        Self {
            cost: self.cost.maximum(other.cost),
            mutations: self.mutations.max(other.mutations),
            obstruction_output: self.obstruction_output.max(other.obstruction_output),
        }
    }
    fn times(self, count: u64) -> Check<Self> {
        Ok(Self {
            cost: Cost {
                steps: self.cost.steps.checked_mul(count).ok_or(())?,
                bytes: self.cost.bytes.checked_mul(count).ok_or(())?,
            },
            mutations: self.mutations.checked_mul(count).ok_or(())?,
            obstruction_output: self.obstruction_output,
        })
    }
    fn costing(cost: Cost) -> Self {
        Self {
            cost,
            ..Self::default()
        }
    }
}

/// The locals a block declares, not yet claimed by a node.
pub(super) type Pending<'a> = BTreeMap<&'a str, &'a Value>;

/// State shared by every block of one body.
pub(super) struct Walk<'s> {
    pub(super) sites: &'s BTreeMap<usize, EffectSite>,
    /// The next instruction's position in the pre-order walk, the key the
    /// lowering relation gave each effect site.
    pub(super) index: usize,
    /// Every local claimed anywhere in the body: identities are unique
    /// across the whole intent.
    pub(super) claimed: BTreeSet<String>,
}

impl<'a> Judgment<'a> {
    /// A block's local table, each declaration well formed and unique.
    pub(super) fn block_locals(&mut self, body: &'a Value) -> Check<Pending<'a>> {
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
        Ok(pending)
    }

    /// Judges a block's nodes in order in `scope`, claiming each local it
    /// introduces from `pending`, which must end empty.
    pub(super) fn effect_block(
        &mut self,
        body: &'a Value,
        scope: &mut Scope<'a>,
        mut pending: Pending<'a>,
        walk: &mut Walk<'_>,
    ) -> Check<Totals> {
        let mut totals = Totals::default();
        for node in list(get(body, "nodes")?)? {
            // Every node, control or instruction, is one runtime step.
            totals = totals.then(Totals::costing(Cost { steps: 1, bytes: 0 }))?;
            let node_totals = match string(get(node, "kind")?)? {
                "for" => self.effect_loop(node, scope, walk)?,
                "branch" => self.effect_branch(node, scope, walk)?,
                kind => {
                    let index = walk.index;
                    walk.index = index.checked_add(1).ok_or(())?;
                    let site = walk.sites.get(&index);
                    self.effect_instruction(kind, node, site, scope, &mut pending, walk)?
                }
            };
            totals = totals.then(node_totals)?;
        }
        if !pending.is_empty() {
            return Err(());
        }
        Ok(totals)
    }

    /// `for binder in iter bounded n`: the literal bound covers the
    /// iterable's list type and the binder, the body's own first claim,
    /// accepts its items.
    fn effect_loop(
        &mut self,
        node: &'a Value,
        scope: &Scope<'a>,
        walk: &mut Walk<'_>,
    ) -> Check<Totals> {
        exact(node, &["kind", "binder", "iter", "bound", "body"])?;
        let iterable = self.expression(get(node, "iter")?, scope, true, 0)?;
        let Ty::List(item, max) = iterable.ty.representation().clone() else {
            return Err(());
        };
        let bound = get(node, "bound")?;
        exact(bound, &["kind", "value"])?;
        if string(get(bound, "kind")?)? != "literal" {
            return Err(());
        }
        let bound = number(get(bound, "value")?)?;
        if max > bound {
            return Err(());
        }
        let body = get(node, "body")?;
        let mut pending = self.block_locals(body)?;
        let mut child = scope.clone();
        let binder = get(node, "binder")?;
        claim(binder, &mut pending, walk)?;
        let binder_ty = self.declaration(binder, &mut child)?;
        if item.is_some_and(|item| !includes(&binder_ty, &item)) {
            return Err(());
        }
        let body = self.effect_block(body, &mut child, pending, walk)?;
        let iteration = Totals::costing(Cost {
            steps: shape(&binder_ty)?.1.checked_add(1).ok_or(())?,
            bytes: 0,
        })
        .then(body)?;
        Totals::costing(iterable.cost).then(iteration.times(bound)?)
    }

    /// `if predicate { then } else { else }`: each arm in its own scope; the
    /// worst case is the larger arm.
    fn effect_branch(
        &mut self,
        node: &'a Value,
        scope: &Scope<'a>,
        walk: &mut Walk<'_>,
    ) -> Check<Totals> {
        exact(node, &["kind", "predicate", "then", "else"])?;
        // A branch predicate is not a fact for either arm.
        let (predicate, _) = self.predicate(get(node, "predicate")?, scope, true, 0)?;
        let mut arms = Vec::with_capacity(2);
        for arm in ["then", "else"] {
            let body = get(node, arm)?;
            let pending = self.block_locals(body)?;
            let mut child = scope.clone();
            arms.push(self.effect_block(body, &mut child, pending, walk)?);
        }
        let [then, otherwise] = arms.as_slice() else {
            return Err(());
        };
        Totals::costing(predicate).then(then.either(*otherwise))
    }

    /// One `let`, `require` or `effect` node, with the effect site the
    /// lowering relation established for it, if any.
    #[allow(
        clippy::too_many_lines,
        reason = "One node's scope, proof and cost are judged together."
    )]
    fn effect_instruction(
        &mut self,
        kind: &str,
        node: &'a Value,
        site: Option<&EffectSite>,
        scope: &mut Scope<'a>,
        pending: &mut Pending<'a>,
        walk: &mut Walk<'_>,
    ) -> Check<Totals> {
        match kind {
            "let" => {
                exact(node, &["kind", "binding", "value"])?;
                if site.is_some() {
                    return Err(());
                }
                claim(get(node, "binding")?, pending, walk)?;
                let fact = self.expression(get(node, "value")?, scope, true, 0)?;
                let ty = self.declaration(get(node, "binding")?, scope)?;
                if !includes(&ty, &fact.ty) {
                    return Err(());
                }
                self.record_bound(get(node, "binding")?, get(node, "value")?, &ty)?;
                Ok(Totals::costing(fact.cost.plus(Cost {
                    steps: shape(&ty)?.1,
                    bytes: 0,
                })?))
            }
            "require" => {
                exact(node, &["kind", "predicate", "onFailure"])?;
                if site.is_some() {
                    return Err(());
                }
                // A require refuses later work; it is not a proof for it.
                let mut cost = self.predicate(get(node, "predicate")?, scope, true, 0)?.0;
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
                let mut payload = Cost {
                    steps: 1,
                    bytes: 64,
                };
                for (name, value) in members(get(reason, "payload")?)? {
                    let name = string(name)?;
                    let fact = self.expression(value, scope, true, 0)?;
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
                Ok(Totals {
                    cost,
                    mutations: 0,
                    obstruction_output: encoded,
                })
            }
            "effect" => self.effect_call(node, site.ok_or(())?, scope, pending, walk),
            _ => Err(()),
        }
    }

    fn effect_call(
        &mut self,
        node: &'a Value,
        site: &EffectSite,
        scope: &mut Scope<'a>,
        pending: &mut Pending<'a>,
        walk: &mut Walk<'_>,
    ) -> Check<Totals> {
        exact(
            node,
            &["kind", "binding", "effect", "input", "obstructionMap"],
        )?;
        let fact = self.expression(get(node, "input")?, scope, true, 0)?;
        let expected = self.ty(&site.input, 0)?;
        if !includes(&expected, &fact.ty) {
            return Err(());
        }
        let output = self.ty(&site.output, 0)?;
        let charge = abi(&site.intrinsic, &expected, &output)?;
        let mutations = u64::from(operation(&site.intrinsic) != Some("node-atom-read"));
        let mut cost = Cost::default();
        let mut obstruction_output = 0;
        let arms = members(get(node, "obstructionMap")?)?;
        if !arms.is_empty() {
            // A mapped read failure encodes an empty payload.
            let (scratch, _, encoded, _) = shape(&Ty::Record(BTreeMap::new()))?;
            cost = cost.plus(scratch)?;
            obstruction_output = encoded;
        }
        for (_, arm) in arms {
            exact(arm, &["binder", "value"])?;
            claim(get(arm, "binder")?, pending, walk)?;
            let reason = get(arm, "value")?;
            exact(reason, &["kind", "callee", "args", "typeArgs"])?;
            if string(get(reason, "kind")?)? != "call"
                || !list(get(reason, "args")?)?.is_empty()
                || !list(get(reason, "typeArgs")?)?.is_empty()
            {
                return Err(());
            }
        }
        claim(get(node, "binding")?, pending, walk)?;
        let declared = self.declaration(get(node, "binding")?, scope)?;
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
        Ok(Totals {
            cost,
            mutations,
            obstruction_output,
        })
    }
}

/// Claims `local` from the block's own table, once in the whole body.
pub(super) fn claim<'a>(
    local: &'a Value,
    pending: &mut Pending<'a>,
    walk: &mut Walk<'_>,
) -> Check<()> {
    let id = string(get(local, "id")?)?;
    if pending.remove(id) == Some(local) && walk.claimed.insert(id.to_owned()) {
        Ok(())
    } else {
        Err(())
    }
}
