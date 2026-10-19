// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Canonical publication fixtures for package and runtime-owner admission tests.
#![allow(dead_code)]
use echo_edict_canonical::{encode_canonical_cbor_v1 as encode, CanonicalValueV1 as V};
use warp_core::edict_effects::EffectPackageError;
use warp_core::ExecutableOperationPackage;
pub fn map(fields: impl IntoIterator<Item = (&'static str, V)>) -> V {
    V::Map(
        fields
            .into_iter()
            .map(|(k, v)| (V::Text(k.into()), v))
            .collect(),
    )
}
pub fn text(s: &str) -> V {
    V::Text(s.into())
}
pub fn id(n: u8) -> V {
    V::Bytes(vec![n; 32])
}
pub fn encoded(v: &V) -> Vec<u8> {
    encode(v).expect("fixture")
}
pub fn hash(domain: &[u8], data: &[u8]) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(domain);
    h.update(&(data.len() as u64).to_le_bytes());
    h.update(data);
    h.finalize().into()
}
pub fn profile(label: &str) -> V {
    V::Bytes(hash(b"echo:operation-profile:v1\0", label.as_bytes()).to_vec())
}
pub fn graph() -> V {
    map([
        (
            "node_types",
            V::Map(vec![(
                id(1),
                map([
                    ("addressing", V::Integer(1)),
                    ("atom_types", V::Array(vec![id(2)])),
                ]),
            )]),
        ),
        ("edge_types", V::Map(vec![])),
    ])
}
pub fn law() -> V {
    map([
        ("read_node_types", V::Array(vec![id(1)])),
        ("write_node_types", V::Array(vec![id(1)])),
        ("atom_types", V::Array(vec![id(2)])),
        ("range_edge_types", V::Array(vec![])),
        ("write_edge_types", V::Array(vec![])),
        ("portal_owner_types", V::Array(vec![])),
        ("max_reads", V::Integer(1)),
        ("max_writes", V::Integer(1)),
        ("max_range_edges", V::Integer(0)),
    ])
}
pub fn footprint(v: &V) -> V {
    let mut h = blake3::Hasher::new();
    h.update(b"echo:operation-footprint-law:v1\0");
    h.update(&encoded(v));
    V::Bytes(h.finalize().as_bytes().to_vec())
}
pub fn compiled_core() -> V {
    map([
        ("apiVersion", text("edict.core/v1")),
        ("coordinate", text("app")),
        ("imports", V::Array(vec![])),
        ("types", map([("Result", map([("kind", text("U32"))]))])),
        (
            "intents",
            map([(
                "update@1",
                map([
                    ("input", text("U32")),
                    ("output", text("app.Result")),
                    (
                        "requiredOperationProfile",
                        text("continuum.profile.total/v1"),
                    ),
                    ("inputConstraints", V::Array(vec![])),
                    (
                        "coreEvaluationBudget",
                        map([
                            ("maxSteps", V::Integer(1)),
                            ("maxAllocatedBytes", V::Integer(16 * 1024 * 1024)),
                            ("maxOutputBytes", V::Integer(65536)),
                        ]),
                    ),
                    (
                        "body",
                        map([
                            (
                                "locals",
                                V::Array(vec![map([
                                    ("id", text("arg.0")),
                                    ("type", text("U32")),
                                    ("alphaName", text("$arg0")),
                                ])]),
                            ),
                            ("nodes", V::Array(vec![])),
                            (
                                "result",
                                map([
                                    ("kind", text("const")),
                                    (
                                        "value",
                                        map([
                                            ("kind", text("int")),
                                            ("width", text("U32")),
                                            ("value", V::Integer(42)),
                                        ]),
                                    ),
                                ]),
                            ),
                        ]),
                    ),
                ]),
            )]),
        ),
        ("requiredCoreCapabilities", V::Array(vec![])),
    ])
}
pub fn get<'a>(v: &'a V, key: &str) -> &'a V {
    let V::Map(fields) = v else { panic!("map") };
    &fields
        .iter()
        .find(|(name, _)| name == &text(key))
        .expect("field")
        .1
}
pub fn artifact_identity(domain: &str, v: &V) -> V {
    V::Bytes(
        echo_edict_canonical::digest_canonical_value_bytes_v1(domain, v)
            .expect("artifact")
            .to_vec(),
    )
}
pub fn compiled_target(core: &V) -> V {
    let coordinate = get(core, "coordinate").clone();
    let intent = get(get(core, "intents"), "update@1");
    map([
        ("domain", text("echo.span-ir/v2")),
        ("kind", text("orderedTargetIrArtifact")),
        ("sourceCoreCoordinate", coordinate.clone()),
        (
            "targetProfile",
            map([
                ("id", text("echo.dpo@1")),
                ("digest", V::Array(vec![text("sha256"), id(6)])),
            ]),
        ),
        (
            "semanticClosure",
            map([
                (
                    "sourceCore",
                    map([
                        ("id", coordinate),
                        (
                            "digest",
                            V::Array(vec![
                                text("sha256"),
                                artifact_identity("edict.core.module/v1", core),
                            ]),
                        ),
                    ]),
                ),
                ("lawpacks", V::Array(vec![])),
            ]),
        ),
        (
            "intents",
            map([(
                "update@1",
                map([
                    ("steps", V::Array(vec![])),
                    ("requirements", V::Array(vec![])),
                    ("pureBindings", V::Array(vec![])),
                    ("executionOrder", V::Array(vec![])),
                    ("result", get(get(intent, "body"), "result").clone()),
                    ("inputConstraints", get(intent, "inputConstraints").clone()),
                    (
                        "coreEvaluationBudget",
                        get(intent, "coreEvaluationBudget").clone(),
                    ),
                    (
                        "operationProfile",
                        get(intent, "requiredOperationProfile").clone(),
                    ),
                ]),
            )]),
        ),
    ])
}
pub fn compiled_exports() -> V {
    map([
        ("types", V::Array(vec![])),
        ("effects", V::Array(vec![])),
        ("constants", V::Array(vec![])),
        ("obstructions", V::Array(vec![])),
        ("pureFunctions", V::Array(vec![])),
        ("operationProfiles", map([])),
    ])
}
pub fn compiled_bundle(core: &V, target: &V, exports: &V, intent: &str) -> V {
    let has_effects = matches!(get(exports, "effects"), V::Array(effects) if !effects.is_empty());
    let lowering = if has_effects {
        repinned_lowering_artifacts()
    } else {
        V::Null
    };
    map([
        ("lowering_artifacts", lowering),
        ("schema", text("echo.compiler-produced-effect-program/v1")),
        ("intent", text(intent)),
        ("core_artifact", V::Bytes(encoded(core))),
        ("target_ir_artifact", V::Bytes(encoded(target))),
        ("lawpack_exports_artifact", V::Bytes(encoded(exports))),
    ])
}
// This is a synthetic, internally repinned test chain built from the retained
// public capture. It is not evidence of a successful compiler/verifier run.
pub fn repinned_lowering_artifacts() -> V {
    lowering_fixture(None)
}
pub fn lowering_fixture(catalog: Option<V>) -> V {
    lowering_fixture_inputs(catalog, None)
}
pub fn lowering_fixture_inputs(catalog: Option<V>, exports_override: Option<V>) -> V {
    let captured = |value: &str| {
        echo_edict_canonical::decode_canonical_cbor_v1(
            &hex::decode(value.trim()).expect("captured bytes"),
        )
        .expect("captured value")
    };
    let mut lawpack = captured(include_str!(
        "../../../echo-edict-provider-lowerer/tests/fixtures/node-atom-read/03-lawpack.hex"
    ));
    let adapter = captured(include_str!(
        "../../../echo-edict-provider-lowerer/tests/fixtures/node-atom-read/01-lawpack-adapter.hex"
    ));
    let exports = exports_override.unwrap_or_else(|| {
        captured(include_str!(
        "../../../echo-edict-provider-lowerer/tests/fixtures/node-atom-read/02-lawpack-exports.hex"
    ))
    });
    let catalog = catalog.unwrap_or_else(|| {
        let V::Array(effects) = get(&exports, "effects") else {
            panic!("effects")
        };
        let effect = &effects[0];
        let implementation = get(
            get(&adapter, "effectImplementations"),
            "jedit.text@1.readFact",
        );
        let V::Map(mappings) = get(implementation, "failureMappings") else {
            panic!("mappings")
        };
        let failures = V::Map(
            mappings
                .iter()
                .map(|(source, tag)| {
                    let V::Text(source) = source else {
                        panic!("failure")
                    };
                    (
                        tag.clone(),
                        get(get(effect, "effectFailures"), source).clone(),
                    )
                })
                .collect(),
        );
        map([
            ("apiVersion", text("edict.target-profile.intrinsics/v1")),
            (
                "intrinsics",
                map([(
                    "echo.dpo@1.node-atom-read",
                    map([
                        ("intrinsicClass", text("effect")),
                        (
                            "argumentTypes",
                            V::Array(vec![get(effect, "inputType").clone()]),
                        ),
                        ("returnType", get(effect, "outputType").clone()),
                        ("typeParameters", V::Array(vec![])),
                        ("effectKind", text("read")),
                        ("writeClass", text("read")),
                        ("guardSupport", V::Bool(true)),
                        ("canParticipateInAtomicGuard", V::Bool(true)),
                        (
                            "costTemplate",
                            get(implementation, "costObligation").clone(),
                        ),
                        (
                            "footprintTemplate",
                            get(implementation, "footprintObligation").clone(),
                        ),
                        ("effectFailures", failures),
                    ]),
                )]),
            ),
        ])
    });
    let mut target_profile = captured(include_str!(
        "../../../echo-edict-provider-lowerer/tests/fixtures/node-atom-read/profile.hex"
    ));
    let mut intrinsics_ref = get(&target_profile, "intrinsics").clone();
    set(
        &mut intrinsics_ref,
        "digest",
        V::Array(vec![
            text("sha256"),
            artifact_identity("echo.dpo.intrinsics/v1", &catalog),
        ]),
    );
    set(&mut target_profile, "intrinsics", intrinsics_ref);
    let pin = |domain, value: &V| V::Array(vec![text("sha256"), artifact_identity(domain, value)]);
    let mut reference = get(&lawpack, "exports").clone();
    set(
        &mut reference,
        "digest",
        pin("edict.lawpack-exports/v1", &exports),
    );
    set(&mut lawpack, "exports", reference);
    let mut adapters = get(&lawpack, "targetAdapters").clone();
    let V::Array(entries) = &mut adapters else {
        panic!("adapters")
    };
    set(
        &mut target_profile,
        "targetIr",
        get(&entries[0], "acceptedTargetIr").clone(),
    );
    let mut target_profile_ref = get(&entries[0], "acceptedTargetProfile").clone();
    set(
        &mut target_profile_ref,
        "digest",
        pin("edict.target-profile/v1", &target_profile),
    );
    set(&mut entries[0], "acceptedTargetProfile", target_profile_ref);
    let mut reference = get(&entries[0], "adapter").clone();
    set(
        &mut reference,
        "digest",
        pin("edict.lawpack-adapter/v1", &adapter),
    );
    set(&mut entries[0], "adapter", reference);
    set(&mut lawpack, "targetAdapters", adapters);
    let original_source = captured(include_str!(
        "../../../echo-edict-provider-lowerer/tests/fixtures/node-atom-read/04-source.hex"
    ));
    let V::Bytes(source) = original_source else {
        panic!("source")
    };
    let source = String::from_utf8(source).expect("utf8");
    let digest = echo_edict_canonical::digest_canonical_value_v1("edict.lawpack/v1", &lawpack)
        .expect("digest");
    let source = source.replace(
        "sha256:2ea164293719d2b03231c4904eb75ddcdf55fd29cc31f4fc5a5319d9ae5e60e7",
        &digest,
    );
    map([
        (
            "source_artifact",
            V::Bytes(encoded(&V::Bytes(source.into_bytes()))),
        ),
        ("lawpack_artifact", V::Bytes(encoded(&lawpack))),
        ("adapter_artifact", V::Bytes(encoded(&adapter))),
        (
            "target_profile_artifact",
            V::Bytes(encoded(&target_profile)),
        ),
        ("intrinsics_artifact", V::Bytes(encoded(&catalog))),
    ])
}
pub fn compiler_program() -> V {
    let core = compiled_core();
    compiled_bundle(
        &core,
        &compiled_target(&core),
        &compiled_exports(),
        "update@1",
    )
}
pub fn compiled_artifact_identity(p: &V, key: &str, domain: &str) -> V {
    let V::Bytes(bytes) = get(p, "compiled_program") else {
        return id(2);
    };
    let Ok(bundle) = echo_edict_canonical::decode_canonical_cbor_v1(bytes) else {
        return id(2);
    };
    let V::Map(fields) = bundle else { return id(2) };
    let Some((_, V::Bytes(bytes))) = fields.iter().find(|(name, _)| name == &text(key)) else {
        return id(2);
    };
    echo_edict_canonical::decode_canonical_cbor_v1(bytes)
        .map_or_else(|_| id(2), |value| artifact_identity(domain, &value))
}
pub fn core_identity(p: &V) -> V {
    compiled_artifact_identity(p, "core_artifact", "edict.core.module/v1")
}
pub fn target_identity(p: &V) -> V {
    compiled_artifact_identity(p, "target_ir_artifact", "edict.target-ir.artifact/v1")
}
pub fn exports_identity(p: &V) -> V {
    compiled_artifact_identity(p, "lawpack_exports_artifact", "edict.lawpack-exports/v1")
}
pub fn program() -> V {
    map([
        ("schema", text("echo.operation-program/v1")),
        ("kind", text("edict-effect-program/v1")),
        (
            "interpreter_profile_identity",
            profile("echo.operation-interpreter.edict-effect/v1"),
        ),
        (
            "intrinsic_profile_identity",
            profile("echo.operation-intrinsics.edict-effect/v1"),
        ),
        ("compiled_program", V::Bytes(encoded(&compiler_program()))),
        (
            "target_ir_identity",
            artifact_identity(
                "edict.target-ir.artifact/v1",
                &compiled_target(&compiled_core()),
            ),
        ),
        ("identity_law", V::Integer(1)),
        ("graph_schema", V::Bytes(encoded(&graph()))),
        ("footprint_law", V::Bytes(encoded(&law()))),
        ("max_input_bytes", V::Integer(65536)),
        ("max_result_bytes", V::Integer(65536)),
        ("max_allocated_bytes", V::Integer(16 * 1024 * 1024)),
        ("output_type", text("app.Result")),
    ])
}
pub fn package(program: &V) -> V {
    map([
        ("schema", text("echo.operation-package/v1")),
        ("operation_coordinate", text("app.update@1")),
        ("obstruction_coordinate", text("app.Refusal")),
        ("authority_profile_identity", id(8)),
        (
            "semantic_closure",
            map([
                ("application_schema_coordinate", text("app.Schema")),
                ("application_schema_identity", exports_identity(program)),
                ("canonical_meaning_identity", id(3)),
                ("core_identity", core_identity(program)),
                ("edict_source_identity", id(1)),
                ("target_ir_identity", target_identity(program)),
            ]),
        ),
        ("program", V::Bytes(encoded(program))),
        (
            "budget_ceiling",
            map([
                ("steps", V::Integer(1)),
                ("read_bytes", V::Integer(0)),
                ("write_bytes", V::Integer(0)),
            ]),
        ),
        (
            "target_profile_identity",
            profile("echo.operation-target.edict-effect/v1"),
        ),
        (
            "application_basis_schema_identity",
            profile("echo.operation.basis.edict-effect/v1"),
        ),
        (
            "input_schema_identity",
            profile("echo.operation.input.edict-effect/v1"),
        ),
        (
            "result_schema_identity",
            profile("echo.operation.result.edict-effect/v1"),
        ),
        (
            "obstruction_schema_identity",
            profile("echo.operation.obstruction.edict-effect/v1"),
        ),
        (
            "result_interpretation_identity",
            profile("echo.operation.result-interpretation.edict-effect/v1"),
        ),
        (
            "obstruction_interpretation_identity",
            profile("echo.operation.obstruction-interpretation.edict-effect/v1"),
        ),
        (
            "evaluation_basis_schema_identity",
            profile("echo.operation.evaluation-basis/v1"),
        ),
        (
            "interpreter_profile_identity",
            profile("echo.operation-interpreter.edict-effect/v1"),
        ),
        (
            "intrinsic_profile_identity",
            profile("echo.operation-intrinsics.edict-effect/v1"),
        ),
        ("footprint_contract_identity", footprint(&law())),
    ])
}
pub fn set(v: &mut V, k: &str, x: V) {
    let V::Map(m) = v else { panic!("map") };
    m.iter_mut()
        .find(|(key, _)| key == &text(k))
        .expect("field")
        .1 = x;
}
pub fn add(v: &mut V, k: &str, x: V) {
    let V::Map(m) = v else { panic!("map") };
    m.push((text(k), x));
}
pub fn prepare(v: &V) -> Result<ExecutableOperationPackage, EffectPackageError> {
    let b = encoded(v);
    let expected = hash(b"echo:operation-package:v1\0", &b);
    ExecutableOperationPackage::prepare(&b, expected)
}
pub fn refused(v: &V) {
    assert!(
        prepare(v).is_err(),
        "must refuse malformed or inconsistent package"
    );
}
