<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# `echo-session-client`

Client helpers for talking to the Echo session hub, plus tool-facing adapters.

## What this crate does

- Provides a minimal async Unix-socket client (`SessionClient`) that can:
  - connect to the hub,
  - send JS-ABI framed messages (handshake, subscribe),
  - poll messages from the session stream.
- Offers blocking helpers for tools:
  - `connect_channels(path)` and `connect_channels_for(path, warp_id)` connect
    to the hub, perform a handshake + WARP subscription, and spawn a background
    thread that decodes packets into:
    - `WarpFrame` snapshots/diffs,
    - `Notification` values,
    delivered over `std::sync::mpsc::Receiver`s.
  - These functions synchronously surface connection errors so UIs can show a
    clear failure message.
- Exposes the `tool` module for hexagonal tool architectures:
  - `tool::SessionPort` trait: abstract interface for draining notifications
    and WARP frames and clearing WARP streams.
  - `tool::ChannelSession`: a simple channel-backed implementation that wraps
    the receivers returned by `connect_channels` / `connect_channels_for`.
- Used by `warp-viewer` and future tools to keep domain logic dependent only on
  a clean `SessionPort`, not on socket or CBOR details.

## Documentation

- See the Tool hexagon pattern and crate map in
  `docs/book/echo/booklet-05-tools.tex` (Echo Editor Tools),
  Section `Echo Tool Hexagon Pattern` (`09-tool-hex-pattern.tex`),
  and the WARP Viewer section (`08-warp-viewer-spec.tex`) for a worked example
  of using `echo-session-client::tool` in a UI.
