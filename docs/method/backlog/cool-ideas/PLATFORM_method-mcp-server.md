<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# METHOD as an MCP server

Expose `method status`, `method inbox`, and `method pull` as MCP
tools so agents can interact with the backlog without shelling out
to `cargo xtask`. The `StatusReport` already derives `Serialize` —
the data layer is ready.
