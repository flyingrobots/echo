// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Private ordered authored execution over the existing expression evaluator and recorded reads.
use super::mutations::{EffectMutation, MutationKind};
use super::{EffectFrameError, EffectGraphSchema, RecordedNodeAccess};
use crate::edict_pure::{
    evaluate::{expression, predicate, validate, Meter, VALUE_CELL_BYTES},
    functions,
    model::{Expr, Helper, Predicate, RuntimeType, MAX_PROGRAM_NODES},
    syntax::Parser,
    values::{array, bytes, field, map, number, text, text_field},
    EvaluationError as Error, EvaluationLimits,
};
use crate::edict_read::{NodeAtomAccess, ReadError, ReadObstruction};
use crate::{NodeId, NodeKey, SlotId, SlotReading, TypeId, WarpId};
use echo_edict_canonical::{encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value};
use std::collections::BTreeMap;

struct Binding {
    name: String,
    ty: RuntimeType,
    value: Expr,
}
struct Reason {
    name: String,
    payload: Expr,
}
enum Instruction {
    Let(Binding),
    Mutation {
        kind: MutationKind,
        binding: Binding,
    },
    Read {
        binding: Binding,
        failures: BTreeMap<String, String>,
        max_bytes: u64,
    },
    Require {
        condition: Predicate,
        reason: Reason,
    },
}
pub(super) struct Evaluation {
    pub(super) output: Vec<u8>,
    pub(super) mutations: Vec<EffectMutation>,
}
pub(super) struct Program {
    input: RuntimeType,
    output: RuntimeType,
    basis: Option<Expr>,
    constraints: Vec<(String, Predicate)>,
    instructions: Vec<Instruction>,
    helpers: BTreeMap<String, Helper>,
    result: Expr,
    limits: EvaluationLimits,
}
impl Program {
    // The existing ordered relation and lowering/ABI validator establish authority.
    // This decodes that already checked body into the shared evaluator's types.
    pub(super) fn prepare(
        module: &Value,
        intent: &Value,
        target: &Value,
        exports: &Value,
        mut limits: EvaluationLimits,
    ) -> Result<Self, Error> {
        let mut parser = Parser {
            types: field(module, "types")?,
            coordinate: text_field(module, "coordinate")?,
            remaining: MAX_PROGRAM_NODES,
        };
        let helpers = functions::decode(module, exports, &mut parser)?;
        let budget = field(intent, "coreEvaluationBudget")?;
        limits.max_steps = limits.max_steps.min(number(field(budget, "maxSteps")?)?);
        limits.max_allocated_bytes = limits
            .max_allocated_bytes
            .min(number(field(budget, "maxAllocatedBytes")?)?);
        limits.max_output_bytes = limits
            .max_output_bytes
            .min(number(field(budget, "maxOutputBytes")?)?);
        let mut constraints = Vec::new();
        for constraint in array(field(intent, "inputConstraints")?)? {
            constraints.push((
                text_field(constraint, "coordinate")?.to_owned(),
                parser.predicate(field(constraint, "predicate")?, 0)?,
            ));
        }
        let mut lowered = BTreeMap::new();
        for step in array(field(target, "steps")?)? {
            lowered.insert(text_field(step, "id")?, step);
        }
        let body = field(intent, "body")?;
        let mut instructions = Vec::new();
        for (node, id) in array(field(body, "nodes")?)?
            .iter()
            .zip(array(field(target, "executionOrder")?)?)
        {
            parser.enter(0)?;
            let instruction = match text_field(node, "kind")? {
                "let" => Instruction::Let(binding(node, "value", &mut parser)?),
                "effect" => {
                    let step = lowered.get(text(id)?).ok_or(Error::InvalidArtifact)?;
                    let binding = binding(node, "input", &mut parser)?;
                    if let Some(kind) =
                        MutationKind::from_intrinsic(text_field(step, "targetIntrinsic")?)
                    {
                        instructions.push(Instruction::Mutation { kind, binding });
                        continue;
                    }
                    let RuntimeType::Bytes { max, .. } = binding.ty else {
                        return Err(Error::UnsupportedProgram);
                    };
                    let mut failures = BTreeMap::new();
                    for (tag, arm) in map(field(step, "obstructionArms")?)? {
                        failures.insert(
                            text(tag)?.to_owned(),
                            text_field(field(arm, "value")?, "callee")?.to_owned(),
                        );
                    }
                    Instruction::Read {
                        binding,
                        failures,
                        max_bytes: max,
                    }
                }
                "require" => {
                    let reason = field(field(node, "onFailure")?, "reason")?;
                    let mut payload = Vec::new();
                    for (key, value) in map(field(reason, "payload")?)? {
                        payload.push((text(key)?.to_owned(), parser.expr(value, 0)?));
                    }
                    Instruction::Require {
                        condition: parser.predicate(field(node, "predicate")?, 0)?,
                        reason: Reason {
                            name: text_field(reason, "reasonKind")?.to_owned(),
                            payload: Expr::Record(payload),
                        },
                    }
                }
                _ => return Err(Error::UnsupportedProgram),
            };
            instructions.push(instruction);
        }
        Ok(Self {
            input: parser.ty(text_field(intent, "input")?, 0)?,
            output: parser.ty(text_field(intent, "output")?, 0)?,
            basis: field(intent, "basis")
                .ok()
                .map(|v| parser.expr(v, 0))
                .transpose()?,
            constraints,
            instructions,
            helpers,
            result: parser.expr(field(body, "result")?, 0)?,
            limits,
        })
    }
    pub(super) fn run(
        &self,
        input: Value,
        access: &mut RecordedNodeAccess<'_, '_>,
        schema: &EffectGraphSchema,
        meter: &mut Meter,
    ) -> Result<Evaluation, EffectFrameError> {
        meter.with_component(self.limits, |meter| {
            self.run_body(input, access, schema, meter)
        })
    }
    fn run_body(
        &self,
        input: Value,
        access: &mut RecordedNodeAccess<'_, '_>,
        schema: &EffectGraphSchema,
        meter: &mut Meter,
    ) -> Result<Evaluation, EffectFrameError> {
        validate(&input, &self.input, meter, 0).map_err(|e| {
            if e == Error::InvalidArtifact {
                Error::InvalidInput
            } else {
                e
            }
        })?;
        meter.charge(&input, 0)?;
        let mut locals = BTreeMap::from([("arg.0", input)]);
        let mut mutations = Vec::new();
        let helpers = &self.helpers;
        if let Some(basis) = &self.basis {
            let basis = expression(basis, &locals, helpers, meter, 0)?;
            if bytes(&basis)?.len() != 32 {
                return Err(Error::InvalidInput.into());
            }
        }
        for (name, condition) in &self.constraints {
            if !predicate(condition, &locals, helpers, meter, 0)? {
                return Err(Error::InputConstraintFailed(name.clone()).into());
            }
        }
        for instruction in &self.instructions {
            // Mutation's fixed step is admitted atomically with its write bytes
            // and row reservation after evaluating its argument.
            if !matches!(instruction, Instruction::Mutation { .. }) {
                meter.step(0)?;
            }
            match instruction {
                Instruction::Mutation { kind, binding } => {
                    let value = expression(&binding.value, &locals, helpers, meter, 0)?;
                    if mutations.len() >= super::EFFECT_COUNT_CEILING_V1 {
                        return Err(EffectFrameError::BudgetExceeded);
                    }
                    let mutation = kind.emit(value)?;
                    let write_bytes = mutation
                        .emission_write_bytes()
                        .ok_or(EffectFrameError::BudgetExceeded)?;
                    access.reads.charge_emission(meter, write_bytes)?;
                    mutations
                        .try_reserve(1)
                        .map_err(|_| Error::HostAllocationFailed)?;
                    mutations.push(mutation);
                    let result = Value::Map(Vec::new());
                    meter.charge(&result, 0)?;
                    validate(&result, &binding.ty, meter, 0)?;
                    locals.insert(binding.name.as_str(), result);
                }
                Instruction::Let(binding) => {
                    let value = expression(&binding.value, &locals, helpers, meter, 0)?;
                    validate(&value, &binding.ty, meter, 0)?;
                    locals.insert(binding.name.as_str(), value);
                }
                Instruction::Read {
                    binding,
                    failures,
                    max_bytes,
                } => {
                    let address = expression(&binding.value, &locals, helpers, meter, 0)?;
                    let (node, expected) = address_value(&address)?;
                    if !access.contains(node) {
                        return Err(ReadError::OutsideAperture(node).into());
                    }
                    let value = match access.atom(node, expected, *max_bytes, meter)? {
                        Ok(value) => {
                            let count = value.len() as u64;
                            meter.charge_steps(count)?;
                            meter.allocate(
                                VALUE_CELL_BYTES
                                    .checked_add(count)
                                    .ok_or(Error::AllocationBudgetExceeded)?,
                            )?;
                            meter.copy_precharged_read_bytes(value)?
                        }
                        Err(cause) => {
                            let tag = match cause {
                                ReadObstruction::Missing => "echo.atom-missing/v1",
                                ReadObstruction::AtomRequired => "echo.atom-required/v1",
                                ReadObstruction::TypeMismatch => "echo.atom-type-mismatch/v1",
                                ReadObstruction::AtomTooLarge => "echo.atom-byte-bound-exceeded/v1",
                                ReadObstruction::Guard => return Err(Error::InvalidArtifact.into()),
                            };
                            return Err(self.obstruct(
                                failures.get(tag).ok_or(Error::InvalidArtifact)?,
                                Value::Map(Vec::new()),
                                meter,
                            )?);
                        }
                    };
                    if !matches!(access.reads.readings.get(&SlotId::Node(node)), Some(SlotReading::Node(Some(record))) if schema.allows_atom(record.ty, expected))
                    {
                        return Err(EffectFrameError::GraphSchemaViolation);
                    }
                    validate(&value, &binding.ty, meter, 0)?;
                    locals.insert(binding.name.as_str(), value);
                }
                Instruction::Require { condition, reason } => {
                    if !predicate(condition, &locals, helpers, meter, 0)? {
                        let payload = expression(&reason.payload, &locals, helpers, meter, 0)?;
                        return Err(self.obstruct(&reason.name, payload, meter)?);
                    }
                }
            }
        }
        let value = expression(&self.result, &locals, helpers, meter, 0)?;
        validate(&value, &self.output, meter, 0)?;
        Ok(Evaluation {
            output: self.output_bytes(&value, meter)?,
            mutations,
        })
    }
    fn output_bytes(&self, value: &Value, meter: &mut Meter) -> Result<Vec<u8>, EffectFrameError> {
        meter.charge(value, 0)?;
        let output = encode(value).map_err(|_| Error::InvalidArtifact)?;
        if output.len() as u64 > self.limits.max_output_bytes {
            return Err(Error::OutputBudgetExceeded.into());
        }
        Ok(output)
    }
    fn obstruct(
        &self,
        name: &str,
        payload: Value,
        meter: &mut Meter,
    ) -> Result<EffectFrameError, EffectFrameError> {
        Ok(EffectFrameError::AuthoredObstruction {
            reason_kind: name.to_owned(),
            payload: self.output_bytes(&payload, meter)?,
        })
    }
}
fn binding(node: &Value, key: &str, parser: &mut Parser<'_>) -> Result<Binding, Error> {
    let local = field(node, "binding")?;
    Ok(Binding {
        name: text_field(local, "id")?.to_owned(),
        ty: parser.ty(text_field(local, "type")?, 0)?,
        value: parser.expr(field(node, key)?, 0)?,
    })
}
fn address_value(value: &Value) -> Result<(NodeKey, TypeId), Error> {
    let id = |key| {
        bytes(field(value, key)?)?
            .try_into()
            .map_err(|_| Error::InvalidInput)
    };
    Ok((
        NodeKey {
            warp_id: WarpId(id("warpId")?),
            local_id: NodeId(id("nodeId")?),
        },
        TypeId(id("typeId")?),
    ))
}
