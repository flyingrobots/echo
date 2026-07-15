<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Checked Edict Provider Artifacts

`v1/` is the exact first generated corpus for Echo's Edict provider contract.
Every entry below that directory is machine-generated and participates in the
snapshot check. Explanatory material stays in this parent directory so the
corpus root can reject every unexpected file without an ignore list.

The 22 files are five canonical-CBOR primary artifacts, fourteen
canonical-CBOR resources, one self-contained CDDL schema, one canonical Wesley
generation-provenance document, and one canonical non-authoritative Wesley
review document. Canonical bytes are copied directly from their validated
owners; the generator does not pretty-print, append newlines, or re-encode them.

Rebuild the corpus from the exact checked semantic source, generation settings,
Edict contract pack, and compile-time provider generator source bundle:

```bash
cargo +1.90.0 run --locked -p echo-wesley-gen \
  --bin echo-edict-provider-artifacts --
```

Check exact paths and bytes without creating, deleting, or rewriting anything:

```bash
cargo +1.90.0 run --locked -p echo-wesley-gen \
  --bin echo-edict-provider-artifacts -- --check
```

Generation writes only expected files and does not delete unexpected entries.
Check mode reports sorted `missing`, `changed`, and `unexpected` diagnostics and
exits unsuccessfully on any drift.

The provenance generator identity binds exact provider source files, Cargo
manifests and lockfile, and `rust-toolchain.toml`. It excludes this generated
directory and all authored inputs already bound separately by Wesley. The
identity describes the source/dependency-lock closure; it is not executable
reproducibility, package installation, Echo runtime admission, or authority.
