#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//
// G3 perf regression gate: compare current criterion bencher output against a
// git-tracked baseline and fail if any benchmark regresses beyond the allowed
// threshold.
//
// Usage:
//   node scripts/check_perf_regression.cjs <baseline.json> <current.log> [--threshold 15]
//
// Baseline format (perf-baseline.json):
//   { "<bench_name>": <median_ns>, ... }
//
// Current format (criterion --output-format bencher):
//   test <bench_name> ... bench:   <N> ns/iter (+/- <M>)
//
// Exit codes:
//   0 — no regressions above threshold
//   1 — one or more regressions above threshold
//   2 — usage error

"use strict";

const fs = require("fs");
const path = require("path");

const USAGE = `Usage: node ${path.basename(__filename)} <baseline.json> <current.log> [--threshold <percent>]`;

function parseArgs(argv) {
  const args = argv.slice(2);
  let threshold = 15;
  const positional = [];

  for (let i = 0; i < args.length; i++) {
    if (args[i] === "--threshold" && i + 1 < args.length) {
      threshold = Number(args[++i]);
      if (Number.isNaN(threshold) || threshold <= 0) {
        console.error("ERROR: --threshold must be a positive number");
        process.exit(2);
      }
    } else if (args[i].startsWith("-")) {
      console.error(`ERROR: unknown flag: ${args[i]}`);
      console.error(USAGE);
      process.exit(2);
    } else {
      positional.push(args[i]);
    }
  }

  if (positional.length !== 2) {
    console.error(USAGE);
    process.exit(2);
  }

  return { baselinePath: positional[0], currentPath: positional[1], threshold };
}

/** Parse criterion bencher output into { name: median_ns } */
function parseBencherOutput(text) {
  const results = {};
  // Format: "test <name> ... bench:       <N> ns/iter (+/- <M>)"
  const re = /^test\s+(\S+)\s+\.\.\.\s+bench:\s+([\d,]+)\s+ns\/iter/gm;
  let match;
  while ((match = re.exec(text)) !== null) {
    const name = match[1];
    const ns = Number(match[2].replace(/,/g, ""));
    results[name] = ns;
  }
  return results;
}

function main() {
  const { baselinePath, currentPath, threshold } = parseArgs(process.argv);

  if (!fs.existsSync(baselinePath)) {
    console.log(`No baseline found at ${baselinePath} — recording current run as baseline.`);
    console.log("G3: SKIP (no baseline to compare against)");
    process.exit(0);
  }

  const baseline = JSON.parse(fs.readFileSync(baselinePath, "utf-8"));
  const currentText = fs.readFileSync(currentPath, "utf-8");
  const current = parseBencherOutput(currentText);

  const benchNames = Object.keys(current);
  if (benchNames.length === 0) {
    console.error("ERROR: no benchmark results found in current output");
    process.exit(2);
  }

  console.log(`G3 perf regression gate (threshold: ${threshold}%)`);
  console.log("─".repeat(72));

  const report = [];
  let regressions = 0;

  for (const name of benchNames) {
    const cur = current[name];
    const base = baseline[name];

    if (base == null) {
      report.push({ name, cur, base: null, delta: null, status: "NEW" });
      continue;
    }

    const deltaPct = ((cur - base) / base) * 100;
    const regressed = deltaPct > threshold;
    if (regressed) regressions++;

    report.push({
      name,
      cur,
      base,
      delta: deltaPct,
      status: regressed ? "REGRESSED" : "OK",
    });
  }

  // Print table
  const nameWidth = Math.max(12, ...report.map((r) => r.name.length));
  const header = `${"Benchmark".padEnd(nameWidth)}  ${"Baseline".padStart(12)}  ${"Current".padStart(12)}  ${"Delta".padStart(8)}  Status`;
  console.log(header);
  console.log("─".repeat(header.length));

  for (const r of report) {
    const baseStr = r.base != null ? `${r.base} ns` : "—";
    const curStr = `${r.cur} ns`;
    const deltaStr = r.delta != null ? `${r.delta > 0 ? "+" : ""}${r.delta.toFixed(1)}%` : "—";
    const statusStr = r.status === "REGRESSED" ? `FAIL (>${threshold}%)` : r.status;
    console.log(
      `${r.name.padEnd(nameWidth)}  ${baseStr.padStart(12)}  ${curStr.padStart(12)}  ${deltaStr.padStart(8)}  ${statusStr}`
    );
  }

  console.log("─".repeat(header.length));

  // Write structured report
  const reportObj = {
    threshold_pct: threshold,
    benchmarks: report,
    regressions,
    passed: regressions === 0,
  };
  fs.writeFileSync("perf-report.json", JSON.stringify(reportObj, null, 2) + "\n");
  console.log("\nWrote perf-report.json");

  if (regressions > 0) {
    console.error(`\nG3: FAILED — ${regressions} benchmark(s) regressed beyond ${threshold}% threshold`);
    process.exit(1);
  }

  console.log(`\nG3: PASSED — all benchmarks within ${threshold}% of baseline`);
}

main();
