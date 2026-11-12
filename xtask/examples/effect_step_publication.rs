// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::print_stdout)]
//! Public compiler witness for authored effect programs with effect steps
//! under the `echo.dpo@2` effect target profile (flyingrobots/echo#985).
//!
//! `--fixtures` names a directory whose `source/` holds one authored lawpack,
//! application and program; `--name` prefixes the retained files. The program
//! is sent through a published Amblicit compiler binary and the checked
//! provider package: the lowerer emits an `edict-effect-program/v1` package
//! and the separate verifier reports on it. Nothing here constructs Core,
//! Target IR or a package. With `--check` the fresh compiler output must equal
//! the retained fixtures.
//!
//! Controls, derived here from the authored lawpack, must each be refused by
//! Echo's lowerer:
//! - `echo.dpo@1`: the adapter accepts the checked `echo.dpo@1` profile, calls
//!   each operation under its namespace and carries that profile's catalog,
//!   which declares none of the runtime intrinsics, while `echo.dpo@1` names
//!   `echo.span-ir/v1` rather than the ordered `echo.span-ir/v2` the adapter
//!   accepts. The lowerer reports every judgment refusal with one message, so
//!   this control shows that `echo.dpo@1` refuses effect steps, not which of
//!   those checks refuses them;
//! - a substituted `echo.dpo@2`: a well-formed profile under the same
//!   coordinate whose bytes differ from the checked one, served by a provider
//!   copy whose manifest names it, with the adapter pinning it;
//! - when `controls/zero-failure-read` exists: the same lawpack with every
//!   failure removed from its exports and adapter, and a program that calls
//!   the read without a failure map, although `node-atom-read` pins four.
use anyhow::{bail, ensure, Context, Result};
use echo_edict_canonical::{
    decode_canonical_cbor_v1, digest_canonical_value_v1, encode_canonical_cbor_v1,
    CanonicalValueV1 as Value,
};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Where one fixture keeps its program and lawpack, derived from its authored
/// source once in `main`.
struct Layout {
    /// Prefix of the retained files, such as `read` for `read-<output>.hex`.
    name: String,
    /// The single authored program under `source/`.
    program: String,
    /// The lawpack build's output directory.
    vendor: String,
    /// The program's pinned lawpack import, up to the digest.
    import: String,
}

static LAYOUT: std::sync::OnceLock<Layout> = std::sync::OnceLock::new();

#[allow(
    clippy::expect_used,
    reason = "main loads the layout before any other step reads it"
)]
fn layout() -> &'static Layout {
    LAYOUT.get().expect("the fixture layout is loaded first")
}
const EFFECT_PROFILE: &str = "generated/primary/target-profile.echo-dpo-2.cbor";
const DPO_PROFILE_DIGEST: &str =
    "sha256:2e2494121aecf5e6a2d920f5fb85408825d394765fad41484c416397c920fb04";
const DPO_CATALOG: &str = "generated/resources/resource.target-intrinsics.cbor";
const EFFECT_NAMESPACE: &str = "echo.dpo@2.";
/// An optional control: the program calling a read whose exported effect
/// declares no failures, compiled without a failure map.
const ZERO_FAILURE_READ: &str = "controls/zero-failure-read";
/// Compiler outputs, retained as `read-<name>.hex`.
const OUTPUTS: [&str; 2] = [
    "executable-operation-package.cbor",
    "verification-report.cbor",
];
/// Lawpack build outputs the lowering request carries, retained as
/// `read-<name>.hex` so native tests can rebuild that request.
const LAWPACK_OUTPUTS: [(&str, &str); 3] = [
    ("manifest.cbor", "lawpack"),
    ("adapter.cbor", "adapter"),
    ("echo-operation-configuration.cbor", "configuration"),
];
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_COMPILER_BYTES: u64 = 256 * 1024 * 1024;

