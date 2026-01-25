// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! TTD IR structs used by echo-ttd-gen.
//!
//! These types represent the JSON IR emitted by Wesley's TTD compiler.
//! Schema discovered from actual Wesley output (ttd-ir/v1).
//!
//! Note: Many fields are deserialized but not yet used in codegen.
//! They are kept for forward compatibility as the IR schema evolves.

#![allow(dead_code)]

use serde::Deserialize;

/// Root TTD IR payload consumed by `echo-ttd-gen`.
#[derive(Debug, Deserialize)]
pub struct TtdIR {
    /// IR schema version tag (e.g. `"ttd-ir/v1"`).
    pub ir_version: Option<String>,

    /// SHA256 hash of the source schema (hex).
    #[serde(default)]
    pub schema_sha256: Option<String>,

    /// Provenance metadata describing the toolchain that produced this IR.
    #[serde(default)]
    pub generated_by: Option<GeneratedBy>,

    /// Generated timestamp (ISO 8601).
    #[serde(default)]
    pub generated_at: Option<String>,

    /// Channel definitions (event buses).
    #[serde(default)]
    pub channels: Vec<ChannelDef>,

    /// Op definitions (operations).
    #[serde(default)]
    pub ops: Vec<OpDef>,

    /// Rule definitions (state machine transitions).
    #[serde(default)]
    pub rules: Vec<RuleDef>,

    /// Global invariants.
    #[serde(default)]
    pub invariants: Vec<InvariantDef>,

    /// Emission declarations.
    #[serde(default)]
    pub emissions: Vec<EmissionDef>,

    /// Footprint specifications (per-op read/write sets).
    #[serde(default)]
    pub footprints: Vec<FootprintDef>,

    /// Registry entries (type IDs).
    #[serde(default)]
    pub registry: Vec<RegistryEntry>,

    /// Codec declarations.
    #[serde(default)]
    pub codecs: Vec<CodecDef>,

    /// Type definitions (structs).
    #[serde(default)]
    pub types: Vec<TypeDef>,

    /// Enum definitions.
    #[serde(default)]
    pub enums: Vec<EnumDef>,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: Option<Metadata>,
}

/// Generator provenance metadata.
#[derive(Debug, Deserialize)]
pub struct GeneratedBy {
    /// Tool name that produced this IR.
    pub tool: String,
    /// Tool version.
    #[serde(default)]
    pub version: Option<String>,
}

/// Additional metadata.
#[derive(Debug, Deserialize)]
pub struct Metadata {
    #[serde(rename = "extractedAt")]
    pub extracted_at: Option<String>,
    #[serde(rename = "ttdVersion")]
    pub ttd_version: Option<String>,
}

// ─── Channel Definitions ─────────────────────────────────────────────────────

/// Channel definition (event bus).
#[derive(Debug, Deserialize)]
pub struct ChannelDef {
    /// Kind tag (always "CHANNEL").
    pub kind: String,
    /// Channel name.
    pub name: String,
    /// Channel version.
    #[serde(default)]
    pub version: Option<u16>,
    /// Event types emitted on this channel.
    #[serde(rename = "eventTypes", default)]
    pub event_types: Vec<String>,
    /// Whether events are ordered.
    #[serde(default)]
    pub ordered: bool,
    /// Whether channel is persistent.
    #[serde(default)]
    pub persistent: bool,
}

// ─── Op Definitions ──────────────────────────────────────────────────────────

/// Op definition.
#[derive(Debug, Deserialize)]
pub struct OpDef {
    /// Kind tag (always "OP").
    pub kind: String,
    /// Op name.
    pub name: String,
    /// Arguments.
    #[serde(default)]
    pub args: Vec<ArgDef>,
    /// Result type name.
    #[serde(rename = "resultType")]
    pub result_type: String,
    /// Whether op is idempotent.
    #[serde(default)]
    pub idempotent: bool,
    /// Whether op is read-only.
    #[serde(default)]
    pub readonly: bool,
    /// Numeric op ID for wire protocol.
    pub op_id: u32,
    /// Rules triggered by this op (embedded).
    #[serde(default)]
    pub rules: Vec<EmbeddedRule>,
}

/// Argument definition.
#[derive(Debug, Deserialize)]
pub struct ArgDef {
    /// Argument name.
    pub name: String,
    /// GraphQL type name.
    #[serde(rename = "type")]
    pub type_name: String,
    /// Whether argument is required.
    #[serde(default)]
    pub required: bool,
    /// Whether argument is a list.
    #[serde(default)]
    pub list: bool,
}

/// Embedded rule within an op.
#[derive(Debug, Deserialize)]
pub struct EmbeddedRule {
    /// Kind tag.
    pub kind: String,
    /// Rule name.
    pub name: String,
    /// Source states.
    #[serde(default)]
    pub from: Vec<String>,
    /// Target state.
    pub to: String,
    /// Op name that triggers this rule.
    #[serde(rename = "opName")]
    pub op_name: String,
    /// Guard expression (optional).
    #[serde(default)]
    pub guard: Option<String>,
}

// ─── Rule Definitions ────────────────────────────────────────────────────────

