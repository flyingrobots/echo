<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Witnessed suffix admission hardening

Status: inbox.

Source: PR #323 retrospective follow-up.

## Why now

PR #323 hardened the witnessed suffix local admission evaluator around
deterministic posture output, canonical source entries, resolved-basis
classification, and missing local digest evidence.

Those fixes should not turn into a wider PR-review workstream, but the review
left useful backlog fuel. Future call sites may construct
`WitnessedSuffixLocalAdmissionPosture` outside the current tests. Obstruction
digest domains may grow beyond the local evaluator family. Other
admission-shaped responses may still expose caller-order leakage at an ABI
boundary. Test fixtures that look clean can also accidentally encode malformed
causal coordinates.

This card captures that follow-up work without reopening the current evaluator
scope.

## What it should look like

- Add canonical constructors or validation helpers for
  `WitnessedSuffixLocalAdmissionPosture` before additional call sites build
  postures directly.
- Add an explicit design paragraph for canonical source suffix invariants:
    - every source entry belongs to the shell source worldline
    - provenance refs are in canonical order
    - entries sit inside the claimed suffix bounds
    - the suffix carries either witness-backed entries or boundary witness
      evidence sufficient for an honest obstruction
- Document obstruction digest domains and how their domain/version strings
  relate to future Continuum evidence hashes.
- Audit admission and settlement response vectors for ABI-visible caller-order
  leakage.
- Strengthen fixture discipline so positive "clean" fixtures cannot encode
  malformed causal coordinates by accident. Malformed coordinates should appear
  only in negative tests that name the invariant they violate.

## Done looks like

- Posture construction has a named canonical path, or direct construction is
  intentionally constrained and documented.
- The witnessed suffix design docs name the canonical source suffix invariants
  explicitly.
- Obstruction digest domain/version policy is documented before more digest
  families appear.
- ABI-visible admission and settlement vectors are audited, with canonicalized
  vectors fixed or caller-law ordering documented.
- Test fixture helpers make valid causal coordinates the default.

## Repo evidence

- `crates/warp-core/src/witnessed_suffix.rs`
- `crates/warp-core/src/witnessed_suffix_tests.rs`
- `crates/echo-wasm-abi/src/witnessed_suffix_tests.rs`
- `docs/design/witnessed-suffix-admission-evaluator.md`
- `docs/design/continuum-runtime-and-cas-readings.md`
- PR #323: `https://github.com/flyingrobots/echo/pull/323`

## Non-goals

- Do not reopen PR #323's evaluator implementation without a new RED.
- Do not redesign the witnessed suffix ABI.
- Do not add Continuum proof, IPA, or commitment machinery.
- Do not change transport, sync, or import execution behavior.
- Do not weaken the evaluator's existing obstruction posture.
