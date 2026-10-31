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
fn read_closed_variants_keep_identity_and_check_helper_results() -> Check<()> {
    let core = core();
    let mut s = scope(&core);
    let expected = resolve(&mut s, "Choice")?;
    assert!(resolve(&mut s, "example@1.Choice")? == expected);
    assert!(resolve(&mut s, "Other")? != expected);
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
    assert!(resolve(&mut scope(&good), "Choice").is_ok());
    for definition in [
        definition("Choice"),
        m([("kind", t("Variant")), ("cases", m([]))]),
        m([("kind", t("Variant")), ("cases", m([("bad-case", m([]))]))]),
    ] {
        let bad = m([
            ("coordinate", t("example@1")),
            ("types", m([("Choice", definition)])),
        ]);
        assert!(resolve(&mut scope(&bad), "Choice").is_err());
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
    let mut limited = scope(&wide);
    set_work(&mut limited, 4);
    assert!(resolve(&mut limited, "Choice").is_err());
}
fn scope(core: &Value) -> Symbols<'_> {
    Symbols {
        core,
        exports: core,
        declared: BTreeMap::new(),
        live: BTreeSet::new(),
        claimed: BTreeSet::new(),
        work: 65_536,
    }
}
fn resolve(s: &mut Symbols<'_>, name: &str) -> Check<Schema> {
    s.schema(name, 0)
}
fn set_work(s: &mut Symbols<'_>, n: usize) {
    s.work = n;
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
    let mut s = scope(&core);
    assert!(matches!(s.expression(&matched(), 0), Ok(Schema::Word(255))));
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
        let mut s = scope(&core);
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
        assert!(s.live.is_empty());
    }
    let mut s = scope(&core);
    assert!(s.expression(&matched(), 0).is_ok());
    assert!(s.live.is_empty());
}
