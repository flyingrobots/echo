#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
"""Check the real generated policy, unknown-source refusal, and CI aperture."""
import importlib.util
from pathlib import Path
import tomllib

root = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("policy", root / "scripts/keep_deny_policy.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
policy_text = (root / "deny.toml").read_text()
manifest_text = (root / "experiments/echo-keep/Cargo.toml").read_text()
derived = tomllib.loads(module.derive_policy(policy_text, manifest_text))
original = tomllib.loads(policy_text)
assert derived["sources"].pop("allow-git") == [module.KEEP_SOURCE]
assert derived == original
for invalid in [manifest_text.replace(module.KEEP_SOURCE, "https://example.com/unknown"), manifest_text.replace("3165890e9291cfb5fe10e81a9d7cd151f3e59464", "main")]:
    try:
        module.derive_policy(policy_text, invalid)
    except ValueError:
        pass
    else:
        raise AssertionError("unreviewed source or unpinned revision was admitted")
workflow = (root / ".github/workflows/echo-keep-experimental.yml").read_text()
assert "manifest-path: experiments/echo-keep/Cargo.toml" in workflow
assert "--config /github/workspace/.echo-keep-deny.toml" in workflow
print("PASS: complete root policy retained, only pinned Keep source admitted, isolated CI graph checked")
