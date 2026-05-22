<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Contract Retention And Semantic Lookup Seams

Status: v0.1.0 release blocker.

Depends on:

- [Contract artifact retention in echo-cas](./PLATFORM_contract-artifact-retention-in-echo-cas.md)
- [Contract reading identity and bounded payloads](./KERNEL_contract-reading-identity-and-bounded-payloads.md)
- [0018 - Contract-Hosted File History Substrate](../../../design/0018-contract-hosted-file-history-substrate/design.md)

## Why now

Large retained artifacts and bounded readings need semantic lookup above raw
CAS identity. Full materialization is allowed as a cache, never as canonical
truth.

## What it should look like

Keep `echo-cas` content-only, but add semantic contract refs above CAS:

- contract family;
- schema/type/layout identity;
- codec and hash algorithm;
- blob or fragment role;
- basis coordinate;
- retention tier.

Add a bounded blob reader seam if `BlobStore::get Arc<[u8]>` cannot support
bounded observers without full materialization. This is retained-payload lookup,
not a streaming subscription surface.

## Acceptance criteria

- Contract payloads can refer to retained blob fragments by CAS ref.
- Bounded contract queries can read retained byte ranges under a budget.
- Retained contract artifacts, receipt material, witness refs, reading
  envelopes, and reading payloads can be looked up by semantic coordinates.
- If retention is unavailable, lookup returns typed obstruction.
- GC or compaction cannot silently produce false successful retained readings.
- Cache hits are accepted only when the semantic coordinate matches.

## Non-goals

- Do not redefine wormholes as rope chunks, portals, or state zoom.
- Do not make CAS content hashes stand in for semantic reading identity.
- Do not build a full distributed storage protocol in this slice.
