// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::print_stdout)]
//! Public compiler witness for typed outgoing-edge and guided closure reads.
//!
//! Sends the authored lawpack and programs under
//! `crates/warp-core/tests/fixtures/edict-graph-read/source` through an
//! unmodified Amblicit compiler binary and a selected provider package. It
//! never constructs Core or Target IR. With `--check` it also requires the
//! fresh package, report and source bytes to equal the retained fixtures.
use anyhow::{bail, ensure, Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const PROGRAMS: [(&str, &str); 2] = [("edge", "EdgeRead.edict"), ("closure", "ClosureRead.edict")];
const OUTPUTS: [&str; 2] = [
    "executable-operation-package.cbor",
    "verification-report.cbor",
];
const VENDOR: &str = "vendor/graph-read";
const IMPORT: &str = "use lawpack consumer.graph@1 digest \"";
/// The node-read control program's lawpack import.
const CONTROL_IMPORT: &str = "use lawpack jedit.text@1 digest \"";
const CONTROL_VENDOR: &str = "vendor/atom-read";
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
/// A compiler executable is far larger than any authored or generated file.
const MAX_COMPILER_BYTES: u64 = 256 * 1024 * 1024;

struct Arguments {
    compiler: PathBuf,
    provider: PathBuf,
    old_provider: PathBuf,
    fixtures: PathBuf,
    control: PathBuf,
    work_root: PathBuf,
    check: bool,
}

fn arguments() -> Result<Arguments> {
    let mut values = std::collections::BTreeMap::new();
    let mut check = false;
    let mut raw = std::env::args().skip(1);
    while let Some(flag) = raw.next() {
        if flag == "--check" {
            check = true;
            continue;
        }
        let value = raw
            .next()
            .with_context(|| format!("{flag} needs a value"))?;
        ensure!(
            values.insert(flag.clone(), PathBuf::from(value)).is_none(),
            "{flag} given twice"
        );
    }
    let mut take = |flag: &str| {
        values
            .remove(flag)
            .with_context(|| format!("missing {flag}"))
    };
    let parsed = Arguments {
        compiler: take("--compiler")?,
        provider: take("--provider")?,
        old_provider: take("--old-provider")?,
        fixtures: take("--fixtures")?,
        control: take("--control")?,
        work_root: take("--work-root")?,
        check,
    };
    ensure!(values.is_empty(), "unknown arguments: {values:?}");
    Ok(parsed)
}

fn read(path: &Path) -> Result<Vec<u8>> {
    read_bounded(path, MAX_FILE_BYTES)
}

fn read_bounded(path: &Path, max: u64) -> Result<Vec<u8>> {
    let metadata = path
        .symlink_metadata()
        .with_context(|| format!("missing {}", path.display()))?;
    ensure!(
        metadata.is_file() && metadata.len() <= max,
        "unbounded or nonregular input {}",
        path.display()
    );
    Ok(fs::read(path)?)
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from).with_context(|| format!("missing {}", from.display()))? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            ensure!(
                kind.is_file(),
                "nonregular input {}",
                entry.path().display()
            );
            fs::write(&target, read(&entry.path())?)?;
        }
    }
    Ok(())
}

/// One compiler invocation: its exit code and its diagnostic events.
struct Build {
    exit_code: i32,
    diagnostics: Vec<serde_json::Value>,
}

fn build(compiler: &Path, root: &Path, document: &str) -> Result<Build> {
    let temporary = root.join("temporary");
    fs::create_dir_all(&temporary)?;
    let mut child = Command::new(compiler)
        .current_dir(root)
        .env("TMPDIR", &temporary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("cannot start {}", compiler.display()))?;
    let settings = serde_json::json!({
        "schema": "edict.compiler.settings/v1",
        "type": "compilerSettings",
        "operation": "build",
        document: format!("edict.{document}.json"),
    });
    child
        .stdin
        .take()
        .context("compiler stdin")?
        .write_all(format!("{settings}\n").as_bytes())?;
    let output = child.wait_with_output()?;
    let exit_code = output.status.code().context("compiler was terminated")?;
    let mut events = Vec::new();
    for stream in [&output.stdout, &output.stderr] {
        for line in String::from_utf8_lossy(stream).lines() {
            if !line.trim().is_empty() {
                events.push(
                    serde_json::from_str::<serde_json::Value>(line)
                        .with_context(|| format!("compiler emitted a non-JSON line: {line}"))?,
                );
            }
        }
    }
    let statuses: Vec<_> = events
        .iter()
        .filter(|event| event["type"] == "status")
        .collect();
    ensure!(
        statuses.len() == 1 && statuses[0]["exitCode"] == exit_code,
        "compiler status and exit code disagree: {events:?}"
    );
    Ok(Build {
        exit_code,
        diagnostics: events
            .into_iter()
            .filter(|event| event["type"] == "diagnostic")
            .collect(),
    })
}

