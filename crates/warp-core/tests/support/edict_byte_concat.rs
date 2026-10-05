// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, digest_canonical_value_bytes_v1 as digest,
    encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value,
};
use warp_core::edict_pure::EvaluationLimits;

pub fn package() -> Vec<u8> {
    hex::decode(
        include_str!("../fixtures/edict-byte-concat/executable-operation-package.cbor.hex").trim(),
    )
    .unwrap()
}

pub fn pin() -> [u8; 32] {
    hex::decode("ed480a332a9d5f84f7425e46ab0b08cca886c8a040e048145b88fba1e6e0415f")
        .unwrap()
        .try_into()
        .unwrap()
}

pub fn limits() -> EvaluationLimits {
    EvaluationLimits {
        max_package_bytes: 32_768,
        max_input_bytes: 2_097_152,
        max_steps: 10_000,
        max_allocated_bytes: 16_777_216,
        max_output_bytes: 2_097_152,
    }
}

pub fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        fields
            .into_iter()
            .map(|(key, value)| (Value::Text(key.into()), value))
            .collect(),
    )
}

pub fn input(left: &[u8], right: &[u8]) -> Vec<u8> {
    encode(&record([
        ("firstFragment", Value::Bytes(left.into())),
        ("secondFragment", Value::Bytes(right.into())),
    ]))
    .unwrap()
}

pub fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    let Value::Map(fields) = value else {
        panic!("map")
    };
    &fields
        .iter()
        .find(|(key, _)| key == &Value::Text(name.into()))
        .unwrap()
        .1
}

pub fn field_mut<'a>(value: &'a mut Value, name: &str) -> &'a mut Value {
    let Value::Map(fields) = value else {
        panic!("map")
    };
    &mut fields
        .iter_mut()
        .find(|(key, _)| key == &Value::Text(name.into()))
        .unwrap()
        .1
}

pub fn set_field(value: &mut Value, name: &str, replacement: Value) {
    *field_mut(value, name) = replacement;
}

// Deliberately matching test-host pins exercise defense in depth, not verification.
pub fn mutate(change: impl FnOnce(&mut Value)) -> (Vec<u8>, [u8; 32]) {
    rewrite(|_, _, target| {
        let intent = field_mut(field_mut(target, "intents"), "assembleRange");
        let Value::Array(bindings) = field_mut(intent, "pureBindings") else { panic!("bindings") };
        change(field_mut(&mut bindings[0], "value"));
    })
}

pub fn narrow_binding() -> (Vec<u8>, [u8; 32]) {
    rewrite(|_, _, target| {
        let intent = field_mut(field_mut(target, "intents"), "assembleRange");
        let Value::Array(bindings) = field_mut(intent, "pureBindings") else { panic!("bindings") };
        set_field(field_mut(&mut bindings[0], "binding"), "type", Value::Text("Bytes<max=4>".into()));
    })
}

pub fn budget(package_key: &str, core_key: &str, value: u64) -> (Vec<u8>, [u8; 32]) {
    rewrite(|artifact, core, target| {
        set_field(field_mut(artifact, "budget_ceiling"), package_key, Value::Integer(value.into()));
        for artifact in [core, target] {
            let intent = field_mut(field_mut(artifact, "intents"), "assembleRange");
            set_field(field_mut(intent, "coreEvaluationBudget"), core_key, Value::Integer(value.into()));
        }
    })
}

fn rewrite(change: impl FnOnce(&mut Value, &mut Value, &mut Value)) -> (Vec<u8>, [u8; 32]) {
    let mut artifact = decode(&package()).unwrap();
    let Value::Bytes(bytes) = field(&artifact, "program") else { panic!("program") };
    let mut program = decode(bytes).unwrap();
    let Value::Bytes(bytes) = field(&program, "core_artifact") else { panic!("core") };
    let mut core = decode(bytes).unwrap();
    let Value::Bytes(bytes) = field(&program, "target_ir_artifact") else { panic!("target") };
    let mut target = decode(bytes).unwrap();
    change(&mut artifact, &mut core, &mut target);
    for (key, closure_key, domain, value) in [
        ("core_artifact", "core_identity", "edict.core.module/v1", core),
        ("target_ir_artifact", "target_ir_identity", "edict.target-ir.artifact/v1", target),
    ] {
        set_field(&mut program, key, Value::Bytes(encode(&value).unwrap()));
        set_field(field_mut(&mut artifact, "semantic_closure"), closure_key, Value::Bytes(digest(domain, &value).unwrap().into()));
    }
    set_field(&mut artifact, "program", Value::Bytes(encode(&program).unwrap()));
    let pin = digest("echo.operation-package/v1", &artifact).unwrap();
    (encode(&artifact).unwrap(), pin)
}

pub fn integer(value: i128, width: &str) -> Value {
    record([
        ("kind", Value::Text("const".into())),
        (
            "value",
            record([
                ("kind", Value::Text("int".into())),
                ("width", Value::Text(width.into())),
                ("value", Value::Integer(value)),
            ]),
        ),
    ])
}

// A test-only alpha-renaming control, not a newly verified compiler artifact.
pub fn neutral_package() -> (Vec<u8>, [u8; 32]) {
    let mut artifact = decode(&package()).unwrap();
    let Value::Bytes(bytes) = field(&artifact, "program") else {
        panic!("program")
    };
    let mut program = decode(bytes).unwrap();
    for (key, closure_key, domain) in [
        (
            "core_artifact",
            Some("core_identity"),
            "edict.core.module/v1",
        ),
        (
            "target_ir_artifact",
            Some("target_ir_identity"),
            "edict.target-ir.artifact/v1",
        ),
        (
            "lawpack_exports_artifact",
            Some("application_schema_identity"),
            "edict.lawpack-exports/v1",
        ),
        ("result_projection_artifact", None, ""),
    ] {
        let Value::Bytes(bytes) = field(&program, key) else {
            panic!("embedded")
        };
        let mut embedded = decode(bytes).unwrap();
        rename(&mut embedded);
        if let Some(closure_key) = closure_key {
            set_field(
                field_mut(&mut artifact, "semantic_closure"),
                closure_key,
                Value::Bytes(digest(domain, &embedded).unwrap().into()),
            );
        }
        set_field(&mut program, key, Value::Bytes(encode(&embedded).unwrap()));
    }
    rename(&mut program);
    set_field(
        &mut artifact,
        "program",
        Value::Bytes(encode(&program).unwrap()),
    );
    rename(&mut artifact);
    let pin = digest("echo.operation-package/v1", &artifact).unwrap();
    (encode(&artifact).unwrap(), pin)
}

pub fn rename(value: &mut Value) {
    match value {
        Value::Text(text) => {
            for (old, new) in [
                ("jedit.text.replace_range", "demo.codec.window_range"),
                ("assembleRange", "appendData"),
                ("RangeBytes", "DataBlock"),
                ("firstFragment", "sourceBytesAA"),
                ("secondFragment", "sourceBytesBBB"),
            ] {
                *text = text.replace(old, new);
            }
        }
        Value::Array(values) => values.iter_mut().for_each(rename),
        Value::Map(fields) => {
            for (key, value) in fields {
                rename(key);
                rename(value);
            }
        }
        _ => {}
    }
}
