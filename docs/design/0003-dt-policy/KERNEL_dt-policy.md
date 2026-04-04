<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# dt policy: fixed timestep vs admitted dt stream

Ref: #243

Decide whether Echo's simulation loop uses a fixed timestep (every
tick is the same duration) or an admitted dt stream (ticks carry
variable time deltas as stream facts).

Fixed timestep is simpler and more deterministic. Variable dt is
more flexible for real-time applications but introduces a new class
of divergence (two clients with different dt streams produce
different states).

This is a fundamental time model decision that gates TT1 work.
