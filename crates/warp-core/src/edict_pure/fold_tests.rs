// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Synthetic fold judgments; public compiler provenance is a separate witness.
use echo_edict_canonical::CanonicalValueV1 as Value;
fn text(s: &str) -> Value {
    Value::Text(s.into())
}
fn map(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(fields.into_iter().map(|(k, v)| (text(k), v)).collect())
}
fn boolean(b: bool) -> Value {
    map([
        ("kind", text("const")),
        (
            "value",
            map([("kind", text("bool")), ("value", Value::Bool(b))]),
        ),
    ])
}
fn function(params: Vec<Value>, result: Value) -> Value {
    map([
        ("params", Value::Array(params)),
        ("returnType", text("Bool")),
        (
            "body",
            map([
                ("locals", Value::Array(vec![])),
                ("bindings", Value::Array(vec![])),
                ("result", result),
            ]),
        ),
    ])
}
fn fixture(bound: Option<u64>) -> (Value, Value) {
    let parameter = map([
        ("id", text("param.0")),
        ("type", text("Bool")),
        ("alphaName", text("state")),
    ]);
    let value = map([("kind", text("local")), ("ref", parameter.clone())]);
    let toggle = map([
        ("kind", text("if")),
        (
            "predicate",
            map([
                ("kind", text("compare")),
                ("op", text("==")),
                ("left", value),
                ("right", boolean(true)),
            ]),
        ),
        ("then", boolean(false)),
        ("else", boolean(true)),
    ]);
    let result = bound.map_or_else(
        || boolean(true),
        |bound| {
            map([
                ("kind", text("fold")),
                ("initial", boolean(true)),
                ("step", text("iteration.example@1.toggle")),
                ("bound", Value::Integer(bound.into())),
                ("stateType", text("Bool")),
            ])
        },
    );
    (
        map([
            ("coordinate", text("iteration.example@1")),
            ("types", map([])),
            ("intents", map([])),
            (
                "requiredCoreCapabilities",
                Value::Array(vec![text("core.fold/v1")]),
            ),
            (
                "functions",
                map([
                    ("toggle", function(vec![parameter], toggle)),
                    ("make", function(vec![], result)),
                ]),
            ),
        ]),
        map([
            ("effects", Value::Array(vec![])),
            ("pureFunctions", Value::Array(vec![])),
        ]),
    )
}

use super::{
    evaluate::{expression, Meter},
    functions,
    model::{Expr, MAX_PROGRAM_NODES},
    syntax::Parser,
    values::field,
    EvaluationLimits,
};
use std::collections::BTreeMap;
#[test]
fn source_fold_executes_exact_non_idempotent_iteration_count() {
    for (bound, expected) in [
        (None, true),
        (Some(0), true),
        (Some(1), false),
        (Some(2), true),
        (Some(3), false),
    ] {
        let (core, exports) = fixture(bound);
        let mut parser = Parser {
            types: field(&core, "types").expect("valid test fixture"),
            coordinate: "iteration.example@1",
            remaining: MAX_PROGRAM_NODES,
        };
        let helpers =
            functions::decode(&core, &exports, &mut parser).expect("fold helper admission");
        functions::check_roots(&helpers, [], []).expect("valid test fixture");
        let mut meter = Meter::new(EvaluationLimits {
            max_package_bytes: 1048576,
            max_input_bytes: 1048576,
            max_steps: 65536,
            max_allocated_bytes: 1048576,
            max_output_bytes: 1048576,
        });
        let actual = expression(
            &Expr::Call("iteration.example@1.make".into(), vec![]),
            &BTreeMap::new(),
            &helpers,
            &mut meter,
            0,
        )
        .expect("valid test fixture");
        assert_eq!(actual, Value::Bool(expected), "bound {bound:?}");
    }
}

