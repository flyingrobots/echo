// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

import { execSync } from "node:child_process";

const files = [
  "AGENTS.md",
  "docs/decision-log.md",
  "TASKS-DAG.md",
  "docs/execution-plan.md",
];

const args = process.argv.slice(2);
const baseArgIndex = args.indexOf("--base");
const cliBase = baseArgIndex !== -1 ? args[baseArgIndex + 1] : null;
const baseRef = process.env.APPEND_ONLY_BASE || cliBase || "origin/main";

const errors = [];

for (const file of files) {
  let diffOutput = "";
  try {
    diffOutput = execSync(`git diff --numstat ${baseRef} -- "${file}"`, {
      encoding: "utf8",
    });
  } catch (err) {
    throw new Error(`Unable to diff ${file} against ${baseRef}: ${err.message}`);
  }
  if (!diffOutput.trim()) continue;
  for (const line of diffOutput.trim().split("\n")) {
    const [added, removed, path] = line.split("\t");
    if (path && path.trim() === file && removed && removed !== "0") {
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
