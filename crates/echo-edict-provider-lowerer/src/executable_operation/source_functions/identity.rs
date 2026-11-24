// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The lowerer's judgment of Echo's `ContentAddressV1` identity intrinsics.
//!
//! Calls are admitted only under `echo.dpo@2`, when the pinned catalog
//! carries exactly the contracts below (see `effect_program::identity`). Each
//! call takes no type arguments, its operands must fit the contract's ceiling
//! types, and it is charged its documented work at the operands' verified
//! maxima: `contentAddress` 71 steps plus one per atom byte and 67 per edge,
//! and 96 bytes plus 64 per edge; `contentEdgeId` 70 steps and
//! `slotEdgeId` 69 steps, 96 bytes each; `attachmentValueDigest` 68 steps
//! plus one per atom byte, and 96 bytes.
use std::collections::BTreeMap;

use super::{includes, Check, Cost, Ty};

const ID: u64 = 32;
const MAX_ATOM_BYTES: u64 = 1_048_576;
const MAX_OUTGOING_EDGES: u64 = 16;

fn id() -> Ty {
    Ty::Bytes(ID, ID)
}

fn edge() -> Ty {
    Ty::Record(BTreeMap::from([
        ("edgeType".to_owned(), id()),
        ("to".to_owned(), id()),
    ]))
}

/// The ceiling type of each operand of the identity intrinsic `name`.
pub(super) fn parameters(name: &str) -> Option<Vec<Ty>> {
    Some(match name {
        "echo.dpo@2.contentAddress" => vec![
            id(),
            id(),
            id(),
            Ty::Bytes(0, MAX_ATOM_BYTES),
            Ty::List(Some(Box::new(edge())), MAX_OUTGOING_EDGES),
        ],
        "echo.dpo@2.contentEdgeId" => vec![id(); 4],
        "echo.dpo@2.slotEdgeId" => vec![id(); 3],
        "echo.dpo@2.attachmentValueDigest" => vec![id(), Ty::Bytes(0, MAX_ATOM_BYTES)],
        _ => return None,
    })
}

/// The result type and charged work of a call to `name` whose operands have
/// the static types `actual`.
pub(super) fn judge(name: &str, actual: &[Ty]) -> Check<(Ty, Cost)> {
    let ceilings = parameters(name).ok_or(())?;
    if ceilings.len() != actual.len()
        || !ceilings
            .iter()
            .zip(actual)
            .all(|(ceiling, actual)| includes(ceiling, actual.representation()))
    {
        return Err(());
    }
    let cost = match (name, actual) {
        ("echo.dpo@2.contentAddress", [_, _, _, atom, edges]) => {
            let Ty::Bytes(_, atom_max) = atom.representation() else {
                return Err(());
            };
            let Ty::List(_, edge_max) = edges.representation() else {
                return Err(());
            };
            Cost {
                steps: 71u64
                    .checked_add(*atom_max)
                    .and_then(|steps| steps.checked_add(edge_max.checked_mul(67)?))
                    .ok_or(())?,
                bytes: edge_max
                    .checked_mul(64)
                    .and_then(|bytes| bytes.checked_add(96))
                    .ok_or(())?,
            }
        }
        ("echo.dpo@2.contentEdgeId", _) => Cost {
            steps: 70,
            bytes: 96,
        },
        ("echo.dpo@2.slotEdgeId", _) => Cost {
            steps: 69,
            bytes: 96,
        },
        ("echo.dpo@2.attachmentValueDigest", [_, atom]) => {
            let Ty::Bytes(_, atom_max) = atom.representation() else {
                return Err(());
            };
            Cost {
                steps: 68u64.checked_add(*atom_max).ok_or(())?,
                bytes: 96,
            }
        }
        _ => return Err(()),
    };
    Ok((id(), cost))
}
