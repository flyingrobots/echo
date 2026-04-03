<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Security/capabilities for fork/rewind/merge

Ref: #246

Define the capability model for timeline mutation operations. Who is
allowed to fork a worldline? Rewind? Merge? What are the security
contexts (local debug, multiplayer session, untrusted plugin)?

This is a Core Echo concern — the kernel owns the capability checks.
warp-ttd will be the primary consumer of these capabilities through
the `TtdHostAdapter` interface. Coordinate with warp-ttd on how
capabilities are declared and enforced.
