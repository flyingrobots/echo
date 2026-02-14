// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Minimal, generic registry interface for Echo WASM helpers.
//!
//! The registry provider is supplied by the application (generated from the
//! GraphQL/Wesley IR). Echo core and `warp-wasm` depend only on this crate and
//! **must not** embed app-specific registries.

#![no_std]

/// Codec identifier used by the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegistryInfo {
    /// Canonical codec identifier (e.g., "cbor-canon-v1").
    pub codec_id: &'static str,
    /// Registry schema version for breaking changes in layout.
    pub registry_version: u32,
    /// Hex-encoded schema hash (lowercase, 64 chars).
    pub schema_sha256_hex: &'static str,
}

/// Error codes for wasm helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelperError {
    /// No registry provider installed.
    NoRegistry,
    /// Unknown operation ID.
    UnknownOp,
    /// Input did not match schema (unknown key, missing required, wrong type, enum mismatch).
    InvalidInput,
    /// Internal failure (encoding).
    Internal,
}

/// Operation kind (query or mutation/command).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpKind {
    /// Read-only operation.
    Query,
    /// State-mutating operation.
    Mutation,
}

/// Descriptor for a single operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpDef {
    /// Operation kind.
    pub kind: OpKind,
    /// Operation name (GraphQL name).
    pub name: &'static str,
    /// Persisted operation identifier.
    pub op_id: u32,
    /// Argument descriptors.
    pub args: &'static [ArgDef],
    /// Result type name (GraphQL return type).
    pub result_ty: &'static str,
}

/// Argument descriptor (flat; sufficient for strict object validation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArgDef {
    /// Field name.
    pub name: &'static str,
    /// GraphQL base type name.
    pub ty: &'static str,
    /// Whether the field is required.
    pub required: bool,
    /// Whether the field is a list.
    pub list: bool,
}

/// Enum descriptor (for validating enum string values).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnumDef {
    /// Enum name.
    pub name: &'static str,
    /// Allowed values (uppercase GraphQL names).
    pub values: &'static [&'static str],
}

/// Object descriptor for result validation (optional; fields may be empty for now).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectDef {
    /// Object name.
    pub name: &'static str,
    /// Fields on the object.
    pub fields: &'static [ArgDef],
}

/// Application-supplied registry provider.
///
/// Implemented by a generated crate in the application build. `warp-wasm`
/// should link against that provider to validate op IDs, expose registry
/// metadata, and (eventually) drive schema-typed encoding/decoding.
pub trait RegistryProvider: Sync {
    /// Return registry metadata (codec, version, schema hash).
    fn info(&self) -> RegistryInfo;

    /// Look up an operation by ID.
    fn op_by_id(&self, op_id: u32) -> Option<&'static OpDef>;

    /// Return all operations (sorted by op_id for deterministic iteration).
    fn all_ops(&self) -> &'static [OpDef];

    /// Return all enums (for validating enum values).
    fn all_enums(&self) -> &'static [EnumDef];

    /// Return all objects (for result validation).
    fn all_objects(&self) -> &'static [ObjectDef];
}
