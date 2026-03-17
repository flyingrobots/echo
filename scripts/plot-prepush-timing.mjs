#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
// Plots pre-push timing data from JSONL using asciichart

import { readFileSync } from 'fs';
import asciichart from 'asciichart';

const logfile = process.argv[2] || '.git/verify-local/timing.jsonl';

let lines;
try {
  lines = readFileSync(logfile, 'utf-8').trim().split('\n');
} catch (e) {
  console.error(`No timing data yet. Run some pushes first!`);
  console.error(`Expected: ${logfile}`);
  process.exit(1);
}

// Parse JSONL with error handling for malformed lines
const data = [];
for (const line of lines) {
  const trimmed = line.trim();
  if (!trimmed) continue; // Skip empty lines
  try {
    data.push(JSON.parse(trimmed));
  } catch (e) {
    console.error(`Warning: skipping malformed line in ${logfile}:`);
    console.error(`  ${trimmed}`);
    console.error(`  ${e.message}`);
  }
}

const median = arr => {
  const sorted = [...arr].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
};

const hasLegacyFormat = data.some(d => Object.prototype.hasOwnProperty.call(d, 'variant'));

let series = [];
let legends = [];

if (hasLegacyFormat) {
  const sequential = data.filter(d => d.variant === 'sequential').map(d => d.duration);
  const parallel = data.filter(d => d.variant === 'parallel').map(d => d.duration);
  if (sequential.length > 0) {
    series.push(sequential);
    legends.push({ label: 'sequential', color: '🔴', stats: data.filter(d => d.variant === 'sequential' && d.exit === 0).map(d => d.duration) });
  }
  if (parallel.length > 0) {
    series.push(parallel);
    legends.push({ label: 'parallel', color: '🟢', stats: data.filter(d => d.variant === 'parallel' && d.exit === 0).map(d => d.duration) });
  }
} else {
  const runRecords = data.filter(d => d.record_type === 'run' && typeof d.elapsed_seconds === 'number');
  const preferredModes = ['full', 'pre-push', 'pr', 'fast', 'ultra-fast'];
  const modeNames = preferredModes.filter(mode => runRecords.some(d => d.mode === mode));
  for (const mode of modeNames) {
    const records = runRecords.filter(d => d.mode === mode);
    series.push(records.map(d => d.elapsed_seconds));
    legends.push({
      label: mode,
      color: ['🔴', '🟢', '🔵', '🟡', '🟣'][legends.length] || '⚪',
      stats: records.filter(d => d.exit_status === 0).map(d => d.elapsed_seconds),
    });
  }
}

if (series.length === 0) {
  console.error('No timing data found.');
  process.exit(1);
}

const maxLen = Math.max(...series.map(s => s.length));
series = series.map(values => {
  const padded = [...values];
  while (padded.length < maxLen) padded.push(undefined);
  return padded;
});

console.log('\n📊 Verify-local Timing Comparison (seconds)\n');
console.log(asciichart.plot(series, {
  height: 15,
  colors: [asciichart.red, asciichart.green, asciichart.blue, asciichart.yellow, asciichart.magenta],
  format: (x) => x.toFixed(1).padStart(6),
}));

console.log('');
for (const entry of legends) {
  console.log(`  ${entry.color} ${entry.label}`);
}
console.log('');

for (const entry of legends) {
  if (entry.stats.length === 0) {
    continue;
  }
  const med = median(entry.stats);
  console.log(`  ${entry.label}: ${entry.stats.length} successful runs, median ${med.toFixed(1)}s`);
}
