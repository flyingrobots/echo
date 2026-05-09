// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! METHOD backlog-to-design promotion.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::graph::TaskGraph;
use crate::workspace::MethodWorkspace;

/// Result of promoting a backlog item into a design cycle.
#[derive(Debug, Clone, Serialize)]
pub struct PullResult {
    /// New cycle number, e.g. `0019`.
    pub cycle_number: String,
    /// New cycle directory name, e.g. `0019-xtask-method-pull`.
    pub cycle: String,
    /// Path to the moved design document.
    pub design_path: PathBuf,
}

/// Promote one backlog file into the next numbered design cycle.
///
/// `selector` may be a relative/absolute markdown path, a backlog file stem, a
/// generated METHOD task id such as `M043`, or a native task id such as
/// `T-6-5-1`. If a selector resolves to more than one backlog file, this fails
/// closed and asks for a more specific selector.
pub fn pull_backlog_item(
    workspace: &MethodWorkspace,
    selector: &str,
) -> Result<PullResult, String> {
    let source = resolve_source(workspace, selector)?;
    let source_stem = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| format!("invalid backlog filename: {}", source.display()))?;
    let design_slug = strip_legend_prefix(source_stem);
    let cycle_number = next_cycle_number(workspace)?;
    let cycle = format!("{cycle_number}-{design_slug}");
    let cycle_dir = workspace.design_root().join(&cycle);
    let design_path = cycle_dir.join(format!("{design_slug}.md"));

    if cycle_dir.exists() {
        return Err(format!(
            "refusing to overwrite existing design cycle: {}",
            cycle_dir.display()
        ));
    }

    fs::create_dir_all(&cycle_dir)
        .map_err(|e| format!("failed to create {}: {e}", cycle_dir.display()))?;
    fs::rename(&source, &design_path).map_err(|e| {
        format!(
            "failed to move {} to {}: {e}",
            source.display(),
            design_path.display()
        )
    })?;

    Ok(PullResult {
        cycle_number,
        cycle,
        design_path,
    })
}

fn resolve_source(workspace: &MethodWorkspace, selector: &str) -> Result<PathBuf, String> {
    let backlog_root = workspace.backlog_root();
    let root = backlog_root
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .ok_or_else(|| "failed to resolve METHOD repo root".to_string())?;

    let selector_path = Path::new(selector);
    if selector_path.extension().is_some_and(|ext| ext == "md") {
        let candidate = if selector_path.is_absolute() {
            selector_path.to_path_buf()
        } else {
            root.join(selector_path)
        };
        ensure_backlog_file(workspace, &candidate)?;
        return Ok(candidate);
    }

    let graph = TaskGraph::build(workspace)?;
    let matches = graph
        .tasks
        .iter()
        .filter(|task| {
            task.id == selector
                || task.native_id.as_deref() == Some(selector)
                || Path::new(&task.source_path)
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .is_some_and(|stem| stem == selector || strip_legend_prefix(stem) == selector)
        })
        .map(|task| root.join(&task.source_path))
        .collect::<BTreeSet<_>>();

    match matches.len() {
        0 => Err(format!("no backlog item matches `{selector}`")),
        1 => {
            let Some(path) = matches.into_iter().next() else {
                return Err(format!("no backlog item matches `{selector}`"));
            };
            ensure_backlog_file(workspace, &path)?;
            Ok(path)
        }
        _ => Err(format!(
            "backlog selector `{selector}` is ambiguous; use a source path"
        )),
    }
}

fn ensure_backlog_file(workspace: &MethodWorkspace, path: &Path) -> Result<(), String> {
    let backlog_root = workspace.backlog_root();
    if !path.starts_with(&backlog_root) {
        return Err(format!(
            "{} is not under {}",
            path.display(),
            backlog_root.display()
        ));
    }
    if !path.is_file() {
        return Err(format!("backlog item not found: {}", path.display()));
    }
    Ok(())
}

fn next_cycle_number(workspace: &MethodWorkspace) -> Result<String, String> {
    let mut max = 0_u32;
    let design_root = workspace.design_root();
    let entries = fs::read_dir(&design_root)
        .map_err(|e| format!("failed to read {}: {e}", design_root.display()))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read design entry: {e}"))?;
        if !entry.path().is_dir() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Some(prefix) = name.split_once('-').map(|(prefix, _)| prefix) else {
            continue;
        };
        if prefix.len() == 4 {
            if let Ok(number) = prefix.parse::<u32>() {
                max = max.max(number);
            }
        }
    }

    Ok(format!("{:04}", max + 1))
}

fn strip_legend_prefix(stem: &str) -> String {
    let stripped = stem
        .split_once('_')
        .filter(|(prefix, _)| {
            !prefix.is_empty() && prefix.chars().all(|ch| ch.is_ascii_uppercase())
        })
        .map_or(stem, |(_, suffix)| suffix);
    stripped.replace('_', "-").to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::strip_legend_prefix;

    #[test]
    fn strips_uppercase_legend_prefix() {
        assert_eq!(
            strip_legend_prefix("PLATFORM_xtask-method-pull"),
            "xtask-method-pull"
        );
    }

    #[test]
    fn leaves_unprefixed_stem_as_slug() {
        assert_eq!(strip_legend_prefix("docs-cleanup"), "docs-cleanup");
    }
}
