# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
"""Build authored source functions with Edict and explicitly selected providers.

Run only in an admitted, resource-guarded reusable Docker worker. This witness
does not build compilers/components, synthesize IR, or execute Echo programs.
The paired runtime witness consumes its unchanged package/report bytes.

The caller supplies a hash-pinned JSON --input-manifest whose fields are the
selected_inputs() result, in addition to the pinned compiler source snapshot.
The compiler binary hash identifies the executed binary; source-to-binary build
provenance belongs to the caller's guarded build receipt. This script does not
claim reproducible compilation from a source/binary hash pair alone.

Eight sequential variants retain at most 64 MiB / 2048 filesystem entries here,
with 1 MiB per child-created file and 120 seconds per compiler invocation.
The shared runner must also enforce the aggregate project budget and host/VM
free-space floors, covering this work root, compiler caches and container layer.
On failure, compiler process groups are stopped and partial evidence is kept.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import shutil
import signal
import stat
import subprocess
import time


MAX_INPUT_BYTES = 16 * 1024 * 1024
MAX_INPUT_FILES = 256
MAX_CHILD_FILE_BYTES = 1024 * 1024
MAX_WORK_BYTES = 64 * 1024 * 1024
MAX_WORK_ENTRIES = 2048
BUILD_TIMEOUT_SECONDS = 120
READ_OLD_STEPS = 1_048_576
READ_HELPER_STEPS = 4_194_304
READ_BUDGET = "jedit.text@1.replaceRangeBudget"
OUTPUTS = {"executable-operation-package.cbor", "verification-report.cbor"}
PURE_COORDINATE = "jedit.text.replace_range@1"
RENAMED_COORDINATE = "consumer.fragment_assembly@1"
PURE_HELPERS = """fn assembleFragments(left: Bytes<max=8>, right: Bytes<max=8>) -> Bytes<max=16> {
  let bytes = joinFragments(left, right);
  return bytes;
}

fn joinFragments(left: Bytes<max=8>, right: Bytes<max=8>) -> Bytes<max=16> {
  let bytes = left + right;
  return bytes;
}

"""
READ_HELPER = """fn retainBytes(value: text.FactBytes) -> text.FactBytes {
  let bytes = value;
  return bytes;
}

