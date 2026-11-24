// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! This module owns the verifier's charge check under the shared-read Core
//! cost schedule, `edict.core-cost/v2`, which a module selects by requiring
//! `core.cost.shared-reads/v1`.
//!
//! The rules are Amblicit's (`target_ir/fold_work/shared_reads.rs` at
//! `26a201e7`), re-derived here independently of the lowerer:
//!
//! - every node costs one tick; a local, field, element or fold-state read,
//!   `if`, `match`, `fold` and a call cost nothing more;
//! - a constant costs its value cell(s) plus one tick and its storage;
//! - a record, list or variant costs its shell: cells plus one tick, and its
//!   bytes; shared children are not charged again;
//! - a helper body costs one tick per parameter plus two;
//! - a fold costs every iteration of its bound, each the body plus one;
//! - `listAt` costs nothing more, `listLen` one fresh cell, and `listAppend`
//!   or `listConcat` a shell of the result's proven length, else its
//!   capacity (amblicit#336; see `bounds`);
//! - a prelude call that builds a value (integer operations, `Option`
//!   readers) is also charged that value in full (amblicit#340);
//! - unsigned and checked arithmetic and min/max keep their copying charge,
//!   one tick per operand cell;
//! - an `Option` consumer (`isSome`, `unwrap`, `default`) keeps its copying
//!   charge: one tick per operand cell plus its operand and result bytes;
//! - a comparison scans both operands at their full static size;
//! - a payload binder costs one tick, and dispatch one per arm plus one;
//! - the intent's input and output cost two ticks per cell and their bytes.
//!
//! - an identity intrinsic costs its catalog rule over its operands'
//!   sequence maxima, plus its operands copied in and its result copied out
//!   in full, each as a built value;
//!
//! Byte intrinsics, `Option` constructors and imported helpers have no
//! shared-read rule in Echo yet, so the module is refused. The ordinary audit runs first and alone establishes
//! typing, totality and depth; this check reads the schemas it recorded.
use super::{get, members, sequence, string, unsigned, Charge, Check, Checker, Schema, Value};
use std::collections::BTreeMap;

/// Core capability selecting the shared-read schedule.
pub(super) const CAPABILITY: &str = "core.cost.shared-reads/v1";

/// Proven upper bounds on list lengths inside one value, keyed by field path
/// (the empty path is the value itself). An absent path is bounded only by
/// its declared maximum.
type Bounds = BTreeMap<Vec<String>, u64>;

/// List-length state of the shared-read check (amblicit#336): the bounds of
/// the current frame's locals, and each helper's charge and result bounds by
/// its argument bounds.
#[derive(Default)]
pub(super) struct Lengths {
    frame: BTreeMap<String, Bounds>,
    helpers: BTreeMap<(String, Vec<Bounds>), (Charge, Bounds)>,
    active: Vec<String>,
}

/// A path stays bounded across two branches only if both bound it.
fn meet(left: &Bounds, right: &Bounds) -> Bounds {
    let mut shared = Bounds::new();
    for (path, bound) in left {
        if let Some(other) = right.get(path) {
            shared.insert(path.clone(), (*bound).max(*other));
        }
    }
    shared
}

/// Prelude calls that build a new value and are charged it in full
/// (amblicit#340): integer operations and the `Option` readers.
fn builds_value(callee: &str) -> bool {
    callee.starts_with("core.integer.")
        || matches!(
            callee,
            "core.option.isSome" | "core.option.unwrap" | "core.option.default"
        )
}

/// Whether the module requires the shared-read schedule.
pub(super) fn required(module: &Value) -> Check<bool> {
    let Some(capabilities) = super::super::map_field(module, "requiredCoreCapabilities") else {
        return Ok(false);
    };
    Ok(sequence(capabilities)?
        .iter()
        .any(|capability| string(capability) == Ok(CAPABILITY)))
}

/// A value's footprint in 64-byte cells and storage bytes.
#[derive(Clone, Copy)]
struct Footprint {
    cells: u128,
    bytes: u128,
}

impl Footprint {
    const CELL: Self = Self {
        cells: 1,
        bytes: 64,
    };

