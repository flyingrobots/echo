// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Minimal Wesley IR structs used by echo-wesley-gen.

use serde::Deserialize;

/// Root Wesley IR payload consumed by `echo-wesley-gen`.
///
/// This is a minimal, serde-friendly representation of the IR JSON emitted by
/// the upstream generator. Unknown fields are ignored by serde; missing fields
/// are defaulted where sensible so the CLI can be tolerant of additive schema
/// changes.
#[derive(Debug, Deserialize)]
pub struct WesleyIR {
    /// IR schema version tag (e.g. `"echo-ir/v1"`).
    #[serde(default)]
    pub ir_version: Option<String>,
    /// Provenance metadata describing the toolchain that produced this IR.
    #[serde(default)]
    pub generated_by: Option<GeneratedBy>,
    /// Optional schema hash of the source GraphQL schema (hex).
    #[serde(default)]
    pub schema_sha256: Option<String>,
    /// Type catalog (enums, objects, etc.) referenced by operations.
    #[serde(default)]
    pub types: Vec<TypeDefinition>,
    /// Operation catalog (query/mutation) with stable op ids and argument info.
    #[serde(default)]
    pub ops: Vec<OpDefinition>,
    /// Canonical codec identifier for encoding/decoding op argument payloads.
    #[serde(default)]
    pub codec_id: Option<String>,
    /// Registry layout version (bumped for breaking changes in the generated output).
    #[serde(default)]
    pub registry_version: Option<u32>,
}

/// Generator provenance metadata embedded in the IR.
#[derive(Debug, Deserialize)]
pub struct GeneratedBy {
    /// Tool name (package/binary) that produced this IR.
    pub tool: String,
    #[serde(default)]
    /// Optional tool version.
    pub version: Option<String>,
}

/// Type definition in the IR type catalog.
#[derive(Debug, Deserialize)]
pub struct TypeDefinition {
    /// GraphQL type name.
    pub name: String,
    /// Kind tag (object vs enum, etc.).
    pub kind: TypeKind,
    /// Object fields (empty for non-object kinds).
    #[serde(default)]
    pub fields: Vec<FieldDefinition>,
    /// Enum values (empty for non-enum kinds).
    #[serde(default)]
    pub values: Vec<String>, // For enums
}

/// Kind tag for IR type definitions.
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TypeKind {
    /// GraphQL object type.
    Object,
    /// GraphQL enum type.
    Enum,
    /// GraphQL scalar type.
    Scalar,
    /// GraphQL interface type.
    Interface,
    /// GraphQL union type.
    Union,
    /// GraphQL input object type.
    InputObject,
}

/// Operation kind (query or mutation).
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OpKind {
    /// Read-only operation.
    Query,
    /// State-mutating operation.
    Mutation,
}

/// Operation definition in the IR operation catalog.
#[derive(Debug, Deserialize)]
pub struct OpDefinition {
    /// Operation kind.
    pub kind: OpKind,
    /// GraphQL operation name.
    pub name: String,
    /// Persisted operation identifier.
    pub op_id: u32,
    /// Argument catalog for strict input validation.
    #[serde(default)]
    pub args: Vec<ArgDefinition>,
    /// GraphQL result type name.
    pub result_type: String,
}

/// Argument definition (used for both operation args and object fields).
///
/// The Wesley IR currently represents operation args and object fields with
/// the same shape (name + base type + required + list). We keep distinct Rust
/// wrapper types (`ArgDefinition` and `FieldDefinition`) so call sites can
/// remain semantically explicit even if the JSON schema evolves.
#[derive(Debug, Deserialize)]
pub struct ArgDefinition {
    /// Field/argument name.
    pub name: String,
    #[serde(rename = "type")]
    /// GraphQL base type name (e.g. `"String"`, `"Theme"`, `"AppState"`).
    pub type_name: String,
    /// Whether the argument is required.
    pub required: bool,
    /// Whether the argument is a list.
    #[serde(default)]
    pub list: bool,
}

/// Object field definition (same schema as [`ArgDefinition`]; kept for semantic clarity).
#[derive(Debug, Deserialize)]
pub struct FieldDefinition {
    /// Field name.
    pub name: String,
    #[serde(rename = "type")]
    /// GraphQL base type name.
    pub type_name: String,
    /// Whether the field is required.
    pub required: bool,
    /// Whether the field is a list.
    #[serde(default)]
    pub list: bool,
}
