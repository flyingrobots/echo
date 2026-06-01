<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Cross-repo METHOD dashboard

Status: active cool idea. Echo has `cargo xtask method status --json` and the
`method` crate reports backlog lanes, active cycles, and legend load. Local
sibling repos currently include `echo`, `warp-ttd`, `bijou`, and `method`;
`git-warp` is referenced as part of the Continuum constellation but is not
present in this checkout. No cross-repo aggregation tool exists yet.

Run `method status --json` across configured repos (for example Echo,
`warp-ttd`, `bijou`, `method`, and `git-warp` when present) and aggregate into
a single view. See legend load and active cycles across the entire project
constellation.

Remaining work:

- Define the repo discovery contract instead of hard-coding local paths.
- Run each repo's Method status command or compatible reader.
- Normalize missing repos and repos without Method support as explicit states.
- Emit a machine-readable aggregate before adding a dashboard UI.
