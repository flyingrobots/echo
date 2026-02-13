<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Deep Storage

> **Priority:** P2 | **Status:** Planned | **Est:** ~45h

echo-cas beyond MemoryTier. DiskTier, GC sweep, wire protocol, and API evolution.

**Blocked By:** First Light

## Exit Criteria

- [ ] DiskTier read/write passing
- [ ] GC sweep evicts cold blobs without data loss
- [ ] Wire protocol enables remote CAS operations
- [ ] API backward-compatible with MemoryTier consumers

## Features

| Feature             | File                                         | Est. | Status      |
| ------------------- | -------------------------------------------- | ---- | ----------- |
| DiskTier            | [disk-tier.md](disk-tier.md)                 | ~11h | Not Started |
| GC Sweep & Eviction | [gc-sweep-eviction.md](gc-sweep-eviction.md) | ~11h | Not Started |
| Wire Protocol       | [wire-protocol.md](wire-protocol.md)         | ~11h | Not Started |
| API Evolution       | [api-evolution.md](api-evolution.md)         | ~13h | Not Started |