"""


def regular_file(path, maximum=MAX_INPUT_BYTES):
    if path.is_symlink() or not path.is_file() or path.stat().st_size > maximum:
        raise RuntimeError(f"Expected a bounded regular file: {path}")
    return path


def sha256(path):
    digest = hashlib.sha256()
    with regular_file(path, maximum=256 * 1024 * 1024).open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def inventory(root):
    """Bound selected trees before copying; reject links and non-file leaves."""
    if root.is_symlink() or not root.is_dir():
        raise RuntimeError(f"Expected a regular directory: {root}")
    result, size = {}, 0
    for path in root.rglob("*"):
        if path.is_symlink():
            raise RuntimeError(f"Selected tree contains a symlink: {path}")
        if path.is_dir():
            continue
        if not path.is_file():
            raise RuntimeError(f"Selected tree contains a special file: {path}")
        size += path.stat().st_size
        if size > MAX_INPUT_BYTES or len(result) >= MAX_INPUT_FILES:
            raise RuntimeError(f"Selected tree exceeds witness bounds: {root}")
        result[str(path.relative_to(root))] = sha256(path)
    return dict(sorted(result.items()))


def verify_compiler(args):
    manifest = json.loads(regular_file(args.compiler_manifest).read_text())
    if not manifest.get("files") or len(manifest["files"]) > 16384:
        raise RuntimeError("Expected a bounded, nonempty compiler source snapshot")
    if manifest["head"] != args.compiler_revision:
        raise RuntimeError("Compiler source revision differs from explicit selection")
    for name, expected in manifest["files"].items():
        relative = Path(name)
        if relative.is_absolute() or ".." in relative.parts:
            raise RuntimeError("Unsafe compiler snapshot path")
        path = args.compiler_source / relative
        if not path.resolve(strict=True).is_relative_to(args.compiler_source):
            raise RuntimeError("Compiler source escaped its explicit root")
        if sha256(path) != expected:
            raise RuntimeError(f"Compiler source changed: {name}")


def selected_inputs(args):
    return {
        "compilerBinarySha256": sha256(args.compiler),
        "compilerManifestSha256": sha256(args.compiler_manifest),
        "pureApplicationSha256": sha256(args.pure_application_source / "edict.application.json"),
        "pureSourceSha256": sha256(args.pure_source),
        "pureCasesSha256": sha256(args.pure_cases),
        "pureVendor": inventory(args.pure_application_source / "vendor"),
        "readApplicationSha256": sha256(args.read_application_source / "edict.application.json"),
        "readLawpackSha256": sha256(args.read_application_source / "edict.lawpack.json"),
        "readSourceSha256": sha256(args.read_source),
        "oldProvider": inventory(args.old_provider),
        "provider": inventory(args.provider),
    }


def replace_once(source, old, new):
    if source.count(old) != 1:
        raise RuntimeError(f"Expected one source anchor: {old!r}")
    return source.replace(old, new, 1)


def source_variants(args):
    baseline = regular_file(args.pure_source).read_text()
    pure = replace_once(baseline, "intent assembleRange(", PURE_HELPERS + "intent assembleRange(")
    pure = replace_once(pure,
                        "let bytes = input.firstFragment + input.secondFragment;",
                        "let bytes = assembleFragments(input.firstFragment, input.secondFragment);")
    renamed = replace_once(pure, f"package {PURE_COORDINATE};",
                           f"package {RENAMED_COORDINATE};")
    renamed = replace_once(renamed, "intent assembleRange(", "intent combineFragments(")
    for old, new in (("assembleFragments", "composePieces"), ("joinFragments", "appendPieces")):
        if renamed.count(old) != 2:
            raise RuntimeError(f"Expected one definition and call for {old}")
        renamed = renamed.replace(old, new)
    changed = replace_once(pure, "let bytes = left + right;", "let bytes = right + left;")
    read = regular_file(args.read_source).read_text()
    read = replace_once(read, "intent replaceRange(", READ_HELPER + "intent replaceRange(")
    read = replace_once(read, "  return actual;",
                        "  let retained = retainBytes(actual);\n  return retained;")
    return {"function-free": baseline, "pure": pure, "renamed-pure": renamed,
            "changed-body": changed, "read": read}


def child_limits():
    # This caps every child-created file, including artifacts and both streams.
    resource.setrlimit(resource.RLIMIT_FSIZE, (MAX_CHILD_FILE_BYTES, MAX_CHILD_FILE_BYTES))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


def stop_group(child):
    try:
        os.killpg(child.pid, signal.SIGTERM)
    except ProcessLookupError:
        return
    try:
        child.wait(timeout=5)
    except subprocess.TimeoutExpired:
        pass
    try:
        os.killpg(child.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    child.wait()


def check_work_bound(root):
    """Supplement the shared guard with a live bound on this witness's outputs."""
    total, entries = 0, 0
    for path in root.rglob("*"):
        entries += 1
        if entries > MAX_WORK_ENTRIES:
            raise RuntimeError("Witness output count exceeded")
        try:
            metadata = path.lstat()
        except FileNotFoundError:
            # Atomic compiler publication can remove a temporary regular file.
            continue
        if stat.S_ISDIR(metadata.st_mode):
            continue
        if not stat.S_ISREG(metadata.st_mode):
            raise RuntimeError(f"Witness output is not a regular file: {path}")
        total += metadata.st_size
        if total > MAX_WORK_BYTES:
            raise RuntimeError("Witness outputs exceed the 64 MiB local ceiling")
    return {"bytes": total, "entries": entries}


