<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# CAS Storage

Priority: P2  
Status: Planned  
Blocked By: First Light

Objective: evolve echo-cas beyond MemoryTier into persistent, tiered, and protocol-capable storage.

## Features

- [F5.1 DiskTier](./F5.1-disktier.md) (Repo: Echo)
- [F5.2 GC Sweep & Eviction](./F5.2-gc-sweep-eviction.md) (Repo: Echo)
- [F5.3 Wire Protocol](./F5.3-wire-protocol.md) (Repo: Echo)
- [F5.4 API Evolution](./F5.4-api-evolution.md) (Repo: Echo)

## Exit Criteria

- Disk tier + tiered-store behavior is validated by deterministic tests.
- GC + eviction policies are measurable and policy-compliant.
- Wire protocol + exchange state machine pass round-trip and stress tests.
