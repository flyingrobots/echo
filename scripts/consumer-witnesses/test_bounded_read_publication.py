# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
"""Check that the public compiler witness preserves unexpected output evidence."""

import contextlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch


SPEC = importlib.util.spec_from_file_location(
    "bounded_read_publication", Path(__file__).with_name("bounded-read-publication.py")
)
WITNESS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(WITNESS)


class BuildOutputTests(unittest.TestCase):
    def invoke(self, result, refusal=False):
        with tempfile.TemporaryDirectory() as directory:
            with patch.object(WITNESS.subprocess, "run", return_value=result):
                with contextlib.redirect_stdout(io.StringIO()):
                    WITNESS.build(Path(directory), "application", refusal)

    def test_crash_reports_return_code_and_both_streams(self):
        result = subprocess.CompletedProcess(
            ["edict"], 101, '{"type":"status","status":"error"}\n',
            "thread 'main' panicked at compiler failure\n",
        )
        with self.assertRaises(Exception) as raised:
            self.invoke(result)
        self.assertIsInstance(raised.exception, RuntimeError)
        message = str(raised.exception)
        for detail in ("returncode=101", "status", "panicked", "stdout=", "stderr="):
            self.assertIn(detail, message)

    def test_json_scalar_preserves_boundary_evidence(self):
        result = subprocess.CompletedProcess(["edict"], 2, "null\n", "")
        with self.assertRaises(Exception) as raised:
            self.invoke(result)
        self.assertIsInstance(raised.exception, RuntimeError)
        self.assertIn("returncode=2", str(raised.exception))
        self.assertIn("null", str(raised.exception))

    def test_unstructured_output_cannot_pass_as_a_success(self):
        result = subprocess.CompletedProcess(["edict"], 0, "", "unexpected log\n")
        with self.assertRaises(Exception) as raised:
            self.invoke(result)
        self.assertIsInstance(raised.exception, RuntimeError)
        self.assertIn("unexpected log", str(raised.exception))

    def test_expected_refusal_still_passes(self):
        diagnostic = {
            "type": "diagnostic", "kind": "InvalidProviderInvocation",
            "message": "ArtifactSchemaMismatch 05-target-configuration",
        }
        result = subprocess.CompletedProcess(
            ["edict"], 2, '{"type":"status","exitCode":2}\n', json.dumps(diagnostic) + "\n",
        )
        self.invoke(result, True)

    def test_clean_success_still_passes(self):
        result = subprocess.CompletedProcess(["edict"], 0, '{"type":"status","exitCode":0}\n', "")
        self.invoke(result)


if __name__ == "__main__":
    unittest.main()