def compiler_build(args, root, document, label):
    request = {"schema": "edict.compiler.settings/v1", "type": "compilerSettings",
               "operation": "build", document: f"edict.{document}.json"}
    logs = root / "logs"
    logs.mkdir(exist_ok=True)
    temporary = root / "temporary"
    temporary.mkdir(exist_ok=True)
    environment = dict(os.environ, TMPDIR=str(temporary), TMP=str(temporary),
                       TEMP=str(temporary), XDG_CACHE_HOME=str(temporary / "cache"))
    streams = [logs / f"{label}.stdout.jsonl", logs / f"{label}.stderr.jsonl"]
    check_work_bound(args.work_root)
    with streams[0].open("xb") as stdout, streams[1].open("xb") as stderr:
        child = subprocess.Popen([str(args.compiler)], cwd=root, stdin=subprocess.PIPE,
                                 stdout=stdout, stderr=stderr, start_new_session=True,
                                 env=environment, preexec_fn=child_limits)
        deadline = time.monotonic() + BUILD_TIMEOUT_SECONDS
        payload = (json.dumps(request) + "\n").encode()
        try:
            while True:
                try:
                    child.communicate(payload, timeout=0.25)
                    break
                except subprocess.TimeoutExpired:
                    payload = None
                    check_work_bound(args.work_root)
                    if time.monotonic() >= deadline:
                        raise RuntimeError(f"Compiler exceeded {BUILD_TIMEOUT_SECONDS}s; "
                                           f"partial streams retained at {logs}")
        finally:
            # Kill descendants even after their direct parent exits successfully.
            stop_group(child)
    check_work_bound(args.work_root)
    events = []
    for path in streams:
        regular_file(path, MAX_CHILD_FILE_BYTES)
        for line in path.read_text().splitlines():
            event = json.loads(line)
            if not isinstance(event, dict):
                raise RuntimeError(f"Non-object compiler event retained in {path}")
            events.append(event)
    statuses = [event for event in events if event.get("type") == "status"]
    if len(statuses) != 1 or statuses[0].get("exitCode") != child.returncode:
        raise RuntimeError(f"Compiler status disagrees with exit {child.returncode}: {streams}")
    output = root / "application-output"
    return {"exitCode": child.returncode,
            "diagnostics": [event for event in events if event.get("type") == "diagnostic"],
            "artifacts": inventory(output) if output.exists() else {},
            "stdoutSha256": sha256(streams[0]), "stderrSha256": sha256(streams[1])}


def require_success(result, application=True):
    if result["exitCode"] != 0 or result["diagnostics"]:
        raise RuntimeError(f"Public compiler refused the authored program: {result}")
    if application and set(result["artifacts"]) != OUTPUTS:
        raise RuntimeError(f"Compiler did not emit exactly one package/report pair: {result}")


def require_old_refusal(result):
    diagnostics = result["diagnostics"]
    if (result["exitCode"] != 2 or result["artifacts"] or len(diagnostics) != 1
            or diagnostics[0].get("kind") != "InvalidProviderInvocation"
            or "ArtifactSchemaMismatch" not in diagnostics[0].get("message", "")
            or "core.artifact" not in diagnostics[0].get("message", "")):
        raise RuntimeError(f"Old provider did not explicitly refuse source Core: {result}")


def require_budget_refusal(result):
    diagnostics = result["diagnostics"]
    if result["exitCode"] != 2 or result["artifacts"] or len(diagnostics) != 1:
        raise RuntimeError(f"Original budget did not produce one explicit refusal: {result}")
    diagnostic = diagnostics[0]
    message = diagnostic.get("message", "")
    boundaries = {
        "ApplicationCompilationFailed": ("InvalidBound", "pure-helper cost",
                                         "exceeds operation budget"),
        "ProviderLowererRefused": ("UnsupportedSemantics", "core.source-functions"),
    }
    required = boundaries.get(diagnostic.get("kind"))
    if required is None or any(detail not in message for detail in required):
        raise RuntimeError(f"Unexpected original-budget refusal: {result}")
    return diagnostic["kind"]


def relative_path(text):
    path = Path(text)
    if not text or path.is_absolute() or ".." in path.parts or path == Path("."):
        raise RuntimeError(f"Expected a contained relative authored path: {text}")
    return path


