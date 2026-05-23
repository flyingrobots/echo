<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Stack Witness 0001 - jedit File History Walking Skeleton

Status: HISTORICAL walking-skeleton packet, superseded for release-grade
generic Echo behavior.
Scope: early jedit-through-Echo fixture story.

This witness was the first serious stack slice. It prevented the stack from
drifting into schema-first protocol architecture before one local
contract-hosted file history could be described.

Important correction: Echo no longer carries this Stack Witness fixture inside
the production WASM kernel. Release-grade Echo must route jedit-shaped
mutations and QueryView reads through installed contract packages and generated
host adapters. A hardcoded `createBuffer`/`replaceRange`/`textWindow` shortcut
inside Echo is an architectural violation, not a proof.

## Claim

A jedit-like contract can create a file-like object, edit it through Echo, and
read a bounded text window through Echo without direct runtime mutation or raw
state reads.

The first story is intentionally small:

```text
createBuffer(name = "demo.txt")
replaceRange(buffer, basis = B0, start = 0, end = 0, text = "hello")
textWindow(buffer, basis = B1, start = 0, length = 5)
=> "hello"
```

## Required Path

The story must run through the same boundary shape jedit will use:

```text
fixture-generated or Wesley-generated EINT
-> Echo dispatch_intent
-> Echo admission/provenance
-> Echo observe(QueryView)
-> ReadingEnvelope + QueryBytes
-> jedit consumes result
```

The first implementation may use an as-if generated fixture artifact before
Wesley emits the final artifact. The fixture must have the same shape Wesley is
expected to generate:

- manifest or artifact id;
- operation ids;
- fixture vars bytes;
- target codec metadata;
- declared footprints;
- mutation handler;
- query handler;
- artifact identity.

## Fixture Encoding vs Target Codec

Stack Witness 0001 currently uses `fixtureVarsEncoding:
utf8-semicolon-kv/v0` and `fixtureVarsBytes` only as temporary,
human-readable fixture metadata. These semicolon-kv strings are not Wesley's
runtime codec and should not fossilize as the architecture's canonical variable
representation.

The durable target is `targetCodec: wesley-binary/v0`: Wesley-generated
deterministic binary codecs shared by Rust and TypeScript. Echo consumes bytes
plus artifact identity and should not care whether those bytes came from Rust,
TypeScript, WASM, CLI, or network transport.

## Operation Surface

### `createBuffer`

Creates the file-like contract object.

Minimum variables:

- `name = "demo.txt"`;
- contract artifact id;
- operation id;
- fixture vars bytes for this witness, later Wesley-generated binary codec
  bytes.

### `replaceRange`

Appends a range edit as witnessed history.

Minimum variables:

- target object id;
- basis ref;
- observed reading id or aperture id when available;
- range coordinate system;
- `start = 0`;
- `end = 0`;
- replacement bytes/text = `"hello"`;
- contract artifact id.

The first coordinate system is UTF-8 byte offsets. Richer editor coordinates
are later work:

- Unicode scalar offsets;
- grapheme clusters;
- line and column;
- rope coordinates;
- syntax-aware spans.

An edit that does not name what it was based on is not admissible for this
witness.

### `textWindow`

Reads a bounded aperture.

Minimum variables:

- target object id;
- basis ref;
- range coordinate system;
- `start = 0`;
- `length = 5`;
- contract artifact id.

Expected payload bytes:

```text
hello
```

## ReadingEnvelope Requirements

The QueryView result must not return naked bytes. It must return
`ReadingEnvelope + QueryBytes`.

Minimum viable envelope fields or slots:

- read identity;
- basis ref;
- observer plan or query id;
- contract artifact id;
- vars digest;
- aperture;
- payload digest;
- payload codec;
- witness refs or witness posture;
- budget posture;
- rights posture;
- residual or obstruction posture.

Some fields may initially be primitive or `not_available`, but the slots must
exist so retention, proof, and debugging do not require later surgery.

## Receipt Requirements

Every admitted contract mutation must leave evidence of:

- contract family id;
- schema or artifact id;
- operation id;
- operation version;
- footprint declaration hash;
- canonical vars digest;
- basis;
- admission outcome;
- receipt id.

## Echo Core Firewall

Echo core may host the fixture contract. Echo core must not become the fixture
contract.

Suspicious names in Echo core:

```text
jedit
rope
TextBuffer
ReplaceRange
TextWindow
Editor
Cursor
Selection
```

Those names are allowed in tests or fixtures only when clearly marked as
fixture vocabulary, for example:

```text
fixture_jedit_contract
test_text_window_contract
```

## Initial RED Witnesses

The first RED tests proved:

1. Contract-shaped EINT must not silently enter Echo as an unauthenticated
   generic inbox event when no generated contract artifact is installed.
2. QueryView/textWindow must return `ReadingEnvelope + QueryBytes`, not
   `UnsupportedQuery` or naked payload bytes.

## Current State

The earlier fixture-backed WASM shortcut has been removed. Current Echo
behavior is:

- `warp-wasm` does not recognize `createBuffer`, `replaceRange`, or
  `textWindow` as kernel-owned operations;
- QueryView observations route through `ObservationService` and installed
  contract query observers only;
- without an installed observer, WASM `observe(...)` returns
  `UNSUPPORTED_QUERY`;
- text, rope, file-history, editor, cursor, and buffer semantics belong in
  jedit-authored contracts, Wesley-generated adapters, or jedit host code;
- Echo core remains a generic deterministic runtime and contract host.

Current targeted command:

```sh
cargo test -p warp-wasm --features engine queryview_without
```

Current witness:

```text
test warp_kernel::tests::queryview_without_installed_contract_observer_is_unsupported ... ok
```

Engine feature compile target:

```sh
cargo check -p warp-wasm --target wasm32-unknown-unknown --features engine
```

Current result:

```text
Finished `dev` profile [unoptimized + debuginfo] target(s)
```

This is intentionally fixture scaffolding. Wesley now publishes the fixture
artifact shape that Echo mirrors, but Echo must still avoid generalizing further
from this state. The next stack move is to replace more of the hand-authored
fixture assumption with generated helpers:

- operation ids;
- fixture vars bytes;
- `targetCodec: wesley-binary/v0`;
- footprints;
- EINT helpers;
- QueryView helper;
- artifact identity.

## Golden Trace Shape

The witness should eventually produce one stack trace:

```text
001 createBuffer request
002 createBuffer receipt
003 replaceRange request
004 replaceRange receipt
005 textWindow request
006 ReadingEnvelope
007 QueryBytes
```

Each repo can consume or produce part of this trace:

- Wesley produces generated helpers, fixture vectors, operation ids, footprint
  hashes, and eventually the canonical binary codecs named by `targetCodec`.
- Echo admits intents, retains provenance, observes QueryView, and emits the
  reading artifact.
- jedit consumes generated helpers and renders the bounded payload.
- warp-ttd explains the receipt, basis, law identity, read identity, and payload
  digest.
- Continuum later publishes the proven boundary as shared protocol family.

Continuum should codify this seam after the local proof exists. It should not
invent the seam before Echo and jedit prove it.
