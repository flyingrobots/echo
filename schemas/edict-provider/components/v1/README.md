<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Edict Provider Components v1

This directory checks the first executable Echo provider component for the
frozen Edict target-provider world. Issue #655 will bind this exact component
into the digest-locked provider package; its presence here does not claim that
Echo has installed, admitted, authorized, or executed it.

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

The checked component is 130,526 bytes with SHA-256
`14b6578e469ac8b2bab754ff13e1aa97cec8d9178235aa2993e1eabec4785a28`.
Its sole contract attestation is the top-level custom section
`edict:target-provider-contract` containing
`edict:target-provider/lowerer@1.0.0`. Its only imports are the frozen WIT's
non-callable protocol instance and equality-bounded request/result type aliases;
its only callable world export is `lower`. It has no core, WASI, or ambient
capability imports.

In the designated immutable `linux/amd64` Rust 1.90.0 builder above, rebuild and
check the artifact without rewriting it:

```sh
cargo +1.90.0 xtask provider-lowerer-component check \
  --target-dir target/provider-lowerer-component \
  --output schemas/edict-provider/components/v1/lowerer.echo-dpo.component.wasm
```

Two independently provisioned containers with fresh target and controlled Cargo
home directories must produce byte-identical component bytes. Builds on other
hosts are local structural and semantic witnesses; Rust/LLVM cross-host code
generation is not claimed to be byte-identical. The standalone Edict-host
witness audits and invokes the checked
portable component and separately proves invocation parity, typed refusal, host
failure separation, replay equality, and cross-process determinism. Exact
component identity is build evidence for this translation crossing only; it is
not runtime Echo authority.

After two designated-builder candidates compare exactly, promote either explicit
candidate through the same structural admission boundary:

```sh
cargo +1.90.0 xtask provider-lowerer-component promote \
  --candidate-a /explicit/build-a/lowerer.echo-dpo.component.wasm \
  --candidate-b /explicit/build-b/lowerer.echo-dpo.component.wasm \
  --output schemas/edict-provider/components/v1/lowerer.echo-dpo.component.wasm \
  --write
```

Promotion performs no discovery. It requires two distinct underlying files,
exact byte equality, the repository-approved SHA-256 identity above, and complete
component/world admission. It then synchronizes a same-directory temporary file
and atomically replaces a regular output; symlink and non-regular outputs fail
closed. The one-build `designated-build` command refuses this checked repository
path, so a refresh must cross the promotion boundary. The command does not infer
how the two files were produced. G4's two separately provisioned designated
container jobs and exact source checkouts—or equivalent explicit operator
evidence—establish independent build provenance.
