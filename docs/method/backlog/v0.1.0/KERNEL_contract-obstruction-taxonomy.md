<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Contract Obstruction Taxonomy

Status: v0.1.0 release blocker.

Depends on:

- [Contract-aware receipts and readings](./KERNEL_contract-aware-receipts-and-readings.md)
- [Contract reading identity and bounded payloads](./KERNEL_contract-reading-identity-and-bounded-payloads.md)
- [Echo v0.1.0 Release Plan](../../../design/v0.1.0-release-plan.md)

## Why now

`v0.1.0` needs failure evidence that is precise enough for application hosts,
tests, and replay. Product-facing callers should not receive generic failure
strings or empty successes when Echo can name the obstruction.

## Required taxonomy

The first release bar needs typed, contract-aware outcomes for at least:

- unsupported operation;
- unsupported query;
- admission obstruction;
- runtime fault;
- missing retention;
- stale basis;
- residual reading;
- budget exceeded.

The taxonomy may reuse existing core variants where they already express the
truth. It should add only the missing generic names needed by contract-hosted
applications.

## Acceptance criteria

- Unsupported mutation ids produce a typed unsupported-operation outcome.
- Unsupported query ids produce a typed unsupported-query outcome.
- Missing retained material returns missing-retention posture, not empty
  success.
- Stale or unavailable basis returns typed obstruction.
- Residual or budget-limited readings report residual/budget posture.
- Runtime-local fault posture does not masquerade as a lawful rejection.
- Contract-aware receipts and readings carry enough identity to route the
  obstruction back to the package, operation/query, basis, and request.

## Non-goals

- Do not invent application-domain obstruction types in Echo core.
- Do not collapse internal runtime faults into normal domain rejections.
- Do not implement full observer-rights revelation governance.