fn accepted(result: &Build, what: &str) -> Result<()> {
    ensure!(
        result.exit_code == 0 && result.diagnostics.is_empty(),
        "public compiler refused {what}: exit {} {:?}",
        result.exit_code,
        result.diagnostics
    );
    Ok(())
}

/// The authored program with its lawpack import pinned to the built lawpack.
fn pinned_source(authored: &str, pin: &str) -> Result<String> {
    pinned_import(authored, IMPORT, pin)
}

fn pinned_import(authored: &str, import: &str, pin: &str) -> Result<String> {
    let start = authored
        .find(import)
        .context("authored program has no pinned lawpack import")?
        + import.len();
    let end = start
        + authored[start..]
            .find('"')
            .context("unterminated lawpack digest")?;
    ensure!(
        authored[end..].matches(import).count() == 0,
        "authored program imports the lawpack twice"
    );
    Ok(format!("{}{pin}{}", &authored[..start], &authored[end..]))
}

/// The digest of a lawpack the public compiler built into `vendor`.
fn lawpack_pin(vendor: &Path) -> Result<String> {
    let pin = String::from_utf8(read(&vendor.join("manifest.sha256"))?)?
        .trim()
        .to_owned();
    ensure!(
        pin.len() == 71
            && pin.starts_with("sha256:")
            && pin[7..]
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')),
        "invalid public lawpack digest {pin}"
    );
    Ok(pin)
}

/// Positive control: the predecessor provider still compiles an existing
/// node-read program, so its refusal of the closure program is about the
/// graph-read configuration rather than a provider that refuses everything.
fn predecessor_control(arguments: &Arguments) -> Result<()> {
    let source = &arguments.control;
    let root = arguments.work_root.join("old-provider-control");
    fs::create_dir_all(root.join("src"))?;
    fs::write(
        root.join("edict.lawpack.json"),
        read(&source.join("edict.lawpack.json"))?,
    )?;
    accepted(
        &build(&arguments.compiler, &root, "lawpack")?,
        "the control lawpack",
    )?;
    let pin = lawpack_pin(&root.join(CONTROL_VENDOR))?;
    let mut application: serde_json::Value =
        serde_json::from_slice(&read(&source.join("edict.application.json"))?)?;
    application["target"]["providerPackage"] = "provider".into();
    application["outputDirectory"] = "application-output".into();
    fs::write(
        root.join("edict.application.json"),
        serde_json::to_vec_pretty(&application)?,
    )?;
    copy_tree(&arguments.old_provider, &root.join("provider"))?;
    let authored = String::from_utf8(read(&source.join("ReplaceRange.edict"))?)?;
    fs::write(
        root.join("src/ReplaceRange.edict"),
        pinned_import(&authored, CONTROL_IMPORT, &pin)?,
    )?;
    accepted(
        &build(&arguments.compiler, &root, "application")?,
        "the predecessor control",
    )?;
    outputs(&root)?;
    Ok(())
}

fn application_root(arguments: &Arguments, name: &str, provider: &Path) -> Result<PathBuf> {
    let source = arguments.fixtures.join("source");
    let root = arguments.work_root.join(name);
    fs::create_dir_all(root.join("src"))?;
    fs::write(
        root.join("edict.application.json"),
        read(&source.join("edict.application.json"))?,
    )?;
    copy_tree(
        &arguments.work_root.join("lawpack").join(VENDOR),
        &root.join(VENDOR),
    )?;
    copy_tree(provider, &root.join("provider"))?;
    Ok(root)
}

fn outputs(root: &Path) -> Result<Vec<Vec<u8>>> {
    let directory = root.join("application-output");
    let mut names: Vec<_> = fs::read_dir(&directory)?
        .map(|entry| Ok(entry?.file_name().to_string_lossy().into_owned()))
        .collect::<Result<_>>()?;
    names.sort();
    ensure!(
        names == OUTPUTS,
        "expected exactly one package/report pair, found {names:?}"
    );
    OUTPUTS
        .iter()
        .map(|name| read(&directory.join(name)))
        .collect()
}

fn compare(evidence: &Path, fixtures: &Path) -> Result<()> {
    let mut differing = Vec::new();
    let mut pairs = Vec::new();
    for (name, file) in PROGRAMS {
        for output in OUTPUTS {
            let retained = format!("{name}-{}.hex", output.trim_end_matches(".cbor"));
            pairs.push((evidence.join(&retained), fixtures.join(&retained)));
        }
        pairs.push((
            evidence.join("source").join(file),
            fixtures.join("source").join(file),
        ));
    }
    for (fresh, retained) in pairs {
        let bytes = read(&fresh)?;
        if fs::read(&retained).ok() != Some(bytes.clone()) {
            // Report the fresh bytes so a reviewer can retain or compare them;
            // nothing here rewrites a fixture.
            let name = fresh.strip_prefix(evidence).unwrap_or(&fresh).display();
            println!("PUBLIC_GRAPH_READ_FRESH {name} {}", hex::encode(&bytes));
            differing.push(retained.display().to_string());
        }
    }
    ensure!(
        differing.is_empty(),
        "retained fixtures differ from fresh public compiler output: {differing:?}"
    );
    Ok(())
}

