#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
import json
import statistics
import sys
from pathlib import Path

CRITERION = Path("target/criterion")

GROUPS = [
    "snapshot_hash",
    "scheduler_drain",
    "scheduler_drain/enqueue",
    "scheduler_drain/drain",
]
INPUTS = [10, 100, 1000, 3000, 10000, 30000]

def fmt_ns(ns):
    if ns < 1000: return f"{ns:.2f} ns"
    if ns < 1e6: return f"{ns/1000:.2f} µs"
    if ns < 1e9: return f"{ns/1e6:.2f} ms"
    return f"{ns/1e9:.2f} s"

def main():
    print("### Benchmark Results (Median from latest run)\n")
    print("| Group | Input (n) | Median Time | Samples |")
    print("| :--- | :--- | :--- | :--- |")

    for group in GROUPS:
        for n in INPUTS:
            # We look for 'new/sample.json' which is the latest run
            path = CRITERION / group / str(n) / "new" / "sample.json"
            
            if not path.exists():
                # Fallback to 'base/sample.json' if 'new' doesn't exist
                path = CRITERION / group / str(n) / "base" / "sample.json"
            
            if not path.exists():
                continue

            try:
                content = json.loads(path.read_text())
                iters = content["iters"]
                times = content["times"]
                
                # Calculate time per iteration (ns)
                samples_ns = [t / i for t, i in zip(times, iters)]
                
                if not samples_ns:
                    continue

                med_ns = statistics.median(samples_ns)
                count = len(samples_ns)
                val_str = fmt_ns(med_ns)
                
                print(f"| {group} | {n} | {val_str} | {count} |")
            except Exception as e:
                pass

if __name__ == "__main__":
    main()
