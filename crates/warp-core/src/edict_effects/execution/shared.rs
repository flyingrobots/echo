// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Effect-program execution under the shared-read Core cost schedule,
//! `edict.core-cost/v2`, selected by `core.cost.shared-reads/v1` (echo#1016).
//!
//! Locals hold shared values, so reading one between instructions never
//! copies it, and every expression and predicate is metered by the shared
//! evaluator. The intent's input and output cross the boundary as a pure
//! shared-read program's do: two steps per cell and the value's bytes. Each
//! effect step keeps its copying-schedule charge exactly: every read, graph
//! read, emission and obstruction is validated and charged as the copying
//! executor charges it.
//! A value crosses into an effect step as its canonical form, and a value an
//! effect step returns is shared from then on.
use super::{
    address_value, forget, EffectFrameError, EffectGraphSchema, Evaluation, GraphRead, Instruction,
    Program, RecordedNodeAccess,
};
use crate::edict_effects::mutations::{EffectMutation, MutationKind};
use crate::edict_pure::{
    evaluate::{validate, Meter, VALUE_CELL_BYTES},
    model::{Expr, Predicate, RuntimeType},
    shared_reads::{
        admit_expression, admit_helpers, admit_predicate, from_canonical, to_canonical, Evaluator,
        Locals, Shared,
    },
    EvaluationError as Error,
};
use crate::edict_read::{
    closure_read, edge_read, GraphReadSession, GraphReadUsage, NodeAtomAccess, ReadError,
    ReadLimits, ReadObstruction,
};
use crate::{SlotId, SlotReading};
use echo_edict_canonical::{encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value};
use std::collections::BTreeMap;

/// Refuses, before any work, a program using an operation without a
/// shared-read rule, in any helper, constraint, instruction or result.
pub(super) fn admit(program: &Program) -> Result<(), Error> {
    admit_helpers(&program.helpers)?;
    if let Some(basis) = &program.basis {
        admit_expression(basis)?;
    }
    for (_, constraint) in &program.constraints {
        admit_predicate(constraint)?;
    }
    admit_block(&program.instructions)?;
    admit_expression(&program.result)
}

fn admit_block(instructions: &[Instruction]) -> Result<(), Error> {
    for instruction in instructions {
        match instruction {
            Instruction::For { iterable, body, .. } => {
                admit_expression(iterable)?;
                admit_block(body)?;
            }
            Instruction::Branch {
                condition,
                then_body,
                else_body,
            } => {
                admit_predicate(condition)?;
                admit_block(then_body)?;
                admit_block(else_body)?;
            }
            Instruction::Let(binding)
            | Instruction::Mutation { binding, .. }
            | Instruction::Read { binding, .. }
            | Instruction::GraphRead { binding, .. } => admit_expression(&binding.value)?,
            Instruction::Require { condition, reason } => {
                admit_predicate(condition)?;
                admit_expression(&reason.payload)?;
            }
            Instruction::Attempt { binding, arms } => {
                admit_expression(&binding.value)?;
                for reason in arms.values() {
                    admit_expression(&reason.payload)?;
                }
            }
        }
    }
    Ok(())
}

/// The state one shared-schedule evaluation threads through its blocks.
struct Run<'p, 'm, 'a, 'b> {
    locals: Locals<'p>,
    mutations: Vec<EffectMutation>,
    access: &'m mut RecordedNodeAccess<'a, 'b>,
    schema: &'m EffectGraphSchema,
    meter: &'m mut Meter,
    usage: GraphReadUsage,
}

impl<'p> Run<'p, '_, '_, '_> {
    fn value(&mut self, program: &'p Program, expr: &'p Expr) -> Result<Shared<'p>, Error> {
        Evaluator {
            helpers: &program.helpers,
            meter: &mut *self.meter,
        }
        .expression(expr, &mut self.locals, 0)
    }

    fn holds(&mut self, program: &'p Program, condition: &'p Predicate) -> Result<bool, Error> {
        Evaluator {
            helpers: &program.helpers,
            meter: &mut *self.meter,
        }
        .predicate(condition, &mut self.locals, 0)
    }

    /// Binds a value an effect step produced: validated and charged as the
    /// copying executor does, then shared.
    fn bind(&mut self, name: &'p str, ty: &'p RuntimeType, value: &Value) -> Result<(), Error> {
        validate(value, ty, self.meter, 0)?;
        let shared = from_canonical(value, ty, 0)?;
        self.locals.insert(name, shared);
        Ok(())
    }
}

