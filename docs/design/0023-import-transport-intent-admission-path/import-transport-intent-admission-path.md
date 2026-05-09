<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Import transport Intent admission path

Status: planned implementation slice.

Depends on:

- [0022 - Continuum transport identity and import idempotence](../../../design/0022-continuum-transport-identity/design.md)

## Why now

Echo now has the right witnessed suffix vocabulary and doctrine, but inbound
transport admission is still only documented. The runtime has
`CausalSuffixBundle`, `ImportSuffixRequest`, and the local
`import_suffix(...)` evaluator, but the external causal path is not real until a
transported suffix is submitted as an EINT Intent and admitted through Echo.

The rule to make executable:

```text
transport adapter receives bytes
-> adapter forms canonical import proposal
-> dispatch_intent(EINT import intent)
-> ingress / scheduler / admission
-> tick + receipt / witness
```

## Goal

Add the first narrow import-transport Intent family for a `CausalSuffixBundle`
against an explicit target basis.

This should prove the external path without trying to implement peer sync,
networking, or full idempotence indexing.

## Likely files touched

- `crates/echo-wasm-abi/src/kernel_port.rs`
- `crates/warp-core/src/witnessed_suffix.rs`
- `crates/warp-core/src/cmd.rs`
- `crates/warp-core/src/engine_impl.rs`
- `crates/warp-wasm/src/warp_kernel.rs`
- `crates/warp-core/tests/**`
- `crates/warp-wasm/tests/**` or `warp_kernel` tests

## Acceptance criteria

- A canonical import-transport EINT payload shape exists for:
    - bundle or retained bundle ref
    - target worldline/focus
    - explicit target basis
    - admission law/version where needed
    - actor/cause placeholder if the current capability model is not ready
- A RED/GREEN test dispatches the import proposal through `dispatch_intent`,
  not by calling `import_suffix(...)` as the external path.
- The dispatched intent enters Echo through `IngressEnvelope::local_intent` and
  is consumed by scheduler/admission machinery.
- The handler/evaluator returns a typed witnessed suffix outcome:
    - admitted
    - staged
    - plural
    - conflict
    - obstructed
- The path emits or preserves receipt/witness evidence for the local decision.
- Malformed EINT or malformed import payload returns typed error/obstruction
  without mutating causal history.
- Existing direct evaluator functions remain available as internal helpers, not
  as public mutation authority.

## Non-goals

- Do not add a sync daemon.
- Do not add networking.
- Do not implement `git-warp` interop here.
- Do not solve full duplicate import retention/indexing here.
- Do not add jedit nouns.
- Do not add a second non-EINT intent envelope.
- Do not make transport arrival itself causal history.

## Test expectations

- One failing test first proves direct `import_suffix(...)` is not the external
  mutation path being exercised.
- One passing test proves the same import proposal goes through EINT,
  `dispatch_intent`, ingress, scheduler/admission, and returns a typed outcome.
- One malformed-payload test proves no direct mutation or fake success occurs.
