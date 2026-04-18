<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Split echo-session-proto into retained bridge contracts vs legacy transport residue

`echo-session-proto` still mixes two different things:

- retained TTD/browser bridge contracts:
    - `EINT v2`
    - `TTDR v2`
    - deterministic CBOR/frame helpers still used by `ttd-browser`
- older Echo-local session/WVP transport types:
    - `Message`
    - `OpEnvelope`
    - `subscribe_warp`
    - `warp_stream`
    - `notification`

After the old viewer/session-service/gateway stack removal, those are no longer
one coherent product boundary.

This slice should answer:

1. what minimal frame/encoding surface must stay for the Echo browser host
   bridge
2. which legacy WVP/session-transport types can be deleted outright
3. whether the retained TTD/browser framing belongs in a renamed crate

The goal is not “save every old protocol type.” The goal is to stop keeping a
dead session-hub ontology fused to the surviving browser/TTD bridge path.
