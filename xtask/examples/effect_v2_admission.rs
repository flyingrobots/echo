// SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::print_stdout)]
//! Asks the public compiler for its static minimum budget of an `echo.dpo@2`
//! effect program under the shared-read schedule (`edict.core-cost/v2`) with
//! Echo's `/v3` effect cost templates (echo#1016, amblicit#341).
//!
//! The checked provider package pins the `/v2` intrinsics catalog, and the
//! compiler reads effect templates only from the catalog the selected profile
//! pins. So this builds a private evaluation provider: a copy of the checked
//! package whose `echo.dpo@2` profile pins Echo's `/v3` catalog (shipped next
//! to the profile), with the manifest naming the substituted profile. The
//! lawpack accepts that profile and carries the same catalog in its operation
//! configuration.
//!
//! The program is compiled with a budget of one step and one byte, so the
//! compiler refuses it and reports the minimum it needs. The minimum is then
//! given back as the budget, and one unit below on each axis, and each
//! outcome is printed. At the minimum the compiler's own admission passes and
//! the checked Echo lowerer component then refuses the substituted profile
//! (`UnsupportedTargetProfile`); one unit below, the compiler refuses with
//! `InvalidBound`. That is the compiler's static minimum, not Echo's.
//!
//! Last, the same program is compiled through the checked provider under the
//! copying schedule (the packaged components do not accept a shared-read
//! Core yet), and the package is written to `checked-package.hex` so Echo's
//! judges can be given the shared-read capability and the compiler's
//! minimum, as `effect_program_judgment.rs` does for its witnesses.
//!
//! Usage (any `edict-effect-*` fixture of `warp-core`):
//!
//! ```text
//! cargo run -p xtask --example effect_v2_admission -- \
//!   --compiler <amblicit 9f10fb91 or later> \
//!   --provider schemas/edict-provider/package/v1 \
//!   --catalog crates/echo-edict-provider-lowerer/catalogs/resource.target-intrinsics-3.cbor \
//!   --fixtures crates/warp-core/tests/fixtures/edict-effect-closure-read \
//!   --work-root <new directory>
//! ```
//!
//! Nothing here rewrites a fixture.
use anyhow::{bail, ensure, Context, Result};
use echo_edict_canonical::{
    decode_canonical_cbor_v1, digest_canonical_value_bytes_v1, digest_canonical_value_v1,
    encode_canonical_cbor_v1, CanonicalValueV1 as Value,
};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const EFFECT_PROFILE: &str = "generated/primary/target-profile.echo-dpo-2.cbor";
const V3_CATALOG: &str = "generated/resources/resource.target-intrinsics-3.cbor";
const CATALOG_DOMAIN: &str = "echo.dpo.intrinsics/v2";
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;

struct Arguments {
    compiler: PathBuf,
    provider: PathBuf,
    catalog: PathBuf,
    fixtures: PathBuf,
    work_root: PathBuf,
}

fn arguments() -> Result<Arguments> {
    let mut values = std::collections::BTreeMap::new();
    let mut raw = std::env::args().skip(1);
    while let Some(flag) = raw.next() {
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
        catalog: take("--catalog")?,
        fixtures: take("--fixtures")?,
        work_root: take("--work-root")?,
    };
    ensure!(values.is_empty(), "unknown arguments: {values:?}");
    Ok(parsed)
}

fn read(path: &Path) -> Result<Vec<u8>> {
    let length = fs::metadata(path)
        .with_context(|| format!("cannot stat {}", path.display()))?
        .len();
    ensure!(length <= MAX_FILE_BYTES, "{} is too large", path.display());
    fs::read(path).with_context(|| format!("cannot read {}", path.display()))
}

fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn field_mut<'a>(value: &'a mut Value, key: &str) -> Result<&'a mut Value> {
    let Value::Map(entries) = value else {
        bail!("expected a map around {key}")
    };
    entries
        .iter_mut()
        .find(|(name, _)| matches!(name, Value::Text(text) if text == key))
        .map(|(_, value)| value)
        .with_context(|| format!("missing {key}"))
}

fn profile_digest(value: &Value) -> Result<String> {
    digest_canonical_value_v1("edict.target-profile/v1", value)
        .map_err(|error| anyhow::anyhow!("profile digest: {error:?}"))
}