    fn with(self, other: Self) -> Check<Self> {
        Ok(Self {
            cells: self.cells.checked_add(other.cells).ok_or(())?,
            bytes: self.bytes.checked_add(other.bytes).ok_or(())?,
        })
    }

    fn larger(self, other: Self) -> Self {
        Self {
            cells: self.cells.max(other.cells),
            bytes: self.bytes.max(other.bytes),
        }
    }

    /// The slot a record field occupies around a child of this footprint.
    fn keyed(self, key: &str) -> Check<Self> {
        self.with(Self {
            cells: 1,
            bytes: 64 + u128::try_from(key.len()).map_err(|_| ())?,
        })
    }

    /// Building a value of this footprint: a tick per cell plus one.
    fn built(self) -> Check<Charge> {
        Ok(Charge {
            ticks: self.cells.checked_add(1).ok_or(())?,
            storage: self.bytes,
        })
    }

    fn shell(items: u128) -> Check<Self> {
        Self::CELL.with(Self {
            cells: items,
            bytes: items.checked_mul(64).ok_or(())?,
        })
    }

    fn of(schema: &Schema) -> Check<Self> {
        let copy = schema.measure()?.copy;
        Ok(Self {
            cells: copy.ticks,
            bytes: copy.storage,
        })
    }
}

fn ticks(count: u128) -> Charge {
    Charge {
        ticks: count,
        storage: 0,
    }
}

fn count(items: usize) -> Check<u128> {
    u128::try_from(items).map_err(|_| ())
}

fn total(parts: impl IntoIterator<Item = Check<Charge>>) -> Check<Charge> {
    let mut sum = Charge::default();
    for part in parts {
        sum.append(part?)?;
    }
    Ok(sum)
}

