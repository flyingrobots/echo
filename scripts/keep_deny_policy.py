#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
"""Derive the experimental policy without weakening the root workspace policy."""
import argparse
import json
from pathlib import Path
import re
import tomllib

KEEP_SOURCE = "https://github.com/flyingrobots/keep"

def derive_policy(policy_text, manifest_text):
    """Preserve all root rules, admitting only the inspected Keep Git source."""
    policy = tomllib.loads(policy_text)
    dependency = tomllib.loads(manifest_text)["dependencies"]["keep"]
    if dependency.get("git") != KEEP_SOURCE or not re.fullmatch(r"[0-9a-f]{40}", dependency.get("rev", "")):
        raise ValueError("experiment requires the reviewed Keep source and an exact revision")
    if "allow-git" in policy["sources"] or policy_text.count("[sources]") != 1:
        raise ValueError("root source policy changed; reconcile the scoped exception explicitly")
    result = policy_text.replace("[sources]", "[sources]\nallow-git = [" + json.dumps(KEEP_SOURCE) + "]", 1)
    parsed = tomllib.loads(result)
    assert parsed["sources"].pop("allow-git") == [KEEP_SOURCE]
    assert parsed == policy
    return result

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    result = derive_policy((root / "deny.toml").read_text(), (root / "experiments/echo-keep/Cargo.toml").read_text())
    with args.output.open("x") as output:
        output.write(result)

if __name__ == "__main__":
    main()