/// A provider copy whose `echo.dpo@2` profile pins `catalog`. Returns the
/// substituted profile's digest.
fn evaluation_provider(provider: &Path, catalog: &[u8], copy: &Path) -> Result<String> {
    copy_tree(provider, copy)?;
    fs::write(copy.join(V3_CATALOG), catalog)?;
    let checked = decode_canonical_cbor_v1(&read(&provider.join(EFFECT_PROFILE))?)
        .map_err(|error| anyhow::anyhow!("checked profile: {error:?}"))?;
    let mut profile = checked.clone();
    let catalog_value =
        decode_canonical_cbor_v1(catalog).map_err(|error| anyhow::anyhow!("catalog: {error:?}"))?;
    let identity = digest_canonical_value_bytes_v1(CATALOG_DOMAIN, &catalog_value)
        .map_err(|error| anyhow::anyhow!("catalog digest: {error:?}"))?;
    let reference = field_mut(&mut profile, "intrinsics")?;
    *field_mut(reference, "digest")? = Value::Array(vec![
        Value::Text("sha256".into()),
        Value::Bytes(identity.to_vec()),
    ]);
    let (checked, substituted) = (profile_digest(&checked)?, profile_digest(&profile)?);
    fs::write(
        copy.join(EFFECT_PROFILE),
        encode_canonical_cbor_v1(&profile)
            .map_err(|error| anyhow::anyhow!("profile: {error:?}"))?,
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
    let mut diagnostics = Vec::new();
    for stream in [&output.stdout, &output.stderr] {
        for line in String::from_utf8_lossy(stream).lines() {
            if line.trim().is_empty() {
                continue;
            }
            let event: serde_json::Value = serde_json::from_str(line)
                .with_context(|| format!("compiler emitted a non-JSON line: {line}"))?;
            if event["type"] == "diagnostic" {
                diagnostics.push(event);
            }
        }
    }
    Ok(Build {
        exit_code,
        diagnostics,
    })
}

/// The program compiled with `budget` (steps, allocated bytes).
fn compile(
    arguments: &Arguments,
    lawpack: &serde_json::Value,
    budget: (u64, u64),
    label: &str,
    evaluation: bool,
) -> Result<Build> {
    let source = arguments.fixtures.join("source");
    let root = arguments.work_root.join(label);
    let mut lawpack = lawpack.clone();
    let budgets = lawpack["lawpack"]["targetAdapters"][0]["budgets"]
        .as_object_mut()
        .context("lawpack budgets")?;
    ensure!(budgets.len() == 1, "exactly one lawpack budget");
    for entry in budgets.values_mut() {
        entry["maxSteps"] = budget.0.into();
        entry["maxAllocatedBytes"] = budget.1.into();
    }
    let lawpack_root = root.join("lawpack");
    fs::create_dir_all(&lawpack_root)?;
    fs::write(
        lawpack_root.join("edict.lawpack.json"),
        serde_json::to_vec_pretty(&lawpack)?,
    )?;
    let built = build(&arguments.compiler, &lawpack_root, "lawpack")?;
    ensure!(
        built.exit_code == 0,
        "lawpack refused: {:?}",
        built.diagnostics
    );
    let vendor = lawpack["outputDirectory"]
        .as_str()
        .context("lawpack outputDirectory")?;
    let pin = String::from_utf8(read(&lawpack_root.join(vendor).join("manifest.sha256"))?)?
        .trim()
        .to_owned();
    let document = &lawpack["lawpack"];
    let import = format!(
        "use lawpack {}@{} digest \"",
        document["id"].as_str().context("lawpack id")?,
        document["version"].as_str().context("lawpack version")?
    );
    let programs: Vec<_> = fs::read_dir(&source)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "edict")
        })
        .collect();
    let [program] = programs.as_slice() else {
        bail!("expected one authored program, found {programs:?}");
    };
    let authored = String::from_utf8(read(program)?)?;
    let start = authored.find(&import).context("lawpack import")? + import.len();
    let end = start + authored[start..].find('"').context("digest end")?;
    let pinned = format!("{}{pin}{}", &authored[..start], &authored[end..]);

    let app = root.join("application");
    fs::create_dir_all(app.join("src"))?;
    fs::write(app.join("src/Program.edict"), pinned)?;
    copy_tree(&lawpack_root.join(vendor), &app.join(vendor))?;
    if evaluation {
        copy_tree(&arguments.work_root.join("provider"), &app.join("provider"))?;
    } else {
        copy_tree(&arguments.provider, &app.join("provider"))?;
    }
    let mut application: serde_json::Value =
        serde_json::from_slice(&read(&source.join("edict.application.json"))?)?;
    if evaluation {
        application["coreCostSchedule"] = "edict.core-cost/v2".into();
        application["target"]["pureIntrinsics"] = serde_json::json!({
            "catalog": V3_CATALOG,
            "required": [{
                "coordinate": "echo.dpo@2.slotEdgeId",
                "contractVersion": "echo.content-address/v1",
            }],
        });
    }
    fs::write(
        app.join("edict.application.json"),
        serde_json::to_vec_pretty(&application)?,
    )?;
    build(&arguments.compiler, &app, "application")
}

