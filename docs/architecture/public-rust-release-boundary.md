<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Public Rust Release Boundary

Status: accepted target architecture. The sealed `echo_runtime::RuntimeHost`
construction and recovery witness exists, but the crate decomposition and
public alpha release closure described here are not yet complete or published.

This page owns Echo's public Rust package boundary for the first alpha release.
It defines which responsibilities may cross Cargo package boundaries, which
authority must remain private, and what evidence is required before a public
release may be proposed.

## Core Rule

Pure computation and passive storage contracts may cross published Cargo
package boundaries. Authority-minting machinery stays private inside
`flyingrobots-echo-runtime`.

Rust has no friend-crate visibility. If a separately published implementation
crate exposes a `pub` constructor so the runtime facade can call it, every
downstream crate can call it. Documentation, hidden re-exports, package posture,
and feature conventions do not repair that authority leak.

Echo therefore distinguishes two categories:

```text
constructible claim
    passive bytes, candidate material, storage observations

admitted history
    runtime-validated authority, settlement, and recovery truth
```

Constructing claim-shaped bytes never makes them authoritative Echo history.

## Target Package Topology

The first public release closure has this dependency shape:

```text
flyingrobots-echo-protocol
        |
        +--> flyingrobots-echo-kernel
        |
        +--> flyingrobots-echo-wal
                    |
protocol + kernel + wal
                    |
                    v
        flyingrobots-echo-runtime
```

### `flyingrobots-echo-protocol`

This package owns passive canonical contracts shared across layers:

- artifact, package, realm, epoch, event, reading, outcome, and evidence
  coordinates;
- authority-, budget-, effect-, release-, and schema-profile references;
- candidate and observed durable-record schemas;
- canonical codecs whose meaning is independently specified;
- typed refusal and fault classifications.

It MUST remain passive. A public caller may construct a protocol claim, but the
package MUST NOT grant runtime authority or declare that a claim committed.

### `flyingrobots-echo-kernel`

This package owns deterministic candidate computation from explicit admitted
inputs and bases:

- graph records and attachments;
- footprints, independence, and conflict calculations;
- candidate patches and snapshot calculations;
- pure validation and deterministic scheduling structures;
- pure application of candidate state transitions.

The kernel performs no persistence, external effects, runtime admission, or
durable settlement. Its output remains candidate material until the runtime
admits it at the serialization boundary.

### `flyingrobots-echo-wal`

This package owns persistence mechanism and storage observations:

- framing, checksums, and LSN continuity;
- retained storage implementations;
- scanning, tail classification, and recovery observations;
- persistence of encoded fencing material.

It may say that bytes were stored, frames scanned, checksums validated, or a
token value was observed. It MUST NOT independently say that a writer was
lawfully authorized, an epoch is authoritative, a candidate committed, a
causal receipt exists, or recovery was accepted. Those conclusions belong to
the runtime after complete validation.

For the first alpha, the WAL package exposes only Echo-retained implementations
whose behavioral guarantees are tested and included in the release closure.
Arbitrary third-party durable backends are deferred until Echo has a provider
guarantee and admission protocol. Implementing a Rust trait is not proof of
`fsync`, fencing, crash atomicity, or durable ordering.

### `flyingrobots-echo-runtime`

This package owns both the sealed public host facade and all authority-bearing
runtime machinery behind private modules. It privately controls:

- authority-realm and epoch construction;
- admission-token construction;
- package correlation and admission;
- candidate serialization and Tick construction;
- settlement and receipt construction;
- committed-state publication;
- recovery validation and acceptance.

The public facade accepts evidence-bearing requests and returns opaque handles,
readings, or typed outcomes. A caller requests work. Echo decides whether
lawful history exists afterward.

The API vocabulary MUST preserve the distinction among verification,
correlation, admission, execution, and settlement. Unless the runtime actually
executes the verifier, use operations shaped like:

```text
correlate_verified_subject
-> admit_and_install_package
-> submit
-> run_until_idle
-> observe_outcome / read_receipt
```

Do not collapse those steps into `verify_and_install_package`.

## Authority Must Not Escape Through Rust APIs

Private constructors are necessary but insufficient. Public admitted types
MUST also refuse indirect construction paths that would let a downstream crate
forge runtime truth, including:

- `Deserialize`, `Default`, or public fields and enum variants;
- `From` or `TryFrom` conversions from passive records or bytes;
- unchecked builders or raw reconstruction helpers;
- `Clone`, `Copy`, or mutable interior access where duplication or mutation
  would mint authority;
- test, fault-injection, compatibility, or unsafe constructors exposed through
  a published feature.

Opaque readings may expose identifiers, outcomes, and canonical evidence bytes.
Canonical bytes are evidence for corroboration; they are not a public inverse
constructor for an admitted runtime value.

Every `Send`, `Sync`, `Clone`, serialization, conversion, and borrowing
implementation on an authority-bearing public type requires an explicit
authority audit.

## Feature Surface Is Part of the Security Boundary

