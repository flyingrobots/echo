// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! METHOD playback-question drift checks.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::status::{ActiveCycle, StatusReport};
use crate::workspace::MethodWorkspace;

/// Drift coverage report for one cycle.
#[derive(Debug, Clone, Serialize)]
pub struct DriftReport {
    /// Cycle directory name.
    pub cycle: String,
    /// Design markdown files inspected.
    pub design_paths: Vec<PathBuf>,
    /// Playback-question coverage rows.
    pub questions: Vec<PlaybackQuestionCoverage>,
}

impl DriftReport {
    /// Number of playback questions with no matching test coverage.
    pub fn missing_count(&self) -> usize {
        self.questions
            .iter()
            .filter(|question| question.matches.is_empty())
            .count()
    }

    /// Whether all discovered playback questions have visible test coverage.
    pub fn covered(&self) -> bool {
        self.missing_count() == 0
    }
}

/// One playback question and the tests that appear to cover it.
#[derive(Debug, Clone, Serialize)]
pub struct PlaybackQuestionCoverage {
    /// Question text extracted from the design doc.
    pub question: String,
    /// Relative test files with matching names/descriptions.
    pub matches: Vec<PathBuf>,
}

/// Check playback questions for an active cycle against committed tests.
///
/// `selector` may be a full cycle directory name or just the numeric prefix.
/// When omitted, the most recent active cycle is checked.
pub fn drift_report(
    workspace: &MethodWorkspace,
    selector: Option<&str>,
) -> Result<DriftReport, String> {
    let cycle = select_cycle(workspace, selector)?;
    let cycle_name = cycle_name(&cycle);
    let cycle_dir = workspace.design_root().join(&cycle_name);
    let design_paths = collect_markdown_files(&cycle_dir)?;
    let repo_root = workspace
        .backlog_root()
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "failed to resolve METHOD repo root".to_string())?;
    let test_files = collect_test_files(&repo_root)?;

    let mut questions = Vec::new();
    for path in &design_paths {
        let text = fs::read_to_string(path)
            .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
        for question in extract_playback_questions(&text) {
            let matches = matching_test_files(&repo_root, &test_files, &question)?;
            questions.push(PlaybackQuestionCoverage { question, matches });
        }
    }

    Ok(DriftReport {
        cycle: cycle_name,
        design_paths,
        questions,
    })
}

fn select_cycle(
    workspace: &MethodWorkspace,
    selector: Option<&str>,
) -> Result<ActiveCycle, String> {
    let mut cycles = StatusReport::build(workspace)?.active_cycles;
    cycles.sort_by_key(cycle_name);

    match selector {
        Some(raw) => {
            let matches = cycles
                .into_iter()
                .filter(|cycle| cycle.number == raw || cycle_name(cycle) == raw)
                .collect::<Vec<_>>();
            match matches.as_slice() {
                [cycle] => Ok(cycle.clone()),
                [] => Err(format!("no active METHOD cycle matches `{raw}`")),
                _ => Err(format!("METHOD cycle selector `{raw}` is ambiguous")),
            }
        }
        None => cycles
            .last()
            .cloned()
            .ok_or_else(|| "no active METHOD cycles found".to_string()),
    }
}

fn cycle_name(cycle: &ActiveCycle) -> String {
    format!("{}-{}", cycle.number, cycle.slug)
}

fn collect_markdown_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    collect_files(root, &mut out, |path| {
        path.extension().is_some_and(|ext| ext == "md")
    })?;
    out.sort();
    Ok(out)
}

fn collect_test_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    for dir in ["crates", "xtask", "scripts", "apps", "packages"] {
        let path = root.join(dir);
        if path.is_dir() {
            collect_files(&path, &mut out, is_test_file)?;
        }
    }
    out.sort();
    Ok(out)
}

