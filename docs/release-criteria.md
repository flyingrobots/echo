<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Release Criteria — Phase 0.5 → Phase 1

Checklist for closing Phase 0.5 and starting Phase 1 implementation.

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
- [ ] Decision log updated with outcomes (including EPI bundle).
- [ ] Documentation index current (spec map).

Once all items checked, open Phase 1 milestone and migrate outstanding tasks to implementation backlog.
