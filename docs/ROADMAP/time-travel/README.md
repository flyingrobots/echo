<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Time Travel

Priority: P2  
Status: Planned  
Blocked By: Time Semantics Lock

Objective: implement the time-travel stack (inspector visibility, core operations, worldline comparison).

## Features

- [F7.2 TT1 Streams Inspector Frame](./F7.2-streams-inspector-frame.md) (Repo: Echo)
- [F7.3 TT2 Time Travel MVP](./F7.3-time-travel-mvp.md) (Repo: Echo)
- [F7.4 TT3 Rulial Diff](./F7.4-rulial-diff.md) (Repo: Echo)

## Exit Criteria

- Pause/rewind/fork/catch-up operations are deterministic and capability-gated.
- Streams and divergence frames are emitted and consumable by tooling.
- Worldline diff outputs are validated by integration tests.
