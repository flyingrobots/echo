#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
// Plots pre-push timing data from JSONL using asciichart

import { readFileSync } from 'fs';
import asciichart from 'asciichart';

const logfile = process.argv[2] || '.githooks/timing.jsonl';

let lines;
try {
  lines = readFileSync(logfile, 'utf-8').trim().split('\n');
} catch (e) {
  console.error(`No timing data yet. Run some pushes first!`);
  console.error(`Expected: ${logfile}`);
  process.exit(1);
}

const data = lines.map(l => JSON.parse(l));

const sequential = data.filter(d => d.variant === 'sequential').map(d => d.duration);
const parallel = data.filter(d => d.variant === 'parallel').map(d => d.duration);

if (sequential.length === 0 && parallel.length === 0) {
  console.error('No timing data found.');
  process.exit(1);
}

// Pad shorter array to match lengths for chart alignment
const maxLen = Math.max(sequential.length, parallel.length);
while (sequential.length < maxLen) sequential.push(undefined);
while (parallel.length < maxLen) parallel.push(undefined);

console.log('\n📊 Pre-push Timing Comparison (seconds)\n');
console.log(asciichart.plot([sequential, parallel], {
  height: 15,
  colors: [asciichart.red, asciichart.green],
  format: (x) => x.toFixed(1).padStart(6),
}));

console.log('\n  🔴 sequential    🟢 parallel\n');

// Stats
const seqValid = data.filter(d => d.variant === 'sequential' && d.exit === 0);
const parValid = data.filter(d => d.variant === 'parallel' && d.exit === 0);

const avg = arr => arr.reduce((a, b) => a + b, 0) / arr.length;

if (seqValid.length > 0) {
  const seqAvg = avg(seqValid.map(d => d.duration));
  console.log(`  Sequential: ${seqValid.length} runs, avg ${seqAvg.toFixed(1)}s`);
}
if (parValid.length > 0) {
  const parAvg = avg(parValid.map(d => d.duration));
  console.log(`  Parallel:   ${parValid.length} runs, avg ${parAvg.toFixed(1)}s`);
}

if (seqValid.length > 0 && parValid.length > 0) {
  const speedup = avg(seqValid.map(d => d.duration)) / avg(parValid.map(d => d.duration));
  console.log(`\n  Speedup: ${speedup.toFixed(1)}x`);
}
