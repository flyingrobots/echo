// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
// Shared inputs only: runtime, lowerer and verifier own their cost judgments.
use echo_edict_canonical::{CanonicalValueError, CanonicalValueV1 as Value};

pub struct Fixture {
    pub core: Value,
    pub expression: Value,
    pub declarations: Vec<Value>,
    pub values: Vec<Value>,
}
fn text(s: &str) -> Value {
    Value::Text(s.into())
}
fn map(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(fields.into_iter().map(|(k, v)| (text(k), v)).collect())
}
fn integer(n: u64) -> Value {
    map([
        ("kind", text("const")),
        (
            "value",
            map([
                ("kind", text("int")),
                ("width", text("U8")),
                ("value", Value::Integer(n.into())),
            ]),
        ),
    ])
}
fn local(id: &str, ty: &str) -> Value {
    map([
        ("id", text(id)),
        ("alphaName", text(id)),
        ("type", text(ty)),
    ])
}
pub fn fixture(binder: bool, selected: usize) -> Fixture {
    assert!(selected < 2);
    let mut arms: Vec<_> = ["Data", "More"]
        .into_iter()
        .map(|case| {
            let mut fields = vec![("case", text(case)), ("body", integer(9))];
            if binder {
                fields.push(("binder", local("payload", "U8")));
            }
            map(fields)
        })
        .collect();
    if selected == 1 {
        arms.swap(0, 1);
    }
    Fixture {
        core: map([
            ("coordinate", text("choices.example@1")),
            (
                "types",
                map([
                    (
                        "Choice",
                        map([
                            ("kind", text("Variant")),
                            (
                                "cases",
                                map([
                                    ("Data", map([("payload", text("U8"))])),
                                    ("More", map([("payload", text("U8"))])),
                                ]),
                            ),
                        ]),
                    ),
                    (
                        "Flags",
                        map([
                            ("kind", text("Record")),
                            ("fields", map([("flag", text("Bool"))])),
                        ]),
                    ),
                ]),
            ),
            ("functions", map([])),
            ("intents", map([])),
        ]),
        expression: map([
            ("kind", text("match")),
            (
                "scrutinee",
                map([
                    ("kind", text("variant")),
                    ("type", text("Choice")),
                    ("case", text("Data")),
                    ("payload", integer(37)),
                ]),
            ),
            ("arms", Value::Array(arms)),
        ]),
        declarations: vec![
            local("word", "U8"),
            local("bytes", "Bytes<exact=4>"),
            local("record", "Flags"),
            local("choice", "Choice"),
        ],
        values: vec![
            Value::Integer(7),
            Value::Bytes(vec![1, 2, 3, 4]),
            map([("flag", Value::Bool(true))]),
            map([
                ("type", text("choices.example@1.Choice")),
                ("case", text("Data")),
                ("payload", Value::Integer(5)),
            ]),
        ],
    }
}
impl Fixture {
    // Optional bounded evidence output for the guarded differential runner.
    // Ordinary unit-test runs have no filesystem side effects.
    pub fn report(
        &self,
        layer: &str,
        binder: bool,
        selected: usize,
        cost: (u64, u64),
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(directory) = std::env::var_os("ECHO_MATCH_FRAME_REPORT") {
            let path = std::path::PathBuf::from(directory)
                .join(format!("{layer}-{binder}-{selected}.txt"));
            let hash = self.fingerprint()?;
            std::fs::write(
                path,
                format!(
                    "MATCH_FRAME {layer} {binder} {selected} {hash} {} {}\n",
                    cost.0, cost.1
                ),
            )?;
        }
        Ok(())
    }
    pub fn fingerprint(&self) -> Result<String, CanonicalValueError> {
        echo_edict_canonical::digest_canonical_value_v1(
            "echo.test.match-frame/v1",
            &map([
                ("core", self.core.clone()),
                ("expression", self.expression.clone()),
                ("declarations", Value::Array(self.declarations.clone())),
                ("values", Value::Array(self.values.clone())),
            ]),
        )
    }
}

// Independent cell arithmetic for the four live values:
// U8 1/64; Bytes(4) 1/68; {flag: Bool} 3/196;
// Data(U8) 7/(7*64 + 15 key bytes + 24 identity bytes + 4 case bytes).
// Total frame copy: 12 steps, 819 bytes. Both case names have four bytes,
// and both carry U8, so the static maximum equals this concrete frame.
pub const FRAME_COPY: (u64, u64) = (12, 819);
// Match entry 1; constructor 3 + payload constant 2; dispatch 1 or 2;
// selected body constant 2. Allocations: 427 metadata + 64 payload + 64 result.
pub fn runtime_cost(binder: bool, selected: usize) -> (u64, u64) {
    (
        9 + selected as u64 + if binder { FRAME_COPY.0 } else { 0 },
        555 + if binder { FRAME_COPY.1 } else { 0 },
    )
}
