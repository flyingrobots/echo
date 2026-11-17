// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Verifier-owned checked integer signatures and `min`/`max`, with charges.
//!
//! Each call is rechecked from its one width coordinate and its two operand
//! schemas; a checked call also needs `core.option/v1`.
use super::{get, sequence, string, Charge, Check, Schema, Value};

const CHECKED: [&str; 5] = [
    "core.integer.checkedAdd",
    "core.integer.checkedSub",
    "core.integer.checkedMul",
    "core.integer.checkedDiv",
    "core.integer.checkedRem",
];

pub(super) fn is_operation(name: &str) -> bool {
    CHECKED.contains(&name) || ["core.integer.min", "core.integer.max"].contains(&name)
}

/// The result schema, the call's charge and its result height.
pub(super) fn infer(
    core: &Value,
    name: &str,
    coordinates: &[Value],
    operands: &[Schema],
) -> Check<(Schema, Charge, usize)> {
    let [coordinate] = coordinates else {
        return Err(());
    };
    let word = Schema::Word(match string(coordinate)? {
        "U8" => u8::MAX.into(),
        "U32" => u32::MAX.into(),
        "U64" => u64::MAX,
        _ => return Err(()),
    });
    if operands != [word.clone(), word.clone()] {
        return Err(());
    }
    if !CHECKED.contains(&name) {
        return Ok((
            word,
            Charge {
                ticks: 3,
                storage: 64,
            },
            0,
        ));
    }
    if !sequence(get(core, "requiredCoreCapabilities")?)?
        .iter()
        .any(|v| string(v) == Ok("core.option/v1"))
    {
        return Err(());
    }
    Ok((
        Schema::Optional(Box::new(word)),
        Charge {
            ticks: 3,
            storage: 128,
        },
        1,
    ))
}
