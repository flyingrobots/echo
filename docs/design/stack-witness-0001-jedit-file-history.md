<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Stack Witness 0001 - jedit File History Walking Skeleton

Status: RED witness spec  
Scope: first jedit-through-Echo executable story.

This witness is the first serious stack slice. It exists to prevent the stack
from drifting into schema-first protocol architecture before one local
contract-hosted file history works.

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
- canonical vars bytes;
- declared footprints;
- mutation handler;
- query handler;
- artifact identity.

## Operation Surface

### `createBuffer`

Creates the file-like contract object.

Minimum variables:

- `name = "demo.txt"`;
- contract artifact id;
- operation id;
- canonical vars bytes.

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

## Current GREEN State

As of branch `wip/stack-witness-0001`, Echo has a fixture-backed walking
skeleton for this witness:

- a static/as-if-generated fixture registry exists for `createBuffer` and
  `replaceRange`;
- unknown Stack Witness contract op ids obstruct with a contract/missing
  artifact error;
- `createBuffer` and `replaceRange("hello")` enter through `dispatch_intent`
  and the existing scheduler path;
- the Stack Witness `textWindow` QueryView routes to a fixture observer;
- the fixture observer returns `ReadingEnvelope + QueryBytes("hello")`;
- Echo mirrors Wesley's Stack Witness 0001 fixture vector and verifies op ids,
  helper entrypoints, buffer-inclusive canonical vars, and expected query bytes
  against that artifact shape;
- Echo core still does not expose public jedit, editor, rope, buffer, cursor,
  or selection APIs.

Targeted command:

```sh
cargo test -p warp-wasm --features engine stack_witness_
```

Current result:

```text
running 4 tests
test warp_kernel::tests::stack_witness_fixture_registry_names_mutations ... ok
test warp_kernel::tests::stack_witness_contract_intent_without_installed_artifact_obstructs ... ok
test warp_kernel::tests::stack_witness_create_buffer_and_replace_range_enter_dispatch_intent ... ok
test warp_kernel::tests::stack_witness_text_window_query_returns_reading_envelope_and_query_bytes ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 44 filtered out
```

This is intentionally fixture scaffolding. Do not generalize Echo further from
this state. The next stack move is for Wesley to replace the cardboard cutout
with a generated fixture artifact shape:

- operation ids;
- canonical vars;
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

- Wesley produces generated helpers, canonical vars, operation ids, footprint
  hashes, and fixture vectors.
- Echo admits intents, retains provenance, observes QueryView, and emits the
  reading artifact.
- jedit consumes generated helpers and renders the bounded payload.
- warp-ttd explains the receipt, basis, law identity, read identity, and payload
  digest.
- Continuum later publishes the proven boundary as shared protocol family.

Continuum should codify this seam after the local proof exists. It should not
invent the seam before Echo and jedit prove it.