def prepare_application(args, root, source, *, read=False, renamed=False, read_steps=None):
    root.mkdir()
    original = args.read_application_source if read else args.pure_application_source
    original_application = regular_file(original / "edict.application.json")
    shutil.copyfile(original_application, root / "upstream-application.json")
    application = json.loads(original_application.read_text())
    application["sources"] = ["source.edict"]
    application["target"]["providerPackage"] = "provider"
    application["outputDirectory"] = "application-output"
    for bundle in application["lawpacks"]:
        for value in bundle.values():
            if relative_path(value).parts[0] != "vendor":
                raise RuntimeError("Selected lawpack inputs must remain in vendor/")
    if renamed:
        if application["coordinate"] != PURE_COORDINATE:
            raise RuntimeError("Unexpected original application coordinate")
        application["coordinate"] = RENAMED_COORDINATE
    (root / "edict.application.json").write_text(json.dumps(application, indent=2) + "\n")
    (root / "source.edict").write_text(source)
    shutil.copyfile(args.read_source if read else args.pure_source,
                    root / "upstream-source.edict")
    if not read:
        shutil.copytree(original / "vendor", root / "vendor")
        if root.name in {"pure", "renamed-pure"}:
            shutil.copyfile(args.pure_cases, root / "cases.json")
        return None
    original_lawpack = regular_file(original / "edict.lawpack.json")
    shutil.copyfile(original_lawpack, root / "upstream-lawpack.json")
    lawpack = json.loads(original_lawpack.read_text())
    if lawpack["dependencyBundles"]:
        raise RuntimeError("The read seed must have no external dependency bundles")
    output = relative_path(lawpack["outputDirectory"])
    if output != Path("vendor/atom-read"):
        raise RuntimeError("Expected the explicit node-atom-read output directory")
    authored = lawpack["lawpack"]
    if len(authored["targetAdapters"]) != 1:
        raise RuntimeError("Expected exactly one authored read adapter")
    adapter = authored["targetAdapters"][0]
    if adapter["coordinate"] != "jedit.text.echo-adapter/v1":
        raise RuntimeError("Unexpected read adapter coordinate")
    budget = adapter["budgets"][READ_BUDGET]
    if budget != {"maxSteps": READ_OLD_STEPS, "maxAllocatedBytes": 16_777_216,
                  "maxOutputBytes": 8_388_608}:
        raise RuntimeError("Original read budget differs from the explicit seed")
    configs = [item for item in authored["localResources"] if item["name"] == "echo-config"]
    if (len(configs) != 1 or configs[0]["value"]["programKind"]
            != "compiler-produced-bounded-read/v1"):
        raise RuntimeError("Read seed must already select the bounded-read profile")
    for item in [adapter, *authored["localResources"]]:
        relative_path(item["output"])
    if read_steps not in {READ_OLD_STEPS, READ_HELPER_STEPS}:
        raise RuntimeError("Unexpected authored budget variant")
    budget["maxSteps"] = read_steps
    definition = root / "edict.lawpack.json"
    if read_steps == READ_OLD_STEPS:
        shutil.copyfile(original_lawpack, definition)
    else:
        definition.write_text(json.dumps(lawpack, indent=2) + "\n")
    result = compiler_build(args, root, "lawpack", "lawpack")
    require_success(result, application=False)
    new_digest = regular_file(root / output / "manifest.sha256").read_text().strip()
    if not re.fullmatch(r"sha256:[0-9a-f]{64}", new_digest):
        raise RuntimeError("Public lawpack build emitted an invalid manifest digest")
    pattern = r'(use lawpack jedit\.text@1 digest ")[^"\n]+(" as text;)'
    updated, count = re.subn(pattern, lambda match: match[1] + new_digest + match[2], source)
    if count != 1:
        raise RuntimeError("Expected exactly one explicit authored lawpack import")
    (root / "source.edict").write_text(updated)
    return {"build": result, "manifestDigest": new_digest,
            "outputs": inventory(root / output),
            "upstreamLawpackSha256": sha256(root / "upstream-lawpack.json"),
            "authoredLawpackSha256": sha256(definition),
            "authoredChanges": [] if read_steps == READ_OLD_STEPS else [{
                "path": f"/lawpack/targetAdapters/0/budgets/{READ_BUDGET}/maxSteps",
                "before": READ_OLD_STEPS, "after": read_steps}],
            "authoredBeforeImportSha256": hashlib.sha256(source.encode()).hexdigest(),
            "authoredAfterImportSha256": sha256(root / "source.edict")}


