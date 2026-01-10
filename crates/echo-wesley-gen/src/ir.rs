// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Minimal Wesley IR structs used by echo-wesley-gen.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct WesleyIR {
    #[serde(default)]
    pub types: Vec<TypeDefinition>,
}

#[derive(Debug, Deserialize)]
pub struct TypeDefinition {
    pub name: String,
    pub kind: TypeKind,
    #[serde(default)]
    pub fields: Vec<FieldDefinition>,
    #[serde(default)]
    pub values: Vec<String>, // For enums
}

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

#[derive(Debug, Deserialize)]
pub struct FieldDefinition {
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
    pub required: bool,
    #[serde(default)]
    pub list: bool,
}