Every feature declared by a published Echo package, alone and in every
selectable combination, MUST remain authority-safe. The release closure may
enumerate behaviorally supported and tested configurations, but omission from
that list is not access control.

Published manifests MUST NOT contain dormant authority escape hatches such as:

```text
native_rule_bootstrap
host_test
raw_authority
trusted_runtime_internals
receipt_construction
fault_injection
```

Compatibility bootstrap, host-test authority, fault injection, and repository
fixtures belong behind `cfg(test)` or in `publish = false` packages.

## Compatibility and Legacy Packages

`warp-core` remains a temporary, `publish = false` compatibility monolith while
the public route is strangled out of it. It is not part of the preferred public
runtime closure and does not need a public-looking intermediate rename.

`flyingrobots-echo-native-bootstrap` is the intended name for any retained
compatibility/test lane once extracted. It remains `publish = false` and MUST
NOT enter the public runtime dependency or feature closure. Repository `xtask`,
`host_test`, and fault-injection packages are likewise excluded.

The first alpha does not publish a separate authority-bearing
`flyingrobots-echo-runtime-core`. Such a package is permissible later only when
every public inter-package API it requires is safe for arbitrary downstream
callers.

## Cargo Package Namespace

Every durable, reusable, or release-relevant Cargo package semantically owned
by Echo uses the `flyingrobots-echo-*` package prefix. Rust library names may
use conventional underscore names, and installed binaries may use product
names such as `echo` or `echod`.

Semantic ownership outranks repository location. A product-neutral library or
a package owned by Git WARP, Continuum, or another sibling system MUST NOT gain
the Echo prefix merely because it currently lives in this repository.

Repository orchestration and disposable fixtures may retain local conventional
names. `warp-core` is an explicit temporary legacy exception whose exception
ends when the public runtime no longer depends on it and its remaining
responsibilities have owners.

## Release Witness and Closure

The public boundary is proven by a clean-room consumer outside the Echo
workspace. `hello-echo-release-host` must consume packaged `.crate` archives
and the exact provider bundle, with no sibling worktrees or repository tooling,
and prove:

```text
release-closure corroboration
-> package evidence correlation and admission
-> submission and scheduling
-> settlement and receipt observation
-> clean shutdown
-> WAL restart
-> recovery of the same settlement
```

Negative clean-room witnesses must also prove that downstream code cannot:

- construct a Tick, receipt, authority epoch, or admission token;
- mutate private runtime registries or append authoritative history directly;
- enable any packaged feature combination that imports `warp-core`, native
  bootstrap, host-test authority, or fault injection.

The `EchoReleaseClosureId` binds the compatible crate archives and checksums,
Cargo dependency closure, runtime semantic and implementation identities,
profiles and ABIs, provider artifacts, target profile, schemas/codecs, and
release evidence. Individually authentic children from different release
closures do not form a valid Echo release.

Crates.io is a distribution and discovery mirror, not historical retention
authority. Release engineering, packaging, dry runs, and clean-room validation
are reversible work. Publishing crates, reserving names, changing ownership,
creating release tags, dispatching publish-capable workflows, or representing a
public release requires separate explicit human authorization.

## Alpha Exit Conditions

Echo is ready to propose its first public Rust alpha only when:

- `flyingrobots-echo-runtime` no longer depends on `warp-core`;
- the public package and feature closure excludes native bootstrap, host-test
  authority, fault injection, `xtask`, and raw authority constructors;
- protocol, kernel, and WAL dependencies point downward without cycles;
- all packaged feature combinations pass the authority-surface audit;
- the external clean-room host packages, executes, restarts, and recovers;
- the exact provider bundle and Cargo dependency closure are bound by one
  retained release-closure identity;
- all package dry runs and evidence generation have completed without any
  publication side effect.

## Current Posture

The unreleased `echo_runtime::RuntimeHost` proves that a sealed facade can own
host and WAL construction plus recovery without exposing the underlying
engine, native bootstrap, or trusted host. It does not yet provide the complete
external lifecycle, and its package remains `publish = false`.

The present repository still routes significant runtime work through
`warp-core`, and the Hello Echo route still relies on repository orchestration.
Those facts make this page a target boundary with an executable first witness,
not a claim that the alpha crate closure already exists.

## Relationships

- Depends on [Runtime authority](../topics/RuntimeAuthority.md).
- Depends on [WAL](../topics/WAL.md).
- Constrains [Generated rule authorship](../topics/GeneratedRules.md).
- Refines the public-host portion of
  [Application contract hosting](application-contract-hosting.md).
- Refines the retained registry/provider/host boundary in
  [ADR 0015](../adr/0015-registry-provider-host-boundary.md).
- Contributes to the [Echo 1.0 release contract](../releases/echo-1.0-contract.md)
  without claiming that contract complete.

## Evidence Anchors

- `crates/echo-runtime/src/lib.rs`
- `crates/echo-runtime/Cargo.toml`
- `crates/warp-core/src/trusted_runtime_host.rs`
- `crates/warp-core/src/engine_impl.rs`
- `xtask/src/run_edict_operation.rs`
- `scripts/verify-local.sh`
