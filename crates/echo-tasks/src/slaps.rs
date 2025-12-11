// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! SLAPS Intent Definition Language models.

use serde::{Deserialize, Serialize};

/// SLAPS (Scope, Limits, Assumptions, Priorities, Success) Intent Definition.
/// This serves as the system call ABI for the JITOS kernel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slaps {
    /// Version of the SLAPS spec used.
    pub slaps_version: String,
    /// High-level goal category (e.g., "FixBug", "Deploy").
    pub intent: String,
    /// The subject entity being acted upon.
    pub target: Target,
    /// Environmental and state context.
    pub context: Context,
    /// Explicit inclusions and exclusions for the task scope.
    #[serde(default)]
    pub scope: Scope,
    /// Negative constraints that prune the search space.
    #[serde(default)]
    pub constraints: Vec<String>,
    /// Conditions assumed to be true.
    #[serde(default)]
    pub assumptions: Vec<String>,
    /// Optimization directives for the scheduler.
    #[serde(default)]
    pub priorities: Vec<String>,
    /// Verifiable acceptance criteria.
    #[serde(default)]
    pub success_criteria: Vec<SuccessCriteria>,
}

/// The entity being acted upon by the intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    /// Name of the target entity.
    pub name: String,
    /// Type of the target entity (e.g., "Service", "Component").
    #[serde(rename = "type")]
    pub kind: String,
    /// Specific reference (e.g., git commit, SWS ID).
    #[serde(rename = "ref")]
    pub ref_: Option<String>,
}

/// The context in which the intent is executed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    /// Target environment (e.g., "Dev", "Prod").
    pub environment: String,
    /// Associated ticket or issue ID.
    pub ticket_id: Option<String>,
    /// Related links and resources.
    #[serde(default)]
    pub links: Vec<ContextLink>,
}

/// A link to external context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextLink {
    /// URL of the resource.
    pub url: String,
    /// Optional title.
    pub title: Option<String>,
    /// Type of link (e.g., "Spec", "Report").
    #[serde(rename = "type")]
    pub kind: String,
}

/// Scope definition for the task.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Scope {
    /// Items explicitly included in scope.
    #[serde(default)]
    pub include: Vec<String>,
    /// Items explicitly excluded from scope.
    #[serde(default)]
    pub exclude: Vec<String>,
}

/// Verifiable criteria for success.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessCriteria {
    /// Type of check (e.g., "TestPass").
    #[serde(rename = "type")]
    pub kind: String,
    /// Value to check against.
    pub value: String,
}
