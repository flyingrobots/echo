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
