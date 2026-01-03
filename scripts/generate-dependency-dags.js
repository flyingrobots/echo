// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

function fail(message) {
  process.stderr.write(`${message}\n`);
  process.exit(1);
}

function runChecked(cmd, args, { cwd } = {}) {
  const result = spawnSync(cmd, args, {
    cwd,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  if (result.error) {
    fail(`Failed to run ${cmd}: ${result.error.message}`);
  }
  if (result.status !== 0) {
    fail(
      `Command failed (${cmd} ${args.join(" ")}):\n${result.stderr || result.stdout || ""}`,
    );
  }
  return result.stdout;
}

function readJsonFile(filePath) {
  const raw = fs.readFileSync(filePath, "utf8");
  return JSON.parse(raw);
}

function maybeWrapIssuesJson(json) {
  if (Array.isArray(json)) {
    return { generated_at: null, issues: json };
  }
  if (json && typeof json === "object" && Array.isArray(json.issues)) {
    return { generated_at: json.generated_at ?? null, issues: json.issues };
  }
  fail(
    `Unsupported issues JSON format: expected an array or { issues: [...] }.`,
  );
}

function maybeWrapMilestonesJson(json) {
  if (Array.isArray(json)) {
    return { generated_at: null, milestones: json };
  }
  if (json && typeof json === "object" && Array.isArray(json.milestones)) {
    return { generated_at: json.generated_at ?? null, milestones: json.milestones };
  }
  fail(
    `Unsupported milestones JSON format: expected an array or { milestones: [...] }.`,
  );
}

function escapeDotString(value) {
  return String(value).replaceAll("\\", "\\\\").replaceAll('"', '\\"');
}

function formatDateYYYYMMDD(date) {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function parseArgs(argv) {
  const args = {
    fetch: false,
    render: false,
    issuesJson: ".cache/echo/deps/open-issues.json",
    milestonesJson: ".cache/echo/deps/milestones-all.json",
    configJson: "docs/assets/dags/deps-config.json",
    outDir: "docs/assets/dags",
    snapshot: null,
    snapshotLabelMode: "auto",
  };

  for (let idx = 2; idx < argv.length; idx += 1) {
    const token = argv[idx];
    if (token === "--fetch") args.fetch = true;
    else if (token === "--render") args.render = true;
    else if (token === "--issues-json") args.issuesJson = argv[++idx];
    else if (token === "--milestones-json") args.milestonesJson = argv[++idx];
    else if (token === "--config") args.configJson = argv[++idx];
    else if (token === "--out-dir") args.outDir = argv[++idx];
    else if (token === "--snapshot") args.snapshot = argv[++idx];
    else if (token === "--snapshot-label") args.snapshotLabelMode = argv[++idx];
    else if (token === "-h" || token === "--help") {
      process.stdout.write(
        [
          "Usage: node scripts/generate-dependency-dags.js [options]",
          "",
          "Options:",
          "  --fetch                 Fetch fresh data via gh (network)",
          "  --render                Render SVGs via graphviz dot",
          "  --issues-json <path>    Read/write issues snapshot JSON",
          "  --milestones-json <path> Read/write milestones snapshot JSON",
          "  --config <path>         Dependency config (edges) JSON",
          "  --out-dir <dir>         Output directory for DOT/SVG",
          "  --snapshot <YYYY-MM-DD> Override label date in output graphs (legacy; prefer --snapshot-label)",
          "  --snapshot-label <mode> Snapshot label: auto|none|rolling|YYYY-MM-DD",
          "",
          "Notes:",
          "  - DOT output: issue-deps.dot, milestone-deps.dot",
          "  - SVG output (with --render): issue-deps.svg, milestone-deps.svg",
          "",
        ].join("\n"),
      );
      process.exit(0);
    } else {
      fail(`Unknown arg: ${token} (try --help)`);
    }
  }

  return args;
}

function ensureDir(dirPath) {
  fs.mkdirSync(dirPath, { recursive: true });
}

function resolveSnapshotLabel({ snapshot, snapshotLabelMode, generatedAt }) {
  if (snapshot != null) {
    return { mode: "date", label: snapshot };
  }

  const modeRaw = snapshotLabelMode ?? "auto";
  if (modeRaw === "none") return { mode: "none", label: null };
  if (modeRaw === "rolling") return { mode: "rolling", label: "rolling" };
  if (modeRaw === "auto") {
    const fallback = generatedAt?.slice(0, 10) ?? formatDateYYYYMMDD(new Date());
    return { mode: "date", label: fallback };
  }

  // Treat any other value as a literal snapshot label (e.g. YYYY-MM-DD).
  return { mode: "custom", label: modeRaw };
}

function writeJsonSnapshot(filePath, payload) {
  ensureDir(path.dirname(filePath));
  fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`, "utf8");
}

function fetchRepoNameWithOwner() {
  const raw = runChecked("gh", ["repo", "view", "--json", "nameWithOwner"]);
  const parsed = JSON.parse(raw);
  if (!parsed?.nameWithOwner) {
    fail("gh repo view did not return nameWithOwner");
  }
  return parsed.nameWithOwner;
}

function fetchOpenIssuesSnapshot() {
  const raw = runChecked("gh", [
    "issue",
    "list",
    "--state",
    "open",
    "--limit",
    "300",
    "--json",
    "number,title,body,labels,milestone,url",
  ]);
  const issues = JSON.parse(raw);
  if (!Array.isArray(issues)) {
    fail("gh issue list returned unexpected JSON (expected array)");
  }
  return { generated_at: new Date().toISOString(), issues };
}

function fetchAllMilestonesSnapshot(nameWithOwner) {
  // GitHub API returns an array of milestone objects with fields like title, html_url, state.
  const raw = runChecked("gh", [
    "api",
    `repos/${nameWithOwner}/milestones?state=all&per_page=100`,
  ]);
  const milestones = JSON.parse(raw).map((m) => ({
    title: m.title,
    url: m.html_url,
    state: m.state,
    number: m.number,
  }));
  return { generated_at: new Date().toISOString(), milestones };
}

function confidenceEdgeAttrs(confidence) {
  if (confidence === "strong") return 'color="black", penwidth=1.4, style="solid"';
  if (confidence === "medium") return 'color="gray40", penwidth=1.2, style="dashed"';
  if (confidence === "weak") return 'color="gray70", penwidth=1.2, style="dotted"';
  fail(`Unknown confidence: ${confidence}`);
}

function milestoneFillFor(title) {
  const palettes = [
    ["TT", "#dbeafe"],
    ["S1", "#ede9fe"],
    ["W1", "#ccfbf1"],
    ["1C", "#dcfce7"],
    ["1E", "#ffedd5"],
    ["1F", "#f3f4f6"],
    ["M2.2", "#fef9c3"],
  ];

  if (typeof title !== "string") return "#ffffff";
  for (const [prefix, color] of palettes) {
    if (title.startsWith(prefix)) {
      return color;
    }
  }
  return "#ffffff";
}

function parseTasksDag(content) {
  const lines = content.split("\n");
  const edges = new Set(); // "from->to" strings

  let currentIssue = null;
  let mode = null; // 'blocks' or 'blocked_by'

  for (const line of lines) {
    if (line.startsWith("## [")) {
       const issueMatch = line.match(/^## \[#(\d+): (.*?)\]\((.*)\)/);
       if (issueMatch) {
         currentIssue = parseInt(issueMatch[1], 10);
         mode = null;
         continue;
       }
    }
    
    if (!currentIssue) continue;

    if (line.trim() === "- Blocked by:") {
      mode = "blocked_by";
      continue;
    }
    if (line.trim() === "- Blocks:") {
      mode = "blocks";
      continue;
    }

    const linkMatch = line.match(/^\s+- \[#(\d+): (.*?)\]\((.*)\)/);
    if (linkMatch) {
      const targetNumber = parseInt(linkMatch[1], 10);
      if (mode === "blocked_by") {
        // Target -> Current
        edges.add(`${targetNumber}->${currentIssue}`);
      } else if (mode === "blocks") {
        // Current -> Target
        edges.add(`${currentIssue}->${targetNumber}`);
      }
    }
  }
  return edges;
}

function emitIssueDot({ issues, issueEdges, snapshotLabel, realityEdges }) {
  const byNum = new Map();
  for (const issue of issues) byNum.set(issue.number, issue);

  const nodes = new Set();
  const configuredEdges = new Set();

  for (const e of issueEdges) {
    nodes.add(e.from);
    nodes.add(e.to);
    configuredEdges.add(`${e.from}->${e.to}`);
  }

  // Add nodes from reality edges if they exist in the issue snapshot
  if (realityEdges) {
    for (const edgeKey of realityEdges) {
      const [u, v] = edgeKey.split("->").map(n => parseInt(n, 10));
      // Only add to graph if both nodes are in the issue snapshot (sanity check)
      if (byNum.has(u) && byNum.has(v)) {
        // We generally only add nodes if they are connected to the "Plan" or extend it.
        // For visual clarity, let's include them if they touch the existing Plan nodes or imply missing plan parts.
        // For now, let's purely strictly add them if they are part of a Red edge.
        if (!configuredEdges.has(edgeKey)) {
           nodes.add(u);
           nodes.add(v);
        }
      }
    }
  }

  const missing = [...nodes].filter((n) => !byNum.has(n)).sort((a, b) => a - b);
  // Warning only for missing nodes in reality edges (dynamic), strict fail for config edges
  // actually existing logic fails on missing config nodes. Let's keep that.
  
  // Filter nodes that don't exist in byNum (stale config or stale reality)
  const validNodes = [...nodes].filter(n => byNum.has(n));

  const groups = new Map();
  for (const n of validNodes.sort((a, b) => a - b)) {
    const issue = byNum.get(n);
    const milestoneTitle = issue?.milestone?.title ?? "(no milestone)";
    const list = groups.get(milestoneTitle) ?? [];
    list.push(n);
    groups.set(milestoneTitle, list);
  }

  const lines = [];
  lines.push("// SPDX-License-Identifier: Apache-2.0");
  lines.push("// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>");
  lines.push("digraph echo_issue_dependencies {");
  lines.push(
    '  graph [rankdir=LR, labelloc="t", fontsize=18, fontname="Helvetica", newrank=true, splines=true];',
  );
  lines.push(
    '  node  [shape=box, style="rounded,filled", fontname="Helvetica", fontsize=10, margin="0.10,0.06"];',
  );
  lines.push('  edge  [fontname="Helvetica", fontsize=9, arrowsize=0.8];');
  const title =
    snapshotLabel == null
      ? "Echo — Issue Dependency Sketch"
      : `Echo — Issue Dependency Sketch (snapshot: ${snapshotLabel})`;
  lines.push(
    `  label="${escapeDotString(
      title,
    )}\\nEdge direction: prerequisite → dependent (do tail before head)\\nEdge styles encode confidence (solid=strong, dashed=medium, dotted=weak).\\nGreen = Confirmed in Issue Body; Red = In Issue Body but missing from Plan.";`,
  );
  lines.push("");

  lines.push("  subgraph cluster_legend {");
  lines.push('    label="Legend";');
  lines.push('    color="gray70";');
  lines.push('    fontcolor="gray30";');
  lines.push('    style="rounded";');
  lines.push('    L1 [label="strong", fillcolor="#ffffff"];');
  lines.push('    L2 [label="medium", fillcolor="#ffffff"];');
  lines.push('    L3 [label="weak", fillcolor="#ffffff"];');
  lines.push('    LG [label="confirmed (reality)", color="green", fontcolor="green"];');
  lines.push('    LR [label="missing from plan", color="red", fontcolor="red"];');
  lines.push(
    `    L1 -> L2 [arrowhead=none, ${confidenceEdgeAttrs("strong")}];`,
  );
  lines.push(
    `    L2 -> L3 [arrowhead=none, ${confidenceEdgeAttrs("medium")}];`,
  );
  lines.push("  }");
  lines.push("");

  const sortedGroupTitles = [...groups.keys()].sort((a, b) =>
    a.localeCompare(b),
  );
  for (const msTitle of sortedGroupTitles) {
    const clusterId =
      "cluster_" +
      msTitle
        .replaceAll(/[^a-zA-Z0-9]/g, "_")
        .slice(0, 48);
    const fill = milestoneFillFor(msTitle);
    lines.push(`  subgraph ${clusterId} {`);
    lines.push(`    label="${escapeDotString(msTitle)}";`);
    lines.push('    style="rounded";');
    lines.push('    color="gray70";');
    lines.push(`    node [fillcolor="${fill}"];`);
    for (const n of groups.get(msTitle) ?? []) {
      const issue = byNum.get(n);
      const title = issue.title ?? "";
      const url = issue.url ?? "";
      const label = `#${n}\\n${title}`;
      lines.push(
        `    i${n} [label="${escapeDotString(label)}", tooltip="${escapeDotString(
          title,
        )}", URL="${escapeDotString(url)}"];`,
      );
    }
    lines.push("  }");
    lines.push("");
  }

  for (const { from, to, confidence, note } of issueEdges) {
    const edgeKey = `${from}->${to}`;
    const inReality = realityEdges && realityEdges.has(edgeKey);
    const colorAttr = inReality ? 'color="green", penwidth=2.0' : confidenceEdgeAttrs(confidence);
    
    // If in reality, override style to solid green, preserving existing penwidth boost
    // Actually confidenceEdgeAttrs returns full string. Let's parse or just conditionally use strings.
    let attrs = confidenceEdgeAttrs(confidence);
    if (inReality) {
       // Replace color and penwidth
       attrs = 'color="green3", penwidth=2.0, style="solid"';
    }

    lines.push(
      `  i${from} -> i${to} [${attrs}, tooltip="${escapeDotString(note)}"];`,
    );
  }

  // Red edges (Reality - Plan)
  if (realityEdges) {
    for (const edgeKey of realityEdges) {
      if (!configuredEdges.has(edgeKey)) {
        const [u, v] = edgeKey.split("->").map(n => parseInt(n, 10));
        if (byNum.has(u) && byNum.has(v)) {
           lines.push(
             `  i${u} -> i${v} [color="red", penwidth=2.0, style="dashed", tooltip="Inferred from Issue Body (missing from Plan)"];`
           );
        }
      }
    }
  }

  lines.push("}");
  lines.push("");
  return lines.join("\n");
}

