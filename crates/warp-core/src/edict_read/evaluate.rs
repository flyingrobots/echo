// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use super::{
    model::{Instruction, Program, ReadInstruction},
    ReadError, ReadObstruction, ReadResult, ReadView,
};
use crate::edict_pure::{
    evaluate::{expression, predicate, validate, Meter, VALUE_CELL_BYTES},
    values::{bytes, exact_fields, field},
    EvaluationError as Error,
};
use crate::{NodeId, NodeKey, TypeId, WarpId};
use echo_edict_canonical::{encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value};
use std::collections::BTreeMap;

pub(super) fn run(
    program: &Program,
    input: Value,
    view: &ReadView<'_>,
) -> Result<ReadResult, ReadError> {
    let mut meter = Meter::new(program.limits.evaluation);
    validate(&input, &program.input_type, &mut meter, 0).map_err(|error| {
        if error == Error::InvalidArtifact {
            Error::InvalidInput
        } else {
            error
        }
    })?;
    meter.charge(&input, 0)?;
    let mut locals = BTreeMap::from([(program.input_id.clone(), input)]);
    let helpers = BTreeMap::new();
    let application_basis = expression(&program.basis, &locals, &helpers, &mut meter, 0)?;
    let application_basis = bytes(&application_basis)?
        .try_into()
        .map_err(|_| Error::InvalidInput)?;
    for (coordinate, constraint) in &program.constraints {
        if !predicate(constraint, &locals, &helpers, &mut meter, 0)? {
            return Err(Error::InputConstraintFailed(coordinate.clone()).into());
        }
    }
    let mut reads = 0_u64;
    let mut read_bytes = 0_u64;
    for instruction in &program.instructions {
        meter.step(0)?;
        match instruction {
            Instruction::Read(read) => {
                let address = expression(&read.input, &locals, &helpers, &mut meter, 0)?;
                let (node, expected_type) = address_value(&address)?;
                if !view.contains(node) {
                    return Err(ReadError::OutsideAperture(node));
                }
                reads = reads.checked_add(1).ok_or(ReadError::ReadCountExceeded)?;
                if reads > program.limits.max_reads {
                    return Err(ReadError::ReadCountExceeded);
                }
                let bytes = view
                    .atom(node, expected_type, read.max_bytes)
                    .map_err(|cause| obstructed(read, cause))?;
                let count = bytes.len() as u64;
                read_bytes = read_bytes
                    .checked_add(count)
                    .ok_or(ReadError::ReadBytesExceeded)?;
                if read_bytes > program.limits.max_read_bytes {
                    return Err(ReadError::ReadBytesExceeded);
                }
                // Charge deterministic I/O work and storage BEFORE copying atom bytes.
                meter.charge_steps(count)?;
                meter.allocate(
                    VALUE_CELL_BYTES
                        .checked_add(count)
                        .ok_or(Error::AllocationBudgetExceeded)?,
                )?;
                let value = Value::Bytes(bytes.to_vec());
                validate(&value, &read.ty, &mut meter, 0)?;
                locals.insert(read.binding.clone(), value);
            }
            Instruction::Let { binding, ty, value } => {
                let value = expression(value, &locals, &helpers, &mut meter, 0)?;
                validate(&value, ty, &mut meter, 0)?;
                locals.insert(binding.clone(), value);
            }
            Instruction::Require {
                predicate: condition,
                obstruction,
            } => {
                if !predicate(condition, &locals, &helpers, &mut meter, 0)? {
                    return Err(ReadError::Obstructed {
                        coordinate: obstruction.clone(),
                        cause: ReadObstruction::Guard,
                    });
                }
            }
        }
    }
    let value = expression(&program.result, &locals, &helpers, &mut meter, 0)?;
    validate(&value, &program.output_type, &mut meter, 0)?;
    meter.charge(&value, 0)?;
    let output = encode(&value).map_err(|_| Error::InvalidArtifact)?;
    if output.len() as u64 > program.limits.evaluation.max_output_bytes {
        return Err(Error::OutputBudgetExceeded.into());
    }
    let (steps, allocated_bytes) = meter.usage();
    Ok(ReadResult {
        output,
        basis: view.basis(),
        application_basis,
        steps,
        allocated_bytes,
        reads,
        read_bytes,
    })
}

fn address_value(value: &Value) -> Result<(NodeKey, TypeId), Error> {
    exact_fields(value, &["nodeId", "typeId", "warpId"])?;
    let id = |key| -> Result<[u8; 32], Error> {
        bytes(field(value, key)?)?
            .try_into()
            .map_err(|_| Error::InvalidArtifact)
    };
    Ok((
        NodeKey {
            warp_id: WarpId(id("warpId")?),
            local_id: NodeId(id("nodeId")?),
        },
        TypeId(id("typeId")?),
    ))
}

fn obstructed(read: &ReadInstruction, cause: ReadObstruction) -> ReadError {
    let coordinate = match cause {
        ReadObstruction::Missing => &read.missing,
        ReadObstruction::AtomRequired => &read.not_atom,
        ReadObstruction::TypeMismatch => &read.type_mismatch,
        ReadObstruction::AtomTooLarge => &read.too_large,
        ReadObstruction::Guard => return Error::InvalidArtifact.into(),
    };
    ReadError::Obstructed {
        coordinate: coordinate.clone(),
        cause,
    }
}