fn main() -> Result<()> {
    let arguments = arguments()?;
    ensure!(
        !arguments.work_root.exists(),
        "select a new work directory; existing evidence is preserved"
    );
    let source = arguments.fixtures.join("source");
    let evidence = arguments.work_root.join("evidence");
    fs::create_dir_all(evidence.join("source"))?;

    let lawpack = arguments.work_root.join("lawpack");
    fs::create_dir_all(&lawpack)?;
    fs::write(
        lawpack.join("edict.lawpack.json"),
        read(&source.join("edict.lawpack.json"))?,
    )?;
    accepted(
        &build(&arguments.compiler, &lawpack, "lawpack")?,
        "the lawpack",
    )?;
    let pin = lawpack_pin(&lawpack.join(VENDOR))?;

    let mut receipt = serde_json::json!({
        "schema": "echo.graph-read-publication/v1",
        "compilerSha256": sha256(&read_bounded(&arguments.compiler, MAX_COMPILER_BYTES)?),
        "providerManifestSha256":
            sha256(&read(&arguments.provider.join("provider-manifest.echo.json"))?),
        "lawpackDigest": pin,
        "programs": {},
    });
    for (name, file) in PROGRAMS {
        let authored = String::from_utf8(read(&source.join(file))?)?;
        let program = pinned_source(&authored, &pin)?;
        let root = application_root(&arguments, name, &arguments.provider)?;
        fs::write(root.join("src/Program.edict"), &program)?;
        accepted(&build(&arguments.compiler, &root, "application")?, name)?;
        let first = outputs(&root)?;
        fs::remove_dir_all(root.join("application-output"))?;
        accepted(&build(&arguments.compiler, &root, "application")?, name)?;
        ensure!(
            first == outputs(&root)?,
            "repeating the public build changed the {name} package or report"
        );
        fs::write(evidence.join("source").join(file), &program)?;
        let mut digests = serde_json::Map::new();
        for (output, bytes) in OUTPUTS.iter().zip(&first) {
            let retained = format!("{name}-{}.hex", output.trim_end_matches(".cbor"));
            fs::write(
                evidence.join(&retained),
                format!("{}\n", hex::encode(bytes)),
            )?;
            digests.insert(retained, sha256(bytes).into());
        }
        digests.insert("sourceSha256".to_owned(), sha256(program.as_bytes()).into());
        digests.insert(
            "authoredPinMatches".to_owned(),
            (authored == program).into(),
        );
        receipt["programs"][name] = digests.into();
        println!("PUBLIC_GRAPH_READ_COMPILE_OK {name}");
    }

    predecessor_control(&arguments)?;
    println!("PUBLIC_GRAPH_READ_CONTROL_OK old-provider");
    // The predecessor provider must refuse the new configuration explicitly.
    let (_, closure) = PROGRAMS[1];
    let program = pinned_source(&String::from_utf8(read(&source.join(closure))?)?, &pin)?;
    let root = application_root(&arguments, "old-provider", &arguments.old_provider)?;
    fs::write(root.join("src/Program.edict"), program)?;
    let refused = build(&arguments.compiler, &root, "application")?;
    let kinds: Vec<_> = refused
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic["kind"].as_str().unwrap_or_default().to_owned())
        .collect();
    receipt["oldProviderRefusal"] = serde_json::json!({
        "exitCode": refused.exit_code,
        "diagnostics": refused.diagnostics,
    });
    fs::write(
        evidence.join("evidence.json"),
        format!("{}\n", serde_json::to_string_pretty(&receipt)?),
    )?;
    // Report the retained-fixture comparison before judging the refusal, so
    // one run shows every difference.
    let retained = if arguments.check {
        compare(&evidence, &arguments.fixtures)
    } else {
        Ok(())
    };
    println!(
        "PUBLIC_GRAPH_READ_REFUSAL exit={} kinds={kinds:?}",
        refused.exit_code
    );
    if refused.exit_code != 2
        || kinds.first().map(String::as_str) != Some("InvalidProviderInvocation")
        || root.join("application-output").exists()
    {
        bail!("predecessor provider did not refuse at its schema boundary: {kinds:?}");
    }
    println!("PUBLIC_GRAPH_READ_REFUSAL_OK old-provider");
    retained?;
    if arguments.check {
        println!("PUBLIC_GRAPH_READ_RETAINED_OK");
    }
    Ok(())
}
