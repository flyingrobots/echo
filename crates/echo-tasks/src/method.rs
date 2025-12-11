// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! HTN Method Definition models.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An HTN Method definition.
/// Represents a reusable decomposition pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Method {
    /// Unique identifier for the method.
    pub name: String,
    /// The high-level intent this method satisfies.
    pub intent: String,
    /// Semantic version of the method.
    pub version: String,
    /// Human-readable description.
    pub description: String,
    /// Conditions under which this method applies.
    pub applicable_when: Applicability,
    /// Preconditions that must hold before expansion.
    #[serde(default)]
    pub preconditions: Vec<Precondition>,
    /// Input variable definitions.
    #[serde(default)]
    pub variables: Vec<VariableDef>,
    /// The recipe steps (DAG nodes).
    pub subtasks: Vec<Subtask>,
    /// Deterministic recovery actions.
    #[serde(default)]
    pub on_failure: Vec<FailureAction>,
}

/// Defines when a method can be selected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Applicability {
    /// Target type required (or "*" for any).
    pub target_type: String,
    /// Allowed environments.
    #[serde(default)]
    pub environment: Vec<String>,
    /// Required constraints in the SLAPS intent.
    #[serde(default)]
    pub constraints: Vec<String>,
    /// Selection priority (lower is better, or higher is fallback depending on logic).
    #[serde(default)]
    pub priority: i32,
}

/// A check that must pass before method expansion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Precondition {
    /// The predicate to evaluate.
    pub check: String,
}

/// Input variable definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableDef {
    /// Variable name.
    pub name: String,
    /// Source expression (e.g., "target.name").
    pub source: Option<String>,
    /// Default value if source is missing.
    pub default: Option<String>,
}

/// A step in the method, representing a task node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subtask {
    /// Local identifier for the subtask.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Type of subtask ("method" or "primitive").
    #[serde(rename = "type")]
    pub kind: String,
    /// Reference to the method or primitive to execute.
    #[serde(rename = "ref")]
    pub ref_: String,
    /// Arguments passed to the subtask.
    #[serde(default)]
    pub args: HashMap<String, String>,
    /// List of subtask IDs that must complete before this one starts.
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Conditional execution expression.
    #[serde(rename = "if")]
    pub if_: Option<String>,
}

/// Action to take on failure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureAction {
    /// Type of action (e.g., "Rollback").
    pub action: String,
    /// Reference to the rollback method.
    #[serde(rename = "ref")]
    pub ref_: String,
}
