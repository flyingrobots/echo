<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# dt Policy: Fixed Timestep

Ref: #243

Status: superseded by `docs/invariants/FIXED-TIMESTEP.md`.

Decision: Echo uses fixed deterministic ticks. `dt` is not admitted as a
variable causal fact. Host-observed elapsed time may wake an adapter and cause
that adapter to propose an Intent, but only admitted ticks and receipts affect
replay, rewind, read identity, and causal ordering.

This file remains only as a historical pointer for #243. The normative doctrine
is now the fixed-timestep invariant.
