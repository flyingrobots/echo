<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Contract Retention And Streaming Seams

Status: planned platform implementation.

Depends on:

- [Contract artifact retention in echo-cas](./PLATFORM_contract-artifact-retention-in-echo-cas.md)
- [Contract inverse admission hook](./KERNEL_contract-inverse-admission-hook.md)
- [0018 - Contract-Hosted File History Substrate](../../../design/0018-contract-hosted-file-history-substrate/design.md)

## Why now

Large files and inverse history both require retained fragments. Full text
materialization is allowed as a cache, never as canonical truth.

## What it should look like

Keep `echo-cas` content-only, but add semantic contract refs above CAS:

- contract family;
- schema/type/layout identity;
- codec and hash algorithm;
- blob or fragment role;
- basis coordinate;
- retention tier.

Add a streaming/blob reader seam if `BlobStore::get Arc<[u8]>` cannot support
bounded observers without full materialization.

## Acceptance criteria

- Contract payloads can refer to retained blob fragments by CAS ref.
- Text-window queries can read visible lines or byte ranges under a budget.
- TickReceipt inverse blob or inverse fragment digest can resolve through
  retention.
- If retention is unavailable, unapply returns typed obstruction.
- GC or compaction cannot silently produce false successful inverse edits.
- Wormhole/checkpoint compression either preserves raw receipts, provides
  rehydratable cold archive, or obstructs fine-grained unapply inside compressed
  ranges.

## Non-goals

- Do not redefine wormholes as rope chunks, portals, or state zoom.
- Do not make CAS content hashes stand in for semantic reading identity.
- Do not build a full distributed storage protocol in this slice.
