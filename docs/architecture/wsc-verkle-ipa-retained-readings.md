<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# WSC, Verkle, IPA, And Retained Readings

Status: future direction, architecture doctrine.

Echo's retained-reading direction is:

```text
WSC   = canonical columnar bytes for a reading or checkpoint
Verkle = authenticated commitment/index over those bytes
IPA   = compact proof mechanism for opening bounded apertures
echo-cas = content-addressed byte retention
```

Short version:

```text
WSC gives us the table.
Verkle gives us the root.
IPA gives us the aperture proof.
echo-cas stores the bytes.
```

This direction does not replace witnessed causal history, `ReadIdentity`, or
Echo's WARP optic doctrine. It names the future storage/proof stack for
retained holograms and checkpoint-style readings.

## Layer Roles

### WSC

WSC means **Write-Streaming Columnar**.

WSC is the canonical physical layout for WARP-shaped readings:

- header
- WARP directory
- node rows
- edge rows
- outbound edge indexes
- attachment indexes
- attachment rows
- blob bytes

It is deterministic, columnar, aligned, and designed for low-copy or future
memory-mapped reads. WSC is a byte layout, not semantic truth.

### Verkle

Verkle is the future authenticated commitment/index layer over WSC sections,
rows, and chunks.

WSC naturally supplies stable coordinates such as:

```text
/wsc/v1/warp/0/nodes/123
/wsc/v1/warp/0/edges/456
/wsc/v1/warp/0/node_atts/789
/wsc/v1/warp/0/blobs/chunk/42
```

A Verkle root can commit to the WSC-backed reading while allowing compact
openings of selected cells, rows, chunks, or aperture-specific bundles.

### IPA

IPA means **Inner Product Argument**.

In this direction, IPA is a proof backend for opening Verkle/vector commitment
claims. It lets an optic carry compact support for a bounded aperture without
materializing the full retained reading.

The exact proof system is future work. The design requirement now is to avoid
closing off the shape:

```text
aperture -> selected WSC coordinates -> opened values + compact proof
```

### echo-cas

`echo-cas` stores opaque bytes by content hash:

```text
BlobHash = BLAKE3(bytes)
```

CAS does not know WSC, Verkle, IPA, jedit, ropes, buffers, or schemas. It stores
bytes. Meaning lives in typed references, reading identities, witnesses, and
optic coordinates above the CAS blob.

## Identity Stack

These identities must stay separate:

```text
CAS hash
  exact byte identity

WSC payload hash
  canonical retained reading/checkpoint bytes

Verkle root
  authenticated commitment to a WSC-backed reading

IPA proof hash
  retained support for selected openings or relations

ReadIdentity
  semantic question answered by the reading
```

The CAS hash says "these bytes." It does not say "this buffer reading." A
`ReadIdentity` names the semantic question and basis those bytes answer.

## jedit And Rope Fit

`jedit` keeps its rope model as the hot editor structure.

The rope is optimized for:

- range replacement
- cursor-relative editing
- line and character metrics
- incremental dirty tracking
- editor ergonomics

WSC is optimized for:

- retained readings
- checkpoints
- deterministic bytes
- low-copy inspection
- CAS retention
- future proof openings

So the boundary is:

```text
jedit rope
  hot app-owned text structure

WSC
  cold canonical retained reading/checkpoint layout

echo-cas
  byte store for WSC, proof, witness, receipt, and payload blobs
```

Echo must not learn rope semantics. The external `jedit` repo owns text law,
buffer law, edit-group law, and rope reconstruction. Echo hosts generated
contract artifacts, admits intents, emits readings, and retains bytes through
generic surfaces.

## jedit Checkpoint Shape

A future jedit checkpoint may look like:

```text
jedit rope
  -> WSC reading:
       nodes: Buffer, RopeRoot, RopeInternal, RopeLeaf, EditGroup, Checkpoint
       edges: parent/child/order/edit/checkpoint relationships
       attachments: weights, line counts, encoding, newline policy, chunk refs
       blobs: text chunk bytes
  -> Verkle root over WSC sections and chunks
  -> IPA proof for a requested aperture
  -> echo-cas retention for WSC/proof/witness bytes
```

For a buffer range read, the response should not need the whole buffer:

```text
request:
  buffer B, range [a..b], checkpoint C

response:
  opened text chunks
  relevant rope leaf metadata
  Verkle root for checkpoint C
  IPA or opening proof ref
  reading envelope naming contract, schema, coordinate, and aperture
```

The editor still needs the text bytes it renders. It should not need the full
WSC, full rope, full Echo graph, or unrelated sibling chunks.

## Materialization Levels

This stack supports graded materialization:

```text
full materialization:
  CAS -> full WSC -> full jedit rope -> full buffer

partial materialization:
  CAS -> selected WSC chunks -> selected rope leaves -> visible viewport

proof-backed aperture:
  opened values + proof + commitment root -> verified bounded reading
```

Graft, warp-ttd, and other WARP optics can consume the same retained evidence at
their own aperture without pretending there is one canonical in-memory graph.

## Current Reality

Current implemented facts:

- `warp-core` has WSC writing, validation, and borrowed view support.
- `echo-cas` stores opaque bytes by content hash.
- contract/read retention cards already require CAS hashes to stay separate
  from semantic reading identity.

Future work:

- multi-warp WSC completion and retention integration
- retained-reading keys that can name WSC payloads honestly
- Verkle or equivalent authenticated indexes over WSC coordinates
- IPA or equivalent compact opening proofs for proof-carrying apertures
- jedit-owned projection from rope checkpoints into WSC-backed readings

## Non-Goals

- Do not make WSC the ontology.
- Do not make Verkle the ontology.
- Do not make IPA a storage substrate.
- Do not make `echo-cas` depend on WSC or proof systems.
- Do not make CAS hashes stand in for `ReadIdentity`.
- Do not add rope, buffer, or text APIs to Echo core.
- Do not require proof systems for the first jedit contract-hosting proof.
- Do not treat proof verification as admission without authority, policy, and
  support-obligation checks.

## Implementation Consequence

Near-term Echo work should preserve slots for:

- payload layout identifiers such as `wsc-v1`
- payload refs stored in `echo-cas`
- commitment family and commitment root
- proof family and proof ref
- opened WSC coordinates or aperture selectors
- verification posture
- residual or obstruction posture when support is unavailable

The first implementation does not need Verkle or IPA. It needs the retained
reading identity to leave room for them.
