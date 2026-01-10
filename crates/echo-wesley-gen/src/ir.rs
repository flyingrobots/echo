// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Minimal Wesley IR structs used by echo-wesley-gen.

use serde::Deserialize;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct WesleyIR {
    #[serde(default)]
    pub ir_version: Option<String>,
    #[serde(default)]
    pub generated_by: Option<GeneratedBy>,
    #[serde(default)]
    pub schema_sha256: Option<String>,
    #[serde(default)]
    pub types: Vec<TypeDefinition>,
    #[serde(default)]
    pub ops: Vec<OpDefinition>,
    #[serde(default)]
    pub codec_id: Option<String>,
    #[serde(default)]
    pub registry_version: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct GeneratedBy {
    #[allow(dead_code)]
    pub tool: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub version: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct TypeDefinition {
    pub name: String,
    pub kind: TypeKind,
    #[serde(default)]
    pub fields: Vec<FieldDefinition>,
    #[serde(default)]
    pub values: Vec<String>, // For enums
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TypeKind {
    Object,
    Enum,
    Scalar,
    Interface,
    Union,
    InputObject,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OpKind {
    Query,
    Mutation,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct OpDefinition {
    pub kind: OpKind,
    pub name: String,
    pub op_id: u32,
    #[serde(default)]
    pub args: Vec<ArgDefinition>,
    pub result_type: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ArgDefinition {
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
    pub required: bool,
    #[serde(default)]
    pub list: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct FieldDefinition {
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
    pub required: bool,
    #[serde(default)]
    pub list: bool,
}