/// The (steps, bytes) the compiler reports it needs, from a refused build.
fn reported_minimum(built: &Build) -> Option<(u64, u64)> {
    built.diagnostics.iter().find_map(|diagnostic| {
        let message = diagnostic.to_string();
        let number_after = |key: &str| -> Option<u64> {
            let at = message.find(key)? + key.len();
            let digits: String = message[at..]
                .chars()
                .skip_while(|c| !c.is_ascii_digit())
                .take_while(char::is_ascii_digit)
                .collect();
            digits.parse().ok()
        };
        Some((number_after("steps=")?, number_after("allocatedBytes=")?))
    })
}

fn summary(built: &Build) -> String {
    format!(
        "exit {} {}",
        built.exit_code,
        serde_json::Value::Array(built.diagnostics.clone())
    )
}

fn main() -> Result<()> {
    let arguments = arguments()?;
    ensure!(
        !arguments.work_root.exists(),
        "select a new work directory; existing evidence is preserved"
    );
    fs::create_dir_all(&arguments.work_root)?;
    let catalog = read(&arguments.catalog)?;
    let profile = evaluation_provider(
        &arguments.provider,
        &catalog,
        &arguments.work_root.join("provider"),
    )?;
    let mut lawpack: serde_json::Value = serde_json::from_slice(&read(
        &arguments.fixtures.join("source/edict.lawpack.json"),
    )?)?;
    let authored = lawpack.clone();
    lawpack["lawpack"]["targetAdapters"][0]["acceptedTargetProfile"] =
        serde_json::json!({"id": "echo.dpo@2", "digest": profile});
    let configuration = lawpack["lawpack"]["localResources"]
        .as_array_mut()
        .context("local resources")?
        .iter_mut()
        .find(|resource| resource["name"] == "echo-config")
        .context("echo-config")?;
    configuration["value"]["intrinsicCatalog"] =
        serde_json::json!({"$edictBytes": hex::encode(&catalog)});

    let probe = compile(&arguments, &lawpack, (1, 1), "probe", true)?;
    println!("EFFECT_V2_PROBE {}", summary(&probe));
    // A literal-bound loop is refused before costing when its iteration
    // count alone exceeds the step budget, so probe again with a step budget
    // the loop check passes and one byte.
    let Some((steps, bytes)) = reported_minimum(&probe).or_else(|| {
        let wide = compile(&arguments, &lawpack, (1 << 20, 1), "wide-probe", true).ok()?;
        println!("EFFECT_V2_WIDE_PROBE {}", summary(&wide));
        reported_minimum(&wide)
    }) else {
        bail!("the compiler reported no minimum");
    };
    println!("EFFECT_V2_COMPILER_MINIMUM {steps} {bytes}");
    for (label, budget) in [
        ("at-minimum", (steps, bytes)),
        ("steps-below", (steps - 1, bytes)),
        ("bytes-below", (steps, bytes - 1)),
    ] {
        let built = compile(&arguments, &lawpack, budget, label, true)?;
        println!("EFFECT_V2_{label} {}", summary(&built));
    }
    // The same program through the checked provider (the `/v2` catalog, no
    // templates), which Echo's lowerer accepts: its package is what Echo judges.
    let generic = compile(&arguments, &authored, (1, 1), "generic-probe", false)?;
    println!("EFFECT_V2_GENERIC_MINIMUM {:?}", reported_minimum(&generic));
    let checked = compile(&arguments, &authored, (1 << 20, 1 << 24), "checked", false)?;
    println!("EFFECT_V2_checked {}", summary(&checked));
    let package = read(
        &arguments
            .work_root
            .join("checked/application/application-output/executable-operation-package.cbor"),
    )?;
    fs::write(
        arguments.work_root.join("checked-package.hex"),
        format!("{}\n", hex::encode(package)),
    )?;
    Ok(())
}
