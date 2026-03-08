#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//
// Parse criterion bencher output and emit a perf-baseline.json to stdout.
//
// Usage:
//   node scripts/generate_perf_baseline.cjs <perf.log> > perf-baseline.json

"use strict";

const fs = require("fs");
const path = require("path");

if (process.argv.length !== 3) {
  console.error(`Usage: node ${path.basename(__filename)} <perf.log>`);
  process.exit(2);
}

const text = fs.readFileSync(process.argv[2], "utf-8");
const baseline = {};

// Format: "test <name> ... bench:       <N> ns/iter (+/- <M>)"
const re = /^test\s+(\S+)\s+\.\.\.\s+bench:\s+([\d,]+)\s+ns\/iter/gm;
let match;
while ((match = re.exec(text)) !== null) {
  baseline[match[1]] = Number(match[2].replace(/,/g, ""));
}

if (Object.keys(baseline).length === 0) {
  console.error("ERROR: no benchmark results found in input");
  process.exit(1);
}

// Sort keys for stable diffs
const sorted = {};
for (const k of Object.keys(baseline).sort()) {
  sorted[k] = baseline[k];
}

console.log(JSON.stringify(sorted, null, 2));
