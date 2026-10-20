# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
"""Build the stack-on-main benchmark ledger from campaign outputs and run logs.

Usage: python3 -I ledger.py RUNS_DIR > ledger.md
       python3 -I ledger.py RUNS_DIR --summary   (body of summary.md)

Both outputs are then formatted with `prettier --write`, which only aligns
table columns.

RUNS_DIR holds the campaign outputs and, under `logs/`, the run logs they
name. The runners write `*.out` and `*.log`; they are stored here as `.txt`
because the repository ignores those extensions. Every figure in the output is copied from a log line;
nothing is typed by hand. Each entry names its log, the commit the runner
printed before running, the cargo command and the exit code.
"""
import re
import sys
from pathlib import Path

# (campaign output, title, note)
CAMPAIGNS = [
    ("campaign1.txt", "Checks and full suite: main, inline worker, commit path",
     "Script: `checks.sh` per worktree, run by `campaign1.sh`. Each run is void unless HEAD and a clean tree hold at both ends."),
    ("campaign2.txt", "F6 on main: checks, debug test, first bench run",
     "Script: `campaign2.sh`. F6 at `f045e04d`, before its clippy fix."),
    ("campaign3.txt", "Measurements: main + benches, + inline worker, + commit path",
     "Script: `campaign3.sh`. Revisions on `claude/measure-stack`: base `73e17289`, inline `af3c3e7c`, both `21c6531e`. "
     "Worker sweep with ECHO_BENCH_REWRITES=100,10000 ECHO_BENCH_WORK=0,100 ECHO_BENCH_WORKERS=1,18 ECHO_BENCH_ROUNDS=25."),
    ("campaign4.txt", "Final checks on every pushed candidate, and profiles",
     "Script: `campaign4.sh`. `checks2.sh` adds `scripts/ban-globals.sh` to the handoff's sequence."),
    ("campaign5.txt", "hello-echo against Echo main; checks on the stack tip",
     "Script: `campaign5.sh`, which runs `hello-fix.sh` (hello-echo's CI sequence; its per-step `sh -x` traces are not committed, and this output carries each step's exit code and summary line) and then `checks2.sh`."),
]

RUN = re.compile(r"^== (?P<label>\S+) @ (?P<rev>\S+) :: cargo (?P<cmd>.*)$")
CHECK = re.compile(r"^== (?P<label>\S+) (?P<what>start|end) HEAD=(?P<rev>[0-9a-f]+) dirty=(?P<dirty>\d+)$")
STEP = re.compile(r"^== (?P<step>[a-z-]+) rc=(?P<rc>-?\d+)$")
KEEP = ("|", "F5 tier=", "F6 scene=", "F5D ", "ALLOC ", "== suite passed=")


def picked(log: Path) -> list[str]:
    if not log.exists():
        return ["(log missing)"]
    lines = log.read_text(errors="replace").splitlines()
    out = [l for l in lines if l.startswith(KEEP) and ("summary" in l or not l.startswith("F5 tier=")) and not l.startswith("F5D round")]
    tests = [l for l in lines if l.startswith("test result:")]
    if tests:
        passed = sum(int(m.group(1)) for l in tests if (m := re.search(r"(\d+) passed", l)))
        failed = sum(int(m.group(1)) for l in tests if (m := re.search(r"(\d+) failed", l)))
        out.append(f"summed `test result:` lines: {passed} passed, {failed} failed")
        out += sorted({l for l in lines if l.startswith("test ") and l.endswith("FAILED")})
    return out or ["(no result lines)"]


REVS = [("base", "73e17289"), ("inline", "af3c3e7c"), ("both", "21c6531e")]


def fields(line: str) -> dict[str, str]:
    return dict(kv.split("=", 1) for kv in line.split() if "=" in kv)


def lines_of(path: Path) -> list[str]:
    return path.read_text(errors="replace").splitlines() if path.exists() else []


def table(head: list[str], rows: list[list[str]]) -> None:
    print("| " + " | ".join(head) + " |")
    print("| " + " | ".join("---" if i == 0 else "---:" for i in range(len(head))) + " |")
    for row in rows:
        print("| " + " | ".join(row) + " |")
    print()


