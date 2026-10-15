#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
"""Check that every feature-gated integration test is declared and scheduled.

An integration test whose crate-level ``#![cfg(...)]`` requires a feature
compiles to an empty target without it, and a run then reports "running 0
tests ... ok". So each such test must (1) declare those features as
``required-features`` in its crate manifest, so a run without them skips the
target, and (2) be executed with them by some CI ``cargo test`` or
``cargo nextest run`` command in a workflow that runs on pushes or pull
requests (#867).
"""
from __future__ import annotations

import glob
import os
import re
import shlex
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FEATURE = re.compile(r'feature\s*=\s*"([^"]+)"')


def balanced(text: str, open_index: int) -> int:
    """Index just past the parenthesis group that starts at ``open_index``."""
    depth = 0
    for i in range(open_index, len(text)):
        if text[i] == "(":
            depth += 1
        elif text[i] == ")":
            depth -= 1
            if depth == 0:
                return i + 1
    return len(text)


def required_features(source: str) -> set[str]:
    """Positive features required by every crate-level ``#![cfg(...)]``."""
    features: set[str] = set()
    for match in re.finditer(r"#!\[cfg\(", source):
        expression = source[match.end() - 1 : balanced(source, match.end() - 1)]
        # Drop negated groups: `not(feature = "b")` requires nothing.
        while (index := expression.find("not(")) != -1:
            expression = expression[:index] + expression[balanced(expression, index + 3) :]
        features.update(FEATURE.findall(expression))
    return features


def manifest_tests(manifest_path: str) -> tuple[str, dict[str, set[str]]]:
    """The package name and each ``[[test]]`` target's required features.

    ``required-features`` may span several lines. (``tomllib`` would parse
    this fully but needs Python 3.11; this reader covers what Cargo test
    tables use.)
    """
    package, tests, section, current = "", {}, None, None
    collecting = None
    with open(manifest_path, encoding="utf-8") as manifest:
        for raw in manifest:
            line = raw.split("#", 1)[0].strip()
            if collecting is not None:
                tests[collecting].update(re.findall(r'"([^"]+)"', line.split("]", 1)[0]))
                if "]" in line:
                    collecting = None
                continue
            if line.startswith("["):
                section, current = line, None
                continue
            if section == "[package]" and line.startswith("name") and not package:
                package = line.split("=", 1)[1].strip().strip('"')
            elif section == "[[test]]" and re.match(r"name\s*=", line):
                current = line.split("=", 1)[1].strip().strip('"')
                tests.setdefault(current, set())
            elif section == "[[test]]" and line.startswith("required-features") and current:
                values = line.split("=", 1)[1]
                tests[current].update(re.findall(r'"([^"]+)"', values.split("]", 1)[0]))
                if "]" not in values:
                    collecting = current
    return package, tests


AUTOMATIC_TRIGGERS = {"push", "pull_request", "pull_request_target", "merge_group"}


def runs_automatically(text: str) -> bool:
    """Whether a workflow's ``on:`` runs it for pushes, pull requests or the
    merge queue. Manual (``workflow_dispatch``) and scheduled-only workflows
    do not gate a change, so their commands are not coverage."""
    lines = text.splitlines()
    for index, raw in enumerate(lines):
        match = re.match(r"""^["']?on["']?\s*:\s*(.*)$""", raw)
        if not match:
            continue
        inline = match.group(1).split("#", 1)[0].strip()
        if inline:
            return bool(AUTOMATIC_TRIGGERS & set(re.findall(r"[a-z_]+", inline)))
        triggers, indent = set(), None
        for line in lines[index + 1 :]:
            if not line.strip() or line.lstrip().startswith("#"):
                continue
            depth = len(line) - len(line.lstrip())
            if depth == 0:
                break
            indent = depth if indent is None else indent
            if depth == indent:
                triggers.add(line.strip().lstrip("- ").split(":", 1)[0].strip())
        return bool(AUTOMATIC_TRIGGERS & triggers)
    return False


