<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# First-class invariant documents

bijou has `docs/invariants/` with named invariants that legends and
design docs link to ("Layout Owns Interaction Geometry", "Commands
Change State, Effects Do Not"). Echo's invariants are scattered
across ADR prose and spec docs.

Extract them as standalone files so design docs can reference them
directly: "No Global State", "Two-Plane Law", "Canonical Merge
Equals Serial", "Deterministic Float Canonicalization".
