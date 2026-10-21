// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Pure serialization of current effect proposals, without verification or authority.

/// Exact artifacts and explicit policy metadata for an untrusted publication.
#[derive(Clone, Copy)]
pub struct EffectPublicationInput<'a> {
    /// Current canonical effect bundle containing retained compiler artifacts.
    pub compiler_program: &'a [u8],
    /// Coordinate of the exports artifact used as application-schema metadata.
    pub application_schema_coordinate: &'a str,
    /// Exact canonical source or authenticated source-closure artifact.
    pub source_artifact: &'a [u8],
    /// Either `edict.source/v1` or `edict.source-closure/v1`.
    pub source_domain: &'a str,
    /// Current canonical authored-effect configuration.
    pub configuration: &'a [u8],
}

/// Refusal to serialize malformed publication material.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectPublicationError {
    /// Artifact shape, canonical bytes, or embedded identity material is invalid.
    InvalidArtifact,
    /// Explicit configuration or its resource limits are invalid.
    InvalidConfiguration,
    /// A required coordinate is missing or unbounded.
    InvalidCoordinate,
}

impl std::fmt::Display for EffectPublicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidArtifact => "invalid effect publication artifact envelope",
            Self::InvalidConfiguration => "invalid effect publication configuration or limits",
            Self::InvalidCoordinate => "missing or unbounded effect publication coordinate",
        })
    }
}
impl std::error::Error for EffectPublicationError {}

