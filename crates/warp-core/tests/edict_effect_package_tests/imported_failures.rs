// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Failures inside imported (lawpack-exported) pure helper bodies
//! (flyingrobots/echo#1006).
//!
//! An exported pure helper is total: Amblicit's lawpack builder refuses a
//! `fail` in its body ("a Core pure-expression discriminant"), so no compiler
//! route produces one. These forged packages put one there anyway and call
//! it from an intent; every judge must refuse each, or accept only the total
//! control.
use super::proven_ranges::{
    at_most, compiled_exports_with, declared, local, package_importing, Node,
};
use super::typed_failures::{boolean, branch, call, fail, int, UNDERFLOW};
use super::*;

const HELPER: &str = "lib@1.boom";

/// What the imported helper returns, and how the intent calls it.
#[derive(Clone, Copy)]
pub(super) enum Import {
    /// Returns `7`; a plain `let` calls it.
    Total,
    /// Fails `Underflow`; a plain `let` calls it.
    Fails,
    /// Fails `Underflow` in one `if` arm; a plain `let` calls it.
    FailsInArm,
    /// Fails `Underflow`; an `attempt` maps `Underflow` and the module
    /// declares `core.failure/v1`.
    FailsAttempted,
}

fn helper(import: Import) -> V {
    let result = match import {
        Import::Total => int(7),
        Import::Fails | Import::FailsAttempted => fail(UNDERFLOW),
        Import::FailsInArm => branch(at_most(int(1), int(0)), int(7), fail(UNDERFLOW)),
    };
    map([
        ("coordinate", text(HELPER)),
        ("typeParameters", V::Array(vec![])),
        ("parameterTypes", V::Array(vec![])),
        ("returnType", text("U64")),
        ("costTemplate", text("lib@1.cost")),
        ("determinismClass", text("total")),
        ("source", text("edict")),
        (
            "body",
            map([
                ("params", V::Array(vec![])),
                (
                    "body",
                    map([
                        ("locals", V::Array(vec![])),
                        ("bindings", V::Array(vec![])),
                        ("result", result),
                    ]),
                ),
            ]),
        ),
    ])
}

/// The program calling the imported helper as `import` describes.
pub(super) fn imported_failure_program(import: Import) -> V {
    let rest = declared("local.rest", "U64");
    let value = call(HELPER, &[], vec![]);
    let node = match import {
        Import::FailsAttempted => Node::Attempt(
            rest.clone(),
            value,
            V::Map(vec![(
                text(UNDERFLOW),
                map([
                    ("reasonKind", text("app.RangeInverted")),
                    ("payload", map([])),
                ]),
            )]),
        ),
        _ => Node::Let(rest.clone(), value),
    };
    package_importing(
        vec![],
        vec![node],
        branch(
            at_most(local(&rest), int(64)),
            boolean(true),
            boolean(false),
        ),
        |core| {
            if matches!(import, Import::FailsAttempted) {
                set(
                    core,
                    "requiredCoreCapabilities",
                    V::Array(vec![text("core.failure/v1")]),
                );
            }
        },
        |_| {},
        compiled_exports_with(vec![helper(import)]),
    )
}
