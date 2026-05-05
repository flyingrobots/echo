// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Backlog task graph parsing, matrix rendering, and scheduling queries.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::Serialize;

use crate::workspace::MethodWorkspace;

const GRAPH_LANES: &[&str] = &["asap", "up-next", "inbox", "cool-ideas", "bad-code"];

const TASK_SECTION_PREFIX: &str = "## T-";

/// A parsed METHOD backlog graph.
#[derive(Debug, Clone, Serialize)]
pub struct TaskGraph {
    /// Parsed task rows, in deterministic matrix order.
    pub tasks: Vec<TaskNode>,
    /// Direct dependency edges from prerequisite to dependent.
    pub edges: Vec<TaskEdge>,
    /// Dependency-shaped references that did not resolve to backlog task rows.
    pub external_refs: Vec<ExternalDependencyRef>,
}

/// One schedulable task row.
#[derive(Debug, Clone, Serialize)]
pub struct TaskNode {
    /// Matrix/task id, e.g. `M001`.
    pub id: String,
    /// Backlog lane.
    pub lane: String,
    /// Native legacy task id when the source is an internal `## T-*` section.
    pub native_id: Option<String>,
    /// Human title.
    pub title: String,
    /// Source markdown path, relative to repo root.
    pub source_path: String,
    /// Optional markdown anchor for internal `## T-*` section tasks.
    pub anchor: Option<String>,
    /// Whether this task is already complete but still present as a backlog
    /// coordination/index card.
    pub completed: bool,
}

/// A directed dependency edge.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub struct TaskEdge {
    /// Prerequisite task id.
    pub prerequisite: String,
    /// Dependent task id.
    pub dependent: String,
}

/// A dependency-shaped reference that points outside the backlog graph.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub struct ExternalDependencyRef {
    /// Task containing the reference.
    pub task_id: String,
    /// Field where it was found.
    pub field: String,
    /// Raw reference text.
    pub reference: String,
}

/// A task with scheduling metrics.
#[derive(Debug, Clone, Serialize)]
pub struct FrontierTask {
    /// Task node.
    pub task: TaskNode,
    /// Number of tasks transitively unblocked by this task.
    pub downstream_count: usize,
    /// Longest downstream chain length from this task, including itself.
    pub downstream_depth: usize,
}

/// Summary of graph health and scheduling state.
#[derive(Debug, Clone, Serialize)]
pub struct GraphSummary {
    /// Total task rows.
    pub tasks: usize,
    /// Direct in-graph dependency edges.
    pub edges: usize,
    /// Open tasks with no in-graph blockers.
    pub open_tasks: usize,
    /// Completed tasks still present in the backlog graph.
    pub completed_tasks: usize,
    /// External/unresolved dependency references.
    pub external_refs: usize,
    /// Task counts by backlog lane.
    pub lanes: BTreeMap<String, usize>,
}

