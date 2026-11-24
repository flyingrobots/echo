// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! This module owns the verifier's charge check under the shared-read Core
//! cost schedule, `edict.core-cost/v2`, which a module selects by requiring
//! `core.cost.shared-reads/v1`.
//!
//! The rules are Amblicit's (`target_ir/fold_work/shared_reads.rs` at
//! `da1574f1`), re-derived here independently of the lowerer:
//!
//! - every node costs one tick; a local, field, element or fold-state read,
//!   `if`, `match`, `fold` and a call cost nothing more;
//! - a constant costs its value cell(s) plus one tick and its storage;
//! - a record, list or variant costs its shell: cells plus one tick, and its
//!   bytes; shared children are not charged again;
//! - a helper body costs one tick per parameter plus two;
//! - a fold costs every iteration of its bound, each the body plus one;
//! - `listAt` costs nothing more, `listLen` one fresh cell, and `listAppend`
//!   or `listConcat` a shell of the result's capacity;
//! - unsigned arithmetic keeps its copying charge, one tick per operand cell;
//! - a comparison scans both operands at their full static size;
//! - a payload binder costs one tick, and dispatch one per arm plus one;
//! - the intent's input and output cost two ticks per cell and their bytes.
//!
//! Byte intrinsics and imported helpers have no shared-read rule in Echo, so
//! the module is refused. The ordinary audit runs first and alone establishes
//! typing, totality and depth; this check reads the schemas it recorded.
use super::{get, members, sequence, string, unsigned, Charge, Check, Checker, Schema, Value};

/// Core capability selecting the shared-read schedule.
pub(super) const CAPABILITY: &str = "core.cost.shared-reads/v1";

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
            charge.append(self.shared_value(get(node, "value")?)?)?;
            charge.append(ticks(1))?;
        }
        charge.append(self.shared_value(get(body, "result")?)?)?;
        Ok(charge)
    }

    fn shared_helper(&mut self, name: &str) -> Check<Charge> {
        if let Some(charge) = self.types.shared_helpers.get(name) {
            return Ok(*charge);
        }
        let definition = self.definitions.get(name).ok_or(())?;
        if !definition.source {
            return Err(());
        }
        let (params, body) = (definition.params, definition.body);
        let mut charge = ticks(count(params.len())?.checked_add(2).ok_or(())?);
        for binding in sequence(get(body, "bindings")?)? {
            charge.append(self.shared_value(get(binding, "value")?)?)?;
        }
        charge.append(self.shared_value(get(body, "result")?)?)?;
        self.types.shared_helpers.insert(name.to_owned(), charge);
        Ok(charge)
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
                let mut iteration = self.shared_helper(string(get(node, "step")?)?)?;
                iteration.append(ticks(1))?;
                let bound = u128::from(unsigned(get(node, "bound")?)?);
                let mut charge = self.shared_value(get(node, "initial")?)?;
                charge.append(Charge {
                    ticks: iteration.ticks.checked_mul(bound).ok_or(())?,
                    storage: iteration.storage.checked_mul(bound).ok_or(())?,
                })?;
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
                Footprint::shell(u128::from(*bound))?.built()
            }
            "core.integer.add" | "core.integer.subtract" => {
                let mut cells = 0_u128;
                for argument in arguments {
                    cells = cells
                        .checked_add(self.footprint(argument)?.cells)
                        .ok_or(())?;
                }
                Ok(ticks(cells))
            }
            _ if callee.starts_with("core.") => Err(()),
            _ => self.shared_helper(callee),
        }
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
