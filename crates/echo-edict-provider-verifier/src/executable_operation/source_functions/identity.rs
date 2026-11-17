// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The verifier's own check of Echo's `ContentAddressV1` identity intrinsics.
//!
//! Admitted only when `effect_program` has bound the pinned `echo.dpo@2`
//! catalog to the exact contracts. A call takes no type arguments, its
//! operands must fit the ceilings, and it is charged the runtime's documented
//! work at the operands' verified maxima.
use std::collections::BTreeMap;

use super::types::{Charge, Schema};
use super::Check;

fn id() -> Schema {
    Schema::Blob { low: 32, high: 32 }
}

fn ceilings(name: &str) -> Option<Vec<Schema>> {
    let edge = Schema::Fields(BTreeMap::from([
        ("edgeType".to_owned(), id()),
        ("to".to_owned(), id()),
    ]));
    Some(match name {
        "echo.dpo@2.contentAddress" => vec![
            id(),
            id(),
            id(),
            Schema::Blob {
                low: 0,
                high: 1_048_576,
            },
            Schema::Sequence {
                element: Some(Box::new(edge)),
                bound: 16,
            },
        ],
        "echo.dpo@2.contentEdgeId" => vec![id(); 4],
        "echo.dpo@2.slotEdgeId" => vec![id(); 3],
        "echo.dpo@2.attachmentValueDigest" => vec![
            id(),
            Schema::Blob {
                low: 0,
                high: 1_048_576,
            },
        ],
        _ => return None,
    })
}

/// Whether `name` is one of the four identity coordinates.
pub(super) fn is_identity(name: &str) -> bool {
    ceilings(name).is_some()
}

/// The result schema and charge of a call to `name` with operand schemas
/// `actual`.
pub(super) fn infer(name: &str, actual: &[Schema]) -> Check<(Schema, Charge)> {
    let expected = ceilings(name).ok_or(())?;
    if expected.len() != actual.len()
        || !expected
            .iter()
            .zip(actual)
            .all(|(want, got)| want.encloses(got.representation()))
    {
        return Err(());
    }
    let charge = match actual {
        [_, _, _, atom, edges] => {
            let Schema::Blob { high: atom, .. } = atom.representation() else {
                return Err(());
            };
            let Schema::Sequence { bound: edges, .. } = edges.representation() else {
                return Err(());
            };
            Charge {
                ticks: 71 + u128::from(*atom) + 67 * u128::from(*edges),
                storage: 96 + 64 * u128::from(*edges),
            }
        }
        [_, _, _, _] => Charge {
            ticks: 70,
            storage: 96,
        },
        [_, _, _] => Charge {
            ticks: 69,
            storage: 96,
        },
        [_, atom] => {
            let Schema::Blob { high: atom, .. } = atom.representation() else {
                return Err(());
            };
            Charge {
                ticks: 68 + u128::from(*atom),
                storage: 96,
            }
        }
        _ => return Err(()),
    };
    Ok((id(), charge))
}
