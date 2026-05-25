<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# WAL/WSC Storage Relationship

Status: v0.1.0 release doctrine blocker.

Depends on:

- [Causal WAL end-to-end design](../../../design/causal-wal-end-to-end.md)
- [WSC causal-history storage](./PLATFORM_wsc-causal-history-storage.md)
- [Retained evidence durability boundary](./PLATFORM_retained-evidence-durability-boundary.md)

## Why now

Echo now has a serious causal WAL doctrine and the jedit release gate needs
recoverable editing history. The WARP graph can represent graph-shaped evidence
about WAL segments, but recovery must not become circular.

The unsafe model is:

```text
graph facts tell Echo where the WAL is
WAL replay tells Echo how to rebuild graph facts
```

That makes recovery depend on the thing being recovered.

The release-bar model is:

```text
WAL bytes are the durable commit authority.
WARP graph facts track WAL segment evidence.
WSC serializes graph facts and may bundle or reference WAL bytes.
```

## Required doctrine

Echo's WAL is the authority for committed causal history. Echo may project WAL
roots, writer epochs, segment references, commit anchors, recovery
certificates, and storage locators into the WARP graph as evidence-bearing
facts. These graph facts make WAL-backed history inspectable, portable,
queryable, and serializable to WSC, but they do not replace the WAL commit
boundary or bootstrap recovery.

A WAL filepath is a storage locator, not causal identity. Causal identity comes
from writer epoch, LSN range, commit digest chain, segment digest, and
validated commit anchors.

## Acceptance criteria

- [ ] Echo documents that WAL segments are the primary durable commit authority.
- [ ] Echo documents that WARP graph WAL nodes are projected evidence facts,
      not bootstrap recovery authority.
- [ ] Echo defines the minimum WAL evidence projected into the graph:
      WAL root, writer epoch, segment reference, segment digest, LSN range,
      commit anchors, and recovery certificate references where applicable.
- [ ] Echo defines WSC export modes for WAL evidence:
      ref-only export, self-contained export, and CAS-addressed export.
- [ ] Echo documents that storage locators are not causal identity.
- [ ] Echo defines recovery bootstrap without requiring pre-existing graph WAL
      nodes.
- [ ] Echo keeps record naming clear: records are recorded, transactions are
      committed, segments are sealed.

## Test plan

- Add a design/doc check or lint fixture proving the WAL/WSC doctrine is linked
  from BEARING, WorkItems, and the WAL design packet.
- Add a future recovery fixture proving recovery can start from a configured
  WAL root or manifest without requiring pre-existing graph WAL nodes.
- Add a future export fixture proving graph-projected WAL segment refs can be
  serialized without treating raw paths as causal identity.

## Non-goals

- Do not store the WAL inside the WARP graph as the primary recovery mechanism.
- Do not make absolute host paths part of causal identity.
- Do not require WSC to embed every WAL byte in every export.
- Do not let graph projection append WAL records or validate recovery.
