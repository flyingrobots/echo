# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
"""Compile authored reads through unmodified Edict and both provider components."""

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess


OUTPUTS = {"executable-operation-package.cbor", "verification-report.cbor"}


def build(root, document, refusal=False):
    request = {
        "schema": "edict.compiler.settings/v1", "type": "compilerSettings",
        "operation": "build", document: f"edict.{document}.json",
    }
    result = subprocess.run(
        [os.environ.get("EDICT_READ_COMPILER", "/edict/target/debug/edict")], cwd=root,
        input=json.dumps(request) + "\n", text=True, capture_output=True,
        timeout=120, check=False,
    )
    events, raw = [], []
    for stream in (result.stdout, result.stderr):
        for line in stream.splitlines():
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                raw.append(line)
                continue
            if isinstance(event, dict):
                events.append(event)
            else:
                raw.append(line)
    diagnostics = [event for event in events if event.get("type") == "diagnostic"]
    statuses = [event for event in events if event.get("type") == "status"]
    code = 2 if refusal else 0
    if (raw or result.returncode != code or len(statuses) != 1
            or statuses[0].get("exitCode") != code):
        raise RuntimeError(
            f"Unexpected public compiler result: {result}\nEvents: {events}\nRaw: {raw}"
        )
    if refusal:
        if (len(diagnostics) != 1
                or diagnostics[0].get("kind") != "InvalidProviderInvocation"
                or any(detail not in diagnostics[0].get("message", "")
                       for detail in ("ArtifactSchemaMismatch", "05-target-configuration"))):
            raise RuntimeError(f"Unexpected old-provider refusal: {diagnostics}")
        if (root / ".build/application").exists():
            raise RuntimeError("Refused compiler published application output")
    elif diagnostics:
        raise RuntimeError(f"Successful compiler produced diagnostics: {diagnostics}")
    for event in events:
        print(json.dumps(event, sort_keys=True), flush=True)


def compile_variant(name, source):
    root = Path("/read-evidence") / name
    root.mkdir(parents=True)
    fixtures = Path("/read-fixtures/source")
    for document in ("edict.application.json", "edict.lawpack.json"):
        shutil.copyfile(fixtures / document, root / document)
    (root / "src").mkdir()
    shutil.copyfile(source, root / "src/ReplaceRange.edict")
    authored = (root / "src/ReplaceRange.edict").read_bytes()
    build(root, "lawpack")
    digest = (root / "vendor/atom-read/manifest.sha256").read_text().strip()
    if authored.count(f'digest "{digest}"'.encode()) != 1:
        raise RuntimeError("Authored import does not match public lawpack build")
    provider = root / ".build/echo-provider"
    shutil.copytree("/old-provider", provider)
    build(root, "application", refusal=True)
    shutil.rmtree(provider)
    shutil.copytree("/read-provider", provider)
    build(root, "application")
    output = root / ".build/application"
    files = {path.name: path.read_bytes() for path in output.iterdir() if path.is_file()}
    if set(files) != OUTPUTS or any(not data for data in files.values()):
        raise RuntimeError("Compiler did not publish exactly the verified package/report pair")
    build(root, "application")
    repeated = {path.name: path.read_bytes() for path in output.iterdir() if path.is_file()}
    if files != repeated or (root / "src/ReplaceRange.edict").read_bytes() != authored:
        raise RuntimeError("Repeated compilation changed artifacts or authored source")
    print(json.dumps({"variant": name, "outputs": {
        path: hashlib.sha256(data).hexdigest() for path, data in sorted(files.items())
    }}, sort_keys=True), flush=True)
    return files


def main():
    if not Path("/.dockerenv").is_file():
        raise RuntimeError("Run inside a guarded Docker worker with copied sources")
    single = compile_variant("single", Path("/read-fixtures/source/ReplaceRange.edict"))
    pair = compile_variant("pair", Path("/read-fixtures/pair/ReplaceRange.edict"))
    if any(single[name] == pair[name] for name in OUTPUTS):
        raise RuntimeError("Independent-address pair did not change package and report")
    print("BOUNDED_READ_PUBLIC_COMPILER_AND_VERIFIER_ACCEPTED", flush=True)


if __name__ == "__main__":
    main()
