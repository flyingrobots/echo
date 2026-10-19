# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
"""Build the benchmark ledger mechanically from campaign outputs and run logs.

Usage: python3 -I ledger.py CAMPAIGN_DIR LOG_DIR > ledger.md

Every figure in the output is copied from a log line; nothing is typed by
hand. Each entry names its campaign output, its log, the commit the run
reported, the cargo command, and the exit code.
"""
import re
import sys
from pathlib import Path

# (campaign output, title, note, log suffix for this campaign's labels)
CAMPAIGNS = [
    ("campaign-1-stopped.out", "Combo, first pass", "Stopped by hand after two runs, when release footprint enforcement was dropped. Script: an earlier version of `campaign.sh` with `-enf` steps.", ""),
    ("campaign.out", "Combo: M2 alone (`2e79511c`) vs M2 + lazy blockers (`e9f29548`)", "Script: `campaign.sh`. Benchmarks used their default settings.", ""),
    ("stack1.out", "Inline single worker (`f5d8b71b` before, `43a8ddda` after)", "Script: `stack1.sh`. Benchmarks used their default settings.", ""),
    ("stack2.out", "Commit-path fixes (`a0bd72da` before, `f118d7ef` after)", "Script: `stack2.sh`, which set ECHO_BENCH_REWRITES=100,10000 ECHO_BENCH_WORK=0,100 ECHO_BENCH_WORKERS=1,18 ECHO_BENCH_ROUNDS=25. **The `stack-fmt`, `stack-clippy` and `stack-clippy-legacy` entries are void**: uncommitted files were added to and then removed from that worktree while they ran.", ""),
    ("hello-1-misrouted.out", "hello-echo, first pass", "**Only `hello-pin` is valid.** For `hello-main` and `hello-stack` the Echo checkout was refused (the lock wrapper's temp directory was missing) and they ran at the pin again, as their headers show.", "-misrouted"),
    ("hello.out", "hello-echo at Echo main and at the stack", "Script: `hello.sh`. Edict at its pin `df80f92`; Rust 1.96 for all three repositories (Echo's pin asks for 1.90). Both runs stop with no message; `hello-hash.out` (script `hello-hash.sh`) shows why: the built verification report's hash differs from the one `tests/runtime.sh` expects.", ""),
    ("hello-edictmain.out", "hello-echo with Edict at its origin/main tip", "Script: `hello-edictmain.sh`. No expected hash overridden. The log traces `tests/runtime.sh` with `sh -x`.", ""),
]
HEADER = re.compile(r"^== (?P<label>\S+) @ (?:echo )?(?P<rev>[^ ,]+)(?: :: cargo (?P<cmd>.*)|, edict (?P<edict>\S+))$")
RC = re.compile(r"^== rc=(?P<rc>-?\d+)$")


def results(log: Path) -> list[str]:
    if not log.exists():
        return ["(log missing)"]
    lines = log.read_text(errors="replace").splitlines()
    out = [l for l in lines if l.startswith("|")]
    out += [l for l in lines if l.startswith("F5 tier=") and "summary" in l]
    out += [l for l in lines if l.startswith("runtime witness suite")]
    out += [l for l in lines if " differ: " in l]
    tests = [l for l in lines if l.startswith("test result:")]
    if tests:
        passed = sum(int(m.group(1)) for l in tests if (m := re.search(r"(\d+) passed", l)))
        failed = sum(int(m.group(1)) for l in tests if (m := re.search(r"(\d+) failed", l)))
        out.append(f"summed `test result:` lines: {passed} passed, {failed} failed "
                   "(parallel output can merge a line; see FAILED names)")
        out += sorted({l for l in lines if l.startswith("test ") and l.endswith("FAILED")})
    return out or ["(no result lines)"]


