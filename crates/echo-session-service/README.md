<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# `echo-session-service`

Headless Echo session hub process.

## What this crate does

- Listens on a Unix socket (by default `DEFAULT_SOCKET_PATH` from
  `echo-session-proto`) and speaks the JS-ABI v1.0 framing defined in
  `echo-session-proto::wire`.
- Handles client handshakes and assigns a logical timestamp (`ts`) to every
  message using a per-hub kernel clock.
- Maintains per-`RmgId` stream state:
  - last epoch and optional state hash,
  - latest snapshot (for late joiners),
  - current producer connection,
  - subscriber list.
- Enforces gapless RMG diffs:
  - accepts a `Snapshot` as a reset for a stream,
  - accepts `Diff` frames only when `from_epoch == last_epoch` and
    `to_epoch == from_epoch + 1`.
- Fans out accepted `RmgStream` frames and `Notification` messages to all
  subscribed clients via per-connection outboxes.

## Documentation

- The high-level role of the session hub and its relationship to tools is
  described in the Echo book’s Tools booklet,
  `docs/book/echo/booklet-05-tools.tex`, Section
  `Echo Session Service and RMG Viewer Sync` (`07-session-service.tex`).
- The underlying JS-ABI framing and RMG streaming semantics are covered in the
  Core booklet (`booklet-02-core.tex`), Sections
  `13-networking-wire-protocol.tex` and `14-rmg-stream-consumers.tex`.
