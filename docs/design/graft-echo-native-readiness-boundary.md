<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Graft Echo-Native Readiness Boundary

Status: active cross-repo boundary note.

Last updated: 2026-06-15.

## Purpose

This note records the current boundary between Echo's landed capabilities and
Graft's ability to claim Echo-native structural history. It exists to prevent
cross-repo integration work from overclaiming what Echo can provide today.

Short form:

```text
Graft can continue schema, model, adapter, and local witness design work.
Graft MUST NOT yet claim production-grade Echo-native structural history.
```

## Current Echo Support

Echo `origin/main` now has enough posture, authority, strand, braid, and
contract-host structure for Graft to continue forward.

Graft may rely on these Echo-side facts for design and prototype work:

- posture and authority primitives exist in `warp-core`, including
  `CausalPosture`, `AuthorityDomainRef`, `CausalAuthority`,
  `RetentionPosture`, `PostureDerivation`, `AdmissionScopeId`,
  `SessionContext`, `MaterializationReceipt`, and `PromotionIntent`;
- `CausalPosture` has no global default;
- `Shared` posture requires an admission scope;
- legacy shared authority does not authorize fresh admission;
- scratch-to-author-only materialization is explicit;
- promotion to shared is intent, witness, authority, and scope shaped;
- strand creation carries retention posture through requests and receipts;
- debugger-created strand work is forced away from shared posture;
- braid history is append-only and supports later member weaving without
  pretending settlement or merge occurred;
- braid application rejects duplicate members, mixed member-reference posture,
  incoherent sequence numbers, empty settlement frontiers, invalid lifecycle
  transitions, and empty collapse witnesses;
- the trusted runtime host owns package registration, ticketed ingress,
  scheduler passes, and runtime control;
- app-safe callers can submit and observe without tick, package-install, or
  scheduler authority;
- the registry verifies contract ABI, codec id, registry version, schema hash,
  Wesley generator version, helper API version, and optional footprint
  certificates;
- installed query observers can emit bounded `QueryView` readings;
- retained evidence posture and coordinates exist in the WASM ABI and core
  surfaces;
- `echo-cas` has semantic retention coordinates above content-only CAS;
- Echo has a serious external-consumer-shaped Rust fixture covering package
  install, app submission, trusted host staging, bounded query reading,
  retained payload, and retained receipt evidence.

## Claim Boundary

Graft can honestly say:

```text
Echo now has enough core ontology for Graft schema/model/adapter work and for
prototype local Echo witness design.
```

Graft MUST NOT yet say:

```text
Normal Graft structural evidence is Echo-native, release-grade, and durably
recoverable through a stable TypeScript dependency.
```

That stronger claim waits on two Echo-side release surfaces and several
Graft-side schema/model changes.

## Echo-Side Blockers

### App-Safe TypeScript / Node Surface

Echo has WASM and ABI crates, but it does not yet publish a stable app-safe
Node or TypeScript package that Graft can depend on as a released integration
surface.

Current caveats:

- root `package.json` is private docs/test infrastructure;
- `echo-wasm-bindings` remains demo-kernel shaped;
- the release plan still treats app-safe WASM, Node, and browser packaging as
  missing or conditional;
- Graft prototypes must treat sibling checkout paths, local WASM builds, and
  process execution as witness-runner details, not released Echo API.

Required future proof:

- stable package identity and versioning;
- documented app-safe API surface;
- generated TypeScript or JavaScript helpers where applicable;
- tests proving application code cannot reach tick, package-install,
  scheduler, or trusted recovery authority through the package.

### Durable Retained Evidence

Echo can report retained evidence posture and semantic coordinates, but not
all witness paths are production-grade durable recovery paths yet.

Current caveats:

- retained evidence posture can honestly report available material or missing
  retained evidence; core distinguishes missing coordinates from missing
  content, and app-safe ABI callers see missing retained material as
  `MissingRetention`;
- semantic retained coordinates exist above CAS byte identity;
- current trusted host witness paths may still use in-memory WAL or local
  fixture storage;
- docs continue to distinguish local retained posture from durable host
  storage and crash protocol.

Required future proof:

- configured host durability mode visible in witness output;
- retained artifact, receipt, witness, and reading recovery after restart;
- obstruction when retained material is unavailable;
- no claim of restart-proof structural-history durability without a host that
  actually provides it.

## Graft-Side Blockers

The following blockers remain Graft-owned and must not be solved by importing
Graft domain nouns into Echo core:

- structural history needs a sanctioned `UNPINNED_COMMITTED` basis kind for
  committed-history readings with no ref or head;
- remaining structural WARP reads need to move behind the schema-first
  structural reading boundary;
- multidimensional `EvidencePosture` remains a future idea, not current schema
  authority;
- Graft product language must sit above Echo's generic contract, reading,
  retained evidence, and witness surfaces.

## Permitted Next Work

Graft may proceed with:

- schema and model cleanup;
- adapter boundaries over current Echo primitives;
- local Echo witness prototypes;
- app-safe posture reporting that stays within Echo's current available and
  missing-retention states;
- experiments that exercise Echo through a sibling checkout or local WASM
  artifact while labeling the path as prototype-only.

Echo may proceed with:

- app-safe package and TypeScript/Node release-surface design;
- durable retained evidence recovery proof;
- generic retained evidence and witness posture improvements;
- no-application-nouns guards around any Graft-facing fixture.

## Non-Goals

- Do not make Graft the Echo release gate. The active `v0.1.0` release gate
  remains jedit.
- Do not add Graft structural nouns to Echo core.
- Do not treat local witness prototypes as released package support.
- Do not claim durable structural history from in-memory or fixture storage.
- Do not replace Graft schema work with Echo-side special cases.

## Acceptance Bar For Stronger Claims

Graft may claim production-grade Echo-native structural evidence only when all
of these are true:

- Graft has schema authority for unpinned committed structural history;
- Graft structural reads go through a schema-first structural reading
  boundary;
- Echo exposes a stable app-safe TypeScript or Node integration surface;
- the integration surface preserves trusted runtime authority boundaries;
- retained evidence recovery is backed by a configured durable host path;
- restart proves retained structural evidence posture honestly;
- the proof runs from the Graft repository without Echo core importing Graft
  domain nouns.
