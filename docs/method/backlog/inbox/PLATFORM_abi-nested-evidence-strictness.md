<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ABI nested evidence strictness

Status: inbox.

Source: PR #322 review follow-up.

## Why now

PR #322 adds `#[serde(deny_unknown_fields)]` to the new witnessed suffix
admission shell DTOs so callers cannot smuggle transport, sync, or stringly
status fields into the shell boundary.

That strictness currently stops at the new shell structs. The shell reuses
existing settlement evidence DTOs such as `SettlementBasisReport`,
`SettlementParentRevalidation`, `SettlementOverlapRevalidation`, `BaseRef`, and
`ProvenanceRef`. Those reused DTOs predate the shell and do not consistently
deny unknown fields.

That means a future caller using a self-describing format could still nest
unexpected keys inside existing evidence objects even when the outer witnessed
suffix shell rejects unknown top-level fields. This was deliberately not fixed
inside PR #322 because it would tighten pre-existing ABI behavior beyond the
shape-only witnessed suffix skeleton.

## What it should look like

Make an explicit ABI policy decision for nested evidence DTO strictness:

- decide whether reused ABI evidence DTOs should reject unknown fields
  everywhere
- identify all public DTOs that carry provenance, basis, settlement, overlap,
  reading, or admission evidence
- add `#[serde(deny_unknown_fields)]` where the public contract should be
  closed
- add deterministic CBOR rejection tests for nested unknown fields
- document any ABI epoch/version impact if existing consumers may depend on
  unknown-field tolerance

## Done looks like

- nested unknown-field behavior is intentional and tested, not incidental
- settlement evidence DTOs have a clear compatibility posture
- witnessed suffix admission tests cover unknown fields inside nested
  `basis_report` evidence if the policy closes that boundary
- any ABI version or epoch consequences are handled in the same focused change

## Repo evidence

- `crates/echo-wasm-abi/src/kernel_port.rs`
- `crates/echo-wasm-abi/src/witnessed_suffix_tests.rs`
- `docs/design/witnessed-suffix-admission-shell.md`
- PR #322 discussion:
  `https://github.com/flyingrobots/echo/pull/322#discussion_r3173541861`

## Non-goals

- Do not fold this into the first witnessed suffix shell skeleton.
- Do not redesign the whole ABI surface opportunistically.
- Do not add transport, sync, or import execution behavior.
- Do not weaken the existing outer-shell `deny_unknown_fields` posture.