/// Serializes the single current operation-package and effect-program formats.
///
/// This constructs an untrusted proposal. Callers must independently verify
/// compiler correspondence, policy and authority before any runtime admission.
/// # Errors
/// Refuses malformed artifact/configuration envelopes, unsupported source
/// domains, undeclared coordinates and invalid limit fields. Graph-law semantics,
/// compiler correspondence and authority remain independent checks.
#[allow(
    clippy::too_many_lines,
    reason = "Keep the current wire projection and its identity recipes visible together."
)]
pub fn encode_effect_publication(
    input: EffectPublicationInput<'_>,
) -> Result<Vec<u8>, EffectPublicationError> {
    use EffectPublicationError::{InvalidArtifact as A, InvalidConfiguration as C};
    let bundle = decode(input.compiler_program).map_err(|_| A)?;
    exact(
        &bundle,
        &[
            "schema",
            "intent",
            "core_artifact",
            "target_ir_artifact",
            "lawpack_exports_artifact",
            "lowering_artifacts",
        ],
        A,
    )?;
    equals(
        &bundle,
        "schema",
        "echo.compiler-produced-effect-program/v1",
        A,
    )?;
    let intent_name = coordinate(field(&bundle, "intent", A)?)?;
    let embedded = |key| {
        let bytes = bytes(field(&bundle, key, A)?, A)?;
        decode(bytes).map_err(|_| A)
    };
    let core = embedded("core_artifact")?;
    let target = embedded("target_ir_artifact")?;
    let exports = embedded("lawpack_exports_artifact")?;
    equals(&core, "apiVersion", "edict.core/v1", A)?;
    equals(&target, "domain", "echo.span-ir/v2", A)?;
    equals(&target, "kind", "orderedTargetIrArtifact", A)?;
    let module = coordinate(field(&core, "coordinate", A)?)?;
    equals(&target, "sourceCoreCoordinate", module, A)?;
    let intent = field(field(&core, "intents", A)?, intent_name, A)?;
    let output = coordinate(field(intent, "output", A)?)?;
    let operation = format!("{module}.{intent_name}");
    coordinate(&text(&operation))?;
    coordinate(&text(input.application_schema_coordinate))?;
    let core_id = identity("edict.core.module/v1", &core)?;
    let target_id = identity("edict.target-ir.artifact/v1", &target)?;
    let exports_id = identity("edict.lawpack-exports/v1", &exports)?;
    let core_reference = field(field(&target, "semanticClosure", A)?, "sourceCore", A)?;
    exact(core_reference, &["id", "digest"], A)?;
    equals(core_reference, "id", module, A)?;
    if field(core_reference, "digest", A)? != &V::Array(vec![text("sha256"), core_id.clone()]) {
        return Err(A);
    }
    let source = decode(input.source_artifact).map_err(|_| A)?;
    match input.source_domain {
        "edict.source/v1" => {
            std::str::from_utf8(bytes(&source, A)?).map_err(|_| A)?;
        }
        "edict.source-closure/v1" => {
            echo_edict_canonical::source_closure::source_closure_root_v1(&source, module)
                .map_err(|_| A)?;
        }
        _ => return Err(A),
    }
    let source_id = identity(input.source_domain, &source)?;
    let config = decode(input.configuration).map_err(|_| C)?;
    exact(
        &config,
        &[
            "apiVersion",
            "programKind",
            "identityLaw",
            "intrinsicCatalog",
            "graphSchema",
            "footprintLaw",
            "maxInputBytes",
            "maxResultBytes",
            "maxAllocatedBytes",
            "budgetCeiling",
            "authorityProfile",
        ],
        C,
    )?;
    equals(
        &config,
        "apiVersion",
        "echo.operation-lowering-configuration/v1",
        C,
    )?;
    equals(&config, "programKind", "edict-effect-program/v1", C)?;
    if number(field(&config, "identityLaw", C)?)? != 1 {
        return Err(C);
    }
    // The requested authority class comes only from the configuration, through
    // the same label digest fixed operations use. It is a request: the runtime
    // owner's admission policy decides what the package may do.
    let V::Text(authority_label) = field(&config, "authorityProfile", C)? else {
        return Err(C);
    };
    if authority_label.is_empty() {
        return Err(C);
    }
    let authority = profile(authority_label);
    let graph = policy_bytes(&config, "graphSchema")?;
    let law = policy_bytes(&config, "footprintLaw")?;
    let catalog = policy_bytes(&config, "intrinsicCatalog")?;
    let catalog_value = decode(catalog).map_err(|_| C)?;
    exact(&catalog_value, &["apiVersion", "intrinsics"], C)?;
    equals(
        &catalog_value,
        "apiVersion",
        "edict.target-profile.intrinsics/v1",
        C,
    )?;
    if !matches!(field(&catalog_value, "intrinsics", C)?, V::Map(_)) {
        return Err(C);
    }
    // This only preserves the chosen bytes. Their profile/signature relation is
    // independently verified before admission, not asserted by this serializer.
    if field(&bundle, "lowering_artifacts", A)? != &V::Null
        && bytes(
            field(
                field(&bundle, "lowering_artifacts", A)?,
                "intrinsics_artifact",
                A,
            )?,
            A,
        )? != catalog
    {
        return Err(C);
    }
    let max_input = limit(&config, "maxInputBytes", 65536)?;
    let max_result = limit(&config, "maxResultBytes", 65536)?;
    let max_allocated = limit(&config, "maxAllocatedBytes", 16 * 1024 * 1024)?;
    let budget = field(&config, "budgetCeiling", C)?;
    exact(budget, &["steps", "readBytes", "writeBytes"], C)?;
    let steps = number(field(budget, "steps", C)?)?;
    if steps == 0 {
        return Err(C);
    }
    let reads = number(field(budget, "readBytes", C)?)?;
    let writes = number(field(budget, "writeBytes", C)?)?;
    let mut footprint = blake3::Hasher::new();
    footprint.update(b"echo:operation-footprint-law:v1\0");
    footprint.update(law);
    let footprint = V::Bytes(footprint.finalize().as_bytes().to_vec());
    let program = map([
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
        (
            "compiled_program",
            V::Bytes(input.compiler_program.to_vec()),
        ),
        ("target_ir_identity", target_id.clone()),
        ("identity_law", V::Integer(1)),
        ("graph_schema", V::Bytes(graph.to_vec())),
        ("footprint_law", V::Bytes(law.to_vec())),
        ("max_input_bytes", V::Integer(i128::from(max_input))),
        ("max_result_bytes", V::Integer(i128::from(max_result))),
        ("max_allocated_bytes", V::Integer(i128::from(max_allocated))),
        ("output_type", text(output)),
    ]);
    let program = encode(&program).map_err(|_| A)?;
    let mut package = vec![
        ("schema", text("echo.operation-package/v1")),
        ("operation_coordinate", text(&operation)),
        ("authority_profile_identity", authority),
        (
            "semantic_closure",
            map([
                ("edict_source_identity", source_id),
                ("canonical_meaning_identity", core_id.clone()),
                ("core_identity", core_id),
                ("target_ir_identity", target_id),
                (
                    "application_schema_coordinate",
                    text(input.application_schema_coordinate),
                ),
                ("application_schema_identity", exports_id),
            ]),
        ),
        ("program", V::Bytes(program)),
        (
            "budget_ceiling",
            map([
                ("steps", V::Integer(i128::from(steps))),
                ("read_bytes", V::Integer(i128::from(reads))),
                ("write_bytes", V::Integer(i128::from(writes))),
            ]),
        ),
        ("footprint_contract_identity", footprint),
    ];
    for (key, label) in [
        (
            "target_profile_identity",
            "echo.operation-target.edict-effect/v1",
        ),
        (
            "interpreter_profile_identity",
            "echo.operation-interpreter.edict-effect/v1",
        ),
        (
            "intrinsic_profile_identity",
            "echo.operation-intrinsics.edict-effect/v1",
        ),
        (
            "application_basis_schema_identity",
            "echo.operation.basis.edict-effect/v1",
        ),
        (
            "input_schema_identity",
            "echo.operation.input.edict-effect/v1",
        ),
        (
            "result_schema_identity",
            "echo.operation.result.edict-effect/v1",
        ),
        (
            "obstruction_schema_identity",
            "echo.operation.obstruction.edict-effect/v1",
        ),
        (
            "result_interpretation_identity",
            "echo.operation.result-interpretation.edict-effect/v1",
        ),
        (
            "obstruction_interpretation_identity",
            "echo.operation.obstruction-interpretation.edict-effect/v1",
        ),
        (
            "evaluation_basis_schema_identity",
            "echo.operation.evaluation-basis/v1",
        ),
    ] {
        package.push((key, profile(label)));
    }
    encode(&map(package)).map_err(|_| A)
}

