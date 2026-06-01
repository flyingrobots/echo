<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Schema-version fingerprint prefix on every encoded payload

Status: cool idea.

Depends on:

- [[0024 — Universal LE Binary Codec]](../../../design/0024-universal-le-binary-codec/design.md)

## The idea

Every framed message that crosses an Echo boundary (WASM call, WSC
record, network frame) carries a 32-byte prefix: the `SCHEMA_SHA256`
that Wesley computed at compile time for the schema the sender was
generated from.

```text
[32-byte SCHEMA_SHA256] [op_id u32 LE] [vars_len u32 LE] [vars...]
```

Receivers verify the prefix before reading the payload. Mismatch →
hard rejection with an explicit "schema version mismatch" error code.
Not silent corruption.

## Why it matters

The standard objection to binary formats is that they're not
self-describing — version mismatch causes silent data corruption.
0024's answer is "Wesley regenerates both sides, so they can't drift."
That's true for code that ships together; it breaks for:

- WSC records read by a newer codec than wrote them
- Network protocol where client and server are deployed
  asynchronously
- Stored intents replayed after a contract upgrade

A 32-byte prefix is a cheap, universal version gate. It promotes
silent corruption to a typed runtime error.

## What it replaces

Today `warp-wasm` reports `codec_id = "cbor-canonical-v1"` (and would
report `"le-binary-v1"` once Phase 3b cleanup lands). That captures
the FORMAT version but not the SCHEMA version. A protocol whose
`codec_id` matches but whose schema fields shifted will still
deserialize to silent garbage.

## Implementation cost

- 32 extra bytes per message — non-trivial for high-frequency RPC,
  trivial for intent envelopes
- One additional Wesley const emit (already produced — `SCHEMA_SHA256`)
- One additional verify step at every boundary

## Open question

Does the prefix go INSIDE the EINT envelope (between magic and op_id)
or OUTSIDE? Outside makes routing trivial; inside makes it harder to
forge a forward-compatible envelope. Probably outside, with EINT magic
right after the schema-hash prefix.