function emitMilestoneDot({ milestones, milestoneEdges, snapshotLabel }) {
  const byKey = new Map();
  for (const m of milestones) {
    const title = m.title ?? "";
    const key = String(title).split(" ")[0];
    if (key) byKey.set(key, m);
  }

  const nodes = new Set();
  for (const e of milestoneEdges) {
    nodes.add(e.from);
    nodes.add(e.to);
  }

  const missing = [...nodes].filter((k) => !byKey.has(k)).sort();
  if (missing.length > 0) {
    fail(
      `Milestone snapshot missing referenced milestone key(s): ${missing.join(
        ", ",
      )}`,
    );
  }

  const lines = [];
  lines.push("// SPDX-License-Identifier: Apache-2.0");
  lines.push("// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>");
  lines.push("digraph echo_milestone_dependencies {");
  lines.push(
    '  graph [rankdir=LR, labelloc="t", fontsize=18, fontname="Helvetica", splines=true];',
  );
  lines.push(
    '  node  [shape=box, style="rounded,filled", fontname="Helvetica", fontsize=12, margin="0.14,0.10"];',
  );
  lines.push('  edge  [fontname="Helvetica", fontsize=10, arrowsize=0.8];');
  const title =
    snapshotLabel == null
      ? "Echo — Milestone Dependency Sketch"
      : `Echo — Milestone Dependency Sketch (snapshot: ${snapshotLabel})`;
  lines.push(
    `  label="${escapeDotString(
      title,
    )}\\nEdge direction: prerequisite → dependent (do tail before head)\\nEdge styles encode confidence (solid=strong, dashed=medium, dotted=weak).";`,
  );
  lines.push("");

  lines.push("  subgraph cluster_legend {");
  lines.push('    label="Legend";');
  lines.push('    color="gray70";');
  lines.push('    fontcolor="gray30";');
  lines.push('    style="rounded";');
  lines.push('    L1 [label="strong", fillcolor="#ffffff"];');
  lines.push('    L2 [label="medium", fillcolor="#ffffff"];');
  lines.push('    L3 [label="weak", fillcolor="#ffffff"];');
  lines.push(
    `    L1 -> L2 [arrowhead=none, ${confidenceEdgeAttrs("strong")}];`,
  );
  lines.push(
    `    L2 -> L3 [arrowhead=none, ${confidenceEdgeAttrs("medium")}];`,
  );
  lines.push("  }");
  lines.push("");

  const sortedKeys = [...nodes].sort((a, b) => a.localeCompare(b));
  for (const key of sortedKeys) {
    const m = byKey.get(key);
    const title = m.title ?? key;
    const url = m.url ?? "";
    const fill = milestoneFillFor(title);
    lines.push(
      `  m${key} [label="${escapeDotString(title)}", fillcolor="${fill}", tooltip="${escapeDotString(
        title,
      )}", URL="${escapeDotString(url)}"];`,
    );
  }

  lines.push("");

  for (const { from, to, confidence, note } of milestoneEdges) {
    lines.push(
      `  m${from} -> m${to} [${confidenceEdgeAttrs(
        confidence,
      )}, tooltip="${escapeDotString(note)}"];`,
    );
  }

  lines.push("}");
  lines.push("");
  return lines.join("\n");
}

