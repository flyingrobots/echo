<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# xtask method drift

Status: complete. `cargo xtask method drift [cycle]` checks playback questions
from active cycle design docs against committed test files, supports JSON output
for agents, reports matched and missing questions, and exits nonzero when any
playback question lacks visible test coverage.

Implement `cargo xtask method drift [cycle]` — check active cycle
playback questions against committed test descriptions.

## Acceptance

- [x] Parses playback questions from the design doc.
- [x] Searches test files for matching test names or descriptions.
- [x] Reports coverage: which questions have tests, which don't.
- [x] Exit code 1 if any playback question has no matching test.
