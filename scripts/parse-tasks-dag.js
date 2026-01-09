// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

const issueRegex = /^##\s*\[#(\d+):\s*(.*?)\]\((.*?)\)/;
const linkRegex = /^\s*-\s*\[#(\d+):\s*(.*?)\]\((.*?)\)/;
const confidenceRegex = /^\s*- Confidence:\s*(.+)/;
const evidenceRegex = /^\s*- Evidence:\s*(.+)/;

export function parseTasksDag(content) {
  const lines = content.split(/\r?\n/);
  const nodes = new Map();
  const edges = [];

  let currentIssue = null;
  let mode = null; // 'blocks' or 'blocked_by'
  let pendingEdge = null;

  lines.forEach((line, idx) => {
    const lineNumber = idx + 1;
    if (line.startsWith("## [")) {
      const issueMatch = line.match(issueRegex);
      if (pendingEdge) {
        edges.push(pendingEdge);
        pendingEdge = null;
      }
      if (issueMatch) {
        const number = parseInt(issueMatch[1], 10);
        const title = issueMatch[2];
        const url = issueMatch[3];
        currentIssue = { number, title, url };
        nodes.set(number, currentIssue);
        mode = null;
      } else {
        console.warn(`Skipping malformed TASKS-DAG header on line ${lineNumber}: ${line}`);
        currentIssue = null;
        mode = null;
      }
      return;
    }

    if (!currentIssue) return;

    if (line.trim() === "- Blocked by:") {
      mode = "blocked_by";
      if (pendingEdge) {
        edges.push(pendingEdge);
        pendingEdge = null;
      }
      return;
    }
    if (line.trim() === "- Blocks:") {
      mode = "blocks";
      if (pendingEdge) {
        edges.push(pendingEdge);
        pendingEdge = null;
      }
      return;
    }

    const linkMatch = line.match(linkRegex);
    if (linkMatch) {
      if (!mode) return;
      if (pendingEdge) {
        edges.push(pendingEdge);
        pendingEdge = null;
      }
      const targetNumber = parseInt(linkMatch[1], 10);
      if (!Number.isFinite(targetNumber)) {
        console.warn(`Skipping entry with invalid issue number on line ${lineNumber}: ${line}`);
        return;
      }
      const targetTitle = linkMatch[2];
      const targetUrl = linkMatch[3];
      if (!nodes.has(targetNumber)) {
        nodes.set(targetNumber, { number: targetNumber, title: targetTitle, url: targetUrl });
      }
      if (mode === "blocked_by") {
        pendingEdge = { from: targetNumber, to: currentIssue.number, confidence: "strong", note: "" };
      } else if (mode === "blocks") {
        pendingEdge = { from: currentIssue.number, to: targetNumber, confidence: "strong", note: "" };
      }
      return;
    }

    if (pendingEdge) {
      const confMatch = line.match(confidenceRegex);
      if (confMatch) {
        pendingEdge.confidence = confMatch[1].trim().toLowerCase();
        return;
      }
      const evMatch = line.match(evidenceRegex);
      if (evMatch) {
        pendingEdge.note = evMatch[1].trim();
        return;
      }
    }
  });

  if (pendingEdge) {
    edges.push(pendingEdge);
  }

  return { nodes, edges };
}
