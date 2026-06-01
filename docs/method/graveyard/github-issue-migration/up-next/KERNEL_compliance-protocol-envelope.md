<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Compliance reporting as a TTD protocol extension

`echo-ttd` produces `Violation` records (policy, footprint,
determinism, hashing) via its `PolicyChecker`. These are valuable
debugging information but have no way to reach warp-ttd's UI.

Propose a protocol extension to warp-ttd:

- `ComplianceViolation` envelope (severity, code, message, channel_id,
  tick, rule_id)
- `ComplianceSummary` envelope (violation counts by severity)
- Capability-gated: adapters declare compliance support in `HostHello`

This lets the TUI show violations inline with the timeline — "tick 47
had a footprint violation" alongside "tick 47 had 3 admitted rewrites."

Coordinate with warp-ttd: they own the protocol, we propose the
extension and provide the domain logic.