def summary(logs: Path) -> None:
    print("## Summary tables\n")
    print("Each cell is copied from the named log. Revisions: "
          + ", ".join(f"{n} `{r}`" for n, r in REVS) + ".\n")
    print("### Engine tick commit, worker sweep (`c3-*-workers.txt`), p50 ms\n")
    rows: dict[tuple[str, str, str], dict[str, str]] = {}
    for n, _ in REVS:
        for line in lines_of(logs / f"c3-{n}-workers.txt"):
            cells = [c.strip() for c in line.strip("|").split("|")]
            if len(cells) == 5 and cells[0].isdigit():
                rows.setdefault((cells[0], cells[1], cells[2]), {})[n] = cells[3]
    table(["rewrites, hashes/rewrite, workers"] + [n for n, _ in REVS],
          [[", ".join(k)] + [v.get(n, "-") for n, _ in REVS] for k, v in rows.items()])
    print("### F5 append and replay (`c3-*-f5.txt`), p50 ms\n")
    f5: dict[tuple[str, str], dict[str, str]] = {}
    for n, _ in REVS:
        for line in lines_of(logs / f"c3-{n}-f5.txt"):
            if line.startswith("F5 tier=") and " summary " in line:
                f = fields(line)
                f5.setdefault((f["tier"], f["profile"]), {})[n] = f"{f['append_p50_ms']} / {f['replay_p50_ms']}"
    table(["tier, profile (append / replay)"] + [n for n, _ in REVS],
          [[", ".join(k)] + [v.get(n, "-") for n, _ in REVS] for k, v in f5.items()])
    print("### F6 frames (`c3-*-f6.txt`), 600 frames per scene\n")
    print("`echo_model` is the frame less twice the reference step time for the same frame (a stated model, not a measurement of Echo alone).\n")
    cols = ["frame_p50_ms", "frame_p99_ms", "frame_max_ms", "over_16.7ms", "physics_p50_ms", "echo_model_p50_ms", "replay_ms"]
    f6rows = []
    for n, _ in REVS:
        for line in lines_of(logs / f"c3-{n}-f6.txt"):
            if line.startswith("F6 scene=") and " frames=" in line:
                f = fields(line)
                f6rows.append([f"{f['scene']}, {f['profile']}, {n}"] + [f.get(c, "-") for c in cols])
    f6rows.sort(key=lambda r: r[0].split(", ")[:2])
    table(["scene, profile, revision"] + cols, f6rows)
    print("### F5 tier M with durability (`c3-*-f5d.txt`)\n")
    drows = []
    for n, _ in REVS:
        for line in lines_of(logs / f"c3-{n}-f5d.txt"):
            if line.startswith("F5D tier=") and " summary " in line:
                f = fields(line)
                drows.append([f"{f['profile']}, {f['durability']}, {n}", f.get("append_p50_ms", "-"),
                              f.get("per_append_p50_ms", "-"), f.get("wal_bytes_max", "-")])
    table(["profile, durability, revision", "append p50 ms", "per-append p50 ms", "WAL bytes (max)"], drows)
    print("### Records in F5's warp after tier M (`c3-*-f5d.txt`, round 1)\n")
    seen = set()
    crows = []
    for n, _ in REVS:
        for line in lines_of(logs / f"c3-{n}-f5d.txt"):
            if line.startswith("F5D census"):
                f = fields(line)
                key = tuple(v for k, v in f.items() if k not in ("profile", "durability"))
                crows.append([f"{f['profile']}, {f['durability']}, {n}"] + list(key))
                head = [k for k in f if k not in ("profile", "durability")]
    if crows:
        table(["profile, durability, revision"] + head, crows)
    alloc = [l for l in lines_of(logs / "c4-alloc.txt") if l.startswith("ALLOC")]
    if alloc:
        print("### Allocations per committed engine tick (`c4-alloc.txt`, stack tip)\n")
        table(["rewrites", "tick", "allocations", "bytes", "allocations/rewrite", "bytes/rewrite"],
              [[f["rewrites"], f["tick"], f["allocations"], f["bytes"], f["allocations_per_rewrite"], f["bytes_per_rewrite"]]
               for f in map(fields, alloc)])
    for name, title in [("c4-prof-10k-sub.txt", "Profile: 10,000-write tick at the stack tip, under `commit_with_receipt`"),
                        ("c4-prof-pile-sub.txt", "Profile: F6 pile, legacy profile, at the stack tip")]:
        body = lines_of(logs / name)
        if body:
            print(f"### {title} (`{name}`)\n\n```text")
            for line in body:
                print(line)
            print("```\n")


def main() -> None:
    runs = Path(sys.argv[1])
    logs = runs / "logs"
    if len(sys.argv) > 2 and sys.argv[2] == "--summary":
        summary(logs)
        return
    print("<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->")
    print("<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->\n")
    print("# Benchmark ledger: the stack on main\n")
    print("Generated by `ledger.py` from the campaign outputs and run logs in "
          "`runs/`. Figures are copied from log lines, not typed. A run's "
          "commit is the one its runner printed before running.\n")
    for name, title, note in CAMPAIGNS:
        path = runs / name
        if not path.exists():
            continue
        print(f"## {title}\n\nCampaign output: `{name}`. {note}\n")
        lines = path.read_text(errors="replace").splitlines()
        rc_after: dict[int, str] = {}
        for i, line in enumerate(lines):
            if line.startswith("== rc="):
                rc_after[i] = line[6:]
        label = None
        for i, line in enumerate(lines):
            if m := RUN.match(line):
                rc = next((rc_after[j] for j in range(i + 1, len(lines)) if j in rc_after), "?")
                print(f"### `{m['label']}` at `{m['rev']}`\n\n`cargo {m['cmd']}`, exit code {rc}, log `logs/{m['label']}.txt`.\n\n```text")
                for out in picked(logs / f"{m['label']}.txt"):
                    print(out[:400])
                print("```\n")
            elif m := CHECK.match(line):
                if m["what"] == "start":
                    label = m["label"]
                    print(f"### Checks `{label}` at `{m['rev'][:8]}`\n\n```text\n{line}")
                else:
                    print(line)
            elif label and line.startswith((f"== {label} RESULT", "== VOID")):
                print(line)
                print("```\n")
                suite = logs / f"{label}-suite.txt"
                if suite.exists():
                    print(f"Suite log `logs/{label}-suite.txt`:\n\n```text")
                    for out in picked(suite):
                        print(out[:400])
                    print("```\n")
                label = None
            elif label and (STEP.match(line) or line.startswith("== suite passed")):
                print(line)
            elif line.startswith(("== hello", "== syntax", "== lock", "== runtime", "== effect", "== patch", "== cleans",
                                  "== assertions", "== host-", "== ABORT", "HELLO FIX DONE")) or (
                    line.startswith("== rc=") and label is None):
                print(f"`{line[:300]}`\n")
            elif line.startswith(("GUARD", '{"event"')):
                print(f"`{line[:300]}`\n")


if __name__ == "__main__":
    main()