pub(super) fn run_body(
    program: &Program,
    input: Value,
    access: &mut RecordedNodeAccess<'_, '_>,
    schema: &EffectGraphSchema,
    meter: &mut Meter,
) -> Result<Evaluation, EffectFrameError> {
    // The intent boundary converts and validates the input at full size, as
    // a pure shared-read program's does: two steps per cell and its bytes.
    let shared = from_canonical(&input, &program.input, 0).map_err(|e| {
        if e == Error::InvalidArtifact {
            Error::InvalidInput
        } else {
            e
        }
    })?;
    Evaluator {
        helpers: &program.helpers,
        meter: &mut *meter,
    }
    .boundary(shared.full_size()?)?;
    let mut run = Run {
        locals: Locals::from([("arg.0", shared)]),
        mutations: Vec::new(),
        access,
        schema,
        meter,
        usage: GraphReadUsage::default(),
    };
    if let Some(basis) = &program.basis {
        let Shared::Bytes(basis) = run.value(program, basis)? else {
            return Err(Error::InvalidInput.into());
        };
        if basis.len() != 32 {
            return Err(Error::InvalidInput.into());
        }
    }
    for (name, condition) in &program.constraints {
        if !run.holds(program, condition)? {
            return Err(Error::InputConstraintFailed(name.clone()).into());
        }
    }
    block(program, &program.instructions, &mut run, 0)?;
    let result = run.value(program, &program.result)?;
    // The output crosses the same boundary.
    Evaluator {
        helpers: &program.helpers,
        meter: &mut *run.meter,
    }
    .boundary(result.full_size()?)?;
    let output = encode(&to_canonical(&result, &program.output, 0)?)
        .map_err(|_| Error::InvalidArtifact)?;
    if output.len() as u64 > program.limits.max_output_bytes {
        return Err(Error::OutputBudgetExceeded.into());
    }
    Ok(Evaluation {
        output,
        mutations: run.mutations,
    })
}

/// Runs one block in order, as the copying executor does: a loop binds each
/// item of the list it evaluated on entry, a branch runs one arm, and the
/// locals a block introduces are forgotten when it ends.
fn block<'p>(
    program: &'p Program,
    instructions: &'p [Instruction],
    run: &mut Run<'p, '_, '_, '_>,
    depth: usize,
) -> Result<(), EffectFrameError> {
    for instruction in instructions {
        if !matches!(instruction, Instruction::Mutation { .. }) {
            run.meter.step(depth)?;
        }
        match instruction {
            Instruction::For {
                binder,
                iterable,
                bound,
                body,
                ..
            } => {
                let Shared::List(items) = run.value(program, iterable)? else {
                    return Err(Error::InvalidArtifact.into());
                };
                if u64::try_from(items.len()).map_err(|_| Error::InvalidArtifact)? > *bound {
                    return Err(Error::InvalidArtifact.into());
                }
                for item in items.iter() {
                    run.meter.step(depth + 1)?;
                    run.locals.insert(binder, item.clone());
                    block(program, body, run, depth + 1)?;
                    forget(body, &mut run.locals);
                    run.locals.remove(binder.as_str());
                }
            }
            Instruction::Branch {
                condition,
                then_body,
                else_body,
            } => {
                let body = if run.holds(program, condition)? {
                    then_body
                } else {
                    else_body
                };
                block(program, body, run, depth + 1)?;
                forget(body, &mut run.locals);
            }
            Instruction::Let(binding) => {
                let value = run.value(program, &binding.value)?;
                run.locals.insert(binding.name.as_str(), value);
            }
            Instruction::Mutation { kind, binding } => emit(program, *kind, binding, run)?,
            Instruction::Read {
                binding,
                failures,
                max_bytes,
            } => read(program, binding, failures, *max_bytes, run)?,
            Instruction::GraphRead {
                binding,
                failures,
                read,
            } => graph_read(program, binding, failures, read, run)?,
            Instruction::Attempt { binding, arms } => {
                let value = match run.value(program, &binding.value) {
                    Err(Error::UnmappedFailure(failure)) => {
                        // Only the selected arm's payload is evaluated.
                        let reason = arms.get(&failure).ok_or(Error::InvalidArtifact)?;
                        let payload = run.value(program, &reason.payload)?.to_value();
                        return Err(program.obstruct(&reason.name, payload, run.meter)?);
                    }
                    value => value?,
                };
                run.locals.insert(binding.name.as_str(), value);
            }
            Instruction::Require { condition, reason } => {
                if !run.holds(program, condition)? {
                    let payload = run.value(program, &reason.payload)?.to_value();
                    return Err(program.obstruct(&reason.name, payload, run.meter)?);
                }
            }
        }
    }
    Ok(())
}