fn get_mut<'a>(value: &'a mut Value, key: &str) -> &'a mut Value {
    let Value::Map(fields) = value else {
        unreachable!("fixture is a map")
    };
    let Some((_, value)) = fields.iter_mut().find(|(k, _)| k == &text(key)) else {
        unreachable!("fixture key exists")
    };
    value
}
fn folded(core: &mut Value) -> &mut Value {
    get_mut(
        get_mut(get_mut(get_mut(core, "functions"), "make"), "body"),
        "result",
    )
}
fn malformed_folds() -> Vec<(&'static str, Value, Value)> {
    let mut cases = Vec::new();
    for (name, key, value) in [
        ("missing step", "step", text("iteration.example@1.missing")),
        ("opaque intrinsic", "step", text("core.bytes.length")),
        ("negative bound", "bound", Value::Integer(-1)),
        ("noninteger bound", "bound", Value::Bool(false)),
        ("wrong invariant type", "stateType", text("U64")),
        (
            "wrong initial type",
            "initial",
            map([
                ("kind", text("const")),
                (
                    "value",
                    map([
                        ("kind", text("int")),
                        ("width", text("U64")),
                        ("value", Value::Integer(0)),
                    ]),
                ),
            ]),
        ),
        (
            "initial cannot see step parameter",
            "initial",
            map([
                ("kind", text("local")),
                (
                    "ref",
                    map([
                        ("id", text("param.0")),
                        ("type", text("Bool")),
                        ("alphaName", text("state")),
                    ]),
                ),
            ]),
        ),
    ] {
        let (mut core, exports) = fixture(Some(0));
        *get_mut(folded(&mut core), key) = value;
        cases.push((name, core, exports));
    }
    let (mut core, exports) = fixture(Some(0));
    *get_mut(&mut core, "requiredCoreCapabilities") = Value::Array(vec![]);
    cases.push(("missing fold capability", core, exports));
    let (mut core, exports) = fixture(Some(0));
    let cycle = folded(&mut core).clone();
    *get_mut(
        get_mut(get_mut(get_mut(&mut core, "functions"), "toggle"), "body"),
        "result",
    ) = cycle;
    cases.push(("cycle at bound zero", core, exports));
    let (mut core, exports) = fixture(Some(0));
    *get_mut(&mut core, "types") = map([("Flag", map([("kind", text("Bool"))]))]);
    *get_mut(
        get_mut(get_mut(&mut core, "functions"), "toggle"),
        "returnType",
    ) = text("Flag");
    cases.push(("return alias is not the declared invariant", core, exports));
    cases
}

fn checked(
    core: &Value,
    exports: &Value,
) -> Result<BTreeMap<String, super::model::Helper>, super::EvaluationError> {
    let mut parser = Parser {
        types: field(core, "types")?,
        coordinate: "iteration.example@1",
        remaining: MAX_PROGRAM_NODES,
    };
    let helpers = functions::decode(core, exports, &mut parser)?;
    functions::check_roots(&helpers, [], [])?;
    Ok(helpers)
}
#[test]
fn source_fold_refuses_invalid_authority_scope_type_and_zero_count_cycles() {
    for (name, core, exports) in malformed_folds() {
        assert!(checked(&core, &exports).is_err(), "{name}");
    }
}
#[test]
fn source_fold_meters_each_iteration_and_stops_at_the_exact_budget() {
    for bound in [0, 1, 2, 3, 65] {
        let (core, exports) = fixture(Some(bound));
        let helpers = checked(&core, &exports).expect("valid test fixture");
        // Six base steps: outer call, fold, initial expression/copy/check,
        // outer result check. Each toggle adds eleven: iteration+argument
        // check, if+comparison, three expression/copy pairs, result check.
        let steps = 6 + 11 * bound;
        let allocated = 64 + 192 * bound; // initial Bool plus three copied Bools/step.
        let limits = EvaluationLimits {
            max_package_bytes: 1048576,
            max_input_bytes: 1048576,
            max_steps: steps,
            max_allocated_bytes: allocated,
            max_output_bytes: 1048576,
        };
        let call = Expr::Call("iteration.example@1.make".into(), vec![]);
        let mut meter = Meter::new(limits);
        assert_eq!(
            expression(&call, &BTreeMap::new(), &helpers, &mut meter, 0)
                .expect("valid test fixture"),
            Value::Bool(bound % 2 == 0)
        );
        assert_eq!(meter.usage(), (steps, allocated));
        let mut meter = Meter::new(EvaluationLimits {
            max_steps: steps - 1,
            ..limits
        });
        assert_eq!(
            expression(&call, &BTreeMap::new(), &helpers, &mut meter, 0),
            Err(super::EvaluationError::StepBudgetExceeded)
        );
        let mut meter = Meter::new(EvaluationLimits {
            max_allocated_bytes: allocated - 1,
            ..limits
        });
        assert_eq!(
            expression(&call, &BTreeMap::new(), &helpers, &mut meter, 0),
            Err(super::EvaluationError::AllocationBudgetExceeded)
        );
    }
}