impl TaskGraph {
    /// Build the task graph from `docs/method/backlog/**`.
    pub fn build(workspace: &MethodWorkspace) -> Result<Self, String> {
        let root = workspace
            .backlog_root()
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        let backlog_root = workspace.backlog_root();
        let mut files = collect_markdown_files(&backlog_root)?;
        files.sort_by(|a, b| {
            let a_lane = lane_for(&backlog_root, a);
            let b_lane = lane_for(&backlog_root, b);
            lane_rank(&a_lane)
                .cmp(&lane_rank(&b_lane))
                .then_with(|| path_string(a).cmp(&path_string(b)))
        });

        let mut tasks = Vec::new();
        let mut file_to_tasks: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        let mut file_text: BTreeMap<String, String> = BTreeMap::new();

        for path in &files {
            let text = fs::read_to_string(path)
                .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
            let rel = relative_path(&root, path)?;
            let lane = lane_for(&backlog_root, path);
            let sections = task_sections(&text);
            let mut indexes = Vec::new();

            if sections.is_empty() {
                indexes.push(tasks.len());
                tasks.push(TaskNode {
                    id: String::new(),
                    lane,
                    native_id: None,
                    title: h1_title(&text).unwrap_or_else(|| fallback_title(path)),
                    source_path: rel.clone(),
                    anchor: None,
                    completed: status_is_complete(&text),
                });
            } else {
                let sections_with_body = task_sections_with_body(&text);
                for (idx, section) in sections.into_iter().enumerate() {
                    indexes.push(tasks.len());
                    let heading = format!("{} {}", section.native_id, section.title);
                    tasks.push(TaskNode {
                        id: String::new(),
                        lane: lane.clone(),
                        native_id: Some(section.native_id),
                        title: section.title,
                        source_path: rel.clone(),
                        anchor: Some(slugify_heading(&heading)),
                        completed: sections_with_body
                            .get(idx)
                            .is_some_and(|section| status_is_complete(&section.body)),
                    });
                }
            }

            file_to_tasks.insert(rel.clone(), indexes);
            file_text.insert(rel, text);
        }

        for (idx, task) in tasks.iter_mut().enumerate() {
            task.id = format!("M{:03}", idx + 1);
        }

        let mut id_by_native: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (idx, task) in tasks.iter().enumerate() {
            if let Some(native_id) = &task.native_id {
                id_by_native.entry(native_id.clone()).or_default().push(idx);
            }
        }

        let mut slug_aliases: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (rel, indexes) in &file_to_tasks {
            let stem = Path::new(rel)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if !stem.is_empty() {
                slug_aliases
                    .entry(stem.clone())
                    .or_default()
                    .extend(indexes.iter().copied());
            }
            if let Some((_, suffix)) = stem.split_once('_') {
                slug_aliases
                    .entry(suffix.to_string())
                    .or_default()
                    .extend(indexes.iter().copied());
            }
        }

        let mut edge_set: BTreeSet<TaskEdge> = BTreeSet::new();
        let mut external_set: BTreeSet<ExternalDependencyRef> = BTreeSet::new();

        for (rel, text) in &file_text {
            let Some(source_indexes) = file_to_tasks.get(rel) else {
                continue;
            };
            for link in file_dependency_links(text) {
                let Some(dep_rel) = resolve_backlog_link(rel, &link) else {
                    for source_idx in source_indexes {
                        external_set.insert(ExternalDependencyRef {
                            task_id: tasks[*source_idx].id.clone(),
                            field: "Depends on".to_string(),
                            reference: link.clone(),
                        });
                    }
                    continue;
                };
                if let Some(dep_indexes) = file_to_tasks.get(&dep_rel) {
                    for source_idx in source_indexes {
                        for dep_idx in dep_indexes {
                            if source_idx != dep_idx {
                                edge_set.insert(TaskEdge {
                                    prerequisite: tasks[*dep_idx].id.clone(),
                                    dependent: tasks[*source_idx].id.clone(),
                                });
                            }
                        }
                    }
                } else {
                    for source_idx in source_indexes {
                        external_set.insert(ExternalDependencyRef {
                            task_id: tasks[*source_idx].id.clone(),
                            field: "Depends on".to_string(),
                            reference: link.clone(),
                        });
                    }
                }
            }
        }

        for (rel, text) in &file_text {
            let Some(source_indexes) = file_to_tasks.get(rel) else {
                continue;
            };
            if source_indexes.len() == 1 && tasks[source_indexes[0]].native_id.is_none() {
                continue;
            }
            let sections = task_sections_with_body(text);
            for (section_idx, section) in sections.iter().enumerate() {
                let Some(source_idx) = source_indexes.get(section_idx) else {
                    continue;
                };
                for raw in blocked_by_values(&section.body) {
                    add_blocker_edges(
                        raw,
                        *source_idx,
                        &tasks,
                        &id_by_native,
                        &slug_aliases,
                        &mut edge_set,
                    );
                }
                for raw in blocking_values(&section.body) {
                    for token in task_tokens(raw) {
                        if let Some(target_indexes) = id_by_native.get(&token) {
                            for target_idx in target_indexes {
                                if *target_idx != *source_idx {
                                    edge_set.insert(TaskEdge {
                                        prerequisite: tasks[*source_idx].id.clone(),
                                        dependent: tasks[*target_idx].id.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        let edges = edge_set.into_iter().collect();
        let external_refs = external_set.into_iter().collect();
        let graph = Self {
            tasks,
            edges,
            external_refs,
        };
        graph.ensure_acyclic()?;
        Ok(graph)
    }

    /// Return graph summary metrics.
    pub fn summary(&self) -> GraphSummary {
        let open_tasks = self.frontier().len();
        let mut lanes = BTreeMap::new();
        for task in &self.tasks {
            *lanes.entry(task.lane.clone()).or_insert(0) += 1;
        }
        GraphSummary {
            tasks: self.tasks.len(),
            edges: self.edges.len(),
            open_tasks,
            completed_tasks: self.tasks.iter().filter(|task| task.completed).count(),
            external_refs: self.external_refs.len(),
            lanes,
        }
    }

    /// Return open frontier tasks, ranked by lane and downstream impact.
    pub fn frontier(&self) -> Vec<FrontierTask> {
        let incoming = self.active_incoming_counts();
        let downstream_count = self.downstream_counts();
        let downstream_depth = self.downstream_depths();
        let mut frontier = self
            .tasks
            .iter()
            .filter(|task| !task.completed)
            .filter(|task| incoming.get(&task.id).copied().unwrap_or(0) == 0)
            .map(|task| FrontierTask {
                task: task.clone(),
                downstream_count: downstream_count.get(&task.id).copied().unwrap_or(0),
                downstream_depth: downstream_depth.get(&task.id).copied().unwrap_or(1),
            })
            .collect::<Vec<_>>();

        frontier.sort_by(|a, b| {
            lane_rank(&a.task.lane)
                .cmp(&lane_rank(&b.task.lane))
                .then_with(|| b.downstream_count.cmp(&a.downstream_count))
                .then_with(|| b.downstream_depth.cmp(&a.downstream_depth))
                .then_with(|| a.task.id.cmp(&b.task.id))
        });
        frontier
    }

    /// Return the unweighted longest dependency path.
    pub fn critical_path(&self) -> Vec<TaskNode> {
        let outgoing = self.active_outgoing_map();
        let order = self.topological_order().unwrap_or_default();
        let mut best_len: BTreeMap<String, usize> = BTreeMap::new();
        let mut next: BTreeMap<String, String> = BTreeMap::new();

        for task_id in order.iter().rev() {
            let mut best = 1usize;
            let mut best_next = None;
            if let Some(children) = outgoing.get(task_id) {
                for child in children {
                    let candidate = 1 + best_len.get(child).copied().unwrap_or(1);
                    if candidate > best {
                        best = candidate;
                        best_next = Some(child.clone());
                    }
                }
            }
            best_len.insert(task_id.clone(), best);
            if let Some(child) = best_next {
                next.insert(task_id.clone(), child);
            }
        }

        let Some(start) = self
            .tasks
            .iter()
            .filter(|task| !task.completed)
            .map(|task| task.id.clone())
            .max_by_key(|id| best_len.get(id).copied().unwrap_or(1))
        else {
            return Vec::new();
        };

        let by_id = self.task_by_id();
        let mut path = Vec::new();
        let mut current = start;
        while let Some(task) = by_id.get(&current) {
            path.push((*task).clone());
            let Some(next_id) = next.get(&current) else {
                break;
            };
            current = next_id.clone();
        }
        path
    }

    /// Render `docs/method/task-matrix.md`.
    pub fn render_matrix_markdown(&self) -> String {
        let summary = self.summary();
        let mut lines = Vec::new();
        lines.push(
            "<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->".to_string(),
        );
        lines.push(
            "<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->".to_string(),
        );
        lines.push(String::new());
        lines.push("# METHOD Task Matrix".to_string());
        lines.push(String::new());
        lines.push(
            "Rows are dependent tasks. Columns are prerequisite tasks. A cell contains".to_string(),
        );
        lines.push(
            "`depends on` when the row task directly depends on the column task.".to_string(),
        );
        lines.push(String::new());
        lines.push(
            "This matrix is generated from `docs/method/backlog/**`. If a backlog file".to_string(),
        );
        lines.push(
            "contains `## T-...` task sections, each section is a task row. Otherwise,".to_string(),
        );
        lines.push(
            "the backlog file itself is one task row. File-level `Depends on:` links are"
                .to_string(),
        );
        lines.push("included when they resolve to another backlog task. Section-level".to_string());
        lines.push(
            "`Blocked By:` / `Blocking:` task IDs are included when they resolve to a".to_string(),
        );
        lines.push("task row.".to_string());
        lines.push(String::new());
        lines.push(
            "Blank cells mean no direct dependency was found. Transitive dependencies are"
                .to_string(),
        );
        lines.push("not expanded.".to_string());
        lines.push(String::new());
        lines.push("## Summary".to_string());
        lines.push(String::new());
        lines.push(format!("- Matrix rows/columns: {}", summary.tasks));
        lines.push(format!(
            "- Direct in-matrix dependency edges: {}",
            summary.edges
        ));
        lines.push(format!(
            "- Completed backlog tasks: {}",
            summary.completed_tasks
        ));
        for lane in GRAPH_LANES {
            if let Some(count) = summary.lanes.get(*lane) {
                lines.push(format!("- `{lane}` tasks: {count}"));
            }
        }
        lines.push(String::new());
        lines.push("## Task IDs".to_string());
        lines.push(String::new());
        for task in &self.tasks {
            let native = task
                .native_id
                .as_ref()
                .map(|id| format!(" `{id}`"))
                .unwrap_or_default();
            let task_link = task_markdown_link(task);
            lines.push(format!(
                "- `{}` `{}`{}: {} (source: [`{}`]({}))",
                task.id, task.lane, native, task_link, task.source_path, task.source_path
            ));
        }
        lines.push(String::new());
        lines.push("## Matrix".to_string());
        lines.push(String::new());
        lines.push("```csv".to_string());
        lines.push(self.render_matrix_csv().trim_end().to_string());
        lines.push("```".to_string());
        lines.push(String::new());

        if !self.external_refs.is_empty() {
            lines.push("## External Or Unresolved Dependency References".to_string());
            lines.push(String::new());
            lines.push(
                "These references were found in dependency-shaped fields but do not resolve to"
                    .to_string(),
            );
            lines.push("a task row in `docs/method/backlog/**`.".to_string());
            lines.push(String::new());
            for reference in &self.external_refs {
                lines.push(format!(
                    "- `{}` {}: `{}`",
                    reference.task_id, reference.field, reference.reference
                ));
            }
            lines.push(String::new());
        }

        lines.join("\n")
    }

    /// Render standalone CSV matrix.
    pub fn render_matrix_csv(&self) -> String {
        let mut lines = Vec::new();
        let mut header = Vec::with_capacity(self.tasks.len() + 1);
        header.push("task".to_string());
        header.extend(self.tasks.iter().map(|task| task.id.clone()));
        lines.push(header.join(","));

        let edge_set = self
            .edges
            .iter()
            .map(|edge| (edge.dependent.as_str(), edge.prerequisite.as_str()))
            .collect::<BTreeSet<_>>();

        for row_task in &self.tasks {
            let mut row = Vec::with_capacity(self.tasks.len() + 1);
            row.push(row_task.id.clone());
            for col_task in &self.tasks {
                if edge_set.contains(&(row_task.id.as_str(), col_task.id.as_str())) {
                    row.push("depends on".to_string());
                } else {
                    row.push(String::new());
                }
            }
            lines.push(row.join(","));
        }

        format!("{}\n", lines.join("\n"))
    }

    /// Render Graphviz DOT.
    pub fn render_dot(&self) -> String {
        let summary = self.summary();
        let incoming = self.active_incoming_counts();
        let open = self
            .frontier()
            .into_iter()
            .map(|task| task.task.id)
            .collect::<BTreeSet<_>>();

        let mut lines = Vec::new();
        lines.push("digraph method_task_dag {".to_string());
        lines.push("  graph [".to_string());
        lines.push(format!(
            "    label=\"METHOD Backlog Task Dependency DAG\\nopen tasks: {} / {}; dependency edges: {}\",",
            summary.open_tasks, summary.tasks, summary.edges
        ));
        lines.push("    labelloc=t,".to_string());
        lines.push("    fontsize=24,".to_string());
        lines.push("    fontname=\"Inter, Helvetica, Arial\",".to_string());
        lines.push("    rankdir=LR,".to_string());
        lines.push("    bgcolor=\"white\",".to_string());
        lines.push("    splines=ortho,".to_string());
        lines.push("    overlap=false,".to_string());
        lines.push("    nodesep=0.35,".to_string());
        lines.push("    ranksep=0.75".to_string());
        lines.push("  ];".to_string());
        lines.push("  node [".to_string());
        lines.push("    shape=box,".to_string());
        lines.push("    style=\"rounded,filled\",".to_string());
        lines.push("    fontsize=10,".to_string());
        lines.push("    fontname=\"Inter, Helvetica, Arial\",".to_string());
        lines.push("    margin=\"0.08,0.06\",".to_string());
        lines.push("    penwidth=1.2".to_string());
        lines.push("  ];".to_string());
        lines.push("  edge [".to_string());
        lines.push("    color=\"#dc2626\",".to_string());
        lines.push("    arrowsize=0.8,".to_string());
        lines.push("    penwidth=2.6".to_string());
        lines.push("  ];".to_string());
        lines.push(String::new());

        for lane in GRAPH_LANES {
            let lane_tasks = self
                .tasks
                .iter()
                .filter(|task| task.lane == *lane)
                .collect::<Vec<_>>();
            if lane_tasks.is_empty() {
                continue;
            }
            let (fill, border) = lane_colors(lane);
            lines.push(format!(
                "  subgraph \"cluster_{}\" {{",
                lane.replace('-', "_")
            ));
            lines.push(format!("    label=\"{lane}\";"));
            lines.push("    color=\"#cbd5e1\";".to_string());
            lines.push("    fontname=\"Inter, Helvetica, Arial\";".to_string());
            lines.push("    fontsize=16;".to_string());
            lines.push("    style=\"rounded\";".to_string());
            for task in lane_tasks {
                let is_open = open.contains(&task.id);
                let is_completed = task.completed;
                let mut label_parts = vec![task.id.clone()];
                if is_completed {
                    label_parts.push("DONE".to_string());
                } else if is_open {
                    label_parts.push("OPEN".to_string());
                }
                if let Some(native_id) = &task.native_id {
                    label_parts.push(native_id.clone());
                }
                label_parts.extend(wrap_title(&task.title, 26, 3));
                let label = dot_label(&label_parts);
                let status = if is_completed {
                    "done"
                } else if is_open {
                    "open"
                } else {
                    "blocked"
                };
                let blockers = incoming.get(&task.id).copied().unwrap_or(0);
                let native = task.native_id.clone().unwrap_or_default();
                let tooltip = dot_escape(&format!(
                    "{} [{}] {}; blockers={}; {} {}",
                    task.id, task.lane, status, blockers, native, task.title
                ));
                let (node_fill, node_border, penwidth) = if is_completed {
                    ("#f1f5f9", "#94a3b8", "1.4")
                } else if is_open {
                    ("#bbf7d0", "#15803d", "2.8")
                } else {
                    (fill, border, "1.2")
                };
                lines.push(format!(
                    "    \"{}\" [label=\"{}\", tooltip=\"{}\", fillcolor=\"{}\", color=\"{}\", penwidth={}];",
                    task.id, label, tooltip, node_fill, node_border, penwidth
                ));
            }
            lines.push("  }".to_string());
            lines.push(String::new());
        }

        for edge in &self.edges {
            let blocking = self
                .task(&edge.prerequisite)
                .zip(self.task(&edge.dependent))
                .is_some_and(|(from, to)| !from.completed && !to.completed);
            if blocking {
                lines.push(format!(
                    "  \"{}\" -> \"{}\";",
                    edge.prerequisite, edge.dependent
                ));
            } else {
                lines.push(format!(
                    "  \"{}\" -> \"{}\" [color=\"#94a3b8\", penwidth=1.0, arrowsize=0.45, style=dashed];",
                    edge.prerequisite, edge.dependent
                ));
            }
        }
        lines.push("}".to_string());
        lines.push(String::new());
        lines.join("\n")
    }

    /// Ensure the dependency graph has no cycles.
    pub fn ensure_acyclic(&self) -> Result<(), String> {
        self.topological_order().map(|_| ())
    }

    fn incoming_counts(&self) -> BTreeMap<String, usize> {
        let mut counts = self
            .tasks
            .iter()
            .map(|task| (task.id.clone(), 0usize))
            .collect::<BTreeMap<_, _>>();
        for edge in &self.edges {
            *counts.entry(edge.dependent.clone()).or_insert(0) += 1;
        }
        counts
    }

    fn active_incoming_counts(&self) -> BTreeMap<String, usize> {
        let mut counts = self
            .tasks
            .iter()
            .map(|task| (task.id.clone(), 0usize))
            .collect::<BTreeMap<_, _>>();
        for edge in &self.edges {
            let Some((from, to)) = self
                .task(&edge.prerequisite)
                .zip(self.task(&edge.dependent))
            else {
                continue;
            };
            if !from.completed && !to.completed {
                *counts.entry(edge.dependent.clone()).or_insert(0) += 1;
            }
        }
        counts
    }

    fn outgoing_map(&self) -> BTreeMap<String, Vec<String>> {
        let mut map = self
            .tasks
            .iter()
            .map(|task| (task.id.clone(), Vec::new()))
            .collect::<BTreeMap<_, _>>();
        for edge in &self.edges {
            map.entry(edge.prerequisite.clone())
                .or_default()
                .push(edge.dependent.clone());
        }
        for values in map.values_mut() {
            values.sort();
        }
        map
    }

    fn active_outgoing_map(&self) -> BTreeMap<String, Vec<String>> {
        let mut map = self
            .tasks
            .iter()
            .map(|task| (task.id.clone(), Vec::new()))
            .collect::<BTreeMap<_, _>>();
        for edge in &self.edges {
            let Some((from, to)) = self
                .task(&edge.prerequisite)
                .zip(self.task(&edge.dependent))
            else {
                continue;
            };
            if !from.completed && !to.completed {
                map.entry(edge.prerequisite.clone())
                    .or_default()
                    .push(edge.dependent.clone());
            }
        }
        for values in map.values_mut() {
            values.sort();
        }
        map
    }

    fn task(&self, id: &str) -> Option<&TaskNode> {
        self.tasks.iter().find(|task| task.id == id)
    }

    fn task_by_id(&self) -> BTreeMap<String, &TaskNode> {
        self.tasks
            .iter()
            .map(|task| (task.id.clone(), task))
            .collect()
    }

    fn topological_order(&self) -> Result<Vec<String>, String> {
        let outgoing = self.outgoing_map();
        let mut incoming = self.incoming_counts();
        let mut ready = self
            .tasks
            .iter()
            .filter(|task| incoming.get(&task.id).copied().unwrap_or(0) == 0)
            .map(|task| task.id.clone())
            .collect::<BTreeSet<_>>();
        let mut order = Vec::new();

        while let Some(next) = ready.pop_first() {
            order.push(next.clone());
            if let Some(children) = outgoing.get(&next) {
                for child in children {
                    if let Some(count) = incoming.get_mut(child) {
                        *count = count.saturating_sub(1);
                        if *count == 0 {
                            ready.insert(child.clone());
                        }
                    }
                }
            }
        }

        if order.len() == self.tasks.len() {
            Ok(order)
        } else {
            let blocked = incoming
                .into_iter()
                .filter_map(|(id, count)| if count > 0 { Some(id) } else { None })
                .collect::<Vec<_>>();
            Err(format!(
                "task dependency graph has at least one cycle; unresolved nodes: {}",
                blocked.join(", ")
            ))
        }
    }

    fn downstream_counts(&self) -> BTreeMap<String, usize> {
        let outgoing = self.active_outgoing_map();
        let mut counts = BTreeMap::new();
        for task in &self.tasks {
            let mut seen = BTreeSet::new();
            let mut stack = outgoing.get(&task.id).cloned().unwrap_or_default();
            while let Some(next) = stack.pop() {
                if !seen.insert(next.clone()) {
                    continue;
                }
                if let Some(children) = outgoing.get(&next) {
                    stack.extend(children.iter().cloned());
                }
            }
            counts.insert(task.id.clone(), seen.len());
        }
        counts
    }

    fn downstream_depths(&self) -> BTreeMap<String, usize> {
        let outgoing = self.active_outgoing_map();
        let order = self.topological_order().unwrap_or_default();
        let mut depth = BTreeMap::new();
        for task_id in order.into_iter().rev() {
            let best_child = outgoing
                .get(&task_id)
                .into_iter()
                .flatten()
                .filter_map(|child| depth.get(child).copied())
                .max()
                .unwrap_or(0);
            depth.insert(task_id, best_child + 1);
        }
        depth
    }
}

/// Paths for generated graph artifacts.
#[derive(Debug, Clone)]
pub struct GraphArtifactPaths {
    /// Markdown matrix path.
    pub matrix_md: PathBuf,
    /// Standalone CSV matrix path.
    pub matrix_csv: PathBuf,
    /// Graphviz DOT path.
    pub dot: PathBuf,
    /// Rendered SVG path.
    pub svg: PathBuf,
}

impl GraphArtifactPaths {
    /// Build default artifact paths under `docs/method`.
    pub fn defaults(workspace: &MethodWorkspace) -> Self {
        let method_root = workspace.method_root();
        Self {
            matrix_md: method_root.join("task-matrix.md"),
            matrix_csv: method_root.join("task-matrix.csv"),
            dot: method_root.join("task-dag.dot"),
            svg: method_root.join("task-dag.svg"),
        }
    }
}

/// Generated text artifacts except SVG, which xtask renders through Graphviz.
#[derive(Debug, Clone)]
pub struct GraphArtifacts {
    /// Markdown matrix.
    pub matrix_md: String,
    /// CSV matrix.
    pub matrix_csv: String,
    /// Graphviz DOT.
    pub dot: String,
}

impl GraphArtifacts {
    /// Render artifacts from a graph.
    pub fn render(graph: &TaskGraph) -> Self {
        Self {
            matrix_md: graph.render_matrix_markdown(),
            matrix_csv: graph.render_matrix_csv(),
            dot: graph.render_dot(),
        }
    }
}

#[derive(Debug, Clone)]
struct TaskSection {
    native_id: String,
    title: String,
}

#[derive(Debug, Clone)]
struct TaskSectionBody {
    body: String,
}

fn collect_markdown_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    collect_markdown_files_inner(root, &mut files)?;
    Ok(files)
}

fn collect_markdown_files_inner(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries =
        fs::read_dir(dir).map_err(|e| format!("failed to read dir {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read dir entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_markdown_files_inner(&path, files)?;
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        {
            files.push(path);
        }
    }
    Ok(())
}

fn lane_for(backlog_root: &Path, path: &Path) -> String {
    path.strip_prefix(backlog_root)
        .ok()
        .and_then(|p| p.components().next())
        .and_then(|c| match c {
            Component::Normal(os) => os.to_str(),
            _ => None,
        })
        .unwrap_or("unknown")
        .to_string()
}

fn lane_rank(lane: &str) -> usize {
    GRAPH_LANES
        .iter()
        .position(|known| *known == lane)
        .unwrap_or(usize::MAX)
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn relative_path(root: &Path, path: &Path) -> Result<String, String> {
    path.strip_prefix(root)
        .map(path_string)
        .map_err(|e| format!("failed to make {} relative: {e}", path.display()))
}

fn h1_title(text: &str) -> Option<String> {
    text.lines()
        .find_map(|line| line.strip_prefix("# ").map(str::trim))
        .map(str::to_string)
}

fn status_is_complete(text: &str) -> bool {
    text.lines()
        .filter_map(|line| line.trim().strip_prefix("Status:"))
        .map(|status| status.trim().to_ascii_lowercase())
        .any(|status| {
            (status.contains("complete") || status.contains("completed") || status.contains("done"))
                && !status.contains("incomplete")
        })
}

fn fallback_title(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("task")
        .replace(['_', '-'], " ")
}

fn task_sections(text: &str) -> Vec<TaskSection> {
    text.lines()
        .filter_map(parse_task_heading)
        .map(|(native_id, title)| TaskSection { native_id, title })
        .collect()
}

fn task_sections_with_body(text: &str) -> Vec<TaskSectionBody> {
    let lines = text.lines().collect::<Vec<_>>();
    let headings = lines
        .iter()
        .enumerate()
        .filter_map(|(idx, line)| parse_task_heading(line).map(|_| idx))
        .collect::<Vec<_>>();
    let mut sections = Vec::new();
    for (idx, heading_line) in headings.iter().enumerate() {
        let body_start = heading_line + 1;
        let body_end = headings.get(idx + 1).copied().unwrap_or(lines.len());
        sections.push(TaskSectionBody {
            body: lines[body_start..body_end].join("\n"),
        });
    }
    sections
}

fn parse_task_heading(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix(TASK_SECTION_PREFIX)?;
    let (id_tail, title) = rest.split_once(':')?;
    let native_id = format!("T-{}", id_tail.trim());
    if native_id.is_empty() || title.trim().is_empty() {
        return None;
    }
    Some((native_id, title.trim().to_string()))
}

fn slugify_heading(text: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in text.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if (ch.is_whitespace() || ch == '_' || ch == '-') && !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    if out.ends_with('-') {
        out.pop();
    }
    out
}

fn file_dependency_links(text: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut collecting = false;
    let mut block = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if !collecting && trimmed == "Depends on:" {
            collecting = true;
            block.clear();
            continue;
        }
        if !collecting {
            continue;
        }
        if trimmed.is_empty() && !block.is_empty() {
            links.extend(markdown_links(&block.join("\n")));
            collecting = false;
            block.clear();
            continue;
        }
        if trimmed.starts_with("## ") {
            links.extend(markdown_links(&block.join("\n")));
            collecting = false;
            block.clear();
            continue;
        }
        if trimmed.starts_with("- ") {
            block.push(line.to_string());
            continue;
        }
        if !block.is_empty() {
            links.extend(markdown_links(&block.join("\n")));
            collecting = false;
            block.clear();
        }
    }

    if collecting && !block.is_empty() {
        links.extend(markdown_links(&block.join("\n")));
    }

    links
}

fn markdown_links(text: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut cursor = 0usize;
    while let Some(open_bracket) = text[cursor..].find('[') {
        let bracket = cursor + open_bracket;
        let Some(close_bracket_rel) = text[bracket..].find(']') else {
            break;
        };
        let close_bracket = bracket + close_bracket_rel;
        let open_paren = close_bracket + 1;
        if text.as_bytes().get(open_paren) != Some(&b'(') {
            cursor = close_bracket + 1;
            continue;
        }
        let Some(close_paren_rel) = text[open_paren + 1..].find(')') else {
            break;
        };
        let close_paren = open_paren + 1 + close_paren_rel;
        links.push(text[open_paren + 1..close_paren].to_string());
        cursor = close_paren + 1;
    }
    links
}

fn resolve_backlog_link(source_rel: &str, link: &str) -> Option<String> {
    if link.contains("://") {
        return None;
    }
    let path_part = link.split('#').next().unwrap_or_default();
    if path_part.is_empty()
        || !Path::new(path_part)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
    {
        return None;
    }
    let source_parent = Path::new(source_rel)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let normalized = normalize_relative_path(source_parent.join(path_part));
    if normalized.starts_with("docs/method/backlog/") {
        Some(normalized)
    } else {
        None
    }
}

fn normalize_relative_path(path: PathBuf) -> String {
    let mut parts: Vec<String> = Vec::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                parts.pop();
            }
            Component::Normal(os) => parts.push(os.to_string_lossy().into_owned()),
            Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
        }
    }
    parts.join("/")
}

fn blocked_by_values(section: &str) -> Vec<&str> {
    field_values(section, "**Blocked By:**")
}

fn blocking_values(section: &str) -> Vec<&str> {
    field_values(section, "**Blocking:**")
}

fn field_values<'a>(section: &'a str, prefix: &str) -> Vec<&'a str> {
    section
        .lines()
        .filter_map(|line| line.trim().strip_prefix(prefix))
        .map(str::trim)
        .collect()
}

fn add_blocker_edges(
    raw: &str,
    source_idx: usize,
    tasks: &[TaskNode],
    id_by_native: &BTreeMap<String, Vec<usize>>,
    slug_aliases: &BTreeMap<String, Vec<usize>>,
    edge_set: &mut BTreeSet<TaskEdge>,
) {
    for token in task_tokens(raw) {
        if let Some(dep_indexes) = id_by_native.get(&token) {
            for dep_idx in dep_indexes {
                if *dep_idx != source_idx {
                    edge_set.insert(TaskEdge {
                        prerequisite: tasks[*dep_idx].id.clone(),
                        dependent: tasks[source_idx].id.clone(),
                    });
                }
            }
        }
    }

    let remainder = remove_task_tokens(raw).to_ascii_lowercase();
    for part in remainder.split([',', ';', '(', ')']) {
        let alias = part.trim().trim_matches('.').trim_matches('`');
        if alias.is_empty()
            || matches!(
                alias,
                "none"
                    | "n/a"
                    | "na"
                    | "none in the task dag"
                    | "operationally blocked until there is at least one"
            )
        {
            continue;
        }
        let alias = alias.replace(' ', "-");
        if let Some(dep_indexes) = slug_aliases.get(&alias) {
            for dep_idx in dep_indexes {
                if *dep_idx != source_idx {
                    edge_set.insert(TaskEdge {
                        prerequisite: tasks[*dep_idx].id.clone(),
                        dependent: tasks[source_idx].id.clone(),
                    });
                }
            }
        }
    }
}

fn task_tokens(raw: &str) -> Vec<String> {
    raw.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-')
        .filter(|token| token.starts_with("T-"))
        .map(|token| token.trim_matches('.').to_string())
        .collect()
}

fn remove_task_tokens(raw: &str) -> String {
    let mut out = Vec::new();
    for token in raw.split_whitespace() {
        if !token
            .trim_matches(|ch: char| ch == ',' || ch == ';')
            .starts_with("T-")
        {
            out.push(token);
        }
    }
    out.join(" ")
}

fn task_markdown_link(task: &TaskNode) -> String {
    let title = task.title.replace('|', "\\|");
    if let Some(anchor) = &task.anchor {
        format!("[{}]({}#{})", title, task.source_path, anchor)
    } else {
        format!("[{}]({})", title, task.source_path)
    }
}

fn lane_colors(lane: &str) -> (&'static str, &'static str) {
    match lane {
        "asap" => ("#fff3bf", "#b08900"),
        "up-next" => ("#dbeafe", "#1d4ed8"),
        "inbox" => ("#e5e7eb", "#4b5563"),
        "cool-ideas" => ("#ede9fe", "#7c3aed"),
        "bad-code" => ("#fee2e2", "#b91c1c"),
        _ => ("#f8fafc", "#64748b"),
    }
}

fn dot_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn dot_label(parts: &[String]) -> String {
    parts
        .iter()
        .filter(|part| !part.is_empty())
        .map(|part| dot_escape(part))
        .collect::<Vec<_>>()
        .join("\\n")
}

fn wrap_title(title: &str, limit: usize, max_lines: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in title.split_whitespace() {
        let next = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        if next.len() <= limit {
            current = next;
        } else {
            if !current.is_empty() {
                lines.push(current);
            }
            current = word.to_string();
        }
        if lines.len() >= max_lines {
            break;
        }
    }
    if !current.is_empty() && lines.len() < max_lines {
        lines.push(current);
    }
    if lines.join(" ").len() < title.len() {
        if let Some(last) = lines.last_mut() {
            *last = format!("{}...", last.trim_end_matches('.'));
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_markdown_links_from_depends_block_only() {
        let text = r"
Depends on:

- [A](./a.md)

Design source:
[B](./b.md)
";
        assert_eq!(file_dependency_links(text), vec!["./a.md"]);
    }

    #[test]
    fn token_parser_finds_legacy_task_ids() {
        assert_eq!(
            task_tokens("T-1-2-3, T-10-6-1a"),
            vec!["T-1-2-3", "T-10-6-1a"]
        );
    }

    #[test]
    fn markdown_anchor_matches_expected_task_heading() {
        assert_eq!(
            slugify_heading("T-6-4-2 Inspect -- attachment payload pretty-printing"),
            "t-6-4-2-inspect-attachment-payload-pretty-printing"
        );
    }
}