impl<'a> Checker<'a, '_> {
    /// The intent's shared-read charge. The ordinary audit of the same intent
    /// has already run and recorded every schema this reads.
    pub(super) fn shared_charge(&mut self, intent: &'a Value) -> Check<Charge> {
        self.types.shared_helpers.frame.clear();
        let input = Footprint::of(&self.types.resolve(string(get(intent, "input")?)?, 0)?)?;
        let output = Footprint::of(&self.types.resolve(string(get(intent, "output")?)?, 0)?)?;
        let mut charge = Charge {
            ticks: input
                .cells
                .checked_add(output.cells)
                .and_then(|cells| cells.checked_mul(2))
                .ok_or(())?,
            storage: input.bytes.checked_add(output.bytes).ok_or(())?,
        };
        for constraint in sequence(get(intent, "inputConstraints")?)? {
            charge.append(self.shared_condition(get(constraint, "predicate")?)?)?;
        }
        if let Some(basis) = super::super::map_field(intent, "basis") {
            charge.append(self.shared_value(basis)?)?;
        }
        let body = get(intent, "body")?;
        for node in sequence(get(body, "nodes")?)? {
            if string(get(node, "kind")?)? != "let" {
                return Err(());
            }
            let value = get(node, "value")?;
            charge.append(self.shared_value(value)?)?;
            charge.append(ticks(1))?;
            let bounds = self.bounds(value)?;
            let id = string(get(get(node, "binding")?, "id")?)?;
            self.types
                .shared_helpers
                .frame
                .insert(id.to_owned(), bounds);
        }
        charge.append(self.shared_value(get(body, "result")?)?)?;
        Ok(charge)
    }

    /// One helper call's charge and result bounds under its arguments'
    /// bounds; arguments and the result pass by reference.
    fn shared_helper(&mut self, name: &str, arguments: Vec<Bounds>) -> Check<(Charge, Bounds)> {
        let key = (name.to_owned(), arguments);
        if let Some(known) = self.types.shared_helpers.helpers.get(&key) {
            return Ok(known.clone());
        }
        let definition = self.definitions.get(name).ok_or(())?;
        let (source, params, body) = (definition.source, definition.params, definition.body);
        if !source
            || params.len() != key.1.len()
            || self
                .types
                .shared_helpers
                .active
                .iter()
                .any(|open| open == name)
        {
            return Err(());
        }
        let mut frame = BTreeMap::new();
        for (parameter, bounds) in params.iter().zip(&key.1) {
            frame.insert(string(get(parameter, "id")?)?.to_owned(), bounds.clone());
        }
        let outer = std::mem::replace(&mut self.types.shared_helpers.frame, frame);
        self.types.shared_helpers.active.push(name.to_owned());
        let result = (|| {
            let mut charge = ticks(count(params.len())?.checked_add(2).ok_or(())?);
            for binding in sequence(get(body, "bindings")?)? {
                let value = get(binding, "value")?;
                charge.append(self.shared_value(value)?)?;
                let bounds = self.bounds(value)?;
                let id = string(get(get(binding, "binding")?, "id")?)?;
                self.types
                    .shared_helpers
                    .frame
                    .insert(id.to_owned(), bounds);
            }
            let value = get(body, "result")?;
            charge.append(self.shared_value(value)?)?;
            Ok((charge, self.bounds(value)?))
        })();
        self.types.shared_helpers.active.pop();
        self.types.shared_helpers.frame = outer;
        let result = result?;
        self.types
            .shared_helpers
            .helpers
            .insert(key, result.clone());
        Ok(result)
    }

    /// A fold charges every round, each one tick plus its step call under the
    /// bounds the previous round left; a round that leaves the bounds
    /// unchanged fixes the charge of every round after it.
    fn rounds(&mut self, mut state: Bounds, step: &str, bound: u64) -> Check<(Charge, Bounds)> {
        let mut charge = Charge::default();
        for round in 0..bound {
            let (mut each, next) = self.shared_helper(step, vec![state.clone()])?;
            each.append(ticks(1))?;
            if next == state {
                let left = u128::from(bound - round);
                charge.append(Charge {
                    ticks: each.ticks.checked_mul(left).ok_or(())?,
                    storage: each.storage.checked_mul(left).ok_or(())?,
                })?;
                return Ok((charge, state));
            }
            charge.append(each)?;
            state = next;
        }
        Ok((charge, state))
    }

    /// Proven list-length bounds inside the value `node` evaluates to.
    fn bounds(&mut self, node: &'a Value) -> Check<Bounds> {
        self.types.step(0)?;
        match string(get(node, "kind")?)? {
            "local" => {
                let id = string(get(get(node, "ref")?, "id")?)?;
                Ok(self
                    .types
                    .shared_helpers
                    .frame
                    .get(id)
                    .cloned()
                    .unwrap_or_default())
            }
            "field" => {
                let field = string(get(node, "field")?)?;
                let mut inner = Bounds::new();
                for (path, bound) in self.bounds(get(node, "base")?)? {
                    if path.first().map(String::as_str) == Some(field) {
                        inner.insert(path[1..].to_vec(), bound);
                    }
                }
                Ok(inner)
            }
            "record" => {
                let mut all = Bounds::new();
                for (key, field) in members(get(node, "fields")?)? {
                    for (path, bound) in self.bounds(field)? {
                        let mut full = vec![string(key)?.to_owned()];
                        full.extend(path);
                        all.insert(full, bound);
                    }
                }
                Ok(all)
            }
            "list" => Ok(Bounds::from([(
                Vec::new(),
                u64::try_from(sequence(get(node, "values")?)?.len()).map_err(|_| ())?,
            )])),
            "if" => {
                let then = self.bounds(get(node, "then")?)?;
                let otherwise = self.bounds(get(node, "else")?)?;
                Ok(meet(&then, &otherwise))
            }
            "match" => {
                let mut met: Option<Bounds> = None;
                for arm in sequence(get(node, "arms")?)? {
                    let arm = self.bounds(get(arm, "body")?)?;
                    met = Some(met.map_or_else(|| arm.clone(), |earlier| meet(&earlier, &arm)));
                }
                Ok(met.unwrap_or_default())
            }
            "fold" => {
                let start = self.bounds(get(node, "initial")?)?;
                let step = string(get(node, "step")?)?;
                Ok(self.rounds(start, step, unsigned(get(node, "bound")?)?)?.1)
            }
            "call" => self.call_bounds(node),
            _ => Ok(Bounds::new()),
        }
    }

    fn call_bounds(&mut self, node: &'a Value) -> Check<Bounds> {
        let callee = string(get(node, "callee")?)?;
        let arguments = sequence(get(node, "args")?)?;
        let growth = match (callee, arguments) {
            ("core.list.append", [_, _]) => Some(1),
            ("core.list.concat", [_, right]) => self.length_bound(right)?,
            _ if self
                .definitions
                .get(callee)
                .is_some_and(|definition| definition.source) =>
            {
                let mut each = Vec::new();
                for argument in arguments {
                    each.push(self.bounds(argument)?);
                }
                return Ok(self.shared_helper(callee, each)?.1);
            }
            _ => return Ok(Bounds::new()),
        };
        let Schema::Sequence { bound, .. } = self.audited(node)?.representation() else {
            return Err(());
        };
        let bound = *bound;
        Ok(match (self.length_bound(&arguments[0])?, growth) {
            (Some(base), Some(growth)) => {
                Bounds::from([(Vec::new(), base.saturating_add(growth).min(bound))])
            }
            _ => Bounds::new(),
        })
    }

    /// The proven length of a list-valued node, when one exists.
    fn length_bound(&mut self, node: &'a Value) -> Check<Option<u64>> {
        Ok(self.bounds(node)?.get(&Vec::new()).copied())
    }

    /// An effect body's expression charges its shared-read work under the
    /// schedule and `copying` otherwise; effect steps keep their copying
    /// charge under either.
    pub(super) fn effect_value(&mut self, node: &'a Value, copying: Charge) -> Check<Charge> {
        if self.types.shared {
            self.shared_value(node)
        } else {
            Ok(copying)
        }
    }

    /// As `effect_value`, for a predicate.
    pub(super) fn effect_condition(
        &mut self,
        predicate: &'a Value,
        copying: Charge,
    ) -> Check<Charge> {
        if self.types.shared {
            self.shared_condition(predicate)
        } else {
            Ok(copying)
        }
    }

    /// Under the schedule, the list-length bounds of a value an effect body
    /// binds, kept for later list shells.
    pub(super) fn effect_bound(&mut self, local: &'a Value, node: &'a Value) -> Check<()> {
        if self.types.shared {
            let bounds = self.bounds(node)?;
            let id = string(get(local, "id")?)?;
            self.types
                .shared_helpers
                .frame
                .insert(id.to_owned(), bounds);
        }
        Ok(())
    }

    /// Under the schedule, building an obstruction payload record: one tick,
    /// its shell as any record's, and each field's shared-read work. `None`
    /// keeps the copying charge.
    pub(super) fn effect_payload(&mut self, fields: &'a [(Value, Value)]) -> Check<Option<Charge>> {
        if !self.types.shared {
            return Ok(None);
        }
        let mut shell = Footprint::CELL;
        let mut charge = ticks(1);
        for (name, value) in fields {
            shell = shell.with(Footprint::CELL.keyed(string(name)?)?)?;
            charge.append(self.shared_value(value)?)?;
        }
        charge.append(shell.built()?)?;
        Ok(Some(charge))
    }

    /// Starts an effect intent with no proven bounds of its own.
    pub(super) fn effect_start(&mut self) {
        self.types.shared_helpers.frame.clear();
    }

    /// The schema the ordinary audit recorded for this node.
    fn audited(&self, node: &Value) -> Check<&Schema> {
        self.types
            .audited
            .get(&std::ptr::from_ref(node).addr())
            .ok_or(())
    }

    fn shared_value(&mut self, node: &'a Value) -> Check<Charge> {
        self.types.step(0)?;
        let mut charge = self.own(node)?;
        let inner = match string(get(node, "kind")?)? {
            "local" | "const" => Charge::default(),
            "field" => self.shared_value(get(node, "base")?)?,
            "variant" => match super::super::map_field(node, "payload") {
                Some(payload) => self.shared_value(payload)?,
                None => Charge::default(),
            },
            "record" => {
                let fields = members(get(node, "fields")?)?;
                total(fields.iter().map(|(_, field)| self.shared_value(field)))?
            }
            "list" => {
                let items = sequence(get(node, "values")?)?;
                total(items.iter().map(|item| self.shared_value(item)))?
            }
            "fold" => {
                let initial = get(node, "initial")?;
                let mut charge = self.shared_value(initial)?;
                let start = self.bounds(initial)?;
                let step = string(get(node, "step")?)?;
                charge.append(self.rounds(start, step, unsigned(get(node, "bound")?)?)?.0)?;
                charge
            }
            "call" => {
                let arguments = sequence(get(node, "args")?)?;
                let mut charge =
                    total(arguments.iter().map(|argument| self.shared_value(argument)))?;
                charge.append(self.shared_call(node, arguments)?)?;
                charge
            }
            "if" => {
                let mut charge = self.shared_condition(get(node, "predicate")?)?;
                let then = self.shared_value(get(node, "then")?)?;
                let otherwise = self.shared_value(get(node, "else")?)?;
                charge.append(Charge::choice(then, otherwise))?;
                charge
            }
            "match" => {
                let mut charge = self.shared_value(get(node, "scrutinee")?)?;
                let arms = sequence(get(node, "arms")?)?;
                let mut worst = Charge::default();
                for arm in arms {
                    let mut arm_charge = self.shared_value(get(arm, "body")?)?;
                    if super::super::map_field(arm, "binder").is_some() {
                        arm_charge.append(ticks(1))?;
                    }
                    worst = Charge::choice(worst, arm_charge);
                }
                charge.append(worst)?;
                charge.append(ticks(count(arms.len())?.checked_add(1).ok_or(())?))?;
                charge
            }
            _ => return Err(()),
        };
        charge.append(inner)?;
        Ok(charge)
    }

    /// The node's own work, excluding its children.
    fn own(&self, node: &Value) -> Check<Charge> {
        match string(get(node, "kind")?)? {
            "const" => self.footprint(node)?.built(),
            "record" => {
                let mut shell = Footprint::CELL;
                for (key, _) in members(get(node, "fields")?)? {
                    shell = shell.with(Footprint::CELL.keyed(string(key)?)?)?;
                }
                shell.built()
            }
            "list" => Footprint::shell(count(sequence(get(node, "values")?)?.len())?)?.built(),
            "variant" => {
                let labels = count(string(get(node, "type")?)?.len())?
                    .checked_add(count(string(get(node, "case")?)?.len())?)
                    .ok_or(())?;
                Footprint::CELL
                    .with(Footprint {
                        cells: 6,
                        bytes: 399_u128.checked_add(labels).ok_or(())?,
                    })?
                    .built()
            }
            "call" if builds_value(string(get(node, "callee")?)?) => self.footprint(node)?.built(),
            _ => Ok(ticks(1)),
        }
    }

    fn shared_call(&mut self, node: &'a Value, arguments: &'a [Value]) -> Check<Charge> {
        let callee = string(get(node, "callee")?)?;
        match callee {
            "core.list.at" => Ok(Charge::default()),
            "core.list.length" => Footprint::CELL.built(),
            "core.list.append" | "core.list.concat" => {
                let Schema::Sequence { bound, .. } = self.audited(node)?.representation() else {
                    return Err(());
                };
                let bound = *bound;
                // Sized by the result's proven length, when one exists.
                let items = self
                    .length_bound(node)?
                    .map_or(bound, |proven| proven.min(bound));
                Footprint::shell(u128::from(items))?.built()
            }
            "core.integer.add"
            | "core.integer.subtract"
            | "core.integer.checkedAdd"
            | "core.integer.checkedSub"
            | "core.integer.checkedMul"
            | "core.integer.checkedDiv"
            | "core.integer.checkedRem"
            | "core.integer.min"
            | "core.integer.max" => {
                let mut cells = 0_u128;
                for argument in arguments {
                    cells = cells
                        .checked_add(self.footprint(argument)?.cells)
                        .ok_or(())?;
                }
                Ok(ticks(cells))
            }
            // An Option consumer keeps its copying charge: one tick per
            // operand cell plus the bytes of its operands and result.
            "core.option.isSome" | "core.option.unwrap" | "core.option.default" => {
                let mut charge = self.footprint(node)?.bytes;
                for argument in arguments {
                    let footprint = self.footprint(argument)?;
                    charge = charge
                        .checked_add(footprint.cells)
                        .and_then(|charge| charge.checked_add(footprint.bytes))
                        .ok_or(())?;
                }
                Ok(ticks(charge))
            }
            _ if callee.starts_with("core.") => Err(()),
            // A target intrinsic runs outside the shared heap: its catalog
            // rule over its operands' sequence maxima, every operand copied
            // in and its result copied out in full.
            _ if self.identity && super::identity::is_identity(callee) => {
                let (base, per_unit) = super::identity::catalog_rule(callee).ok_or(())?;
                let mut charge = self.footprint(node)?.built()?;
                let mut rule = base;
                for (index, argument) in arguments.iter().enumerate() {
                    charge.append(self.footprint(argument)?.built()?)?;
                    let coefficient = *per_unit.get(index).ok_or(())?;
                    if coefficient > 0 {
                        rule = self
                            .longest(argument)?
                            .checked_mul(coefficient)
                            .and_then(|work| work.checked_add(rule))
                            .ok_or(())?;
                    }
                }
                charge.append(ticks(rule))?;
                Ok(charge)
            }
            _ => {
                let mut each = Vec::new();
                for argument in arguments {
                    each.push(self.bounds(argument)?);
                }
                Ok(self.shared_helper(callee, each)?.0)
            }
        }
    }

    /// The longest byte string or list `node` may evaluate to, as a target
    /// intrinsic's cost rule measures an operand: a list literal by its
    /// items, a branch by its longer arm, a failure as empty, and anything
    /// else by its declared bound.
    fn longest(&self, node: &Value) -> Check<u128> {
        Ok(match string(get(node, "kind")?)? {
            "list" => count(sequence(get(node, "values")?)?.len())?,
            "fail" => 0,
            "if" => self
                .longest(get(node, "then")?)?
                .max(self.longest(get(node, "else")?)?),
            "match" => {
                let mut longest = 0;
                for arm in sequence(get(node, "arms")?)? {
                    longest = self.longest(get(arm, "body")?)?.max(longest);
                }
                longest
            }
            _ => match self.audited(node)?.representation() {
                Schema::Blob { high, .. } => u128::from(*high),
                Schema::Sequence { bound, .. } => u128::from(*bound),
                _ => return Err(()),
            },
        })
    }

    /// The largest value `node` may evaluate to.
    fn footprint(&self, node: &Value) -> Check<Footprint> {
        match string(get(node, "kind")?)? {
            "record" => {
                let mut footprint = Footprint::CELL;
                for (key, field) in members(get(node, "fields")?)? {
                    footprint = footprint.with(self.footprint(field)?.keyed(string(key)?)?)?;
                }
                Ok(footprint)
            }
            "list" => {
                let mut footprint = Footprint::CELL;
                for item in sequence(get(node, "values")?)? {
                    footprint = footprint.with(self.footprint(item)?)?;
                }
                Ok(footprint)
            }
            "if" => Ok(self
                .footprint(get(node, "then")?)?
                .larger(self.footprint(get(node, "else")?)?)),
            "match" => {
                let mut footprint = Footprint::CELL;
                for arm in sequence(get(node, "arms")?)? {
                    footprint = footprint.larger(self.footprint(get(arm, "body")?)?);
                }
                Ok(footprint)
            }
            _ => Footprint::of(self.audited(node)?),
        }
    }

    fn shared_condition(&mut self, predicate: &'a Value) -> Check<Charge> {
        self.types.step(0)?;
        let mut charge = match string(get(predicate, "kind")?)? {
            "not" => self.shared_condition(get(predicate, "value")?)?,
            "all" | "any" => {
                let items = sequence(get(predicate, "values")?)?;
                total(items.iter().map(|item| self.shared_condition(item)))?
            }
            "compare" => {
                let (left, right) = (get(predicate, "left")?, get(predicate, "right")?);
                // A comparison scans both operands in full.
                let scan = self
                    .footprint(left)?
                    .bytes
                    .max(self.footprint(right)?.bytes);
                let mut charge = self.shared_value(left)?;
                charge.append(self.shared_value(right)?)?;
                charge.append(ticks(scan))?;
                charge
            }
            _ => return Err(()),
        };
        charge.append(ticks(1))?;
        Ok(charge)
    }
}