def ci_commands() -> list[list[str]]:
    """Token lists of every executable cargo test / nextest command in CI."""
    commands = []
    for workflow in sorted(glob.glob(os.path.join(ROOT, ".github", "workflows", "*.yml"))):
        with open(workflow, encoding="utf-8") as handle:
            text = handle.read()
        if not runs_automatically(text):
            continue
        lines = text.splitlines()
        joined, buffer = [], ""
        for raw in lines:
            line = raw.strip()
            if line.startswith("run:"):
                line = line[4:].strip()
            if line.startswith("#"):
                continue
            if line.endswith("\\"):
                buffer += line[:-1] + " "
                continue
            joined.append(buffer + line)
            buffer = ""
        for line in joined:
            for segment in re.split(r"&&|\|\||;|\|", line):
                try:
                    tokens = shlex.split(segment)
                except ValueError:
                    continue
                if "cargo" not in tokens:
                    continue
                rest = tokens[tokens.index("cargo") + 1 :]
                rest = [token for token in rest if not token.startswith("+")]
                if rest[:1] == ["test"]:
                    commands.append(rest[1:])
                elif rest[:2] == ["nextest", "run"]:
                    commands.append(rest[2:])
    return commands


def runs_target(command: list[str], package: str, target: str, features: set[str]) -> bool:
    """Whether ``command`` executes integration test ``target`` of ``package``."""
    packages, enabled, tests = set(), set(), set()
    all_features = all_tests = other_filter = False
    index = 0
    while index < len(command):
        token = command[index]
        if token == "--":
            break
        value = command[index + 1] if index + 1 < len(command) else ""
        if token in ("-p", "--package"):
            packages.add(value)
            index += 2
            continue
        if token.startswith("--package="):
            packages.add(token.split("=", 1)[1])
        elif token in ("-F", "--features"):
            enabled.update(re.split(r"[ ,]+", value))
            index += 2
            continue
        elif token.startswith("--features="):
            enabled.update(re.split(r"[ ,]+", token.split("=", 1)[1]))
        elif token == "--all-features":
            all_features = True
        elif token == "--test":
            tests.add(value)
            index += 2
            continue
        elif token == "--tests":
            all_tests = True
        elif token in ("--lib", "--bins", "--bin", "--doc", "--examples", "--example",
                       "--benches", "--bench"):
            other_filter = True
        index += 1
    if package not in packages:
        return False
    if tests:
        selected = target in tests
    elif all_tests:
        selected = True
    else:
        selected = not other_filter
    return selected and (all_features or features <= enabled)


def main() -> int:
    os.chdir(ROOT)
    commands = ci_commands()
    problems = []
    for test_file in sorted(glob.glob("crates/*/tests/*.rs")):
        with open(test_file, encoding="utf-8") as handle:
            features = required_features(handle.read())
        if not features:
            continue
        crate_dir = test_file.split("/tests/", 1)[0]
        package, declared = manifest_tests(os.path.join(crate_dir, "Cargo.toml"))
        target = os.path.splitext(os.path.basename(test_file))[0]
        missing = sorted(features - declared.get(target, set()))
        if missing:
            problems.append(
                f"MISSING required-features: {package} test `{target}` needs "
                f"{', '.join(missing)} in [[test]] of {crate_dir}/Cargo.toml"
            )
        if not any(runs_target(command, package, target, features) for command in commands):
            problems.append(
                f"UNSCHEDULED: no CI cargo test/nextest command runs {package} test "
                f"`{target}` with features {', '.join(sorted(features))}"
            )
    for problem in problems:
        print(problem)
    if problems:
        print(f"check-feature-gated-tests: {len(problems)} problem(s)", file=sys.stderr)
        return 1
    print("check-feature-gated-tests: every feature-gated test target is declared and scheduled")
    return 0


if __name__ == "__main__":
    sys.exit(main())
