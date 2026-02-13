<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Lock the Hashes

> **Priority:** P0 | **Status:** Pending Review | **Est:** ~20h
> **Evidence:** Audit [Issue #22](https://github.com/flyingrobots/echo/issues/22#issuecomment-3894974740)

Complete domain-separated hashing and benchmark umbrella close-out to lock deterministic hash foundations. The core commitment hashes (`state_root`, `patch_digest`, `commit_id`) and the RenderGraph canonical bytes hash currently use bare `Hasher::new()` without domain-separation prefixes; this milestone adds unique domain-separation tags to each hash context and audits/closes the benchmarks pipeline umbrella.

**Blocked By:** none

## Exit Criteria

- [x] All domain-separation prefixes defined and applied
- [x] Golden hash vectors updated and committed
- [x] Cross-domain collision tests pass in CI
- [x] Benchmarks umbrella [issue #22](https://github.com/flyingrobots/echo/issues/22) audited and closed
- [x] No open hash-drift issues

## Features

| Feature                        | File                                                     | Est. | Status      |
| ------------------------------ | -------------------------------------------------------- | ---- | ----------- |
| Domain-Separated Hash Contexts | [domain-separated-hashes.md](domain-separated-hashes.md) | ~8h  | In Progress |
| Benchmarks Pipeline Cleanup    | [benchmarks-cleanup.md](benchmarks-cleanup.md)           | ~4h  | In Progress |