struct Arguments {
    compiler: PathBuf,
    name: String,
    provider: PathBuf,
    fixtures: PathBuf,
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
        name: take("--name")?
            .to_str()
            .context("--name is UTF-8")?
            .to_owned(),
        provider: take("--provider")?,
        fixtures: take("--fixtures")?,
        work_root: take("--work-root")?,
        check,
    };
    ensure!(values.is_empty(), "unknown arguments: {values:?}");
    Ok(parsed)
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

fn read(path: &Path) -> Result<Vec<u8>> {
    read_bounded(path, MAX_FILE_BYTES)
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

/// Builds `lawpack` (an authored lawpack-build document) under `root` and
/// returns the digest of the lawpack the compiler built.
fn build_lawpack(compiler: &Path, root: &Path, lawpack: &serde_json::Value) -> Result<String> {
    fs::create_dir_all(root)?;
    fs::write(
        root.join("edict.lawpack.json"),
        serde_json::to_vec_pretty(lawpack)?,
    )?;
    accepted(&build(compiler, root, "lawpack")?, "the lawpack")?;
    let pin = String::from_utf8(read(&root.join(&layout().vendor).join("manifest.sha256"))?)?
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

/// The authored program with its lawpack import pinned to `pin`.
fn pinned_source(authored: &str, pin: &str) -> Result<String> {
    let start = authored
        .find(layout().import.as_str())
        .context("authored program has no pinned lawpack import")?
        + layout().import.len();
    let end = start
        + authored[start..]
            .find('"')
            .context("unterminated lawpack digest")?;
    ensure!(
        authored[end..].matches(layout().import.as_str()).count() == 0,
        "authored program imports the lawpack twice"
    );
    Ok(format!("{}{pin}{}", &authored[..start], &authored[end..]))
}

/// An application root that compiles `program` against the lawpack built
/// under `lawpack_root`, selecting `profile` from `provider`.
fn application_root(
    root: &Path,
    lawpack_root: &Path,
    provider: &Path,
    application: &serde_json::Value,
    program: &str,
) -> Result<PathBuf> {
    fs::create_dir_all(root.join("src"))?;
    fs::write(
        root.join("edict.application.json"),
        serde_json::to_vec_pretty(application)?,
    )?;
    copy_tree(
        &lawpack_root.join(&layout().vendor),
        &root.join(&layout().vendor),
    )?;
    copy_tree(provider, &root.join("provider"))?;
    fs::write(root.join("src/Program.edict"), program)?;
    Ok(root.to_path_buf())
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

fn retained_name(output: &str) -> String {
    format!("{}-{}.hex", layout().name, output.trim_end_matches(".cbor"))
}

fn compare(evidence: &Path, fixtures: &Path) -> Result<()> {
    let mut names: Vec<String> = OUTPUTS.iter().map(|output| retained_name(output)).collect();
    names.extend(
        LAWPACK_OUTPUTS
            .iter()
            .map(|(_, name)| format!("{}-{name}.hex", layout().name)),
    );
    let mut pairs: Vec<_> = names
        .iter()
        .map(|name| (evidence.join(name), fixtures.join(name)))
        .collect();
    pairs.push((
        evidence.join("source").join(&layout().program),
        fixtures.join("source").join(&layout().program),
    ));
    let mut differing = Vec::new();
    for (fresh, retained) in pairs {
        let bytes = read(&fresh)?;
        if fs::read(&retained).ok() != Some(bytes.clone()) {
            // Report the fresh bytes so a reviewer can retain or compare them;
            // nothing here rewrites a fixture.
            let name = fresh.strip_prefix(evidence).unwrap_or(&fresh).display();
            println!("PUBLIC_EFFECT_STEP_FRESH {name} {}", hex::encode(&bytes));
            differing.push(retained.display().to_string());
        }
    }
    ensure!(
        differing.is_empty(),
        "retained fixtures differ from fresh public compiler output: {differing:?}"
    );
    Ok(())
}

fn field_mut<'a>(value: &'a mut Value, key: &str) -> Result<&'a mut Value> {
    let Value::Map(entries) = value else {
        bail!("{key}: parent is not a map")
    };
    entries
        .iter_mut()
        .find(|(name, _)| name == &Value::Text(key.to_owned()))
        .map(|(_, value)| value)
        .with_context(|| format!("missing {key}"))
}

/// A provider copy whose `echo.dpo@2` profile is substituted by well-formed
/// bytes that differ from the checked ones, with the manifest naming them.
/// Returns the substituted profile's digest.
fn substituted_provider(provider: &Path, copy: &Path) -> Result<String> {
    copy_tree(provider, copy)?;
    let mut profile = decode_canonical_cbor_v1(&read(&copy.join(EFFECT_PROFILE))?)
        .map_err(|error| anyhow::anyhow!("checked echo.dpo@2 profile: {error:?}"))?;
    let postconditions = field_mut(&mut profile, "postconditionSupport")?;
    let Value::Bool(supported) = *postconditions else {
        bail!("postconditionSupport is not a boolean")
    };
    *postconditions = Value::Bool(!supported);
    let checked = digest_canonical_value_v1(
        "edict.target-profile/v1",
        &decode_canonical_cbor_v1(&read(&provider.join(EFFECT_PROFILE))?)
            .map_err(|error| anyhow::anyhow!("checked profile: {error:?}"))?,
    )
    .map_err(|error| anyhow::anyhow!("checked profile digest: {error:?}"))?;
    let substituted = digest_canonical_value_v1("edict.target-profile/v1", &profile)
        .map_err(|error| anyhow::anyhow!("substituted profile digest: {error:?}"))?;
    fs::write(
        copy.join(EFFECT_PROFILE),
        encode_canonical_cbor_v1(&profile)
            .map_err(|error| anyhow::anyhow!("substituted profile: {error:?}"))?,
    )?;
    let manifest = copy.join("provider-manifest.echo.json");
    let text = String::from_utf8(read(&manifest)?)?;
    ensure!(
        text.matches(&checked).count() == 1,
        "the manifest names the checked echo.dpo@2 profile exactly once"
    );
    fs::write(&manifest, text.replace(&checked, &substituted))?;
    Ok(substituted)
}

/// The authored lawpack with its adapter re-pointed at another profile, each
/// effect calling the same operation under that profile's namespace.
fn lawpack_for(
    authored: &serde_json::Value,
    profile: &str,
    digest: &str,
    catalog: Option<&[u8]>,
) -> Result<serde_json::Value> {
    let mut lawpack = authored.clone();
    let adapter = &mut lawpack["lawpack"]["targetAdapters"][0];
    adapter["acceptedTargetProfile"] = serde_json::json!({"id": profile, "digest": digest});
    let implementations = adapter["effectImplementations"]
        .as_object_mut()
        .context("effect implementations")?;
    ensure!(
        !implementations.is_empty(),
        "at least one effect implementation"
    );
    for implementation in implementations.values_mut() {
        let intrinsic = implementation["targetIntrinsic"]
            .as_str()
            .and_then(|name| name.strip_prefix(EFFECT_NAMESPACE))
            .context("an echo.dpo@2 intrinsic")?
            .to_owned();
        implementation["targetIntrinsic"] = serde_json::json!(format!("{profile}.{intrinsic}"));
    }
    if let Some(catalog) = catalog {
        let resources = lawpack["lawpack"]["localResources"]
            .as_array_mut()
            .context("local resources")?;
        let configuration = resources
            .iter_mut()
            .find(|resource| resource["name"] == "echo-config")
            .context("echo-config")?;
        configuration["value"]["intrinsicCatalog"] =
            serde_json::json!({"$edictBytes": hex::encode(catalog)});
    }
    Ok(lawpack)
}

/// Compiles the program as a control and requires Echo's lowerer to refuse it
/// with a message containing `expected`.
#[allow(clippy::too_many_arguments)]
fn control(
    arguments: &Arguments,
    name: &str,
    lawpack: &serde_json::Value,
    application: &serde_json::Value,
    authored: &str,
    provider: &Path,
    expected: &str,
    receipt: &mut serde_json::Value,
) -> Result<()> {
    let root = arguments.work_root.join(name);
    let pin = build_lawpack(&arguments.compiler, &root.join("lawpack"), lawpack)?;
    let application_root = application_root(
        &root.join("application"),
        &root.join("lawpack"),
        provider,
        application,
        &pinned_source(authored, &pin)?,
    )?;
    let refused = build(&arguments.compiler, &application_root, "application")?;
    receipt[name] = serde_json::json!({
        "exitCode": refused.exit_code,
        "diagnostics": refused.diagnostics,
    });
    let first = refused.diagnostics.first();
    let kind = first
        .and_then(|diagnostic| diagnostic["kind"].as_str())
        .unwrap_or_default();
    let message = first
        .and_then(|diagnostic| diagnostic["message"].as_str())
        .unwrap_or_default();
    println!(
        "PUBLIC_EFFECT_STEP_REFUSAL {name} exit={} kind={kind}",
        refused.exit_code
    );
    if refused.exit_code != 2
        || kind != "ProviderLowererRefused"
        || !message.contains(expected)
        || application_root.join("application-output").exists()
    {
        bail!("{name} was not refused at Echo's lowerer: {kind} {message}");
    }
    println!("PUBLIC_EFFECT_STEP_REFUSAL_OK {name}");
    Ok(())
}

/// When the fixture carries it, compiles the zero-failure read control: the
/// authored lawpack whose effects declare no failures and map none, and a
/// program that calls the read without a failure map. The adapter still binds
/// `echo.dpo@2.node-atom-read`, whose catalog entry pins four failures, so
/// Echo's lowerer must refuse it.
fn zero_failure_control(
    arguments: &Arguments,
    authored_lawpack: &serde_json::Value,
    application: &serde_json::Value,
    receipt: &mut serde_json::Value,
) -> Result<()> {
    let directory = arguments.fixtures.join(ZERO_FAILURE_READ);
    if !directory.exists() {
        return Ok(());
    }
    let program = String::from_utf8(read(&directory.join(&layout().program))?)?;
    let mut lawpack = authored_lawpack.clone();
    for effect in lawpack["lawpack"]["exports"]["effects"]
        .as_array_mut()
        .context("exported effects")?
    {
        effect["effectFailures"] = serde_json::json!({});
    }
    for implementation in lawpack["lawpack"]["targetAdapters"][0]["effectImplementations"]
        .as_object_mut()
        .context("effect implementations")?
        .values_mut()
    {
        implementation["failureMappings"] = serde_json::json!({});
    }
    control(
        arguments,
        "zero-failure-read",
        &lawpack,
        application,
        &program,
        &arguments.provider,
        "the compiled effect program is outside the checked profile",
        receipt,
    )
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
    let authored_lawpack: serde_json::Value =
        serde_json::from_slice(&read(&source.join("edict.lawpack.json"))?)?;
    let authored_application: serde_json::Value =
        serde_json::from_slice(&read(&source.join("edict.application.json"))?)?;
    let programs: Vec<_> = fs::read_dir(&source)?
        .map(|entry| Ok(entry?.file_name().to_string_lossy().into_owned()))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter(|name| {
            Path::new(name)
                .extension()
                .is_some_and(|extension| extension == "edict")
        })
        .collect();
    let [program_name] = programs.as_slice() else {
        bail!("expected exactly one authored program, found {programs:?}");
    };
    let lawpack_document = &authored_lawpack["lawpack"];
    let loaded = Layout {
        name: arguments.name.clone(),
        program: program_name.clone(),
        vendor: authored_lawpack["outputDirectory"]
            .as_str()
            .context("lawpack outputDirectory")?
            .to_owned(),
        import: format!(
            "use lawpack {}@{} digest \"",
            lawpack_document["id"].as_str().context("lawpack id")?,
            lawpack_document["version"]
                .as_str()
                .context("lawpack version")?
        ),
    };
    ensure!(LAYOUT.set(loaded).is_ok(), "layout loaded once");
    let authored = String::from_utf8(read(&source.join(&layout().program))?)?;

    let lawpack = arguments.work_root.join("lawpack");
    let pin = build_lawpack(&arguments.compiler, &lawpack, &authored_lawpack)?;
    for (file, name) in LAWPACK_OUTPUTS {
        fs::write(
            evidence.join(format!("{}-{name}.hex", layout().name)),
            format!(
                "{}\n",
                hex::encode(read(&lawpack.join(&layout().vendor).join(file))?)
            ),
        )?;
    }
    let program = pinned_source(&authored, &pin)?;
    let root = application_root(
        &arguments.work_root.join(&layout().name),
        &lawpack,
        &arguments.provider,
        &authored_application,
        &program,
    )?;
    accepted(
        &build(&arguments.compiler, &root, "application")?,
        "the public program",
    )?;
    let first = outputs(&root)?;
    fs::remove_dir_all(root.join("application-output"))?;
    accepted(
        &build(&arguments.compiler, &root, "application")?,
        "the public program",
    )?;
    ensure!(
        first == outputs(&root)?,
        "repeating the public build changed the package or report"
    );
    fs::write(evidence.join("source").join(&layout().program), &program)?;
    let mut receipt = serde_json::json!({
        "schema": "echo.effect-step-publication/v1",
        "compilerSha256": sha256(&read_bounded(&arguments.compiler, MAX_COMPILER_BYTES)?),
        "providerManifestSha256":
            sha256(&read(&arguments.provider.join("provider-manifest.echo.json"))?),
        "lawpackDigest": pin,
        "sourceSha256": sha256(program.as_bytes()),
        "authoredPinMatches": authored == program,
    });
    for (output, bytes) in OUTPUTS.iter().zip(&first) {
        let name = retained_name(output);
        fs::write(evidence.join(&name), format!("{}\n", hex::encode(bytes)))?;
        receipt[name] = sha256(bytes).into();
    }
    println!("PUBLIC_EFFECT_STEP_COMPILE_OK {}", layout().name);

    // The same program written against echo.dpo@1, which names echo.span-ir/v1
    // and whose catalog declares none of the runtime intrinsics.
    let mut dpo_application = authored_application.clone();
    dpo_application["target"]["profile"] = serde_json::json!("echo.dpo@1");
    let catalog = read(&arguments.provider.join(DPO_CATALOG))?;
    let dpo_lawpack = lawpack_for(
        &authored_lawpack,
        "echo.dpo@1",
        DPO_PROFILE_DIGEST,
        Some(&catalog),
    )?;
    let checked_results = control(
        &arguments,
        "echo-dpo-1",
        &dpo_lawpack,
        &dpo_application,
        &authored,
        &arguments.provider,
        "the compiled effect program is outside the checked profile",
        &mut receipt,
    );

    // A substituted echo.dpo@2 profile, pinned by the adapter and named by
    // the provider manifest.
    let substituted = arguments.work_root.join("substituted-provider");
    let digest = substituted_provider(&arguments.provider, &substituted)?;
    let substituted_lawpack = lawpack_for(&authored_lawpack, "echo.dpo@2", &digest, None)?;
    let substituted_results = control(
        &arguments,
        "substituted-echo-dpo-2",
        &substituted_lawpack,
        &authored_application,
        &authored,
        &substituted,
        "accepts only the exact checked Echo target profiles",
        &mut receipt,
    );
    let zero_failure_results = zero_failure_control(
        &arguments,
        &authored_lawpack,
        &authored_application,
        &mut receipt,
    );
    fs::write(
        evidence.join("evidence.json"),
        format!("{}\n", serde_json::to_string_pretty(&receipt)?),
    )?;
    checked_results?;
    substituted_results?;
    zero_failure_results?;
    if arguments.check {
        compare(&evidence, &arguments.fixtures)?;
        println!("PUBLIC_EFFECT_STEP_RETAINED_OK");
    }
    Ok(())
}
