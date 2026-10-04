# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
"""Prove schema selection reaches a semantic refusal through real Edict builds."""

import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile


def build(root, document, expected_kind):
    request = {
        "schema": "edict.compiler.settings/v1", "type": "compilerSettings",
        "operation": "build", document: f"edict.{document}.json",
    }
    result = subprocess.run(
        ["/edict/target/debug/edict"], cwd=root,
        input=json.dumps(request) + "\n", text=True, capture_output=True,
        timeout=120, check=False,
    )
    events = [json.loads(line) for stream in (result.stdout, result.stderr)
              for line in stream.splitlines()]
    diagnostics = [event for event in events if event.get("type") == "diagnostic"]
    if expected_kind is None:
        valid = result.returncode == 0 and not diagnostics
    else:
        valid = (result.returncode == 2 and len(diagnostics) == 1
                 and diagnostics[0]["kind"] == expected_kind)
    if not valid:
        raise RuntimeError(f"Unexpected public build boundary: {result}\n{events}")
    if expected_kind is not None:
        required_details = {
            "TargetLoweringFailed": ["obstruction_requirement_step_output_dependency"],
            "InvalidProviderInvocation": ["ArtifactSchemaMismatch", "06-target-ir"],
            "ProviderLowererRefused": ["UnsupportedSemantics", "core.echo-pure-operation"],
        }[expected_kind]
        if any(detail not in diagnostics[0]["message"] for detail in required_details):
            raise RuntimeError(f"Unexpected refusal detail: {diagnostics}")
    if document == "application":
        output = root / ".build/application"
        if output.exists() and list(output.rglob("*")):
            raise RuntimeError("Refused build published application artifacts")
    for diagnostic in diagnostics:
        print(json.dumps(diagnostic, sort_keys=True))


def main():
    if not Path("/.dockerenv").is_file():
        raise RuntimeError("Run inside a guarded Docker worker with copied sources")
    source = Path("/consumer-source/edict/replace-range-probes/state-read")
    with tempfile.TemporaryDirectory(prefix="ordered-publication-") as scratch:
        root = Path(scratch)
        for document in ("edict.lawpack.json", "edict.application.json"):
            shutil.copyfile(source / document, root / document)
        shutil.copytree(source / "src", root / "src")
        provider = root / ".build/echo-provider"
        shutil.copytree(Path("/echo-source/schemas/edict-provider/package/v1"), provider)
        build(root, "lawpack", None)
        build(root, "application", "TargetLoweringFailed")

        digest_path = root / "vendor/state-probe/manifest.sha256"
        old_digest = digest_path.read_text().strip()
        definition_path = root / "edict.lawpack.json"
        definition = json.loads(definition_path.read_text())
        selection = definition["lawpack"]["targetAdapters"][0]["acceptedTargetIr"]
        selection["id"] = "echo.span-ir/v2"
        # Experimental binding only; no released v2 semantic contract is claimed.
        schema = Path("/edict/docs/abi/edict-target-ir.cddl").read_bytes()
        selection["digest"] = "sha256:" + hashlib.sha256(schema).hexdigest()
        definition_path.write_text(json.dumps(definition, indent=2) + "\n")
        build(root, "lawpack", None)
        new_digest = digest_path.read_text().strip()
        authored = root / "src/ReplaceRange.edict"
        text = authored.read_text()
        if old_digest == new_digest or text.count(old_digest) != 1:
            raise RuntimeError("Expected exactly one changed lawpack import")
        authored.write_text(text.replace(old_digest, new_digest))
        build(root, "application", "InvalidProviderInvocation")

        # Change only the provider package for the final crossing. No executable
        # package is fabricated, and the authored operation body stays identical.
        shutil.rmtree(provider)
        shutil.copytree(Path("/ordered-provider"), provider)
        build(root, "application", "ProviderLowererRefused")
    print("ORDERED_PUBLICATION_REACHED_SEMANTIC_REFUSAL")


if __name__ == "__main__":
    main()
