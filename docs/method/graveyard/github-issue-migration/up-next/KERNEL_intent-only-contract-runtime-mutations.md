---
audit-date: 2026-06-15
audit-commit: 5f85dae5727d36acf4a82aad8d7cdb0488cb67be
audit-status: keep
topics:
    - intent
    - runtime
    - contract
accuracy: 0.95
issue: 477
findings:
    - claim: "External mutations (like settling a strand, pinning support) enter through host ABI"
      ruling: true
      evidence:
          filepath: crates/echo-wasm-abi/src/lib.rs
          line: 1
    - claim: "External mutations are refactored to enter through the dispatch_intent pipeline"
      ruling: false
---

<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Intent-Only Contract Runtime Mutations

Status: planned kernel/runtime implementation.

Depends on:

- [Installed Wesley contract host dispatch](../asap/PLATFORM_installed-wesley-contract-host-dispatch.md)
- [Contract Strands And Counterfactuals](./KERNEL_contract-strands-and-counterfactuals.md)
- [0018 - Contract-Hosted File History Substrate](../../../design/0018-contract-hosted-file-history-substrate/design.md)

## Why now

Some Echo services mutate directly today. Those services can remain internal, [🟢, 95%, [crates/echo-wasm-abi/src/lib.rs:1](file:///Users/james/git/echo/crates/echo-wasm-abi/src/lib.rs#L1)]
but external application flows for contract families, strands, braids, [🔴, 90%, (no supporting evidence was found - may be false 🤥)]
settlement, support pins, and inverse operations must have intent paths.

## What it should look like

Add generic EINT-facing operations for externally visible runtime mutations:

- create contract strand;
- pin and unpin support when exposed to application flows;
- settle strand;
- settle braid;
- admit braid projection;
- provenance fork when exposed as contract flow.

Each operation should enter through:

```text
dispatch_intent(EINT)
  -> IngressEnvelope
  -> scheduler/admission
  -> handler
  -> witnessed provenance
```

## Acceptance criteria

- A jedit-style test can create a buffer worldline, create a strand or braid,
  append a member, settle, and unapply without direct external service calls.
- Existing internal services may still implement the mutation behind the
  handler.
- Provenance records the same MergeImport or ConflictArtifact semantics as the
  direct service path.
- Direct ABI calls remain compatibility/debug only and are not required by the
  proof path.

## Non-goals

- Do not delete existing direct internal services in the first slice.
- Do not add text-specific mutations to Echo core.
- Do not weaken scheduler or footprint validation.
