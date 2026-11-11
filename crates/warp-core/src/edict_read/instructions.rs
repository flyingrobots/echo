// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use super::graph;
use super::model::{Instruction, ReadInstruction, ReadKind};
use crate::edict_pure::{
    model::{RuntimeType, MAX_PROGRAM_NODES},
    syntax::Parser,
    values::{array, exact_fields, field, map, require_text, text, text_field},
    EvaluationError as Error,
};
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Instructions {
    pub ordered: Vec<Instruction>,
    pub pure_producers: BTreeMap<String, String>,
    pub read_producers: BTreeMap<String, String>,
}

pub(super) fn parse(
    target: &Value,
    parser: &mut Parser<'_>,
    input_id: &str,
    graph_reads: bool,
) -> Result<Instructions, Error> {
    let mut pending = BTreeMap::new();
    let mut pure_producers = BTreeMap::new();
    let mut read_producers = BTreeMap::new();
    let mut locals = BTreeSet::from([input_id.to_owned()]);
    for (collection, kind) in [
        ("steps", "read"),
        ("requirements", "require"),
        ("pureBindings", "let"),
    ] {
        if collection == "pureBindings" && field(target, collection).is_err() {
            continue;
        }
        for value in array(field(target, collection)?)? {
            if pending.len() >= MAX_PROGRAM_NODES {
                return Err(Error::UnsupportedProgram);
            }
            let id = text_field(value, "id")?.to_owned();
            let instruction = match kind {
                "read" => {
                    let read = read(value, parser, graph_reads)?;
                    if !locals.insert(read.binding.clone()) {
                        return Err(Error::InvalidArtifact);
                    }
                    read_producers.insert(id.clone(), read.binding.clone());
                    Instruction::Read(read)
                }
                "let" => {
                    exact_fields(value, &["id", "binding", "value"])?;
                    let binding = field(value, "binding")?;
                    let name = text_field(binding, "id")?.to_owned();
                    if !locals.insert(name.clone()) {
                        return Err(Error::InvalidArtifact);
                    }
                    pure_producers.insert(id.clone(), name.clone());
                    Instruction::Let {
                        binding: name,
                        ty: parser.ty(text_field(binding, "type")?, 0)?,
                        value: parser.expr(field(value, "value")?, 0)?,
                    }
                }
                "require" => {
                    exact_fields(value, &["id", "predicate", "onFailure"])?;
                    let failure = field(value, "onFailure")?;
                    exact_fields(failure, &["kind", "reason"])?;
                    require_text(failure, "kind", "terminal")?;
                    let reason = field(failure, "reason")?;
                    exact_fields(reason, &["reasonKind", "payload"])?;
                    if !map(field(reason, "payload")?)?.is_empty() {
                        return Err(Error::UnsupportedProgram);
                    }
                    Instruction::Require {
                        predicate: parser.predicate(field(value, "predicate")?, 0)?,
                        obstruction: text_field(reason, "reasonKind")?.to_owned(),
                    }
                }
                _ => return Err(Error::UnsupportedProgram),
            };
            if pending.insert(id, instruction).is_some() {
                return Err(Error::InvalidArtifact);
            }
        }
    }
    let order = array(field(target, "executionOrder")?)?;
    if order.len() != pending.len() || read_producers.is_empty() {
        return Err(Error::InvalidArtifact);
    }
    let mut ordered = Vec::with_capacity(order.len());
    for id in order {
        ordered.push(pending.remove(text(id)?).ok_or(Error::InvalidArtifact)?);
    }
    Ok(Instructions {
        ordered,
        pure_producers,
        read_producers,
    })
}

const NODE_ATOM_FAILURES: [&str; 4] = [
    "echo.atom-missing/v1",
    "echo.atom-required/v1",
    "echo.atom-type-mismatch/v1",
    "echo.atom-byte-bound-exceeded/v1",
];

fn read(
    value: &Value,
    parser: &mut Parser<'_>,
    graph_reads: bool,
) -> Result<ReadInstruction, Error> {
    exact_fields(
        value,
        &[
            "id",
            "binding",
            "effect",
            "input",
            "targetIntrinsic",
            "obstructionArms",
            "obstructionFailures",
        ],
    )?;
    let binding = field(value, "binding")?;
    let ty = parser.ty(text_field(binding, "type")?, 0)?;
    let intrinsic = text_field(value, "targetIntrinsic")?;
    let (kind, failures): (_, Vec<&'static str>) = match intrinsic {
        "echo.dpo@1.node-atom-read" => {
            let RuntimeType::Bytes { min: 0, max } = ty else {
                return Err(Error::UnsupportedProgram);
            };
            if max == 0 || max > 67_108_864 {
                return Err(Error::UnsupportedProgram);
            }
            (
                ReadKind::NodeAtom { max_bytes: max },
                NODE_ATOM_FAILURES.to_vec(),
            )
        }
        // A graph read is a declared capability of the lowering
        // configuration; a package that does not declare it cannot use one.
        graph::EDGE_READ if graph_reads => (
            ReadKind::OutgoingEdges {
                max_edges: graph::edge_shape(&ty)?,
            },
            graph::EDGE_FAILURES
                .iter()
                .map(|failure| failure.runtime_tag())
                .collect(),
        ),
        graph::CLOSURE_READ if graph_reads => (
            ReadKind::Closure(graph::closure_shape(&ty)?),
            graph::CLOSURE_FAILURES
                .iter()
                .map(|failure| failure.runtime_tag())
                .collect(),
        ),
        _ => return Err(Error::UnsupportedProgram),
    };
    let declared = field(value, "obstructionArms")?;
    exact_fields(declared, &failures)?;
    let mut arms = BTreeMap::new();
    for failure in failures {
        arms.insert(failure, obstruction(declared, failure)?);
    }
    Ok(ReadInstruction {
        binding: text_field(binding, "id")?.to_owned(),
        ty,
        input: parser.expr(field(value, "input")?, 0)?,
        kind,
        arms,
    })
}

fn obstruction(arms: &Value, name: &str) -> Result<String, Error> {
    let arm = field(arms, name)?;
    exact_fields(arm, &["binder", "value"])?;
    let value = field(arm, "value")?;
    exact_fields(value, &["kind", "callee", "args", "typeArgs"])?;
    require_text(value, "kind", "call")?;
    if !array(field(value, "args")?)?.is_empty() || !array(field(value, "typeArgs")?)?.is_empty() {
        return Err(Error::UnsupportedProgram);
    }
    Ok(text_field(value, "callee")?.to_owned())
}
