// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Read-relation boundaries, independent of source-function body admission.
use super::*;
fn t(s: &str) -> Value {
    Value::Text(s.into())
}
fn m(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(fields.into_iter().map(|(k, v)| (t(k), v)).collect())
}
fn definition(payload: &str) -> Value {
    m([
        ("kind", t("Variant")),
        (
            "cases",
            m([("Empty", m([])), ("Data", m([("payload", t(payload))]))]),
        ),
    ])
}
fn core() -> Value {
    m([
        ("coordinate", t("example@1")),
        (
            "types",
            m([("Choice", definition("U8")), ("Other", definition("U8"))]),
        ),
        (
            "functions",
            m([(
                "make",
                m([
                    ("params", Value::Array(vec![])),
                    ("returnType", t("Choice")),
                ]),
            )]),
        ),
    ])
}
#[test]
fn read_closed_variants_keep_identity_and_check_helper_results() -> Result<(), ProviderRefusalV1> {
    let core = core();
    let s = scope(&core);
    let expected = resolve(&s, "Choice")?;
    assert!(resolve(&s, "example@1.Choice")? == expected);
    assert!(resolve(&s, "Other")? != expected);
    let call = m([
        ("kind", t("call")),
        ("callee", t("example@1.make")),
        ("args", Value::Array(vec![])),
        ("typeArgs", Value::Array(vec![])),
    ]);
    assert!(s.expression(&call, 0)? == expected);
    let projection = m([("kind", t("field")), ("base", call), ("field", t("type"))]);
    assert!(s.expression(&projection, 0).is_err());
    Ok(())
}
#[test]
fn read_closed_variants_charge_and_bound_the_entire_type_graph() {
    let good = core();
    assert!(resolve(&scope(&good), "Choice").is_ok());
    for definition in [
        definition("Choice"),
        m([("kind", t("Variant")), ("cases", m([]))]),
        m([("kind", t("Variant")), ("cases", m([("bad-case", m([]))]))]),
    ] {
        let bad = m([
            ("coordinate", t("example@1")),
            ("types", m([("Choice", definition)])),
        ]);
        assert!(resolve(&scope(&bad), "Choice").is_err());
    }
    let wide = m([
        ("coordinate", t("example@1")),
        (
            "types",
            m([(
                "Choice",
                m([
                    ("kind", t("Variant")),
                    (
                        "cases",
                        Value::Map((0..8).map(|n| (t(&format!("C{n}")), m([]))).collect()),
                    ),
                ]),
            )]),
        ),
    ]);
    let limited = scope(&wide);
    set_work(&limited, 4);
    assert!(resolve(&limited, "Choice").is_err());
}
fn scope(core: &Value) -> Scope<'_> {
    Scope {
        core,
        exports: core,
        inventory: BTreeMap::new(),
        available: BTreeMap::new(),
        used: BTreeSet::new(),
        input: core,
        remaining_type_work: Cell::new(65_536),
    }
}
fn resolve(s: &Scope<'_>, name: &str) -> Result<ReadType, ProviderRefusalV1> {
    s.ty(name)
}
fn set_work(s: &Scope<'_>, n: usize) {
    s.remaining_type_work.set(n);
}

fn matched() -> Value {
    let binder = m([
        ("id", t("payload.0")),
        ("alphaName", t("payload")),
        ("type", t("U8")),
    ]);
    m([
        ("kind", t("match")),
        (
            "scrutinee",
            m([
                ("kind", t("call")),
                ("callee", t("example@1.make")),
                ("args", Value::Array(vec![])),
                ("typeArgs", Value::Array(vec![])),
            ]),
        ),
        (
            "arms",
            Value::Array(vec![
                m([
                    ("case", t("Empty")),
                    (
                        "body",
                        m([
                            ("kind", t("const")),
                            (
                                "value",
                                m([
                                    ("kind", t("int")),
                                    ("width", t("U8")),
                                    ("value", Value::Integer(0)),
                                ]),
                            ),
                        ]),
                    ),
                ]),
                m([
                    ("case", t("Data")),
                    ("binder", binder.clone()),
                    ("body", m([("kind", t("local")), ("ref", binder)])),
                ]),
            ]),
        ),
    ])
}
#[test]
fn read_matches_bind_payloads_and_join_exhaustive_arms() {
    let core = core();
    let s = scope(&core);
    assert!(matches!(
        s.expression(&matched(), 0),
        Ok(ReadType::Unsigned(255))
    ));
}

