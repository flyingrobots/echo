// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Forward judgment for checked integer calls and `min`/`max`.
//!
//! One width type argument (`U8`, `U32` or `U64`) and two operands of exactly
//! that width; checked calls return `Option<W>` under `core.option/v1`.
use super::{get, list, string, Check, Cost, Fact, Judgment, Ty, Value};

pub(super) fn is_checked(name: &str) -> bool {
    matches!(
        name,
        "core.integer.checkedAdd"
            | "core.integer.checkedSub"
            | "core.integer.checkedMul"
            | "core.integer.checkedDiv"
            | "core.integer.checkedRem"
    )
}

pub(super) fn is_operation(name: &str) -> bool {
    is_checked(name) || name == "core.integer.min" || name == "core.integer.max"
}

impl Judgment<'_> {
    pub(super) fn integer_operation(&self, value: &Value, actual: &[Ty]) -> Check<Fact> {
        let name = string(get(value, "callee")?)?;
        let [width] = list(get(value, "typeArgs")?)? else {
            return Err(());
        };
        let word = match string(width)? {
            "U8" => Ty::Word(u8::MAX.into()),
            "U32" => Ty::Word(u32::MAX.into()),
            "U64" => Ty::Word(u64::MAX),
            _ => return Err(()),
        };
        let [left, right] = actual else {
            return Err(());
        };
        if *left != word || *right != word {
            return Err(());
        }
        if !is_checked(name) {
            // Two operand validations and one copied word.
            return Ok(Fact {
                ty: word,
                cost: Cost {
                    steps: 3,
                    bytes: 64,
                },
                depth: 0,
            });
        }
        if !list(get(self.core, "requiredCoreCapabilities")?)?
            .iter()
            .any(|v| string(v) == Ok("core.option/v1"))
        {
            return Err(());
        }
        // Two operand validations, one copied word and its presence cell.
        Ok(Fact {
            ty: Ty::Option(Box::new(word)),
            cost: Cost {
                steps: 3,
                bytes: 128,
            },
            depth: 1,
        })
    }
}
