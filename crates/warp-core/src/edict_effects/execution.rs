// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Private ordered or structured authored execution over the existing
//! expression evaluator and recorded reads.
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
use crate::edict_read::{
    closure_read, closure_shape, edge_read, edge_shape, selector_conforms, ClosureShape,
    GraphReadSession, GraphReadUsage, GraphReads, NodeAtomAccess, ReadError, ReadLimits,
    ReadObstruction,
};
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
    /// A bounded loop over a list the body sees one item at a time.
    For {
        binder: String,
        item: RuntimeType,
        iterable: Expr,
        bound: u64,
        body: Vec<Instruction>,
    },
    /// A statement branch that runs exactly one arm.
    Branch {
        condition: Predicate,
        then_body: Vec<Instruction>,
        else_body: Vec<Instruction>,
    },
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
    /// A typed outgoing-edge or guided closure read (echo#926) as an effect
    /// step: the bounded-read primitive over the recording frame.
    GraphRead {
        binding: Binding,
        failures: BTreeMap<String, String>,
        read: GraphRead,
    },
    Require {
        condition: Predicate,
        reason: Reason,
    },
    /// A call to a failing source helper. A failure it ends with ends the
    /// run with that failure's mapped terminal reason.
    Attempt {
        binding: Binding,
        arms: BTreeMap<String, Reason>,
    },
}
enum GraphRead {
    Edges { max_edges: u64 },
    Closure(ClosureShape),
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
    graph_reads: Option<GraphReads>,
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
        graph_reads: Option<&GraphReads>,
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
        // The relations already paired each Core instruction with its Target
        // instruction; the effect steps are looked up by the node's address.
        let steps = super::control_order::instructions(intent, target)?
            .into_iter()
            .map(|(node, step)| (std::ptr::from_ref(node), step))
            .collect::<BTreeMap<_, _>>();
        let body = field(intent, "body")?;
        let instructions = decode_block(body, &steps, &mut parser, 0)?;
        declared_graph_reads(&instructions, &helpers, graph_reads)?;
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
            graph_reads: graph_reads.cloned(),
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
        let locals = BTreeMap::from([("arg.0", input)]);
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
        let mut run = Run {
            locals,
            mutations: Vec::new(),
            access,
            schema,
            meter,
            usage: GraphReadUsage::default(),
        };
        self.run_block(&self.instructions, &mut run, 0)?;
        let value = expression(&self.result, &run.locals, helpers, run.meter, 0)?;
        validate(&value, &self.output, run.meter, 0)?;
        Ok(Evaluation {
            output: self.output_bytes(&value, run.meter)?,
            mutations: run.mutations,
        })
    }
    /// Runs one block in order. A loop runs its body once per item of the
    /// list it evaluated on entry, binding each item fresh; a branch runs only
    /// the arm its predicate selects. Locals a block introduces are forgotten
    /// when it ends, and every emission joins the one atomic buffer.
    fn run_block<'p>(
        &'p self,
        instructions: &'p [Instruction],
        run: &mut Run<'p, '_, '_, '_>,
        depth: usize,
    ) -> Result<(), EffectFrameError> {
        for instruction in instructions {
            // Mutation's fixed step is admitted atomically with its write
            // bytes and row reservation after evaluating its argument.
            if !matches!(instruction, Instruction::Mutation { .. }) {
                run.meter.step(depth)?;
            }
            match instruction {
                Instruction::For {
                    binder,
                    item,
                    iterable,
                    bound,
                    body,
                } => self.run_loop(binder, item, iterable, *bound, body, run, depth)?,
                Instruction::Branch {
                    condition,
                    then_body,
                    else_body,
                } => {
                    let body = if predicate(condition, &run.locals, &self.helpers, run.meter, 0)? {
                        then_body
                    } else {
                        else_body
                    };
                    self.run_block(body, run, depth + 1)?;
                    forget(body, &mut run.locals);
                }
                Instruction::Mutation { kind, binding } => self.emit(*kind, binding, run)?,
                Instruction::Let(binding) => {
                    let value =
                        expression(&binding.value, &run.locals, &self.helpers, run.meter, 0)?;
                    validate(&value, &binding.ty, run.meter, 0)?;
                    run.locals.insert(binding.name.as_str(), value);
                }
                Instruction::Read {
                    binding,
                    failures,
                    max_bytes,
                } => self.read(binding, failures, *max_bytes, run)?,
                Instruction::GraphRead {
                    binding,
                    failures,
                    read,
                } => self.graph_read(binding, failures, read, run)?,
                Instruction::Attempt { binding, arms } => {
                    let value = match expression(
                        &binding.value,
                        &run.locals,
                        &self.helpers,
                        run.meter,
                        0,
                    ) {
                        Err(Error::UnmappedFailure(failure)) => {
                            // Only the selected arm's payload is evaluated.
                            let reason = arms.get(&failure).ok_or(Error::InvalidArtifact)?;
                            let payload = expression(
                                &reason.payload,
                                &run.locals,
                                &self.helpers,
                                run.meter,
                                0,
                            )?;
                            return Err(self.obstruct(&reason.name, payload, run.meter)?);
                        }
                        value => value?,
                    };
                    validate(&value, &binding.ty, run.meter, 0)?;
                    run.locals.insert(binding.name.as_str(), value);
                }
                Instruction::Require { condition, reason } => {
                    if !predicate(condition, &run.locals, &self.helpers, run.meter, 0)? {
                        let payload =
                            expression(&reason.payload, &run.locals, &self.helpers, run.meter, 0)?;
                        return Err(self.obstruct(&reason.name, payload, run.meter)?);
                    }
                }
            }
        }
        Ok(())
    }
    /// The iterable is evaluated once, in the enclosing scope, and the
    /// evaluator has charged its storage. Each item is validated against the
    /// binder's type and bound fresh for one run of the body.
    #[allow(
        clippy::too_many_arguments,
        reason = "The loop's own fields, with the shared run state."
    )]
    fn run_loop<'p>(
        &'p self,
        binder: &'p str,
        item: &RuntimeType,
        iterable: &Expr,
        bound: u64,
        body: &'p [Instruction],
        run: &mut Run<'p, '_, '_, '_>,
        depth: usize,
    ) -> Result<(), EffectFrameError> {
        let Value::Array(values) = expression(iterable, &run.locals, &self.helpers, run.meter, 0)?
        else {
            return Err(Error::InvalidArtifact.into());
        };
        if u64::try_from(values.len()).map_err(|_| Error::InvalidArtifact)? > bound {
            return Err(Error::InvalidArtifact.into());
        }
        for value in values {
            run.meter.step(depth + 1)?;
            validate(&value, item, run.meter, 0)?;
            run.locals.insert(binder, value);
            self.run_block(body, run, depth + 1)?;
            forget(body, &mut run.locals);
            run.locals.remove(binder);
        }
        Ok(())
    }
    fn emit<'p>(
        &'p self,
        kind: MutationKind,
        binding: &'p Binding,
        run: &mut Run<'p, '_, '_, '_>,
    ) -> Result<(), EffectFrameError> {
        let value = expression(&binding.value, &run.locals, &self.helpers, run.meter, 0)?;
        if run.mutations.len() >= super::EFFECT_COUNT_CEILING_V1 {
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
        validate(&result, &binding.ty, run.meter, 0)?;
        run.locals.insert(binding.name.as_str(), result);
        Ok(())
    }
    fn read<'p>(
        &'p self,
        binding: &'p Binding,
        failures: &BTreeMap<String, String>,
        max_bytes: u64,
        run: &mut Run<'p, '_, '_, '_>,
    ) -> Result<(), EffectFrameError> {
        let address = expression(&binding.value, &run.locals, &self.helpers, run.meter, 0)?;
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
                return Err(self.obstruct(
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
        validate(&value, &binding.ty, run.meter, 0)?;
        run.locals.insert(binding.name.as_str(), value);
        Ok(())
    }
    fn graph_read<'p>(
        &'p self,
        binding: &'p Binding,
        failures: &BTreeMap<String, String>,
        read: &GraphRead,
        run: &mut Run<'p, '_, '_, '_>,
    ) -> Result<(), EffectFrameError> {
        let input = expression(&binding.value, &run.locals, &self.helpers, run.meter, 0)?;
        let graph = self.graph_reads.as_ref().ok_or(Error::InvalidArtifact)?;
        // Attempts and delivered bytes are bounded by the frame's typed law
        // and meter, so the bounded-read route's own read-count and
        // read-byte ceilings impose nothing further here.
        let limits = ReadLimits {
            evaluation: self.limits,
            max_reads: u64::MAX,
            max_read_bytes: u64::MAX,
        };
        let mut session = GraphReadSession {
            graph,
            helpers: &self.helpers,
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
                return Err(self.obstruct(
                    failures
                        .get(cause.runtime_tag())
                        .ok_or(Error::InvalidArtifact)?,
                    Value::Map(Vec::new()),
                    run.meter,
                )?);
            }
        };
        validate(&value, &binding.ty, run.meter, 0)?;
        run.locals.insert(binding.name.as_str(), value);
        Ok(())
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
/// A graph read is a declared capability, as on the bounded-read route: the
/// declaration is present exactly when a step reads the graph, and every
/// closure is guided by the declared source helper over its own node and
/// guide types.
fn declared_graph_reads(
    instructions: &[Instruction],
    helpers: &BTreeMap<String, Helper>,
    graph: Option<&GraphReads>,
) -> Result<(), Error> {
    if graph.is_some() != (graph_read_count(instructions, helpers, graph)? > 0) {
        return Err(Error::UnsupportedProgram);
    }
    Ok(())
}
/// Counts the graph reads in `instructions` and the blocks nested in them,
/// checking each closure's selector against the declaration on the way.
fn graph_read_count(
    instructions: &[Instruction],
    helpers: &BTreeMap<String, Helper>,
    graph: Option<&GraphReads>,
) -> Result<usize, Error> {
    let mut reads = 0_usize;
    for instruction in instructions {
        match instruction {
            Instruction::GraphRead { read, .. } => {
                reads = reads.saturating_add(1);
                if let GraphRead::Closure(shape) = read {
                    let selector = graph
                        .and_then(|graph| graph.selector.as_deref())
                        .and_then(|name| helpers.get(name))
                        .ok_or(Error::UnsupportedProgram)?;
                    selector_conforms(selector, shape)?;
                }
            }
            Instruction::For { body, .. } => {
                reads = reads.saturating_add(graph_read_count(body, helpers, graph)?);
            }
            Instruction::Branch {
                then_body,
                else_body,
                ..
            } => {
                reads = reads.saturating_add(graph_read_count(then_body, helpers, graph)?);
                reads = reads.saturating_add(graph_read_count(else_body, helpers, graph)?);
            }
            Instruction::Let(_)
            | Instruction::Mutation { .. }
            | Instruction::Read { .. }
            | Instruction::Require { .. }
            | Instruction::Attempt { .. } => {}
        }
    }
    Ok(reads)
}
/// The state one evaluation threads through its blocks.
struct Run<'p, 'm, 'a, 'b> {
    locals: BTreeMap<&'p str, Value>,
    mutations: Vec<EffectMutation>,
    access: &'m mut RecordedNodeAccess<'a, 'b>,
    schema: &'m EffectGraphSchema,
    meter: &'m mut Meter,
    /// Graph reads share one scan allowance per evaluation.
    usage: GraphReadUsage,
}
/// Removes the locals `instructions` bound, so none outlives its block.
/// Nested blocks have already forgotten their own.
fn forget(instructions: &[Instruction], locals: &mut BTreeMap<&str, Value>) {
    for instruction in instructions {
        match instruction {
            Instruction::Let(binding)
            | Instruction::Read { binding, .. }
            | Instruction::GraphRead { binding, .. }
            | Instruction::Mutation { binding, .. }
            | Instruction::Attempt { binding, .. } => {
                locals.remove(binding.name.as_str());
            }
            Instruction::For { .. } | Instruction::Branch { .. } | Instruction::Require { .. } => {}
        }
    }
}
/// A terminal obstruction reason; its payload fields evaluate as one record.
fn reason(reason: &Value, parser: &mut Parser<'_>) -> Result<Reason, Error> {
    let mut payload = Vec::new();
    for (key, value) in map(field(reason, "payload")?)? {
        payload.push((text(key)?.to_owned(), parser.expr(value, 0)?));
    }
    Ok(Reason {
        name: text_field(reason, "reasonKind")?.to_owned(),
        payload: Expr::Record(payload),
    })
}
/// Decodes one checked Core block. `steps` holds each effect node's Target
/// step by the node's address.
fn decode_block(
    body: &Value,
    steps: &BTreeMap<*const Value, &Value>,
    parser: &mut Parser<'_>,
    depth: usize,
) -> Result<Vec<Instruction>, Error> {
    let mut instructions = Vec::new();
    for node in array(field(body, "nodes")?)? {
        parser.enter(depth)?;
        let instruction = match text_field(node, "kind")? {
            "let" => Instruction::Let(binding(node, "value", parser)?),
            "effect" => {
                let step = steps
                    .get(&std::ptr::from_ref(node))
                    .ok_or(Error::InvalidArtifact)?;
                effect(node, step, parser)?
            }
            "require" => Instruction::Require {
                condition: parser.predicate(field(node, "predicate")?, 0)?,
                reason: reason(field(field(node, "onFailure")?, "reason")?, parser)?,
            },
            "attempt" => {
                let mut arms = BTreeMap::new();
                for (failure, arm) in map(field(node, "failureMap")?)? {
                    arms.insert(text(failure)?.to_owned(), reason(arm, parser)?);
                }
                Instruction::Attempt {
                    binding: binding(node, "value", parser)?,
                    arms,
                }
            }
            "for" => {
                let binder = field(node, "binder")?;
                let bound = field(node, "bound")?;
                if text_field(bound, "kind")? != "literal" {
                    return Err(Error::UnsupportedProgram);
                }
                Instruction::For {
                    binder: text_field(binder, "id")?.to_owned(),
                    item: parser.ty(text_field(binder, "type")?, 0)?,
                    iterable: parser.expr(field(node, "iter")?, 0)?,
                    bound: number(field(bound, "value")?)?,
                    body: decode_block(field(node, "body")?, steps, parser, depth + 1)?,
                }
            }
            "branch" => Instruction::Branch {
                condition: parser.predicate(field(node, "predicate")?, 0)?,
                then_body: decode_block(field(node, "then")?, steps, parser, depth + 1)?,
                else_body: decode_block(field(node, "else")?, steps, parser, depth + 1)?,
            },
            _ => return Err(Error::UnsupportedProgram),
        };
        instructions.push(instruction);
    }
    Ok(instructions)
}
/// An effect node is a mutation, an atom read or a graph read, by its
/// step's intrinsic.
fn effect(node: &Value, step: &Value, parser: &mut Parser<'_>) -> Result<Instruction, Error> {
    let binding = binding(node, "input", parser)?;
    let intrinsic = text_field(step, "targetIntrinsic")?;
    if let Some(kind) = MutationKind::from_intrinsic(intrinsic) {
        return Ok(Instruction::Mutation { kind, binding });
    }
    // Arms are keyed by catalog name; the read raises runtime tags.
    let mut arms = BTreeMap::new();
    for (name, arm) in map(field(step, "obstructionArms")?)? {
        arms.insert(
            text(name)?.to_owned(),
            text_field(field(arm, "value")?, "callee")?.to_owned(),
        );
    }
    let failures = super::failure_names::runtime_arms(intrinsic, arms)?;
    Ok(match super::intrinsic_names::operation(intrinsic) {
        Some("outgoing-edge-read") => Instruction::GraphRead {
            read: GraphRead::Edges {
                max_edges: edge_shape(&binding.ty)?,
            },
            binding,
            failures,
        },
        Some("closure-read") => Instruction::GraphRead {
            read: GraphRead::Closure(closure_shape(&binding.ty)?),
            binding,
            failures,
        },
        _ => {
            let RuntimeType::Bytes { max, .. } = binding.ty else {
                return Err(Error::UnsupportedProgram);
            };
            Instruction::Read {
                binding,
                failures,
                max_bytes: max,
            }
        }
    })
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