fn nested_fixture(outer: u64, inner: u64) -> (Value, Value) {
    let (mut core, exports) = fixture(Some(outer));
    let toggle = get_mut(get_mut(&mut core, "functions"), "toggle").clone();
    let mut inner_fold = folded(&mut core).clone();
    *get_mut(&mut inner_fold, "step") = text("iteration.example@1.inner");
    *get_mut(&mut inner_fold, "bound") = Value::Integer(inner.into());
    *get_mut(&mut inner_fold, "initial") = map([
        ("kind", text("local")),
        (
            "ref",
            map([
                ("id", text("param.0")),
                ("type", text("Bool")),
                ("alphaName", text("state")),
            ]),
        ),
    ]);
    *get_mut(
        get_mut(get_mut(get_mut(&mut core, "functions"), "toggle"), "body"),
        "result",
    ) = inner_fold;
    let Value::Map(functions) = get_mut(&mut core, "functions") else {
        unreachable!()
    };
    functions.push((text("inner"), toggle));
    (core, exports)
}

#[test]
fn source_fold_nested_iterations_share_no_state_frame_and_meter_multiplicatively() {
    let (core, exports) = nested_fixture(3, 2);
    let helpers = checked(&core, &exports).expect("valid test fixture");
    // Six base steps + three outer iterations of (two admission steps +
    // nested fold, initial copy/check, two toggle invocations, result check).
    let mut meter = Meter::new(EvaluationLimits {
        max_package_bytes: 1048576,
        max_input_bytes: 1048576,
        max_steps: 93,
        max_allocated_bytes: 1408,
        max_output_bytes: 1048576,
    });
    assert_eq!(
        expression(
            &Expr::Call("iteration.example@1.make".into(), vec![]),
            &BTreeMap::new(),
            &helpers,
            &mut meter,
            0
        )
        .expect("valid test fixture"),
        Value::Bool(true)
    );
    assert_eq!(meter.usage(), (93, 1408));
}

#[test]
fn source_fold_main_expression_requires_the_retained_capability() {
    let (mut core, exports) = fixture(None);
    *get_mut(&mut core, "requiredCoreCapabilities") = Value::Array(vec![]);
    let helpers = checked(&core, &exports).expect("non-fold helpers remain valid");
    let expr = Expr::Fold {
        initial: Box::new(Expr::Constant(Value::Bool(true))),
        step: "iteration.example@1.toggle".into(),
        bound: 0,
        state: super::model::RuntimeType::Bool,
    };
    let mut meter = Meter::new(EvaluationLimits {
        max_package_bytes: 1048576,
        max_input_bytes: 1048576,
        max_steps: 100,
        max_allocated_bytes: 1024,
        max_output_bytes: 1024,
    });
    let checked = functions::check_roots(&helpers, [&expr], []).is_err();
    let executed = expression(&expr, &BTreeMap::new(), &helpers, &mut meter, 0).is_err();
    assert!(
        checked && executed,
        "missing capability: static refused={checked}, runtime refused={executed}"
    );
}
