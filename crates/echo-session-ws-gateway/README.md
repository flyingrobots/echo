<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# echo-session-ws-gateway

WebSocket ↔ Unix-socket bridge for the Echo session hub. It terminates browser WebSocket connections, enforces JS-ABI frame sizing, and forwards binary frames to the Unix-domain socket exposed by `echo-session-service`.

## Usage

```
cargo run -p echo-session-ws-gateway -- \
  --unix-socket /tmp/echo-session.sock \
  --listen 0.0.0.0:8787 \
  --allow-origin https://your.host \
  --tls-cert cert.pem --tls-key key.pem
```

## Features
- Binary WS frames → JS-ABI packets over UDS
- Payload guard (8 MiB default)
- Optional origin allowlist
- Optional TLS (rustls)
- Ping/pong keepalive
