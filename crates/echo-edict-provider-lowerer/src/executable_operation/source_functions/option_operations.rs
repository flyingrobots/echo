// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Forward judgment for `core.option.*` calls and their presence proofs.
//!
//! `unwrap` is total only on a local that a dominating fact proves present;
//! the compiler's source refinement is not trusted.
use super::{get, list, shape, string, Check, Cost, Fact, Judgment, Ty, Value, DEPTH};

const CAPABILITY: &str = "core.option/v1";
const IS_SOME: &str = "core.option.isSome";

/// Every `core.option.*` callee is gated; unknown operations are refused.
pub(super) fn is_operation(name: &str) -> bool {
    name.starts_with("core.option.")
}

/// Whether an `Option` occurs anywhere in `ty`. Echo defines no Option wire
/// encoding, so an intent's input and output hold none.
pub(super) fn holds_option(ty: &Ty) -> bool {
    match ty {
        Ty::Option(_) => true,
        Ty::List(item, _) => item.as_deref().is_some_and(holds_option),
        Ty::Record(fields) => fields.values().any(holds_option),
        Ty::Nominal(_, inner) => holds_option(inner),
        Ty::Variant { cases, .. } => cases.values().flatten().any(holds_option),
        Ty::Bool | Ty::Word(_) | Ty::Bytes(..) => false,
    }
}

impl Judgment<'_> {
    pub(super) fn option_operation(
        &mut self,
        value: &Value,
        actual: &[Ty],
        declared: &[Ty],
    ) -> Check<Fact> {
        if !list(get(self.core, "requiredCoreCapabilities")?)?
            .iter()
            .any(|v| string(v) == Ok(CAPABILITY))
        {
            return Err(());
        }
        let name = string(get(value, "callee")?)?;
        let [item] = declared else {
            return Err(());
        };
        let option = Ty::Option(Box::new(item.clone()));
        // The operand's item must be exactly the type argument, as in Target.
        let holds_item = |operand: &Ty| operand == &option;
        let (_, item_checks, _, _) = shape(item)?;
        let (_, option_checks, _, height) = shape(&option)?;
        let (result, cost) = match (name, actual) {
            ("core.option.some", [payload]) if super::includes(item, payload) => (
                option,
                Cost {
                    steps: item_checks,
                    bytes: 64,
                },
            ),
            ("core.option.none", []) => (
                option,
                Cost {
                    steps: 1,
                    bytes: 64,
                },
            ),
            ("core.option.isSome", [operand]) if holds_item(operand) => (
                Ty::Bool,
                Cost {
                    steps: option_checks.checked_add(1).ok_or(())?,
                    bytes: 64,
                },
            ),
            ("core.option.default", [operand, fallback])
                if holds_item(operand) && super::includes(item, fallback) =>
            {
                (
                    item.clone(),
                    Cost {
                        steps: option_checks,
                        bytes: 0,
                    },
                )
            }
            ("core.option.unwrap", [operand]) if holds_item(operand) => {
                let [argument] = list(get(value, "args")?)? else {
                    return Err(());
                };
                if !self.present(argument)? {
                    return Err(());
                }
                (
                    item.clone(),
                    Cost {
                        steps: option_checks,
                        bytes: 0,
                    },
                )
            }
            _ => return Err(()),
        };
        Ok(Fact {
            ty: result,
            cost,
            depth: height,
        })
    }

    /// Whether a dominating fact proves `argument`, a local, present.
    fn present(&mut self, argument: &Value) -> Check<bool> {
        if string(get(argument, "kind")?)? != "local" {
            return Ok(false);
        }
        let reference = get(argument, "ref")?;
        let facts: Vec<(&Value, bool)> = self
            .proven
            .iter()
            .map(|p| (*p, true))
            .chain(self.path.iter().copied())
            .collect();
        for (predicate, holds) in facts {
            if establishes(predicate, holds, reference, &mut self.work, 0)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

/// Whether `predicate` evaluating to `holds` proves `isSome(reference)`.
fn establishes(
    predicate: &Value,
    holds: bool,
    reference: &Value,
    work: &mut usize,
    depth: usize,
) -> Check<bool> {
    *work = work.checked_sub(1).ok_or(())?;
    if depth > DEPTH {
        return Err(());
    }
    match string(get(predicate, "kind")?)? {
        "compare" => {
            let left = get(predicate, "left")?;
            let right = get(predicate, "right")?;
            if string(get(left, "kind")?)? != "call" || string(get(left, "callee")?)? != IS_SOME {
                return Ok(false);
            }
            let [tested] = list(get(left, "args")?)? else {
                return Ok(false);
            };
            if string(get(tested, "kind")?)? != "local" || get(tested, "ref")? != reference {
                return Ok(false);
            }
            if string(get(right, "kind")?)? != "const" {
                return Ok(false);
            }
            let constant = get(right, "value")?;
            if string(get(constant, "kind")?)? != "bool" {
                return Ok(false);
            }
            let Value::Bool(value) = get(constant, "value")? else {
                return Ok(false);
            };
            Ok(match string(get(predicate, "op")?)? {
                "==" => holds == *value,
                "!=" => holds != *value,
                _ => false,
            })
        }
        "not" => establishes(get(predicate, "value")?, !holds, reference, work, depth + 1),
        kind @ ("all" | "any") => {
            // A true `all` makes every item true; a false `any` every item false.
            if (kind == "all") != holds {
                return Ok(false);
            }
            for item in list(get(predicate, "values")?)? {
                if establishes(item, holds, reference, work, depth + 1)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        _ => Ok(false),
    }
}
