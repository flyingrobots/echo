<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# v0.1.0 Replay And DIND Proof

Status: narrow local release witness implemented; broader DIND gate remains.

Depends on:

- [Durable witnessed submission persistence](./KERNEL_witnessed-intent-submission-persistence.md)
- [External contract proof fixture](./PLATFORM_external-contract-proof-fixture.md)
- [Contract artifact retention in echo-cas](./PLATFORM_contract-artifact-retention-in-echo-cas.md)

## Why now

The release sentence depends on deterministic replay. Echo must prove that the
local contract-host path can reproduce receipts, outcomes, and readings from
the retained package, submission, scheduler, and reading evidence.

## Required witness

Add a local replay/DIND proof for the app contract path:

```text
same package
+ same accepted submissions
+ same scheduler policy
+ same retained evidence
-> same receipts, outcomes, and readings
```

## Acceptance criteria

- [x] The external proof fixture participates in replay.
- [x] Accepted submissions replay with stable submission identity.
- [x] Tick receipts reproduce for the same scheduler-owned decision set.
- [x] Reading envelopes reproduce for the same query basis, vars, aperture, and
      observer identity.
- [x] Missing retained material produces obstruction rather than fake success.
- [x] `cargo xtask dind` or a narrower documented release witness covers the
      path.

## Implemented local slice

`cargo xtask test-slice contract-path-release` now runs the explicit local
contract path release witness: installed contract pipeline replay, reference
trusted host loop, and the serious external-consumer-shaped fixture. The command
is a narrow release witness, not a replacement for broader DIND CI gates.

## Non-goals

- Do not require distributed replica import.
- Do not implement settlement shells.
- Do not require full observer-rights revelation proofs.