use echo_edict_canonical::{
    decode_canonical_cbor_v1 as decode, digest_canonical_value_bytes_v1 as digest,
    encode_canonical_cbor_v1 as encode, CanonicalValueV1 as V,
};
use EffectPublicationError as E;
fn text(s: &str) -> V {
    V::Text(s.to_owned())
}
fn map(entries: impl IntoIterator<Item = (&'static str, V)>) -> V {
    V::Map(entries.into_iter().map(|(k, v)| (text(k), v)).collect())
}
fn field<'a>(v: &'a V, key: &str, e: E) -> Result<&'a V, E> {
    let V::Map(fields) = v else { return Err(e) };
    fields
        .iter()
        .find_map(|(k, v)| (k == &text(key)).then_some(v))
        .ok_or(e)
}
fn exact(v: &V, keys: &[&str], e: E) -> Result<(), E> {
    let V::Map(fields) = v else { return Err(e) };
    if fields.len() != keys.len()
        || fields
            .iter()
            .any(|(k, _)| !matches!(k,V::Text(key) if keys.contains(&key.as_str())))
    {
        return Err(e);
    }
    Ok(())
}
fn equals(v: &V, key: &str, expected: &str, e: E) -> Result<(), E> {
    (field(v, key, e)? == &text(expected))
        .then_some(())
        .ok_or(e)
}
fn coordinate(v: &V) -> Result<&str, E> {
    match v {
        V::Text(s) if !s.is_empty() && s.len() <= 1024 => Ok(s),
        _ => Err(E::InvalidCoordinate),
    }
}
fn bytes(v: &V, e: E) -> Result<&[u8], E> {
    match v {
        V::Bytes(b) => Ok(b),
        _ => Err(e),
    }
}
fn number(v: &V) -> Result<u64, E> {
    match v {
        V::Integer(n) => u64::try_from(*n).map_err(|_| E::InvalidConfiguration),
        _ => Err(E::InvalidConfiguration),
    }
}
fn policy_bytes<'a>(v: &'a V, key: &str) -> Result<&'a [u8], E> {
    let b = bytes(
        field(v, key, E::InvalidConfiguration)?,
        E::InvalidConfiguration,
    )?;
    if b.is_empty() || b.len() > 16 * 1024 * 1024 {
        return Err(E::InvalidConfiguration);
    }
    decode(b).map_err(|_| E::InvalidConfiguration)?;
    Ok(b)
}
fn limit(v: &V, key: &str, max: u64) -> Result<u64, E> {
    let n = number(field(v, key, E::InvalidConfiguration)?)?;
    if n > max {
        return Err(E::InvalidConfiguration);
    }
    Ok(n)
}
fn identity(domain: &str, value: &V) -> Result<V, E> {
    digest(domain, value)
        .map(|v| V::Bytes(v.to_vec()))
        .map_err(|_| E::InvalidArtifact)
}
fn profile(label: &str) -> V {
    V::Bytes(crate::executable_operation::profile_digest(label).to_vec())
}
