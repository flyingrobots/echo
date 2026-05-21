<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Installed Wesley Contract Host Dispatch

Status: local installed package dispatch proof complete; external consumer
contract proof remains downstream.

Depends on:

- [0018 - Contract-Hosted File History Substrate](../../../design/0018-contract-hosted-file-history-substrate/design.md)
- [0017 - Authenticated Wesley Intent Admission Posture](../../../design/0017-authenticated-wesley-intent-admission-posture/design.md)
- [0016 - Wesley To Echo Toy Contract Proof](../../../design/0016-wesley-to-echo-toy-contract-proof/design.md)

## Why now

Echo can accept EINT bytes and now routes package-supported generated contract
mutations through witnessed submission, ticketed runtime ingress, and
scheduler-owned ticks. The remaining platform work is proving the same generic
surface with an external Wesley-compiled consumer package while keeping
application nouns out of `warp-core`.

## Current Checkpoint

`warp-core` now exposes the scheduler-owned EINT contract-host helper seam used
by installed `cmd/*` rules:

- match a scheduler-materialized EINT runtime ingress event by generated op id;
- borrow canonical vars bytes for generated decoding;
- declare the standard runtime-ingress read footprint and extend it with
  handler-specific writes;
- prove handlers run during `SchedulerCoordinator::super_tick(...)`, not during
  application dispatch.

`echo-wesley-gen --contract-host` now emits std-only generated mutation helper
rules for that seam:

- stable command-rule names bound to schema hash, op id, and operation name;
- op-id matchers for scheduler-materialized EINT runtime ingress events;
- typed vars decoders using the generated CBOR shape;
- base runtime-ingress read footprint helpers;
- rule constructors that accept host-supplied executor and footprint functions.

`warp-core` now has a registry-verified installed contract package boundary that
verifies a generated `RegistryProvider`, binds package identity, mutation
handler rules, and query observers, and rejects unsupported operations,
mutation rule/op-id mismatches, duplicate package operation ids, and duplicate
package rule identities before handlers or observers install into `Engine`.

Direct `native_rule_bootstrap` registration remains available only as an
internal fixture and transitional engine-test path. It is not the registry
package boundary and does not provide package identity guarantees.

The local dispatch proof now verifies:

- package-supported EINT mutation ids enter runtime only through witnessed
  submission plus ticketed runtime ingress;
- unsupported installed-contract mutation ids are rejected before they become
  runtime-visible work;
- handler execution occurs during `SchedulerCoordinator::super_tick(...)`, not
  during application submission or ticketed ingress;
- receipt/outcome observation reports applied and rejected tick decisions;
- footprint conflicts are final for that tick attempt, with blocker attribution
  and no hidden retry ingress;
- witnessed submission replay restores pending ingress history without staging
  inbox work; and
- replayed installed-contract pipeline runs converge to the same receipt
  correlation and observed outcome.

Remaining work moved out of this card: external consumer proof fixtures,
contract-aware receipt/reading polish, and broader DIND replay closure.

## RED

Added failing tests with a tiny generated-shaped contract fixture:

- install one mutation op id and generated handler;
- submit canonical EINT bytes through witnessed submission;
- prove no direct test-only mutation service is called;
- assert worldline/provenance state changes only after scheduler execution;
- reject unsupported package op ids before runtime ingress;
- expose receipt-level applied/rejected decisions; and
- prove replay convergence.

## GREEN

Added package-supported op-id lookup during the ticketed runtime handoff and
receipt/outcome observation over scheduler-owned decisions. Generated vars
decode and handler dispatch remain inside installed mutation rules.

## Acceptance criteria

- Unsupported op id obstructs or errors when contract-hosting validation is
  enabled.
- Handler execution is inside Echo admission/witness/provenance.
- Footprint authority is not accepted from caller-supplied JSON.
- Echo core does not import jedit or text-domain Rust types.

## Non-goals

- Do not implement dynamic plugin loading.
- Do not invent an EINT replacement before a RED proves EINT v1 insufficient.
- Do not implement production crypto in this card.
