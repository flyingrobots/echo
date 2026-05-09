<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Optics Adapter Notes

This note describes where future consumer adapters sit relative to the Echo
Optics API.

The boundary rule is:

```text
Optic observes.
Admission admits.
Retention retains.
Plumber maintains.
Debug explains.
```

Adapters may make the API ergonomic for a product, protocol, or tool. They must
not turn Echo into a global graph API, a mutable state API, a file handle API, a
sync daemon, or a host-bag abstraction.

## Adapter Shape

An adapter may:

- open a typed optic descriptor for a consumer workflow;
- choose an aperture and budget for a read;
- call `observe_optic`;
- decode observer-relative reading payload bytes into consumer-owned types;
- construct an explicit-base `dispatch_optic_intent` request;
- stage or submit generated EINT bytes;
- retain or reveal reading bytes by `ReadIdentity`.

An adapter must not:

- mutate Echo state by holding a handle;
- call direct service mutation paths as the public write model;
- replace typed admission outcomes with booleans or string statuses;
- silently retry against the latest frontier when a base coordinate is stale;
- satisfy reads by falling back to full materialization;
- hide missing witness, rights, budget, or attachment evidence;
- treat CAS content hashes as semantic reading identity.

## Layering

The intended layering is narrow:

```text
Consumer UI / tool
  -> consumer adapter
  -> generated or handwritten optic request builder
  -> Echo Optics API
  -> observation / admission / retention services
  -> witnessed causal history and receipts
```

Consumer adapters own workflow vocabulary. Echo owns causal coordinates,
capability checks, bounded observation, admission outcomes, receipts, witness
refs, and retained reading identity.

## GraphQL And Wesley

GraphQL is an authoring or adapter illustration, not the Echo runtime
substrate.

Wesley-generated code may expose GraphQL-shaped helper names because those names
belong to the authored contract. Generated helpers should still lower into Echo
as generic Optics requests:

- generated query helpers build `ObserveOpticRequest`;
- generated mutation helpers build EINT v1 payloads and
  `DispatchOpticIntentRequest`;
- generated decoding helpers decode observer payload bytes after Echo has
  emitted a reading.

The helper may hide byte packing from application code. It must not hide the
fact that an intent was proposed against an explicit causal basis and then
admitted, staged, obstructed, pluralized, or conflicted by Echo.

## Consumer Notes

Editors may use optics for bounded visible-window readings and explicit-base
edit proposals. `jedit` is a useful ergonomic example because it stresses
bounded text reads, stale-basis handling, attachment boundaries, retained
fragments, and undo-as-inverse-intent. It is not an Echo core ontology. Echo
must not gain privileged jedit, editor, rope, buffer, or file setter APIs.

Debuggers may use optics to inspect coordinates, frontiers, receipts, witness
sets, and replay slices. A debugger adapter may explain why a reading is
obstructed or budget-limited, but it must not bypass `observe_optic` with a
private materializer to make the UI look complete.

Inspectors may use optics to reveal structural metadata, head identity,
attachment refs, retained-reading refs, and obstruction posture. Inspector
adapters should prefer small apertures and explicit recursive descent.

Replay tools may use optics to read checkpoint-plus-tail identities, compare
frontiers, and build bounded reveal requests. A replay adapter must not present
a checkpoint hash as the live result unless the read identity honestly names
the live tail witness basis.

Import/export flows may combine optics with witnessed suffix export/import, but
the adapter remains a coordinator. Import is still admission. Export is still a
read of witnessed causal material. Neither path should become a sync daemon or
latest-writer-wins merge policy.

Retained reading caches may store payload bytes for a reading, but the cache key
must include semantic `ReadIdentity`, codec identity, byte length, and content
hash. The CAS hash names bytes. The read identity names the question those
bytes answer.

## Deterministic Boundary

Adapters may use convenient host-language DTOs internally, including serde on
non-authoritative diagnostic or bridge shapes. Anything that affects intents,
graph-preserved facts, causal history, receipts, witness material, read
identity, retained-reading identity, or admission posture must cross into Echo
as canonical deterministic bytes.

Boundary code must normalize nondeterministic value shapes before admission or
retention. In particular, floats and other host-sensitive representations must
be canonicalized before they can affect hashes, receipts, witnesses, or graph
history.

## Rejected Shapes

These names and shapes are intentionally rejected:

- `RuntimeFacade`;
- `ObservationManager`;
- `UniversalMaterializer`;
- `GraphLikeRuntimeAdapter`;
- global `getGraph` / `setGraph` APIs;
- mutable file or buffer handles;
- hidden materialization caches;
- GraphQL-first runtime dispatch;
- direct host-time ordering as admission law;
- adapter-owned causal history.

If a future consumer needs one of those shapes for local ergonomics, it must
remain outside Echo and prove that the Echo-facing calls still go through
bounded observation, explicit-base intent dispatch, typed admission, and
retained witness identity.