# Runs whose outputs are quoted verbatim: (output, title, note).
VERBATIM = [
    ("hello-hash.out", "hello-echo build artifact hashes", "Script: `hello-hash.sh`. `tests/runtime.sh` at the old pins expected package `3665d692…` and verification report `2541c826…`. The guard's `rc=1` comes from the script's last line, a `grep` that matched nothing; the three hash lines are complete."),
    ("hello-override.out", "hello-echo with the report hash overridden (run the user rejected)", "Script: `hello-override.sh`. The user rejected this command, but it had already started: it edited `tests/runtime.sh`, passed at Echo `main`, and was cut off during the stack run. Not used as evidence of anything except that it ran."),
    ("hello-fix-1.out", "hello-echo fix, run 1", "Script: `hello-fix.sh` (hello-echo's CI sequence). Pins advanced to Edict `0c8cf37`, Echo `4e801ef2`; causal-cell re-vendored."),
    ("hello-fix-2.out", "hello-echo fix, run 2", "Source import digest updated from the fixture's `manifest.sha256`."),
    ("hello-fix-3.out", "hello-echo fix, run 3", "Package, verification-report and manifest goldens set to the new producers' outputs."),
    ("hello-fix-4.out", "hello-echo fix, run 4", "Projection identity golden updated (Edict `cfcf61e`)."),
    ("hello-fix-5.out", "hello-echo fix, run 5: every CI step passes at Echo main", "Typed result identities recomputed with `result_ids.py`, which first reproduced all 13 previous values."),
    ("hello-fix-stack.out", "hello-echo CI sequence at the stack `f118d7ef`", "Same as run 5 with the lock's Echo commit set to `f118d7ef` for this run only."),
]


def verbatim(campaigns: Path) -> None:
    for name, title, note in VERBATIM:
        path = campaigns / name
        if not path.exists():
            continue
        print(f"## {title}\n\nCampaign output: `{name}`. {note}\n\n```text")
        for line in path.read_text(errors="replace").splitlines():
            if line.startswith(("== ", "GUARD END", "pin ", "main ", "stack ")) or "passed" in line or "assertions: all" in line:
                if not line.startswith("+"):
                    print(line[:240])
        print("```\n")


def main() -> None:
    campaigns, logs = Path(sys.argv[1]), Path(sys.argv[2])
    print("<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->")
    print("<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->\n")
    print("# Benchmark ledger\n")
    print("Generated by `ledger.py` from the campaign outputs and run logs. "
          "Figures are copied from log lines, not typed. A run's commit is the "
          "one its runner printed before running.\n")
    machine = Path(sys.argv[3]).read_text().strip() if len(sys.argv) > 3 else ""
    if machine:
        print(f"Machine: {machine}\n")
    for name, title, note, suffix in CAMPAIGNS:
        path = campaigns / name
        if not path.exists():
            continue
        print(f"## {title}\n\nCampaign output: `{name}`. {note}\n")
        current = None
        for line in path.read_text(errors="replace").splitlines():
            if m := HEADER.match(line):
                current = m.groupdict()
                continue
            if (m := RC.match(line)) and current:
                label = current["label"]
                log_name = f"{label}{suffix if label != 'hello-pin' else ''}.log"
                if current["cmd"]:
                    print(f"### `{label}`\n\n- commit: `{current['rev']}`\n"
                          f"- command: `cargo {current['cmd']}`\n- exit code: {m.group('rc')}\n"
                          f"- log: `{log_name}`\n")
                else:
                    print(f"### `{label}`\n\n- Echo: `{current['rev']}`, Edict: `{current['edict']}`\n"
                          f"- exit code: {m.group('rc')}\n- log: `{log_name}`\n")
                print("```text")
                for result in results(logs / log_name):
                    print(result)
                print("```\n")
                current = None
            elif line.startswith("GUARD END") or line.startswith("== ABORT"):
                print(f"Runner: `{line}`\n")


if __name__ == "__main__":
    main()
    verbatim(Path(sys.argv[1]))
