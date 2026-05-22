<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Product-Facing Intent Outcome API

Status: implemented local core surface; adapter polish remains.

Depends on:

- [Durable witnessed submission persistence](./KERNEL_witnessed-intent-submission-persistence.md)
- [Contract obstruction taxonomy](./KERNEL_contract-obstruction-taxonomy.md)
- [Contract-aware receipts and readings](./KERNEL_contract-aware-receipts-and-readings.md)

## Why now

Echo has internal correlation between witnessed submissions, admission tickets,
ticketed runtime ingress, and tick receipts. A developer building on Echo needs
a small app-safe API that says what happened to a submitted intent without
exposing scheduler control.

## Shape

The product-facing surface should support the equivalent of:

```rust
let submission = echo.submit_intent(intent_bytes)?;

match echo.observe_intent_outcome(submission.id)? {
    IntentOutcome::Pending { .. } => {}
    IntentOutcome::Applied { receipt, .. } => {}
    IntentOutcome::Rejected { reason, blocked_by, .. } => {}
    IntentOutcome::Obstructed { obstruction, .. } => {}
    IntentOutcome::Unknown { .. } => {}
}
```

The exact names may differ, but the authority boundary may not.

Current local core surface:

- `WorldlineRuntime::submit_app_intent(...)`;
- `IntentSubmissionHandle`;
- `WorldlineRuntime::observe_app_intent_outcome(...)`;
- `IntentOutcome`;
- `IntentOutcomeReceipt`.

The remaining work is adapter/API polish above this core surface.

## Acceptance criteria

- Submitting an intent returns stable submission identity and generation.
- Submission does not tick, dispatch handlers, or mutate application state.
- Outcome observation can report unknown, pending, applied, rejected, and
  obstructed states.
- Applied/rejected outcomes bind to the relevant tick receipt evidence.
- Obstructed outcomes use the contract obstruction taxonomy.
- The API exposes no trusted runtime control and no tick/step command.
- Duplicate submission behavior is documented and tested.

## Non-goals

- Do not implement streaming subscriptions.
- Do not add hidden retry.
- Do not expose trusted scheduler control to application code.
- Do not make `submit_intent` mean "run this now."
