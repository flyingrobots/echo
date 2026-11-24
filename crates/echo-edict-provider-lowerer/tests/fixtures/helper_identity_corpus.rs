// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Shared corpus for `echo.dpo@2` identity constructors called from source
//! helpers and fold steps (flyingrobots/echo#1005).
//!
//! The provider lowerer judges each case's compiled program, the provider
//! verifier judges its package, and warp-core admits or refuses the package;
//! all three must reach every case's verdict. The accepted cases are unchanged
//! public compiler output: `edict-helper-identity` calls `contentAddress` from
//! a helper and from a fold step, and `edict-content-address` calls all three
//! constructors from the intent body. Each forged case changes exactly one
//! fact of the helper package and recomputes every identity that fact feeds,
//! so the package is well bound and only the named defect remains:
//! - `wrong-catalog`: the lowering artifacts carry the frozen `echo.dpo@1`
//!   catalog instead of the one the `echo.dpo@2` profile pins;
//! - `substituted-intrinsic`: a helper calls `echo.dpo@1.contentAddress`,
//!   a name no catalog admits;
//! - `insufficient-budget`: the intent declares a one-step evaluation budget.
#![allow(
    clippy::panic,
    clippy::expect_used,
    reason = "a malformed retained fixture fails the test loudly"
)]
use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, digest_canonical_value_bytes_v1 as digest,
    encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value,
};

const HELPER_PACKAGE: &str =
    include_str!("../../../warp-core/tests/fixtures/edict-helper-identity/helper-executable-operation-package.hex");
const BODY_PACKAGE: &str =
    include_str!("../../../warp-core/tests/fixtures/edict-content-address/identity-executable-operation-package.hex");
/// The frozen `echo.dpo@1` catalog the `echo.dpo@2` profile does not pin.
const DPO_1_CATALOG: &[u8] = include_bytes!(
    "../../../../schemas/edict-provider/package/v1/generated/resources/resource.target-intrinsics.cbor"
);

/// One corpus entry: a package and the verdict all three judges must reach.
pub(crate) struct Case {
    pub(crate) name: &'static str,
    pub(crate) package: Vec<u8>,
    pub(crate) accepted: bool,
}

impl Case {
    /// The compiled program the package carries, which the lowerer judges.
    pub(crate) fn compiled_program(&self) -> Vec<u8> {
        let package = decode(&self.package).expect("canonical package");
        let program = decode(bytes(field(&package, "program"))).expect("canonical program");
        bytes(field(&program, "compiled_program")).to_vec()
    }
}

fn hex_bytes(text: &str) -> Vec<u8> {
    hex::decode(text.trim()).expect("retained hex")
}

fn field<'v>(value: &'v Value, name: &str) -> &'v Value {
    let Value::Map(entries) = value else {
        panic!("{name}: parent is not a map")
    };
    &entries
        .iter()
        .find(|(key, _)| key == &Value::Text(name.to_owned()))
        .unwrap_or_else(|| panic!("{name}: absent"))
        .1
}

fn field_mut<'v>(value: &'v mut Value, name: &str) -> &'v mut Value {
    let Value::Map(entries) = value else {
        panic!("{name}: parent is not a map")
    };
    &mut entries
        .iter_mut()
        .find(|(key, _)| key == &Value::Text(name.to_owned()))
        .unwrap_or_else(|| panic!("{name}: absent"))
        .1
}

fn bytes(value: &Value) -> &[u8] {
    let Value::Bytes(bytes) = value else {
        panic!("not bytes")
    };
    bytes
}

fn identity(domain: &str, value: &Value) -> Value {
    Value::Bytes(digest(domain, value).expect("identity").to_vec())
}

/// Rewrites the helper package's Core and lowering artifacts, then rebinds
/// the Target IR, the program and the package to the changed Core.
fn forged(edit: impl FnOnce(&mut Value, &mut Value)) -> Vec<u8> {
    let mut package = decode(&hex_bytes(HELPER_PACKAGE)).expect("package");
    let mut program = decode(bytes(field(&package, "program"))).expect("program");
    let mut bundle = decode(bytes(field(&program, "compiled_program"))).expect("bundle");
    let mut core = decode(bytes(field(&bundle, "core_artifact"))).expect("core");
    let mut target = decode(bytes(field(&bundle, "target_ir_artifact"))).expect("target");
    edit(&mut core, &mut bundle);
    let core_identity = identity("edict.core.module/v1", &core);
    let source_core = field_mut(field_mut(&mut target, "semanticClosure"), "sourceCore");
    let Value::Array(source) = field_mut(source_core, "digest") else {
        panic!("Target IR source digest")
    };
    *source.last_mut().expect("digest bytes") = core_identity.clone();
    let target_identity = identity("edict.target-ir.artifact/v1", &target);
    *field_mut(&mut bundle, "core_artifact") = Value::Bytes(encode(&core).expect("core"));
    *field_mut(&mut bundle, "target_ir_artifact") = Value::Bytes(encode(&target).expect("target"));
    *field_mut(&mut program, "compiled_program") = Value::Bytes(encode(&bundle).expect("bundle"));
    *field_mut(&mut program, "target_ir_identity") = target_identity.clone();
    *field_mut(&mut package, "program") = Value::Bytes(encode(&program).expect("program"));
    let closure = field_mut(&mut package, "semantic_closure");
    *field_mut(closure, "core_identity") = core_identity.clone();
    *field_mut(closure, "canonical_meaning_identity") = core_identity;
    *field_mut(closure, "target_ir_identity") = target_identity;
    encode(&package).expect("package")
}

/// Replaces every `from` text in `value` with `to`; returns how many.
fn rename(value: &mut Value, from: &str, to: &str) -> usize {
    match value {
        Value::Text(text) if text == from => {
            to.clone_into(text);
            1
        }
        Value::Array(items) => items.iter_mut().map(|item| rename(item, from, to)).sum(),
        Value::Map(entries) => entries
            .iter_mut()
            .map(|(_, item)| rename(item, from, to))
            .sum(),
        _ => 0,
    }
}

/// Every case, accepted and forged.
pub(crate) fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "helper-and-fold-step",
            package: hex_bytes(HELPER_PACKAGE),
            accepted: true,
        },
        Case {
            name: "intent-body",
            package: hex_bytes(BODY_PACKAGE),
            accepted: true,
        },
        Case {
            name: "wrong-catalog",
            package: forged(|_, bundle| {
                let lowering = field_mut(bundle, "lowering_artifacts");
                *field_mut(lowering, "intrinsics_artifact") = Value::Bytes(DPO_1_CATALOG.to_vec());
            }),
            accepted: false,
        },
        Case {
            name: "substituted-intrinsic",
            package: forged(|core, _| {
                let functions = field_mut(core, "functions");
                let renamed = rename(
                    functions,
                    "echo.dpo@2.contentAddress",
                    "echo.dpo@1.contentAddress",
                );
                assert!(renamed > 0, "the helpers call contentAddress");
            }),
            accepted: false,
        },
        Case {
            name: "insufficient-budget",
            package: forged(|core, _| {
                let Value::Map(intents) = field_mut(core, "intents") else {
                    panic!("intents")
                };
                let budget = field_mut(&mut intents[0].1, "coreEvaluationBudget");
                *field_mut(budget, "maxSteps") = Value::Integer(1);
            }),
            accepted: false,
        },
    ]
}
