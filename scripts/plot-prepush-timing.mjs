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

let series = [];
let legends = [];
let plotColors = [];
const glyphPalette = ['🔴', '🟢', '🔵', '🟡', '🟣', '🟤', '⚫', '⚪'];
const colorPalette = [
  asciichart.red,
  asciichart.green,
  asciichart.blue,
  asciichart.yellow,
  asciichart.magenta,
  asciichart.cyan,
  asciichart.lightgray,
];
const runRecords = data.filter(d => d.record_type === 'run' && typeof d.elapsed_seconds === 'number');
const legacyRecords = data.filter(d => Object.prototype.hasOwnProperty.call(d, 'variant'));

if (runRecords.length === 0 && legacyRecords.length > 0) {
  const sequential = legacyRecords
    .filter(d => d.variant === 'sequential' && d.exit === 0)
    .map(d => d.duration);
  const parallel = legacyRecords
    .filter(d => d.variant === 'parallel' && d.exit === 0)
    .map(d => d.duration);
  if (sequential.length > 0) {
    series.push(sequential);
    legends.push({ label: 'sequential', color: '🔴', stats: sequential });
    plotColors.push(colorPalette[0]);
  }
  if (parallel.length > 0) {
    series.push(parallel);
    legends.push({ label: 'parallel', color: '🟢', stats: parallel });
    plotColors.push(colorPalette[1]);
  }
}

if (runRecords.length > 0) {
  const preferredModes = ['full', 'pre-push', 'pr', 'fast', 'ultra-fast'];
  const seenModes = [...new Set(runRecords.map(d => d.mode).filter(Boolean))];
  const modeNames = [
    ...preferredModes.filter(mode => seenModes.includes(mode)),
    ...seenModes.filter(mode => !preferredModes.includes(mode)).sort(),
  ];
  for (const [index, mode] of modeNames.entries()) {
    const records = runRecords.filter(d => d.mode === mode);
    const successful = records.filter(d => d.exit_status === 0);
    if (successful.length === 0) {
      continue;
    }
    const paletteIndex = index % colorPalette.length;
    series.push(successful.map(d => d.elapsed_seconds));
    legends.push({
      label: mode,
      color: glyphPalette[index % glyphPalette.length],
      stats: successful.map(d => d.elapsed_seconds),
    });
    plotColors.push(colorPalette[paletteIndex]);
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
  colors: plotColors,
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
