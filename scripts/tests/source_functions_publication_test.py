# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
"""Deterministic publication-witness regressions; execute only in Docker."""
import contextlib
import importlib.util
import io
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    "publication", Path(__file__).parents[1] / "consumer-witnesses/source-functions-publication.py")
publication = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(publication)


class PublicationTests(unittest.TestCase):
    def repeat(self, emit_repeated):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            provider = root / "selected-provider"
            provider.mkdir()
            (provider / "provider-manifest.echo.json").write_text("{}")
            compiler = root / "compiler"
            compiler.write_bytes(b"selected compiler")
            manifest = root / "compiler-manifest"
            manifest.write_text("{}")
            work = root / "work"
            work.mkdir()
            args = SimpleNamespace(work_root=work, compiler_revision="selected",
                compiler=compiler, compiler_manifest=manifest, input_manifest_sha256="selected")

            def prepare(args, destination, source, **options):
                destination.mkdir()
                for name in ["source.edict", "upstream-source.edict", "edict.application.json", "upstream-application.json"]:
                    (destination / name).write_text(source)
                return {}

            def build(args, destination, document, label):
                output = destination / "application-output"
                if label == "first" or emit_repeated:
                    output.mkdir(exist_ok=True)
                    for name in publication.OUTPUTS:
                        (output / name).write_bytes(b"first build")
                return {"exitCode": 0, "diagnostics": [],
                    "artifacts": publication.inventory(output) if output.exists() else {}}

            with patch.object(publication, "prepare_application", prepare), patch.object(publication, "compiler_build", build), contextlib.redirect_stdout(io.StringIO()):
                return publication.run_variant(args, "repeat", "authored source", provider)

    def test_successful_repeat_without_outputs_cannot_reuse_first_build(self):
        with self.assertRaisesRegex(RuntimeError, "did not emit exactly one package/report pair"):
            self.repeat(False)

    def test_repeat_emitting_the_same_pair_is_accepted(self):
        result = self.repeat(True)
        self.assertEqual(result["firstBuild"]["artifacts"], result["repeatedBuild"]["artifacts"])


if __name__ == "__main__":
    unittest.main()
