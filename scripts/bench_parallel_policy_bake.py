#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

"""
Export the parallel policy matrix benchmark as JSON and bake an inline HTML view.

Reads Criterion estimates from:
  target/criterion/parallel_policy_matrix/**/new|base|change/estimates.json

Emits:
  - docs/benchmarks/parallel-policy-matrix.json
  - docs/benchmarks/parallel-policy-matrix-inline.html
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CRITERION = ROOT / "target" / "criterion" / "parallel_policy_matrix"
TEMPLATE = ROOT / "docs" / "benchmarks" / "parallel-policy-matrix.html"
DEFAULT_JSON_OUT = ROOT / "docs" / "benchmarks" / "parallel-policy-matrix.json"
DEFAULT_HTML_OUT = ROOT / "docs" / "benchmarks" / "parallel-policy-matrix-inline.html"


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


def build_inline_script(results):
    data_json = json.dumps(results, indent=2)
    return f"<script>\nwindow.__POLICY_MATRIX__ = {data_json};\n</script>\n"


def bake_html(results, out_path: Path):
    if not TEMPLATE.exists():
        sys.exit(f"Template not found: {TEMPLATE}")

    html = TEMPLATE.read_text()
    marker = "<script>\n      const DATA_URL = 'parallel-policy-matrix.json';"
    inject = build_inline_script(results)
    if marker in html:
        html_out = html.replace(marker, inject + marker, 1)
    else:
        html_out = html.replace("</body>", inject + "</body>")
    out_path.write_text(html_out)
    print(f"[bench-parallel-policy] Wrote {out_path.relative_to(ROOT)}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--json-out", type=Path, default=DEFAULT_JSON_OUT)
    ap.add_argument("--html-out", type=Path, default=DEFAULT_HTML_OUT)
    args = ap.parse_args()

    json_out = args.json_out if args.json_out.is_absolute() else (ROOT / args.json_out)
    html_out = args.html_out if args.html_out.is_absolute() else (ROOT / args.html_out)
    json_out.parent.mkdir(parents=True, exist_ok=True)
    html_out.parent.mkdir(parents=True, exist_ok=True)

    results = collect_results()
    payload = {
        "group": "parallel_policy_matrix",
        "results": results,
    }
    json_out.write_text(json.dumps(payload, indent=2) + "\n")
    print(f"[bench-parallel-policy] Wrote {json_out.relative_to(ROOT)}")
    bake_html(results, html_out)


if __name__ == "__main__":
    main()
