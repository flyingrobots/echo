<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Post-Contract Evidence Roadmap Refresh

Status: accepted docs slice.

## Purpose

PR #371 completed the local contract evidence, reading identity, semantic
retention, and generic external proof fixture batch. The roadmap signpost must
therefore move from that completed work to the remaining `v0.1.0` release-bar
work.

## Current Truth

- Contract-aware receipt correlation and QueryView reading evidence exist as
  local proof.
- Query readings carry stable `QueryReadingIdentity`.
- `echo-cas` has a local semantic retention index above content-only blobs.
- A generic external-consumer-shaped fixture proves mutation, QueryView,
  retained evidence, and replay without application nouns in Echo core.

## Next Release-Bar Order

The next work should keep narrowing the local contract-host release:

1. contract obstruction taxonomy;
2. retained evidence refs and missing-retention posture;
3. durable witnessed submission persistence;
4. product-facing intent outcome API;
5. versioned contract/API compatibility;
6. reference trusted runtime host loop;
7. serious external consumer proof fixture;
8. local replay/DIND proof for the contract path;
9. release-grade quickstart;
10. authority boundary audit.

## Non-Goals

- Do not reopen the completed contract-evidence batch as future work.
- Do not add new runtime behavior in this documentation slice.
- Do not weaken the authority boundary: application dispatch still does not
  execute synchronously or tick.
