// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! STATUS command: backlog lane counts, active cycles, legend load.

use std::collections::BTreeMap;
use std::fs;

use serde::Serialize;

use crate::workspace::{MethodWorkspace, LANES};

/// Full status report for a METHOD workspace.
#[derive(Debug, Serialize)]
pub struct StatusReport {
    /// File count per backlog lane (e.g., "asap" -> 15).
    pub lanes: BTreeMap<String, usize>,
    /// Cycles with a design doc but no matching retro.
    pub active_cycles: Vec<ActiveCycle>,
    /// Backlog item count per legend prefix across all lanes.
    pub legend_load: BTreeMap<String, usize>,
    /// Total backlog items across all lanes.
    pub total_items: usize,
}

/// An active (not yet retro'd) cycle.
#[derive(Clone, Debug, Serialize)]
pub struct ActiveCycle {
    /// Cycle number (e.g., "0002").
    pub number: String,
    /// Cycle slug (e.g., "xtask-method-status").
    pub slug: String,
    /// Legend prefix if the cycle's design doc came from a prefixed backlog item.
    pub legend: Option<String>,
}

impl StatusReport {
    /// Build a status report from a METHOD workspace.
    pub fn build(workspace: &MethodWorkspace) -> Result<Self, String> {
        let mut lanes = BTreeMap::new();
        let mut legend_load: BTreeMap<String, usize> = BTreeMap::new();
        let mut total_items = 0;

        for lane in LANES {
            let lane_path = workspace.backlog_root().join(lane);
            let count = count_md_files(&lane_path);
            collect_legend_prefixes(&lane_path, &mut legend_load);
            total_items += count;
            lanes.insert((*lane).to_string(), count);
        }

        let active_cycles = find_active_cycles(workspace);

        Ok(Self {
            lanes,
            active_cycles,
            legend_load,
            total_items,
        })
    }
}

/// Count `.md` files in a directory (non-recursive).
fn count_md_files(dir: &std::path::Path) -> usize {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        })
        .count()
}

/// Parse `LEGEND_` prefixes from `.md` filenames and count per legend.
fn collect_legend_prefixes(dir: &std::path::Path, load: &mut BTreeMap<String, usize>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        {
            let filename = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            let legend = extract_legend_prefix(filename);
            *load.entry(legend).or_insert(0) += 1;
        }
    }
}

/// Extract the legend prefix from a filename like `KERNEL_foo` -> `"KERNEL"`.
/// Returns `"(none)"` if no prefix is found.
fn extract_legend_prefix(stem: &str) -> String {
    if let Some(idx) = stem.find('_') {
        let prefix = &stem[..idx];
        // A legend prefix is all uppercase.
        if !prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_uppercase()) {
            return prefix.to_string();
        }
    }
    "(none)".to_string()
}

/// Find cycles that have a design doc but no matching retro.
fn find_active_cycles(workspace: &MethodWorkspace) -> Vec<ActiveCycle> {
    let design_root = workspace.design_root();
    let retro_root = workspace.retro_root();

    let Ok(entries) = fs::read_dir(&design_root) else {
        return Vec::new();
    };

    let retro_dirs: Vec<String> = fs::read_dir(&retro_root)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().to_str().map(String::from))
        .collect();

    entries
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let name = e.file_name().to_str()?.to_string();
            // Cycle dirs look like "0002-xtask-method-status"
            let dash_pos = name.find('-')?;
            let number = name[..dash_pos].to_string();
            let slug = name[dash_pos + 1..].to_string();

            // Check if retro exists for this cycle (match by full dir name).
            let has_retro = retro_dirs.iter().any(|r| r == &name);
            if has_retro {
                return None;
            }

            Some(ActiveCycle {
                number,
                slug,
                legend: None,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_legend_kernel() {
        assert_eq!(extract_legend_prefix("KERNEL_foo"), "KERNEL");
    }

    #[test]
    fn extract_legend_none() {
        assert_eq!(extract_legend_prefix("some-idea"), "(none)");
    }

    #[test]
    fn extract_legend_mixed_case_not_legend() {
        assert_eq!(extract_legend_prefix("Kernel_foo"), "(none)");
    }
}
