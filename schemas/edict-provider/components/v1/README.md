<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Edict Provider Components v1

This directory checks the first executable Echo provider components for the
frozen Edict target-provider worlds. Issue #655 will bind these exact components
into the digest-locked provider package; their presence here does not claim that
Echo has installed, admitted, authorized, or executed them.

`lowerer.echo-dpo.component.wasm` implements
`edict:target-provider/lowerer@1.0.0`. It was built from
`echo-edict-provider-lowerer` in the `linux/amd64` image
`docker.io/library/rust@sha256:3914072ca0c3b8aad871db9169a651ccfce30cf58303e5d6f2db16d1d8a7e58f`.
Inside that immutable builder, the absolute rustup-resolved Rust 1.90.0 compiler
is commit `1159e78c4747b02ef996e55082b704c09b970588` and Cargo is commit
`840b83a10fb0e039a83f4d70ad032892c287570a`. The build binds Cargo to that
exact compiler, explicitly disables compiler wrappers, creates a controlled
Cargo home beneath the build target, removes ambient Cargo profile/build/target
overrides, and remaps that dependency source root to `/cargo`. The core module
is componentized with `wit-component` 0.251.0.
The source WIT is the exact 7,392-byte Edict contract with SHA-256
`2971fe44def7e51d5271dfc0f04f3088aa58754cffdc847681a587605aac749e`.

The checked component is 130,679 bytes with SHA-256
`03edee44c6bc70eb998c0c17662a214809746af3bba0740f3407c18a4016309e`.
Its sole contract attestation is the top-level custom section
`edict:target-provider-contract` containing
`edict:target-provider/lowerer@1.0.0`. Its only imports are the frozen WIT's
non-callable protocol instance and equality-bounded request/result type aliases;
its only callable world export is `lower`. It has no core, WASI, or ambient
capability imports.

`verifier.echo-dpo.component.wasm` implements
`edict:target-provider/verifier@1.0.0`. It uses the same immutable builder,
authenticated Rust/Cargo identities, frozen WIT bytes, path-remapping law, and
`wit-component` version recorded above. Its checked component is 188,736 bytes
with SHA-256
`e13eda6e02d5a46d2aecdec0546d53a7bf66f2580f8d5ec06e5d76710716a27b`.
Its sole contract attestation is the top-level custom section
`edict:target-provider-contract` containing
`edict:target-provider/verifier@1.0.0`. Its only imports are the frozen WIT's
non-callable protocol instance and equality-bounded verification request/result
type aliases; its only callable world export is `verify`. It has no core, WASI,
or ambient capability imports.

In the designated immutable `linux/amd64` Rust 1.90.0 builder above, rebuild and
check the artifact without rewriting it:

```sh
cargo +1.90.0 xtask provider-lowerer-component check \
  --target-dir target/provider-lowerer-component \
  --output schemas/edict-provider/components/v1/lowerer.echo-dpo.component.wasm

cargo +1.90.0 xtask provider-verifier-component check \
  --target-dir target/provider-verifier-component \
  --output schemas/edict-provider/components/v1/verifier.echo-dpo.component.wasm
```

Two independently provisioned containers with fresh target and controlled Cargo
home directories must produce byte-identical bytes for each component
independently. Builds on other hosts are local structural and semantic witnesses;
Rust/LLVM cross-host code generation is not claimed to be byte-identical. The
standalone witness audits and invokes both checked components through the pinned
Edict host. Before verifier invocation, the host preflights the exact request
artifacts and declared report schema; afterward, it admits and manifests each
returned accepted or well-formed rejected report. A typed output-role refusal
has neither response nor manifest. Fresh-store replay and separate host
processes reproduce each completed verifier outcome identically. Exact
component identity is build evidence for its translation or verification
crossing only; neither host replay nor identity is runtime Echo authority.

After two designated-builder candidates compare exactly, promote either explicit
candidate through the same structural admission boundary:

```sh
cargo +1.90.0 xtask provider-lowerer-component promote \
  --candidate-a /explicit/build-a/lowerer.echo-dpo.component.wasm \
  --candidate-b /explicit/build-b/lowerer.echo-dpo.component.wasm \
  --output schemas/edict-provider/components/v1/lowerer.echo-dpo.component.wasm \
  --write

cargo +1.90.0 xtask provider-verifier-component promote \
  --candidate-a /explicit/build-a/verifier.echo-dpo.component.wasm \
  --candidate-b /explicit/build-b/verifier.echo-dpo.component.wasm \
  --output schemas/edict-provider/components/v1/verifier.echo-dpo.component.wasm \
  --write
```

Promotion performs no discovery. It requires two distinct underlying files,
exact byte equality, the corresponding repository-approved SHA-256 identity
above, and complete component/world admission. It then synchronizes a
same-directory temporary file and atomically replaces a regular output; symlink
and non-regular outputs fail closed. Each one-build `designated-build` command
refuses its checked repository path, so a refresh must cross the promotion
boundary. Neither command infers how the two files were produced. G4's two
separately provisioned designated container jobs and exact source checkouts—or
equivalent explicit operator evidence—establish independent build provenance.
