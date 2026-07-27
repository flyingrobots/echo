<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Generic Edict Executable-Operation Lowering

## Goal

Let Edict invoke Echo's target provider with an arbitrary application Core
module and a declarative lawpack closure, producing one exact
`ExecutableOperationPackageV1` without teaching Echo any application-owned
coordinate, operation name, type name, or obstruction name.

This is the target-lowering crossing selected by ADR 0023. Provider v1 remains
unchanged compatibility infrastructure; it is not the application execution
path established here.

## Hill

The current target-provider implementation proves only a fixed synthetic
provider-v1 projection. The abandoned Hello Echo spike added a second fixed
application dispatch. Neither shape is acceptable for a language/runtime
boundary: changing an application package or intent name must not require an
Echo source change.

The target provider must instead derive the executable package from six
digest-bound semantic roles:

1. Edict source;
2. canonical Core;
3. lawpack exports;
4. the lawpack manifest;
5. the selected declarative Echo adapter and target configuration;
6. the compiler-produced Echo Target IR.

Application coordinates are opaque authored data. Echo owns only the supported
target intrinsic, program kind, package schema, profile identities, resource
bounds, and relation checks.

## Acceptance Criteria

- One lowerer binary accepts two fixtures with unrelated application, intent,
  lawpack, effect, obstruction, type-profile, and authority-profile names.
- Lowering uses semantic input kinds and digest-bound resource references,
  never application names encoded in invocation roles or provider code.
- The emitted bytes match the independent `warp-core`
  `ExecutableOperationPackageV1` model exactly.
- The output operation coordinate is derived from the Core package coordinate
  and authored intent name.
- The package binds the exact source, Core, Target IR, exports, and lawpack
  identities supplied by Edict.
- A structurally separate verifier recomputes the relation without calling or
  importing lowerer logic.
- The verifier accepts both unrelated fixtures and rejects a canonical package
  whose operation coordinate is rebound.
- The target-provider manifest and schemas expose generic input/output roles
  and generic artifact domains.
- A standalone external Edict application can build exact package and
  verification-report bytes through the public Edict surface.

## Playback Questions

- Can an application rename its package and intent without changing Echo?
- Can a different lawpack coordinate select the same supported Echo primitive?
- Does changing any bound closure artifact change or invalidate the package?
- Can the verifier reject a self-consistent package mutation independently?
- Does the external application build without a handwritten package, fake
  transport, or native application callback?

## Non-Goals

- Expanding provider v1 callbacks or generated Rust helpers.
- Adding application-specific intrinsics to Echo.
- Claiming runtime-neutral executable package bytes.
- Supporting more than one bounded target operation in a package.
- Supporting multi-record atomic creation.
- Publishing crates, Git branches, pull requests, releases, or repositories.
- Migrating Graft data from git-warp.
