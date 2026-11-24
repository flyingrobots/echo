// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Produces the shared `edict.core-cost/v2` corpus from the pinned compiler.
//!
//! For each authored program in the corpus directory, this producer compiles it
//! with the shared-read schedule and binary-searches the smallest step,
//! allocation and output budgets the compiler accepts. It requires native
//! Target to agree on the step and allocation minimums, then writes the
//! canonical Core module compiled at exactly those minimums. Echo's runtime,
//! lowerer and verifier read the same bytes, so all three judges are measured
//! against one compiler minimum.
//!
//! Usage: `echo-edict-core-cost-v2-corpus <corpus-dir> [--check]`.
use amblicit_syntax::{
    compile_to_core, encode_core_module, lower_to_target_ir, parse_module, CompilerContext,
    CoreBudget, CoreCostSchedule, CoreModule, ResourceRef, TargetIrLoweringFacts,
    TargetLoweringStatus,
};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Search ceiling used by the compiler's own boundary tests.
const CEILING: u64 = u64::MAX / 4;
/// Capability that selects the shared-read schedule.
const SHARED: &str = "core.cost.shared-reads/v1";

/// One corpus program: its source file stem, its intent and its budget name.
struct Program {
    name: &'static str,
    intent: &'static str,
    profile: &'static str,
    budget: &'static str,
}

const PROGRAMS: [Program; 7] = [
    Program {
        name: "reads",
        intent: "run",
        profile: "p.read",
        budget: "p.b",
    },
    Program {
        name: "fill",
        intent: "run",
        profile: "p.read",
        budget: "p.b",
    },
    Program {
        name: "nested",
        intent: "run",
        profile: "p.read",
        budget: "p.b",
    },
    Program {
        name: "matches",
        intent: "run",
        profile: "p.read",
        budget: "p.b",
    },
    Program {
        name: "integers",
        intent: "run",
        profile: "p.read",
        budget: "p.b",
    },
    Program {
        name: "bytes",
        intent: "run",
        profile: "p.read",
        budget: "p.b",
    },
    Program {
        name: "rope",
        intent: "splitRope",
        profile: "rope.read",
        budget: "rope.split",
    },
];

fn budget(steps: u64, bytes: u64, output: u64) -> CoreBudget {
    CoreBudget {
        max_steps: steps,
        max_allocated_bytes: bytes,
        max_output_bytes: output,
    }
}

fn compile(source: &str, program: &Program, budget: CoreBudget) -> Option<CoreModule> {
    compile_or_explain(source, program, budget).ok()
}

/// Compiles `source`, keeping the parser's or compiler's refusal.
fn compile_or_explain(
    source: &str,
    program: &Program,
    budget: CoreBudget,
) -> Result<CoreModule, String> {
    let module = parse_module(source).map_err(|e| format!("{}: {e:?}", program.name))?;
    compile_to_core(
        &module,
        &CompilerContext::new()
            .with_core_cost_schedule(CoreCostSchedule::SharedReads)
            .with_operation_profile(program.profile, "continuum.profile.read-only/v1")
            .with_budget(program.budget, budget),
    )
    .map_err(|e| format!("{}: {e:?}", program.name))
}

/// Smallest value in `0..=CEILING` that `accepts`, which must be monotone.
fn minimum(mut accepts: impl FnMut(u64) -> bool) -> Result<u64, String> {
    if !accepts(CEILING) {
        return Err("refused even at the search ceiling".into());
    }
    let (mut low, mut high) = (0, CEILING);
    while low < high {
        let middle = low + (high - low) / 2;
        if accepts(middle) {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    Ok(high)
}

fn facts() -> TargetIrLoweringFacts {
    TargetIrLoweringFacts {
        target_profile: ResourceRef {
            coordinate: "echo.dpo@1".into(),
            digest: Some(format!("sha256:{}", "1".repeat(64))),
        },
        target_ir_domain: "echo.span-ir/v3".into(),
        operation_profiles: vec!["continuum.profile.read-only/v1".into()],
        obstruction_coordinates: Vec::new(),
        effect_lowerings: Vec::new(),
        effect_signatures: Vec::new(),
        pure_functions: Vec::new(),
    }
}

/// Native Target's minimum on one axis, holding the others at the ceiling.
fn native_minimum(
    core: &CoreModule,
    intent: &str,
    axis: fn(u64) -> CoreBudget,
) -> Result<u64, String> {
    minimum(|value| {
        let mut core = core.clone();
        let Some(lowered) = core.intents.get_mut(intent) else {
            return false;
        };
        lowered.core_evaluation_budget = axis(value);
        lower_to_target_ir(&core, &facts()).status == TargetLoweringStatus::Lowered
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

/// Compiles one program and returns its minimum budgets and Core bytes.
fn produce(directory: &Path, program: &Program) -> Result<([u64; 3], Vec<u8>), String> {
    let path = directory.join(format!("{}.amblicit", program.name));
    let source = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    compile_or_explain(&source, program, budget(CEILING, CEILING, CEILING))?;
    let steps = minimum(|v| compile(&source, program, budget(v, CEILING, CEILING)).is_some())?;
    let bytes = minimum(|v| compile(&source, program, budget(CEILING, v, CEILING)).is_some())?;
    let output = minimum(|v| compile(&source, program, budget(CEILING, CEILING, v)).is_some())?;
    let core = compile(&source, program, budget(steps, bytes, output))
        .ok_or_else(|| format!("{}: refused at its own minimums", program.name))?;
    if !core.required_core_capabilities.iter().any(|c| c == SHARED) {
        return Err(format!("{}: Core does not require {SHARED}", program.name));
    }
    let native = [
        native_minimum(&core, program.intent, |v| budget(v, CEILING, CEILING))?,
        native_minimum(&core, program.intent, |v| budget(CEILING, v, CEILING))?,
    ];
    if native != [steps, bytes] {
        return Err(format!(
            "{}: source minimums {steps}/{bytes} differ from native {native:?}",
            program.name
        ));
    }
    let encoded = encode_core_module(&core).map_err(|e| format!("{}: {e:?}", program.name))?;
    Ok(([steps, bytes, output], encoded))
}

fn run(directory: &Path, check: bool) -> Result<(), String> {
    let mut manifest = String::from(
        "# Produced by tests/edict-core-cost-v2-corpus; do not edit.\n\
         # name intent max_steps max_allocated_bytes max_output_bytes\n",
    );
    for program in &PROGRAMS {
        let ([steps, bytes, output], core) = produce(directory, program)?;
        let _ = writeln!(
            manifest,
            "{} {} {steps} {bytes} {output}",
            program.name, program.intent
        );
        write_or_check(
            &directory.join(format!("{}.core.cbor.hex", program.name)),
            &format!("{}\n", hex(&core)),
            check,
        )?;
    }
    write_or_check(&directory.join("minimums.txt"), &manifest, check)
}

fn write_or_check(path: &Path, contents: &str, check: bool) -> Result<(), String> {
    if check {
        let retained =
            std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if retained != contents {
            return Err(format!(
                "{} differs from the producer output",
                path.display()
            ));
        }
        return Ok(());
    }
    std::fs::write(path, contents).map_err(|e| format!("{}: {e}", path.display()))
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(directory) = args.next().map(PathBuf::from) else {
        eprintln!("usage: echo-edict-core-cost-v2-corpus <corpus-dir> [--check]");
        return ExitCode::FAILURE;
    };
    let check = match args.next().as_deref() {
        None => false,
        Some("--check") => true,
        Some(other) => {
            eprintln!("unknown argument {other}");
            return ExitCode::FAILURE;
        }
    };
    match run(&directory, check) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}
