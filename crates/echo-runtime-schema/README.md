<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# echo-runtime-schema

Shared ADR-0008 runtime schema primitives for Echo.

This crate is the Echo-local shared owner for runtime-schema types that are not
inherently ABI-only:

- opaque runtime identifiers
- logical monotone counters
- structural runtime key types

`warp-core` consumes or re-exports these semantic types. `echo-wasm-abi`
converts to and from them where the host wire format differs.
