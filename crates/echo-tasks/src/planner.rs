// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! HTN Planner Logic.

use crate::method::Method;
use crate::slaps::Slaps;
use anyhow::{anyhow, Result};
use std::collections::HashMap;

/// The HTN Planner.
/// Responsible for matching SLAPS intents to Methods and expanding them into a DAG.
pub struct Planner {
    methods: HashMap<String, Method>,
}

impl Default for Planner {
    fn default() -> Self {
        Self::new()
    }
}

impl Planner {
    /// Creates a new Planner with an empty method library.
    pub fn new() -> Self {
        Self {
            methods: HashMap::new(),
        }
    }

    /// Loads a method from a YAML string and adds it to the library.
    pub fn load_method_from_yaml(&mut self, yaml: &str) -> Result<()> {
        let method: Method = serde_yaml::from_str(yaml)?;
        self.methods.insert(method.name.clone(), method);
        Ok(())
    }

    /// Generates a plan (DAG) for the given SLAPS intent.
    ///
    /// Currently returns a string description of the selected method.
    pub fn plan(&self, slaps: &Slaps) -> Result<String> {
        // Find matching method
        let method = self
            .find_matching_method(slaps)
            .ok_or_else(|| anyhow!("No matching method found for intent: {}", slaps.intent))?;

        // Expand (Placeholder for now)
        Ok(format!(
            "Plan for {} using method {}",
            slaps.intent, method.name
        ))
    }

    fn find_matching_method(&self, slaps: &Slaps) -> Option<&Method> {
        // Simple exact intent match
        // In the future: implement robust `applicable_when` logic
        self.methods.values().find(|m| m.intent == slaps.intent)
    }
}
