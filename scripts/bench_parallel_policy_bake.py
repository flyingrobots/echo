#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

"""
Export the parallel policy matrix benchmark as raw JSON.

Reads Criterion estimates from:
  target/criterion/parallel_policy_matrix/**/new|base|change/estimates.json

Emits:
  - docs/benchmarks/parallel-policy-matrix.json
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CRITERION = ROOT / "target" / "criterion" / "parallel_policy_matrix"
DEFAULT_JSON_OUT = ROOT / "docs" / "benchmarks" / "parallel-policy-matrix.json"


def load_estimate(bench_dir: Path):
    for kind in ("new", "base", "change"):
        p = bench_dir / kind / "estimates.json"
        if p.exists():
            try:
                obj = json.loads(p.read_text())
                mean = (
                    obj.get("mean", {}).get("point_estimate")
                    if isinstance(obj.get("mean"), dict)
                    else None
                )
                if mean is None and isinstance(obj.get("Mean"), dict):
                    mean = obj["Mean"].get("point_estimate")
                lb = (
                    obj.get("mean", {})
                    .get("confidence_interval", {})
                    .get("lower_bound")
                )
                ub = (
                    obj.get("mean", {})
                    .get("confidence_interval", {})
                    .get("upper_bound")
                )
                if mean is None:
                    return None
                return {
                    "path": str(p.relative_to(ROOT)),
                    "mean_ns": float(mean),
                    "lb_ns": float(lb) if lb is not None else None,
                    "ub_ns": float(ub) if ub is not None else None,
                }
            except (json.JSONDecodeError, KeyError, TypeError, ValueError):
                return None
    return None


def parse_case(rel_parts: tuple[str, ...]):
    if len(rel_parts) == 2:
        policy_case, load = rel_parts
        m = re.fullmatch(
            r"(?P<policy>.+)_(?P<workers>(?:1w|4w|8w))",
            policy_case,
        )
        if m:
            policy = m.group("policy")
            workers = m.group("workers")
        else:
            policy = policy_case
            workers = "dedicated"
    elif len(rel_parts) == 3:
        policy, workers, load = rel_parts
    else:
        return None

    try:
        load_int = int(load)
    except ValueError:
        return None

    return {
        "policy": policy,
        "workers": workers,
        "load": load_int,
    }


def collect_results():
    if not CRITERION.is_dir():
        return []

    results = []
    for bench_dir in CRITERION.rglob("*"):
        if not bench_dir.is_dir():
            continue
        estimate = load_estimate(bench_dir)
        if estimate is None:
            continue
        rel = bench_dir.relative_to(CRITERION)
        case = parse_case(rel.parts)
        if case is None:
            continue
        results.append({
            **case,
            **estimate,
            "series": f"{case['policy']}:{case['workers']}",
        })

    results.sort(key=lambda r: (r["workers"], r["policy"], r["load"]))
    return results


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--json-out", type=Path, default=DEFAULT_JSON_OUT)
    args = ap.parse_args()

    json_out = args.json_out if args.json_out.is_absolute() else (ROOT / args.json_out)
    json_out.parent.mkdir(parents=True, exist_ok=True)

    results = collect_results()
    payload = {
        "group": "parallel_policy_matrix",
        "results": results,
    }
    json_out.write_text(json.dumps(payload, indent=2) + "\n")
    print(f"[bench-parallel-policy] Wrote {json_out.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
