#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
import subprocess
import json
import statistics
import sys
from pathlib import Path

ROOT = Path(".").resolve()
CRITERION = ROOT / "target" / "criterion"
OUT_JSON = ROOT / "docs" / "benchmarks" / "data-raw-accumulated.json"

GROUPS = [
    "snapshot_hash",
    "scheduler_drain",
    "scheduler_drain/enqueue",
    "scheduler_drain/drain",
]
INPUTS = [10, 100, 1000, 3000, 10000, 30000]

def run_bench():
    # Run cargo bench with reduced time (1s) to make 10 runs feasible
    cmd = ["cargo", "bench", "-p", "warp-benches", "--", "--measurement-time", "1"]
    # Stream output to console so user sees progress; capture stderr for error reporting
    subprocess.run(cmd, check=True, capture_output=False)

def extract_samples(run_idx):
    data = []
    for group in GROUPS:
        for n in INPUTS:
            path = CRITERION / group / str(n) / "new" / "sample.json"
            if not path.exists():
                # Try finding without "new" if it was a regression run?
                # Criterion structure is fairly stable.
                continue
            
            try:
                content = json.loads(path.read_text())
                iters = content["iters"]
                times = content["times"]

                # Validate lengths match before zipping
                if len(times) != len(iters):
                    print(f"Warning: {path}: times/iters length mismatch ({len(times)} vs {len(iters)})", file=sys.stderr)
                    continue

                # Calculate time per iteration (ns)
                samples_ns = [t / i for t, i in zip(times, iters, strict=True)]

                data.append({
                    "group": group,
                    "n": n,
                    "run": run_idx,
                    "samples_ns": samples_ns
                })
            except json.JSONDecodeError as e:
                print(f"Error parsing JSON {path}: {e}", file=sys.stderr)
            except KeyError as e:
                print(f"Missing key in {path}: {e}", file=sys.stderr)
            except ValueError as e:
                print(f"Value error reading {path}: {e}", file=sys.stderr)
    return data

def fmt_ns(ns):
    if ns < 1000:
        return f"{ns:.2f} ns"
    if ns < 1e6:
        return f"{ns/1000:.2f} µs"
    if ns < 1e9:
        return f"{ns/1e6:.2f} ms"
    return f"{ns/1e9:.2f} s"

def main():
    accumulated = []
    
    print("Starting 10 benchmark runs (approx 4-5 mins)...")
    for i in range(1, 11):
        print(f"Run {i}/10...", end="", flush=True)
        try:
            run_bench()
            run_data = extract_samples(i)
            accumulated.extend(run_data)
            print(" Done.")
        except subprocess.CalledProcessError as e:
            stderr_msg = getattr(e, 'stderr', None)
            if stderr_msg:
                print(f"\nBenchmark failed: {stderr_msg.decode() if isinstance(stderr_msg, bytes) else stderr_msg}")
            else:
                print(f"\nBenchmark failed: {e}")
            sys.exit(1)
        
    # Save raw accumulated
    OUT_JSON.parent.mkdir(parents=True, exist_ok=True)
    OUT_JSON.write_text(json.dumps(accumulated, separators=( ",", ":")))
    print(f"\nSaved accumulated data to {OUT_JSON}")
    
    # Process for Median Table
    grouped = {}
    for entry in accumulated:
        key = (entry["group"], entry["n"])
        if key not in grouped:
            grouped[key] = []
        grouped[key].extend(entry["samples_ns"])
        
    print("\n### Benchmark Results (Median of 10 runs)\n")
    print("| Group | Input (n) | Median Time | Samples |")
    print("| :--- | :--- | :--- | :--- |")
    
    # Sort groups for consistent order
    for group in GROUPS:
        for n in INPUTS:
            key = (group, n)
            if key not in grouped:
                continue
            
            samples = grouped[key]
            med_ns = statistics.median(samples)
            count = len(samples)
            val_str = fmt_ns(med_ns)
            print(f"| {group} | {n} | {val_str} | {count} |")

if __name__ == "__main__":
    main()
