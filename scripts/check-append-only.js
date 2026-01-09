// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

import { execFileSync } from "node:child_process";

const files = [
  "AGENTS.md",
  "docs/decision-log.md",
  "TASKS-DAG.md",
  "docs/execution-plan.md",
];

const args = process.argv.slice(2);
const baseArgIndex = args.indexOf("--base");
let cliBase = null;
if (baseArgIndex !== -1) {
  cliBase = args[baseArgIndex + 1];
  if (!cliBase) {
    console.error("Error: --base requires a value (e.g., --base origin/main)");
    process.exit(2);
  }
}
// Precedence: explicit CLI > environment override > default.
const baseRef = cliBase || process.env.APPEND_ONLY_BASE || "origin/main";

const errors = [];

for (const file of files) {
  let diffOutput = "";
  try {
    diffOutput = execFileSync("git", ["diff", "--numstat", baseRef, "--", file], {
      encoding: "utf8",
    });
  } catch (err) {
    throw new Error(`Unable to diff ${file} against ${baseRef}: ${err.message}`);
  }
  if (!diffOutput.trim()) continue;
  for (const line of diffOutput.trim().split("\n")) {
    const parts = line.split("\t");
    if (parts.length < 3) continue;
    const [, removedRaw, pathRaw] = parts;
    const path = pathRaw?.trim();
    const removed = Number.parseInt(removedRaw, 10);
    if (!path || !Number.isFinite(removed)) continue;
    if (path === file && removed > 0) {
      errors.push(
        `${file} has ${removed} deletions when compared to ${baseRef}; append-only edits must not remove or change existing lines.`,
      );
    }
  }
}

if (errors.length > 0) {
  throw new Error(["Append-only invariant violations detected:", ...errors].join("\n"));
}

console.log(`Append-only check passed (base: ${baseRef}).`);
