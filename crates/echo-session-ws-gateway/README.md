<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# echo-session-ws-gateway

WebSocket ↔ Unix-socket bridge for the Echo session hub. It terminates browser WebSocket connections, enforces JS-ABI frame sizing, and forwards binary frames to the Unix-domain socket exposed by `echo-session-service`.

## Usage

```bash
cargo run -p echo-session-ws-gateway -- \
  --unix-socket /tmp/echo-session.sock \
  --listen 0.0.0.0:8787 \
  --allow-origin https://your.host \
  --tls-cert cert.pem --tls-key key.pem
```

Then open:

- `http://localhost:8787/dashboard` (session dashboard)
- `http://localhost:8787/api/metrics` (JSON metrics)
- `ws://localhost:8787/ws` (WebSocket endpoint)

## Features

- Binary WS frames → JS-ABI packets over UDS
- Payload guard (8 MiB default)
- Built-in hub observer for `/dashboard` metrics (disable with `--no-observer`; configure with `--observe-warp`)
- Optional Origin allowlist
- Optional TLS (rustls)
- Ping/pong keepalive

## Origin allowlist (strict)

If you pass one or more `--allow-origin` values, the gateway switches into a **strict Origin allowlist** mode for the WebSocket endpoint (`/ws`):

- If the request includes an `Origin` header, it must match one of the configured values.
- If the request does **not** include an `Origin` header, the connection is rejected (403).

This is intentionally strict: opting into `--allow-origin` means “require an explicit, allowlisted Origin”.

### Examples

Allow only the embedded dashboard origin:

```bash
cargo run -p echo-session-ws-gateway -- \
  --unix-socket /tmp/echo-session.sock \
  --listen 127.0.0.1:8787 \
  --allow-origin http://127.0.0.1:8787
```

Non-browser WebSocket clients must set `Origin` explicitly. For example, with `websocat`:

```bash
websocat -H='Origin: http://127.0.0.1:8787' ws://127.0.0.1:8787/ws
```
