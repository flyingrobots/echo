<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Release Criteria — Phase 0.5 → Phase 1

Checklist for closing Phase 0.5 and starting Phase 1 implementation.

## How to Use This Checklist

- Treat each item as a gate: “done” means it is implemented **and** verified.
- Link evidence (tests, docs, or CI runs) in the Phase 0.5 tracking issue.
- If a requirement moves, update the checklist so it stays authoritative.

## Required Criteria

- [ ] Branch tree spec v0.5 implemented (roaring bitmaps, epochs, hashing).
- [ ] Codex’s Baby Phase 0.5 features implemented (event envelope, bridge, backpressure).
- [ ] Temporal bridge integrated with branch tree and CB.
- [ ] Serialization protocol implemented with content-addressed blocks.
- [ ] Replay CLI (`echo replay --verify`) passes golden hash suite.
- [ ] Entropy observers and inspector packets verified.
- [ ] Capability tokens and security envelopes enforced.
- [ ] Determinism test suite green on Node, Chromium, WebKit.
- [ ] Deterministic config loader produces `configHash`.
- [ ] Plugin manifest loader validates capabilities and records `pluginsManifestHash`.
- [ ] Inspector JSONL writer produces canonical frames.
- [ ] Documentation index current (spec map).

## Evidence Expectations (Examples)

- Determinism suite: CI logs or `echo-dind-harness` transcript.
- Replay CLI: golden hashes checked in `testdata/` with a reproducible runner.
- Protocol gates: a spec doc + a passing conformance test.
- Docs: `docs/meta/docs-index.md` updated with links to current specs.

Once all items checked, open Phase 1 milestone and migrate outstanding tasks to implementation backlog.
