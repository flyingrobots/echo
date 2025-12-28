<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# WARP View Protocol Tasks

- [x] Define the “WARP View Protocol” package: channel naming, WarpId + owner identity, publisher-only writes, message pattern (snapshot + diff, gapless epochs, hashes/acks), transport (canonical CBOR, MAX_PAYLOAD, non-blocking).
- [x] Generalize as an Echo Interaction Pattern (EIP) template capturing roles, authority, message types, flow styles (req/resp, pub/sub, bidir), reliability/validation hooks for future services.
- [x] Enforce authority: session-service rejects non-owner writes on the WARP channel; client surfaces errors.
- [ ] Dirty-flag sync loop in viewer: mark dirty on mutation, publish snapshot/diff on net tick when dirty, clear on ack; throttle/batch as needed.
- [ ] Publish/subscribe toggles in UI: enable/disable sending my WARP and receiving per WarpId, preserving epoch/hash continuity when re-enabled.
- [ ] Session-service wiring: add publish endpoint, validate owner + gapless epochs/hashes, rebroadcast to other subscribers; explicit error codes.
- [ ] Client wiring: implement publish call + retry/backoff; handle authority/hash errors and resync requests.
- [ ] Demo path: script/doc for one session-service + two viewers (one publisher, one subscriber) showing shared WARP changes.
- [ ] Tests: protocol conformance (authority rejection, gapless enforcement, dirty-loop behavior, toggle respect) and integration test with two clients + server loopback.
- [ ] Docs sync: update execution-plan intents and decision-log entries as slices land.
