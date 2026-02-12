<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Changelog

## Unreleased

### Changed — Gateway Resilience (`echo-session-ws-gateway`)

- **Hub observer reconnect** now uses `ninelives` retry policy with exponential
  backoff (250 ms → 3 s) and full jitter, replacing hand-rolled backoff state.
  Retries are grouped into bursts of 10 attempts; on exhaustion a 10 s cooldown
  separates bursts. This prevents synchronized retry storms across gateway
  instances and improves recovery behavior during prolonged hub outages.
- Connection setup (connect + handshake + subscribe) extracted into
  `hub_observer_try_connect`, separating connection logic from retry
  orchestration.
- Entire connection attempt (connect + handshake + subscribe) is now wrapped in
  a single 5 s timeout, preventing a stalled peer from hanging the retry loop.
- Retry policy construction uses graceful error handling instead of `.expect()`,
  so a misconfiguration disables the observer with a log rather than panicking
  inside a fire-and-forget `tokio::spawn`.
- Added 1 s cooldown after the read loop exits to prevent tight reconnect loops
  when the hub accepts connections but immediately closes them.

### Fixed

- **Security:** upgraded `bytes` 1.11.0 → 1.11.1 to fix RUSTSEC-2026-0007
  (integer overflow in `BytesMut::reserve`).
