<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# WARP View Protocol Demo (Hub + 2 Viewers)

This guide walks through a minimal, local demo of the **WARP View Protocol (WVP)**:

- one **session hub** (`echo-session-service`) acting as the pub/sub relay
- one **publisher viewer** (`warp-viewer`) publishing `warp_stream` frames
- one **subscriber viewer** (`warp-viewer`) receiving and applying those frames

The goal is to prove the end-to-end “snapshot + gapless diffs” contract works in practice.

For the protocol details, see `docs/spec-warp-view-protocol.md`.

## What You’re Exercising

At a high level:

- The publisher sends `warp_stream` frames:
  - one `Snapshot(epoch = e)`
  - then a sequence of `Diff(from = e, to = e + 1)` frames (gapless)
- The hub enforces:
  - **single-producer authority** per `warp_id`
  - **snapshot required** before the first diff
  - **gapless diffs** (no epoch gaps)
- Subscribers receive:
  - the latest snapshot on subscribe (if known)
  - then diffs as they arrive

In `warp-viewer`, the “publisher” demo mutation updates the *first node’s* payload with a CBOR map:

```text
{ pos: [x,y,z], color: [r,g,b] }
```

The viewer already understands this payload shape and uses it to position/color nodes for rendering.

## Prerequisites

- Rust toolchain installed (workspace toolchain).
- You can build and run workspace binaries:
  - `cargo run -p echo-session-service`
  - `cargo run -p warp-viewer`

## Step 1: Start the Session Hub

In terminal A:

```bash
cargo run -p echo-session-service
```

By default the hub listens on a local Unix socket:

- `$XDG_RUNTIME_DIR/echo-session.sock`, or
- `/tmp/echo-session.sock` (fallback)

## Step 2: Start the Publisher Viewer

In terminal B:

```bash
cargo run -p warp-viewer
```

In the viewer UI:

1. Click `Connect`
2. Click `Connect` again (defaults to the local socket; WARP id defaults to `1`)
3. Open `Menu` → `Publish Local WARP`
4. Check `Enable publishing (I am the producer)`
5. Click `Pulse mutation` a few times

Expected behavior:

- The hub will accept the first publish on `warp_id = 1` and treat this viewer as the producer.
- Each pulse generates a deterministic `UpdateNode` op and publishes a gapless diff.

## Step 3: Start the Subscriber Viewer

In terminal C:

```bash
cargo run -p warp-viewer
```

In the viewer UI:

1. Click `Connect` → `Connect`
2. Open `Menu` → `Subscribe to WARP`
3. Ensure `Apply incoming WARP frames` is enabled
4. Confirm the `WARP id` matches the publisher (`1` by default)

Expected behavior:

- On subscribe, the hub sends the latest snapshot for `warp_id = 1` (if any exists).
- As the publisher pulses, the subscriber’s rendered graph updates.

## Common Failure Modes (And What They Mean)

### `E_FORBIDDEN_PUBLISH` (403)

You tried to publish on a `warp_id` already owned by another connection.

Fix:

- Use a different `warp_id`, or
- Restart the hub, or
- Close the existing producer connection.

### `E_WARP_SNAPSHOT_REQUIRED` (409)

A diff was sent before any snapshot was accepted for that stream.

Fix:

- In the publisher, click `Publish snapshot now`.

### `E_WARP_EPOCH_GAP` (409)

The publisher tried to send a diff that wasn’t `last_epoch -> last_epoch + 1`.

Fix:

- Restart the hub and retry, or
- Force a snapshot publish to reset server state to your current epoch.

## Current Limitations (v0)

- The viewer connection assumes “one WARP stream per socket”; changing `warp_id` requires reconnect.
- There is no explicit resync request message yet; resync is done by re-subscribing (hub sends latest snapshot) or reconnecting.