/// Rule definition (state machine transition).
#[derive(Debug, Deserialize)]
pub struct RuleDef {
    /// Kind tag (always "RULE").
    pub kind: String,
    /// Rule name.
    pub name: String,
    /// Source states.
    #[serde(default)]
    pub from: Vec<String>,
    /// Target state.
    pub to: String,
    /// Op name that triggers this rule.
    #[serde(rename = "opName")]
    pub op_name: String,
    /// Guard expression (optional).
    #[serde(default)]
    pub guard: Option<String>,
}

// ─── Invariant Definitions ───────────────────────────────────────────────────

/// Invariant definition.
#[derive(Debug, Deserialize)]
pub struct InvariantDef {
    /// Kind tag (always "INVARIANT").
    pub kind: String,
    /// Invariant name.
    pub name: String,
    /// Expression.
    pub expr: String,
    /// Severity level.
    #[serde(default)]
    pub severity: Option<String>,
}

// ─── Emission Definitions ────────────────────────────────────────────────────

/// Emission declaration.
#[derive(Debug, Deserialize)]
pub struct EmissionDef {
    /// Kind tag (always "EMISSION").
    pub kind: String,
    /// Target channel.
    pub channel: String,
    /// Event type (optional).
    #[serde(default)]
    pub event: Option<String>,
    /// Op that emits this.
    #[serde(rename = "opName")]
    pub op_name: String,
    /// Condition for emission (optional).
    #[serde(default)]
    pub condition: Option<String>,
    /// Timing constraint in ms (optional).
    #[serde(rename = "withinMs", default)]
    pub within_ms: Option<u64>,
}

// ─── Footprint Definitions ───────────────────────────────────────────────────

/// Footprint specification (per-op read/write sets).
#[derive(Debug, Deserialize)]
pub struct FootprintDef {
    /// Kind tag (always "FOOTPRINT").
    pub kind: String,
    /// Op name this footprint belongs to.
    #[serde(rename = "opName")]
    pub op_name: String,
    /// Types read by this op.
    #[serde(default)]
    pub reads: Vec<String>,
    /// Types written by this op.
    #[serde(default)]
    pub writes: Vec<String>,
    /// Types created by this op.
    #[serde(default)]
    pub creates: Vec<String>,
    /// Types deleted by this op.
    #[serde(default)]
    pub deletes: Vec<String>,
}

// ─── Registry Entries ────────────────────────────────────────────────────────

/// Registry entry (type ID mapping).
#[derive(Debug, Deserialize)]
pub struct RegistryEntry {
    /// Kind tag (always "REGISTRY_ENTRY").
    pub kind: String,
    /// Type name.
    #[serde(rename = "typeName")]
    pub type_name: String,
    /// Numeric ID.
    pub id: u32,
    /// Whether deprecated.
    #[serde(default)]
    pub deprecated: bool,
}

// ─── Codec Definitions ───────────────────────────────────────────────────────

/// Codec declaration.
#[derive(Debug, Deserialize)]
pub struct CodecDef {
    /// Kind tag (always "CODEC").
    pub kind: String,
    /// Type name.
    #[serde(rename = "typeName")]
    pub type_name: String,
    /// Encoding format (e.g., "cbor").
    pub format: String,
    /// Whether canonical encoding.
    #[serde(default)]
    pub canonical: bool,
}

// ─── Type Definitions ────────────────────────────────────────────────────────

/// Type definition (struct).
#[derive(Debug, Deserialize)]
pub struct TypeDef {
    /// Type name.
    pub name: String,
    /// Version (optional).
    #[serde(default)]
    pub version: Option<TypeVersion>,
    /// Fields.
    #[serde(default)]
    pub fields: Vec<FieldDef>,
}

/// Type version.
#[derive(Debug, Deserialize)]
pub struct TypeVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

/// Field definition.
#[derive(Debug, Deserialize)]
pub struct FieldDef {
    /// Field name.
    pub name: String,
    /// GraphQL type name.
    #[serde(rename = "type")]
    pub type_name: String,
    /// Whether field is required.
    #[serde(default)]
    pub required: bool,
    /// Whether field is a list.
    #[serde(default)]
    pub list: bool,
    /// State field metadata (optional).
    #[serde(rename = "stateField", default)]
    pub state_field: Option<StateFieldMeta>,
    /// Constraint (optional).
    #[serde(default)]
    pub constraint: Option<FieldConstraint>,
}

/// State field metadata.
#[derive(Debug, Deserialize)]
pub struct StateFieldMeta {
    /// Whether this is a key field.
    #[serde(default)]
    pub key: bool,
    /// Whether this field is derived.
    #[serde(default)]
    pub derived: bool,
    /// Derivation expression (if derived).
    #[serde(default)]
    pub derivation: Option<String>,
}

/// Field constraint.
#[derive(Debug, Deserialize)]
pub struct FieldConstraint {
    /// Minimum value.
    #[serde(default)]
    pub min: Option<i64>,
    /// Maximum value.
    #[serde(default)]
    pub max: Option<i64>,
}

// ─── Enum Definitions ────────────────────────────────────────────────────────

/// Enum definition.
#[derive(Debug, Deserialize)]
pub struct EnumDef {
    /// Enum name.
    pub name: String,
    /// Enum values.
    #[serde(default)]
    pub values: Vec<String>,
}
