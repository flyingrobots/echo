<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# WARP View Protocol Tasks

- [x] Define the “WARP View Protocol” package: channel naming, WarpId + owner identity, publisher-only writes, message pattern (snapshot + diff, gapless epochs, hashes/acks), transport (canonical CBOR, MAX_PAYLOAD, non-blocking).
- [x] Generalize as an Echo Interaction Pattern (EIP) template capturing roles, authority, message types, flow styles (req/resp, pub/sub, bidir), reliability/validation hooks for future services.
- [x] Enforce authority: session-service rejects non-owner writes on the WARP channel; client surfaces errors.
- [x] Dirty-flag sync loop in viewer: mark dirty on mutation, publish snapshot/diff on net tick when dirty; throttle/batch as needed.
- [x] Publish/subscribe toggles in UI: enable/disable sending my WARP and receiving per WarpId (v0: one warp per socket; reconnect to change).
- [x] Session-service wiring: publish endpoint, validate owner + gapless epochs, rebroadcast to subscribers; explicit error codes.
- [x] Client wiring: bidirectional tool connection (receive + publish); surface authority/epoch errors as notifications.
- [x] Demo path: doc for one session-service + two viewers (publisher + subscriber) showing shared WARP changes (`docs/guide/wvp-demo.md`).
- [x] Tests: protocol conformance (authority rejection, gapless enforcement, dirty-loop behavior, toggle respect) and integration test with two clients + server loopback. (Tracking: #169)
- [x] Docs sync: update execution-plan intents as slices land.
