#!/usr/bin/env python3
"""
Bake Criterion results into a self-contained HTML report that works over file://

Reads estimates from target/criterion for known groups and injects them into
docs/benchmarks/index.html, producing docs/benchmarks/report-inline.html with
`window.__CRITERION_DATA__` and `window.__CRITERION_MISSING__` prepopulated.

Usage:
  python3 scripts/bench_bake.py [--out docs/benchmarks/report-inline.html]
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CRITERION = ROOT / "target" / "criterion"
TEMPLATE = ROOT / "docs" / "benchmarks" / "index.html"
DEFAULT_OUT = ROOT / "docs" / "benchmarks" / "report-inline.html"

# Only bake groups the dashboard renders by default
GROUPS = [
    ("snapshot_hash", "Snapshot Hash"),
    ("scheduler_drain", "Scheduler Drain"),
    ("scheduler_drain/enqueue", "Scheduler Enqueue"),
    ("scheduler_drain/drain", "Scheduler Drain Phase"),
]
INPUTS = [10, 100, 1000, 3000, 10000, 30000]


def load_estimate(group: str, n: int):
    base = CRITERION / group / str(n)
    for kind in ("new", "base", "change"):
        p = base / kind / "estimates.json"
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
                    return {
                        "ok": False,
                        "path": str(p.relative_to(ROOT)),
                        "error": "missing mean.point_estimate",
                    }
                return {
                    "ok": True,
                    "path": str(p.relative_to(ROOT)),
                    "mean": float(mean),
                    "lb": float(lb) if lb is not None else None,
                    "ub": float(ub) if ub is not None else None,
                }
            except (json.JSONDecodeError, KeyError, TypeError, ValueError) as e:
                return {
                    "ok": False,
                    "path": str(p.relative_to(ROOT)),
                    "error": f"parse error: {e}",
                }
    return {
        "ok": False,
        "path": str((base / "new" / "estimates.json").relative_to(ROOT)),
        "error": "not found (tried new/base/change)",
    }


def build_inline_script(results, missing) -> str:
    data_json = json.dumps(results, separators=(",", ":"))
    missing_json = json.dumps(missing, separators=(",", ":"))
    return (
        f"<script>\n"
        f"window.__CRITERION_DATA__ = {data_json};\n"
        f"window.__CRITERION_MISSING__ = {missing_json};\n"
        f"</script>\n"
    )


def bake_html(out_path: Path):
    if not TEMPLATE.exists():
        sys.exit(f"Template not found: {TEMPLATE}")

    results = []
    missing = []
    for key, _label in GROUPS:
        for n in INPUTS:
            r = load_estimate(key, n)
            if r["ok"]:
                results.append({
                    "group": key,
                    "n": n,
                    "mean": r["mean"],
                    "lb": r.get("lb"),
                    "ub": r.get("ub"),
                })
            else:
                missing.append({"group": key, "n": n, "path": r["path"], "error": r["error"]})

    html = TEMPLATE.read_text()
    # Inject inline data just before the main logic script that defines GROUPS
    marker = "<script>\n      const GROUPS = ["
    inject = build_inline_script(results, missing)
    if marker in html:
        html_out = html.replace(marker, inject + marker, 1)
    else:
        # Fallback: append before closing body
        html_out = html.replace("</body>", inject + "</body>")

    out_path.write_text(html_out)
    rel = out_path.relative_to(ROOT)
    print(f"[bench-bake] Wrote {rel}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = ap.parse_args()
    out_path: Path = args.out if args.out.is_absolute() else (ROOT / args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    bake_html(out_path)


if __name__ == "__main__":
    main()
