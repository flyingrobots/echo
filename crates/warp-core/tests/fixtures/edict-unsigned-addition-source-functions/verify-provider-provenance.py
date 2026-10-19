#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
"""Refuse a rebuild whose provider differs from the retained publication."""
import json
from pathlib import Path
import sys

app = Path(sys.argv[1])
expected = json.loads((app / "provenance.json").read_text())["providerPackage"]
actual = json.loads(
    (app / ".build/echo-provider/provider-manifest.echo.json").read_text()
)["provider"]
if actual != expected:
    raise SystemExit(f"provider provenance mismatch: {actual!r} != {expected!r}")
print("Provider publication matches retained provenance")