def run_variant(args, name, source, provider, *, refusal=None, read=False,
                renamed=False, read_steps=None):
    root = args.work_root / name
    lawpack = prepare_application(args, root, source, read=read, renamed=renamed,
                                  read_steps=read_steps)
    shutil.copytree(provider, root / "provider")
    authored_hash = sha256(root / "source.edict")
    result = compiler_build(args, root, "application", "first")
    provenance = {"kind": "public-compiler-output", "compilerRevision": args.compiler_revision,
                  "compilerBinarySha256": sha256(args.compiler),
                  "compilerManifestSha256": sha256(args.compiler_manifest),
                  "providerManifestSha256": sha256(provider / "provider-manifest.echo.json"),
                  "inputManifestSha256": args.input_manifest_sha256,
                  "applicationSha256": sha256(root / "edict.application.json"),
                  "upstreamApplicationSha256": sha256(root / "upstream-application.json"),
                  "sourceSha256": authored_hash, "firstBuild": result,
                  "upstreamSourceSha256": sha256(root / "upstream-source.edict"),
                  "authoredTransformation": name,
                  "casesSha256": sha256(root / "cases.json") if (root / "cases.json").exists() else None,
                  "lawpack": lawpack}
    (root / "provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")
    if refusal == "old-publication":
        require_old_refusal(result)
        provenance["refusalBoundary"] = "InvalidProviderInvocation"
    elif refusal == "original-budget":
        provenance["refusalBoundary"] = require_budget_refusal(result)
    else:
        require_success(result)
        shutil.rmtree(root / "application-output")
        repeated = compiler_build(args, root, "application", "repeated")
        provenance["repeatedBuild"] = repeated
        (root / "provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")
        require_success(repeated)
        if result["artifacts"] != repeated["artifacts"]:
            raise RuntimeError(f"Repeated public build changed exact output bytes: {name}")
        for filename in sorted(OUTPUTS):
            original = root / "application-output" / filename
            if original.stat().st_size == 0:
                raise RuntimeError("Compiler emitted an empty artifact")
            shutil.copyfile(original, root / filename)
            if sha256(root / filename) != result["artifacts"][filename]:
                raise RuntimeError("Retained compiler output bytes changed during copy")
    (root / "provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")
    if sha256(root / "source.edict") != authored_hash:
        raise RuntimeError("Public compilation changed authored source")
    # All authoritative provider inputs are retained externally and rehashed.
    # Recycle only this witness's disposable copy to keep peak storage bounded.
    if inventory(root / "provider") != inventory(provider):
        raise RuntimeError("Public compilation changed its provider copy")
    shutil.rmtree(root / "provider")
    check_work_bound(args.work_root)
    print(json.dumps({"variant": name, **provenance}, sort_keys=True), flush=True)
    return provenance


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("compiler", "compiler-source", "compiler-manifest", "pure-application-source",
                 "pure-source", "pure-cases", "read-application-source", "read-source",
                 "old-provider", "provider", "data-root", "work-root", "input-manifest"):
        parser.add_argument("--" + name, type=Path, required=True)
    for name in ("compiler-revision", "old-provider-manifest-sha256", "provider-manifest-sha256",
                 "input-manifest-sha256"):
        parser.add_argument("--" + name, required=True)
    args = parser.parse_args()
    if not Path("/.dockerenv").is_file():
        raise RuntimeError("This witness requires the shared guarded Docker worker")
    if args.work_root.exists() or not args.work_root.resolve().is_relative_to(args.data_root.resolve()):
        raise RuntimeError("Use a new work directory inside the guarded data root")
    for name, value in vars(args).items():
        if isinstance(value, Path):
            if value.is_symlink():
                raise RuntimeError(f"Explicit input may not be a symlink: {value}")
            setattr(args, name, value.resolve(strict=name != "work_root"))
    if sha256(args.input_manifest) != args.input_manifest_sha256:
        raise RuntimeError("Input manifest differs from explicit selection")
    expected_inputs = json.loads(regular_file(args.input_manifest).read_text())
    for name, value in vars(args).items():
        if isinstance(value, Path) and name not in {"data_root", "work_root"}:
            if value == args.work_root or value.is_relative_to(args.work_root):
                raise RuntimeError("Authoritative inputs must remain outside the output root")
    verify_compiler(args)
    for root, expected in [(args.old_provider, args.old_provider_manifest_sha256),
                           (args.provider, args.provider_manifest_sha256)]:
        if sha256(root / "provider-manifest.echo.json") != expected:
            raise RuntimeError("Provider manifest differs from explicit selection")
    original = selected_inputs(args)
    if original != expected_inputs:
        raise RuntimeError("Compiler/source/provider inputs differ from the selected manifest")
    cases = json.loads(regular_file(args.pure_cases).read_text())
    if len(cases["cases"]) != 12:
        raise RuntimeError("Expected the original twelve literal assembly cases")
    variants = source_variants(args)
    args.work_root.mkdir()
    evidence = {"selectedInputs": original, "inputManifestSha256": args.input_manifest_sha256,
                "limits": {"workBytes": MAX_WORK_BYTES, "workEntries": MAX_WORK_ENTRIES,
                           "childFileBytes": MAX_CHILD_FILE_BYTES,
                           "buildTimeoutSeconds": BUILD_TIMEOUT_SECONDS}, "results": {}}
    shutil.copyfile(args.input_manifest, args.work_root / "selected-inputs.json")
    try:
        specs = [
            ("function-free-old", variants["function-free"], args.old_provider, {}),
            ("source-function-old", variants["pure"], args.old_provider, {"refusal": "old-publication"}),
            ("function-free-new", variants["function-free"], args.provider, {}),
            ("pure", variants["pure"], args.provider, {}),
            ("renamed-pure", variants["renamed-pure"], args.provider, {"renamed": True}),
            ("changed-body", variants["changed-body"], args.provider, {}),
            ("read-original-budget", variants["read"], args.provider,
             {"read": True, "read_steps": READ_OLD_STEPS, "refusal": "original-budget"}),
            ("read", variants["read"], args.provider,
             {"read": True, "read_steps": READ_HELPER_STEPS}),
        ]
        for name, source, provider, options in specs:
            evidence["results"][name] = run_variant(args, name, source, provider, **options)
            (args.work_root / "evidence.json").write_text(json.dumps(evidence, indent=2) + "\n")
        pure = evidence["results"]["pure"]["firstBuild"]["artifacts"]
        for name in ["renamed-pure", "changed-body"]:
            other = evidence["results"][name]["firstBuild"]["artifacts"]
            if any(pure[path] == other[path] for path in OUTPUTS):
                raise RuntimeError(f"Changing {name} did not change package and report identity")
        if (evidence["results"]["read-original-budget"]["lawpack"]["manifestDigest"]
                == evidence["results"]["read"]["lawpack"]["manifestDigest"]):
            raise RuntimeError("Authored budget change did not change lawpack authority")
        if sha256(args.input_manifest) != args.input_manifest_sha256:
            raise RuntimeError("Selected input manifest changed during the witness")
        if selected_inputs(args) != original:
            raise RuntimeError("An authoritative input changed during the witness")
        verify_compiler(args)
        evidence["finalUsage"] = check_work_bound(args.work_root)
        evidence["outcome"] = "public-compiler-and-provider-verifier-accepted"
        evidence["runtimeExecution"] = "not-run-by-this-witness"
        (args.work_root / "evidence.json").write_text(json.dumps(evidence, indent=2) + "\n")
    except BaseException as error:
        evidence["failure"] = {"type": type(error).__name__, "message": str(error)}
        (args.work_root / "evidence.json").write_text(json.dumps(evidence, indent=2) + "\n")
        raise
    print("SOURCE_FUNCTION_PUBLIC_COMPILER_AND_VERIFIER_ACCEPTED", flush=True)


def cancelled(signum, _frame):
    # Convert the runner's cancellation into finally cleanup of the compiler group.
    raise RuntimeError(f"Witness cancelled by signal {signum}")


if __name__ == "__main__":
    signal.signal(signal.SIGTERM, cancelled)
    main()
