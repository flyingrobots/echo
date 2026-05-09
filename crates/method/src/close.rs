// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! METHOD cycle closeout scaffolding.

use std::fs;
use std::path::PathBuf;

use serde::Serialize;

use crate::status::{ActiveCycle, StatusReport};
use crate::workspace::MethodWorkspace;

/// Result of creating a cycle retro scaffold.
#[derive(Debug, Clone, Serialize)]
pub struct CloseCycleResult {
    /// Full cycle directory name, e.g. `0018-echo-optics-api-design`.
    pub cycle: String,
    /// Retro markdown path.
    pub retro_path: PathBuf,
    /// Witness artifact directory path.
    pub witness_dir: PathBuf,
}

/// Create a retro template and witness directory for an active cycle.
///
/// When `selector` is `None`, the most recent active cycle is selected by
/// cycle number. A selector may be either the full cycle directory name or just
/// the numeric prefix.
pub fn close_cycle(
    workspace: &MethodWorkspace,
    selector: Option<&str>,
) -> Result<CloseCycleResult, String> {
    if let Some(raw) = selector {
        if let Some(existing_retro) = existing_retro_for_selector(workspace, raw)? {
            return Err(format!(
                "refusing to overwrite existing retro directory: {}",
                existing_retro.display()
            ));
        }
    }

    let mut cycles = StatusReport::build(workspace)?.active_cycles;
    cycles.sort_by_key(cycle_name);

    let cycle = match selector {
        Some(raw) => find_cycle(&cycles, raw)?,
        None => cycles
            .last()
            .cloned()
            .ok_or_else(|| "no active METHOD cycles found".to_string())?,
    };

    let cycle_dir_name = cycle_name(&cycle);
    let retro_dir = workspace.retro_root().join(&cycle_dir_name);
    let retro_path = retro_dir.join("retro.md");
    let witness_dir = retro_dir.join("witness");

    if retro_dir.exists() {
        return Err(format!(
            "refusing to overwrite existing retro directory: {}",
            retro_dir.display()
        ));
    }

    fs::create_dir_all(&witness_dir)
        .map_err(|e| format!("failed to create {}: {e}", witness_dir.display()))?;
    fs::write(&retro_path, retro_template(&cycle))
        .map_err(|e| format!("failed to write {}: {e}", retro_path.display()))?;

    Ok(CloseCycleResult {
        cycle: cycle_dir_name,
        retro_path,
        witness_dir,
    })
}

fn existing_retro_for_selector(
    workspace: &MethodWorkspace,
    raw: &str,
) -> Result<Option<PathBuf>, String> {
    let retro_root = workspace.retro_root();
    let entries = match fs::read_dir(&retro_root) {
        Ok(entries) => entries,
        Err(err) => return Err(format!("failed to read {}: {err}", retro_root.display())),
    };

    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read retro entry: {e}"))?;
        if !entry.path().is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == raw
            || name
                .strip_prefix(raw)
                .is_some_and(|suffix| suffix.starts_with('-'))
        {
            return Ok(Some(entry.path()));
        }
    }

    Ok(None)
}

fn find_cycle(cycles: &[ActiveCycle], raw: &str) -> Result<ActiveCycle, String> {
    let matches = cycles
        .iter()
        .filter(|cycle| cycle.number == raw || cycle_name(cycle) == raw)
        .cloned()
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [cycle] => Ok(cycle.clone()),
        [] => Err(format!("no active METHOD cycle matches `{raw}`")),
        _ => Err(format!("METHOD cycle selector `{raw}` is ambiguous")),
    }
}

fn cycle_name(cycle: &ActiveCycle) -> String {
    format!("{}-{}", cycle.number, cycle.slug)
}

fn retro_template(cycle: &ActiveCycle) -> String {
    let cycle_name = cycle_name(cycle);
    format!(
        "<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->\n\
         <!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->\n\
         \n\
         # Retro: {cycle_name}\n\
         \n\
         Cycle: `{cycle_name}`\n\
         Design: [`docs/design/{cycle_name}/`](../../../design/{cycle_name}/)\n\
         Witness: [`witness/`](./witness/)\n\
         \n\
         ## Outcome\n\
         \n\
         - Status: TODO\n\
         - Summary: TODO\n\
         \n\
         ## Evidence\n\
         \n\
         - TODO\n\
         \n\
         ## Drift Check\n\
         \n\
         - TODO\n\
         \n\
         ## Follow-Up\n\
         \n\
         - TODO\n"
    )
}