fn collect_files(
    root: &Path,
    out: &mut Vec<PathBuf>,
    include: fn(&Path) -> bool,
) -> Result<(), String> {
    let entries =
        fs::read_dir(root).map_err(|e| format!("failed to read {}: {e}", root.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read directory entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            if matches!(name, "target" | "node_modules" | "dist" | ".git") {
                continue;
            }
            collect_files(&path, out, include)?;
        } else if include(&path) {
            out.push(path);
        }
    }
    Ok(())
}

fn is_test_file(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
        return false;
    };
    let Some(path_text) = path.to_str() else {
        return false;
    };
    matches!(
        ext,
        "rs" | "ts" | "tsx" | "js" | "mjs" | "sh" | "bats" | "md"
    ) && (path_text.contains("/test")
        || path_text.contains("/tests")
        || path_text.contains("_test.")
        || path_text.contains(".test.")
        || path_text.contains(".spec."))
}

fn extract_playback_questions(text: &str) -> Vec<String> {
    let mut questions = Vec::new();
    let mut in_playback = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            let lower = trimmed.to_ascii_lowercase();
            in_playback = lower.contains("playback") && !lower.contains("not playback");
            continue;
        }
        if !in_playback {
            continue;
        }
        if let Some(question) = table_question(trimmed).or_else(|| list_question(trimmed)) {
            questions.push(question);
        }
    }

    questions
}

fn table_question(line: &str) -> Option<String> {
    if !line.starts_with('|') || line.contains("---") {
        return None;
    }
    line.split('|')
        .map(str::trim)
        .find(|cell| cell.contains('?'))
        .map(clean_question)
        .filter(|question| !question.is_empty())
}

fn list_question(line: &str) -> Option<String> {
    if !line.contains('?') {
        return None;
    }
    let without_checkbox = line
        .trim_start_matches(|ch: char| {
            ch == '-' || ch == '*' || ch == '+' || ch.is_ascii_digit() || ch == '.' || ch == ' '
        })
        .trim_start_matches("[ ]")
        .trim_start_matches("[x]")
        .trim_start_matches("[X]")
        .trim();
    Some(clean_question(without_checkbox)).filter(|question| !question.is_empty())
}

fn clean_question(text: &str) -> String {
    text.trim()
        .trim_matches('|')
        .trim()
        .trim_matches('"')
        .trim()
        .to_string()
}

fn matching_test_files(
    repo_root: &Path,
    test_files: &[PathBuf],
    question: &str,
) -> Result<Vec<PathBuf>, String> {
    let question_norm = normalize(question);
    let terms = significant_terms(&question_norm);
    let mut matches = Vec::new();

    for path in test_files {
        let text = fs::read_to_string(path)
            .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
        let haystack = normalize(&text);
        let exact = !question_norm.is_empty() && haystack.contains(&question_norm);
        let term_match = terms.len() >= 3 && terms.iter().all(|term| haystack.contains(term));
        if exact || term_match {
            matches.push(path.strip_prefix(repo_root).unwrap_or(path).to_path_buf());
        }
    }

    Ok(matches)
}

fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(' ');
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn significant_terms(normalized: &str) -> Vec<String> {
    normalized
        .split_whitespace()
        .filter(|term| term.len() >= 4)
        .filter(|term| {
            !matches!(
                *term,
                "does"
                    | "with"
                    | "from"
                    | "that"
                    | "this"
                    | "have"
                    | "what"
                    | "when"
                    | "where"
                    | "which"
                    | "should"
                    | "would"
                    | "could"
                    | "agent"
                    | "human"
                    | "test"
                    | "tests"
            )
        })
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::extract_playback_questions;

    #[test]
    fn extracts_numbered_and_table_playback_questions() {
        let questions = extract_playback_questions(
            r"# Design

## Human playback

1. Does the command exit 0?

## Agent playback

| Question | Expected |
| -------- | -------- |
| Can JSON be parsed? | Yes |

## Not playback

1. Does this get ignored?
",
        );

        assert_eq!(
            questions,
            vec!["Does the command exit 0?", "Can JSON be parsed?"]
        );
    }
}
