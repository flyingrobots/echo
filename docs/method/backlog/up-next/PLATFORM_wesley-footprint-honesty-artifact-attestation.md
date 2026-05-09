<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Wesley Footprint Honesty Artifact Attestation

Status: planned generated-artifact hardening.

Depends on:

- [Continuum proof family runtime cutover](./PLATFORM_continuum-proof-family-runtime-cutover.md)
- [Contract-aware receipts and readings](./KERNEL_contract-aware-receipts-and-readings.md)
- [Authenticated Wesley intent admission posture](./PLATFORM_authenticated-wesley-intent-admission-posture.md)

## Why now

`warp-core` already has runtime footprint guards that catch executor reads and
writes outside a declared [`Footprint`](../../../crates/warp-core/src/footprint.rs).
Those guards are the right safety net for debug builds, development, and
untrusted code paths.

For Wesley-compiled GraphQL contracts, Echo also needs a stronger generated
artifact story:

```text
GraphQL footprint declaration
  -> Wesley compile-time access surface
  -> Rust/TypeScript generated artifacts
  -> artifact hash / certificate
  -> Echo load-time attestation
  -> release runtime may skip per-access footprint guards for trusted artifacts
```

The point is not to remove footprint honesty. The point is to move the primary
enforcement for trusted Wesley artifacts to compile time, then verify the
compiled artifact identity once when it is loaded.

## What it should look like

Wesley should lower authored footprint directives into an artifact-level
footprint contract. Generated Rust should expose only the declared read/write
capabilities to the operation implementation. If the implementation reaches
outside its declared footprint, it should fail to compile.

Generated TypeScript should carry the same footprint metadata and artifact
fingerprint for toolchain and cross-runtime agreement. TypeScript cannot provide
the same hard boundary as Rust in every case, but it can still participate in
schema/hash parity, generated metadata checks, and fixture-level compile
verification.

The generated artifact should include a stable footprint certificate naming:

- contract family
- schema hash
- operation id and operation name
- declared read/write footprint
- generator identity and version
- Echo ABI or registry version
- generated Rust artifact hash
- generated TypeScript artifact hash, when present
- footprint certificate hash

At runtime, Echo or the host should compare the loaded artifact's certificate
hash and generated artifact hash with the trusted registry metadata on first
load. After that:

- trusted compile-time-certified artifacts may use the optimized release path
  without per-access footprint guards;
- debug builds and `footprint_enforce_release` may still run guards as a safety
  net;
- missing, mismatched, or unsupported certificates reject or fall back to an
  explicitly runtime-guarded posture according to host policy;
- no path silently treats an uncertified artifact as footprint-honest.

## Acceptance criteria

- One Wesley-generated Rust proof slice exposes a declared footprint as typed
  capabilities.
- One valid generated Rust implementation compiles and runs through the Echo
  host path.
- One invalid implementation that reads or writes outside the declared
  footprint fails to compile.
- The generated Rust and TypeScript artifacts name the same schema hash,
  footprint certificate hash, and operation footprint metadata.
- Echo load-time registration compares the artifact hash or certificate hash
  before enabling the trusted optimized posture.
- A hash mismatch returns a typed rejection or obstruction, not a silent
  runtime downgrade.
- Runtime footprint guards remain available as debug / opt-in / untrusted-path
  enforcement.

## Non-goals

- Do not remove `FootprintGuard`.
- Do not make TypeScript the sole hard enforcement boundary.
- Do not add app-specific footprint nouns to Echo core.
- Do not require IPA, Verkle, SNARK, STARK, or proof-carrying apertures.
- Do not skip runtime checks for artifacts whose hash or certificate was not
  verified.
- Do not trust generated source text without a stable compiled-artifact
  identity.

## Notes

Initial Echo-side runway landed:

- `echo-registry-api::OpDef` can carry an optional no-std
  `FootprintCertificate`.
- `echo-wesley-gen` emits deterministic per-operation footprint artifact and
  certificate hashes from `@wes_footprint` metadata, operation argument shape,
  and the generated Rust artifact manifest hash.
- Generated registry consumers can compare a certificate hash at load time via
  `OpDef::footprint_certificate_matches(...)`.
- Hosts can verify a generated registry artifact with
  `echo_registry_api::verify_contract_artifact(...)`, which checks schema,
  codec, registry layout, expected footprint certificate hashes, optional
  generated artifact hashes, and a policy requiring mutation operations to be
  backed by expected certificates.

This is not the full closeout. The remaining hardening is the real
compile-time capability boundary and cross-artifact Rust/TypeScript compiled
artifact identity.

The expected posture vocabulary is:

```text
CompileTimeCertified
RuntimeGuarded
UntrustedRejected
UnsupportedObstructed
```

This keeps release performance honest without weakening the safety model:
runtime checks become the fallback and development guardrail, while trusted
Wesley-compiled artifacts carry a load-time certificate that proves the
compile-time footprint boundary was the one Echo is about to execute.