fn emit<'p>(
    program: &'p Program,
    kind: MutationKind,
    binding: &'p super::Binding,
    run: &mut Run<'p, '_, '_, '_>,
) -> Result<(), EffectFrameError> {
    let value = run.value(program, &binding.value)?.to_value();
    if run.mutations.len() >= super::super::EFFECT_COUNT_CEILING_V1 {
        return Err(EffectFrameError::BudgetExceeded);
    }
    let mutation = kind.emit(value)?;
    let write_bytes = mutation
        .emission_write_bytes()
        .ok_or(EffectFrameError::BudgetExceeded)?;
    run.access.reads.charge_emission(run.meter, write_bytes)?;
    run.mutations
        .try_reserve(1)
        .map_err(|_| Error::HostAllocationFailed)?;
    run.mutations.push(mutation);
    let result = Value::Map(Vec::new());
    run.meter.charge(&result, 0)?;
    run.bind(binding.name.as_str(), &binding.ty, &result)?;
    Ok(())
}

fn read<'p>(
    program: &'p Program,
    binding: &'p super::Binding,
    failures: &BTreeMap<String, String>,
    max_bytes: u64,
    run: &mut Run<'p, '_, '_, '_>,
) -> Result<(), EffectFrameError> {
    let address = run.value(program, &binding.value)?.to_value();
    let (node, expected) = address_value(&address)?;
    if !run.access.contains(node) {
        return Err(ReadError::OutsideAperture(node).into());
    }
    let value = match run.access.atom(node, expected, max_bytes, run.meter)? {
        Ok(value) => {
            let count = value.len() as u64;
            run.meter.charge_steps(count)?;
            run.meter.allocate(
                VALUE_CELL_BYTES
                    .checked_add(count)
                    .ok_or(Error::AllocationBudgetExceeded)?,
            )?;
            run.meter.copy_precharged_read_bytes(value)?
        }
        Err(cause) => {
            let tag = match cause {
                ReadObstruction::Missing => "echo.atom-missing/v1",
                ReadObstruction::AtomRequired => "echo.atom-required/v1",
                ReadObstruction::TypeMismatch => "echo.atom-type-mismatch/v1",
                ReadObstruction::AtomTooLarge => "echo.atom-byte-bound-exceeded/v1",
                ReadObstruction::Guard => return Err(Error::InvalidArtifact.into()),
            };
            return Err(program.obstruct(
                failures.get(tag).ok_or(Error::InvalidArtifact)?,
                Value::Map(Vec::new()),
                run.meter,
            )?);
        }
    };
    if !matches!(run.access.reads.readings.get(&SlotId::Node(node)), Some(SlotReading::Node(Some(record))) if run.schema.allows_atom(record.ty, expected))
    {
        return Err(EffectFrameError::GraphSchemaViolation);
    }
    run.bind(binding.name.as_str(), &binding.ty, &value)?;
    Ok(())
}

fn graph_read<'p>(
    program: &'p Program,
    binding: &'p super::Binding,
    failures: &BTreeMap<String, String>,
    read: &GraphRead,
    run: &mut Run<'p, '_, '_, '_>,
) -> Result<(), EffectFrameError> {
    let input = run.value(program, &binding.value)?.to_value();
    let graph = program.graph_reads.as_ref().ok_or(Error::InvalidArtifact)?;
    // As on the copying route, the frame's typed law and meter bound the
    // read; the bounded-read route's own ceilings impose nothing further.
    let limits = ReadLimits {
        evaluation: program.limits,
        max_reads: u64::MAX,
        max_read_bytes: u64::MAX,
    };
    // A closure's selector is a source helper the read runs itself, on
    // canonical values, at its copying charge.
    let mut session = GraphReadSession {
        graph,
        helpers: &program.helpers,
        view: &mut *run.access,
        meter: &mut *run.meter,
        usage: &mut run.usage,
        limits: &limits,
    };
    let outcome = match read {
        GraphRead::Edges { max_edges } => edge_read(*max_edges, &input, &mut session)?,
        GraphRead::Closure(shape) => closure_read(shape, &input, &mut session)?,
    };
    let value = match outcome {
        Ok(value) => value,
        Err(cause) => {
            return Err(program.obstruct(
                failures
                    .get(cause.runtime_tag())
                    .ok_or(Error::InvalidArtifact)?,
                Value::Map(Vec::new()),
                run.meter,
            )?);
        }
    };
    run.bind(binding.name.as_str(), &binding.ty, &value)?;
    Ok(())
}
