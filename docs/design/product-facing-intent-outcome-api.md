<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Product-Facing Intent Outcome API

Status: accepted and implemented local core surface.

## Claim

Echo now has a small app-facing submit/observe outcome surface over the
existing witnessed submission and scheduler receipt correlation machinery. The
surface returns stable submission handles and product-level outcome states
without exposing trusted runtime control.

## Boundary

`WorldlineRuntime::submit_app_intent(...)` records witnessed ingress history and
returns `IntentSubmissionHandle`. It does not tick, stage runtime ingress,
dispatch handlers, execute contracts, or mutate application state.

`WorldlineRuntime::observe_app_intent_outcome(...)` is a read-only polling
surface returning `IntentOutcome`:

- `Unknown`;
- `Pending`;
- `Applied`;
- `Rejected`;
- `Obstructed`.

Applied and rejected outcomes carry `IntentOutcomeReceipt`, which names the
ticketed ingress id, admission ticket digest, exact `CausalTickReceiptRef`, tick
receipt content digest, commit hash, worldline tick, entry index, rule id,
installed contract evidence when present, and retained receipt evidence posture
where Echo can name a lawful contract coordinate. Obstructed outcomes use the
contract-host obstruction taxonomy and use the same retained receipt coordinate
for generated contract work.

## Invariants

- Submission handles are not tick receipts.
- `TickReceipt::digest()` is a repeatable content commitment, not causal event
  identity.
- Later intents cite `IntentOutcomeReceipt::causal_receipt_ref`; they must not
  cite `tick_receipt_digest` as though it uniquely identified one admission.
- A causal receipt reference is a coordinate, not authority. Recovery validates
  it against retained submission, ticket, worldline, tick, commit, receipt, and
  provenance evidence before restoring an outcome.
- Outcome observation is read-only.
- Pending does not imply failure or retry.
- Rejected is a lawful scheduler outcome, not a runtime fault.
- Obstructed means Echo cannot honestly interpret required evidence.
- Missing receipt evidence maps to `MissingRetention`.
- Contract-backed outcomes surface retained receipt posture honestly. A missing
  retained descriptor is reported as `MissingCoordinate`; it is not an empty
  success and not a fabricated retained byte reference.
- The surface exposes no tick, step, scheduler, or trusted runtime authority.

## Non-Goals

- Do not implement streaming subscriptions.
- Do not add hidden retry.
- Do not expose scheduler control.
- Do not make submit mean execute.
- Do not replace lower-level receipt correlation APIs.

## Witnesses

- `cargo test -p warp-core --lib app_intent`
