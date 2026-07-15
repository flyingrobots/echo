<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Edict Provider Components v1

This directory checks the first executable Echo provider component for the
frozen Edict target-provider world. Issue #655 will bind this exact component
into the digest-locked provider package; its presence here does not claim that
Echo has installed, admitted, authorized, or executed it.

`lowerer.echo-dpo.component.wasm` implements
`edict:target-provider/lowerer@1.0.0`. It was built with Rust 1.90.0 from
`echo-edict-provider-lowerer` and componentized with `wit-component` 0.251.0.
The source WIT is the exact 7,392-byte Edict contract with SHA-256
`2971fe44def7e51d5271dfc0f04f3088aa58754cffdc847681a587605aac749e`.

The checked component is 112,718 bytes with SHA-256
`ea068940b8ca520585c395c63f18855c243a5b2ce731d601e61e5b508a7c6bf7`.
Its sole contract attestation is the top-level custom section
`edict:target-provider-contract` containing
`edict:target-provider/lowerer@1.0.0`. Its only imports are the frozen WIT's
non-callable protocol instance and equality-bounded request/result type aliases;
its only callable world export is `lower`. It has no core, WASI, or ambient
capability imports.

Check the artifact without rewriting it:

```sh
cargo +1.90.0 xtask provider-lowerer-component \
  --target-dir target/provider-lowerer-component \
  --output schemas/edict-provider/components/v1/lowerer.echo-dpo.component.wasm
```

Two independent fresh target directories must produce byte-identical component
bytes. The standalone Edict-host witness separately proves invocation parity,
typed refusal, host failure separation, replay equality, and cross-process
determinism. Exact component identity is build evidence for this translation
crossing only; it is not runtime Echo authority.