function main() {
  const args = parseArgs(process.argv);
  const config = readJsonFile(args.configJson);

  if (!Array.isArray(config.issue_edges) || !Array.isArray(config.milestone_edges)) {
    fail(`Invalid config: expected issue_edges and milestone_edges arrays in ${args.configJson}`);
  }

  if (args.fetch) {
    const nameWithOwner = fetchRepoNameWithOwner();
    const issuesSnapshot = fetchOpenIssuesSnapshot();
    const milestonesSnapshot = fetchAllMilestonesSnapshot(nameWithOwner);
    writeJsonSnapshot(args.issuesJson, issuesSnapshot);
    writeJsonSnapshot(args.milestonesJson, milestonesSnapshot);
  }

  if (!fs.existsSync(args.issuesJson)) {
    fail(
      `Missing issues snapshot at ${args.issuesJson}. Run with --fetch (requires gh + network) or point --issues-json at an existing snapshot.`,
    );
  }
  if (!fs.existsSync(args.milestonesJson)) {
    fail(
      `Missing milestones snapshot at ${args.milestonesJson}. Run with --fetch (requires gh + network) or point --milestones-json at an existing snapshot.`,
    );
  }

  const issuesWrapped = maybeWrapIssuesJson(readJsonFile(args.issuesJson));
  const milestonesWrapped = maybeWrapMilestonesJson(readJsonFile(args.milestonesJson));

  let realityEdges = null;
  if (fs.existsSync("TASKS-DAG.md")) {
    const tasksDagContent = fs.readFileSync("TASKS-DAG.md", "utf8");
    realityEdges = parseTasksDag(tasksDagContent);
  }

  const snapshotResolved = resolveSnapshotLabel({
    snapshot: args.snapshot,
    snapshotLabelMode: args.snapshotLabelMode,
    generatedAt: issuesWrapped.generated_at,
  });

  ensureDir(args.outDir);

  const issueDot = emitIssueDot({
    issues: issuesWrapped.issues,
    issueEdges: config.issue_edges,
    snapshotLabel: snapshotResolved.label,
    realityEdges,
  });
  const milestoneDot = emitMilestoneDot({
    milestones: milestonesWrapped.milestones,
    milestoneEdges: config.milestone_edges,
    snapshotLabel: snapshotResolved.label,
  });

  const issueDotPath = path.join(args.outDir, "issue-deps.dot");
  const milestoneDotPath = path.join(args.outDir, "milestone-deps.dot");
  fs.writeFileSync(issueDotPath, issueDot, "utf8");
  fs.writeFileSync(milestoneDotPath, milestoneDot, "utf8");

  if (args.render) {
    runChecked("dot", ["-Tsvg", issueDotPath, "-o", path.join(args.outDir, "issue-deps.svg")]);
    runChecked("dot", ["-Tsvg", milestoneDotPath, "-o", path.join(args.outDir, "milestone-deps.svg")]);
  }

  process.stdout.write(
    [
      "Generated dependency DAGs:",
      `- ${issueDotPath}`,
      `- ${milestoneDotPath}`,
      args.render ? `- ${path.join(args.outDir, "issue-deps.svg")}` : "- (SVGs not rendered; pass --render)",
      args.render ? `- ${path.join(args.outDir, "milestone-deps.svg")}` : "- (SVGs not rendered; pass --render)",
      "",
      "Tip: fetch fresh GitHub data with --fetch (requires gh auth + network).",
    ].join("\n") + "\n",
  );
}

main();
