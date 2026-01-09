// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

const INPUT_FILE = "TASKS-DAG.md";
const OUT_DIR = "docs/assets/dags";
const DOT_FILE = path.join(OUT_DIR, "tasks-dag.dot");
const SVG_FILE = path.join(OUT_DIR, "tasks-dag.svg");

function fail(message) {
  process.stderr.write(`${message}\n`);
  process.exit(1);
}

function runChecked(cmd, args) {
  const result = spawnSync(cmd, args, { encoding: "utf8" });
  if (result.error) fail(`Failed to run ${cmd}: ${result.error.message}`);
  if (result.status !== 0) fail(`Command failed: ${cmd} ${args.join(" ")}\n${result.stderr}`);
}

function parseTasksDag(content) {
  const lines = content.split("\n");
  const nodes = new Map(); // number -> { number, title, url }
  const edges = []; // { from, to, confidence, note }

  let currentIssue = null;
  let mode = null; // 'blocks' or 'blocked_by'

  const issueRegex = /^## \[#(\d+): (.+)](.+)"/;
  const linkRegex = /^\s+- \[#(\d+): (.*?)\]\((.*)\)/;
  const confidenceRegex = /^\s+- Confidence: (.+)/;
  const evidenceRegex = /^\s+- Evidence: (.+)/;

  let pendingEdge = null;

  for (const line of lines) {
    if (line.startsWith("## [")) {
       const issueMatch = line.match(/^## \[#(\d+): (.*?)\]\((.*)\)/);
       if (issueMatch) {
         if (pendingEdge) { edges.push(pendingEdge); pendingEdge = null; }
         const number = parseInt(issueMatch[1], 10);
         currentIssue = { number, title: issueMatch[2], url: issueMatch[3] };
         nodes.set(number, currentIssue);
         mode = null;
         continue;
       } else {
         console.warn("Failed to match header:", line);
       }
    }
    
    if (!currentIssue) continue;

    // Section Headers
    if (line.trim() === "- Blocked by:") {
      mode = "blocked_by";
      if (pendingEdge) { edges.push(pendingEdge); pendingEdge = null; }
      continue;
    }
    if (line.trim() === "- Blocks:") {
      mode = "blocks";
      if (pendingEdge) { edges.push(pendingEdge); pendingEdge = null; }
      continue;
    }

    // Dependency Link
    const linkMatch = line.match(linkRegex);
    if (linkMatch) {
      if (pendingEdge) { edges.push(pendingEdge); pendingEdge = null; }
      const targetNumber = parseInt(linkMatch[1], 10);
      const targetTitle = linkMatch[2];
      const targetUrl = linkMatch[3];
      
      // Ensure target node exists (even if we haven't reached its header yet)
      if (!nodes.has(targetNumber)) {
        nodes.set(targetNumber, { number: targetNumber, title: targetTitle, url: targetUrl });
      }

      if (mode === "blocked_by") {
        // Target -> Current
        pendingEdge = { from: targetNumber, to: currentIssue.number, confidence: "strong", note: "" };
      } else if (mode === "blocks") {
        // Current -> Target
        pendingEdge = { from: currentIssue.number, to: targetNumber, confidence: "strong", note: "" };
      }
      continue;
    }

    // Metadata for the pending edge
    if (pendingEdge) {
      const confMatch = line.match(confidenceRegex);
      if (confMatch) {
        pendingEdge.confidence = confMatch[1].trim().toLowerCase();
        continue;
      }
      const evMatch = line.match(evidenceRegex);
      if (evMatch) {
        pendingEdge.note = evMatch[1].trim();
        continue;
      }
    }
  }
  if (pendingEdge) { edges.push(pendingEdge); }

  return { nodes, edges };
}

function escapeDotString(str) {
  return String(str).replace(/\\/g, "\\\\").replace(/"/g, '\\"');
}

function confidenceAttrs(confidence) {
  switch (confidence) {
    case "strong": return 'color="black", penwidth=1.4, style="solid"';
    case "medium": return 'color="gray40", penwidth=1.2, style="dashed"';
    case "weak": return 'color="gray70", penwidth=1.2, style="dotted"';
    default: return 'color="black", style="solid"'; // fallback
  }
}

// Heuristic to guess cluster name from title
function getClusterName(title) {
  const prefixes = [
    "TT0", "TT1", "TT2", "TT3", 
    "S1", "M1", "M2", "M4", "W1", 
    "Demo 2", "Demo 3", 
    "Spec:", "Draft", "Tooling:", "Backlog:"
  ];
  for (const p of prefixes) {
    if (title.startsWith(p)) return p.replace(":", "");
  }
  return "Misc";
}

function generateDot(nodes, edges) {
  // Filter out isolated nodes
  const connectedNodeIds = new Set();
  for (const e of edges) {
    connectedNodeIds.add(e.from);
    connectedNodeIds.add(e.to);
  }
  
  // Create a filtered map of nodes
  const filteredNodes = new Map();
  for (const [id, node] of nodes) {
    if (connectedNodeIds.has(id)) {
      filteredNodes.set(id, node);
    }
  }

  const lines = [];
  lines.push('digraph tasks_dag {');
  lines.push('  graph [rankdir=LR, labelloc="t", fontsize=18, fontname="Helvetica", newrank=true, splines=true];');
  lines.push('  node [shape=box, style="rounded,filled", fontname="Helvetica", fontsize=10, margin="0.10,0.06"];');
  lines.push('  edge [fontname="Helvetica", fontsize=9, arrowsize=0.8];');
  lines.push('  label="Echo — Tasks DAG (from TASKS-DAG.md)\nGenerated by scripts/generate-tasks-dag.js";');
  lines.push('');

  lines.push("  subgraph cluster_legend {");
  lines.push('    label="Legend";');
  lines.push('    color="gray70";');
  lines.push('    fontcolor="gray30";');
  lines.push('    style="rounded";');
  lines.push('    LG [label="confirmed in issue body", color="green", fontcolor="green"];');
  lines.push("  }");
  lines.push("");

  // Clusters
  const clusters = new Map();
  for (const node of filteredNodes.values()) {
    const cluster = getClusterName(node.title);
    if (!clusters.has(cluster)) clusters.set(cluster, []);
    clusters.get(cluster).push(node);
  }

  for (const [name, groupNodes] of clusters) {
    // Sanitize cluster name for ID
    const clusterId = "cluster_" + name.replace(/[^a-zA-Z0-9]/g, "_");
    lines.push(`  subgraph ${clusterId} {`);
    lines.push(`    label="${escapeDotString(name)}";`);
    lines.push('    style="rounded"; color="gray70";');
    // Simple color cycle for clusters
    const colors = ["#dbeafe", "#dcfce7", "#ffedd5", "#f3f4f6", "#fef9c3", "#ede9fe", "#ccfbf1", "#fee2e2"];
    const color = colors[Math.abs(name.split('').reduce((a,c)=>a+c.charCodeAt(0),0)) % colors.length];
    lines.push(`    node [fillcolor="${color}"];`);
    
    for (const node of groupNodes) {
      const label = `#${node.number}\n${node.title.replace(/"/g, "'")}`; // escape quotes in label for DOT safety (though escapeDotString handles the attribute wrapper)
      // limit label length?
      let safeLabel = escapeDotString(label);
      if (safeLabel.length > 50) {
         // insert line break roughly
         safeLabel = safeLabel.replace(/(.{30,}?)\s/, "$1\\n"); 
      }

      lines.push(`    i${node.number} [label="${safeLabel}", URL="${node.url}", tooltip="${escapeDotString(node.title)}"];`);
    }
    lines.push('  }');
  }

  lines.push('');
  for (const edge of edges) {
    // Only add edge if both nodes exist in our set (which they should)
    if (nodes.has(edge.from) && nodes.has(edge.to)) {
      // Force Green for "Confirmed in Issue Body" (which is everything here)
      lines.push(`  i${edge.from} -> i${edge.to} [color="green3", penwidth=2.0, style="solid", tooltip="${escapeDotString(edge.note || "")}"];`);
    }
  }

  lines.push('}');
  return lines.join("\n");
}

function main() {
  if (!fs.existsSync(INPUT_FILE)) fail(`Input file not found: ${INPUT_FILE}`);
  
  const content = fs.readFileSync(INPUT_FILE, "utf8");
  const { nodes, edges } = parseTasksDag(content);
  
  const dotContent = generateDot(nodes, edges);
  
  if (!fs.existsSync(OUT_DIR)) fs.mkdirSync(OUT_DIR, { recursive: true });
  fs.writeFileSync(DOT_FILE, dotContent);
  console.log(`Wrote DOT file to ${DOT_FILE}`);

  try {
    runChecked("dot", ["-Tsvg", DOT_FILE, "-o", SVG_FILE]);
    console.log(`Rendered SVG to ${SVG_FILE}`);
  } catch (e) {
    console.warn("Warning: Failed to render SVG (is graphviz installed?). Only DOT file generated.");
  }
}

main();