fn field_mut<'a>(value: &'a mut Value, key: &str) -> &'a mut Value {
    let Value::Map(fields) = value else {
        unreachable!()
    };
    let Some((_, value)) = fields.iter_mut().find(|(name, _)| name == &t(key)) else {
        unreachable!()
    };
    value
}
#[test]
fn read_matches_reject_bad_arms_without_leaking_binders() {
    let core = core();
    for mutation in 0..6 {
        let s = scope(&core);
        let mut value = matched();
        let Value::Array(arms) = field_mut(&mut value, "arms") else {
            unreachable!()
        };
        match mutation {
            0 => {
                arms.pop();
            }
            1 => {
                *field_mut(&mut arms[1], "case") = t("Empty");
            }
            2 => {
                *field_mut(&mut arms[1], "case") = t("Unknown");
            }
            3 => {
                *field_mut(field_mut(&mut arms[1], "binder"), "type") = t("Bool");
            }
            4 => {
                let body = field_mut(&mut arms[1], "body").clone();
                *field_mut(&mut arms[0], "body") = body;
            }
            5 => {
                *field_mut(field_mut(field_mut(&mut arms[1], "body"), "ref"), "id") = t("missing");
            }
            _ => unreachable!(),
        }
        assert!(s.expression(&value, 0).is_err(), "mutation {mutation}");
        assert!(s.available.is_empty());
    }
    let s = scope(&core);
    assert!(s.expression(&matched(), 0).is_ok());
    assert!(s.available.is_empty());
}

#[test]
fn read_matches_carry_proof_work_and_preserve_available_locals() {
    let core = core();
    let outer = m([
        ("id", t("outer")),
        ("alphaName", t("outer")),
        ("type", t("U8")),
    ]);
    let mut value = matched();
    let Value::Array(arms) = field_mut(&mut value, "arms") else {
        unreachable!()
    };
    // The binder's inner work must reach the outer counter before Empty runs.
    arms.swap(0, 1);
    // Choice expansion: four units. Data: arm + binder type + scope copy +
    // local body type = four. Empty: arm + constant type = two. Total: ten.
    for budget in [9, 10, 20] {
        let mut s = scope(&core);
        s.available.insert("outer", &outer);
        let before = s.available.clone();
        set_work(&s, budget);
        assert_eq!(s.expression(&value, 0).is_ok(), budget >= 10);
        assert_eq!(s.remaining_type_work.get(), budget.saturating_sub(10));
        assert_eq!(s.available, before);
        assert_eq!(
            matches!(
                s.expression(&m([("kind", t("local")), ("ref", outer.clone())]), 0),
                Ok(ReadType::Unsigned(255))
            ),
            budget > 10
        );
    }

    let mut s = scope(&core);
    s.available.insert("outer", &outer);
    let before = s.available.clone();
    let Value::Array(arms) = field_mut(&mut value, "arms") else {
        unreachable!()
    };
    // Refuse only after the binder body has spent a type unit: scalar field
    // projection is invalid, but its local base must first resolve to U8.
    let body = field_mut(&mut arms[0], "body").clone();
    *field_mut(&mut arms[0], "body") = m([
        ("kind", t("field")),
        ("base", body),
        ("field", t("missing")),
    ]);
    set_work(&s, 20);
    assert!(s.expression(&value, 0).is_err());
    assert_eq!(s.remaining_type_work.get(), 12);
    assert_eq!(s.available, before);
}

#[test]
fn read_matches_refuse_capture_of_an_available_local() -> Result<(), ProviderRefusalV1> {
    let core = core();
    let value = matched();
    let Value::Array(arms) = field(&value, "arms")? else {
        unreachable!()
    };
    let binder = field(&arms[1], "binder")?;
    let mut s = scope(&core);
    s.available.insert("payload.0", binder);
    let before = s.available.clone();
    assert!(s.expression(&value, 0).is_err());
    assert_eq!(s.available, before);
    Ok(())
}
