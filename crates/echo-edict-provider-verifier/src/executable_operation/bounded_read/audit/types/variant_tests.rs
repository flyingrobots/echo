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
