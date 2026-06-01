<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Explicit negative test mapping for decoder controls

Ref: #279

Map every decoder control (CBOR boundary validation, envelope
rejection, malformed input handling) to an explicit negative test.

Status: active and partially implemented. `docs/determinism/sec-claim-map.json`
maps SEC-001 through SEC-005 to named `echo-scene-codec` negative tests, and the
G2 workflow verifies those test IDs exist. The remaining gap is exhaustiveness:
CI proves mapped tests exist, but it does not prove every decoder rejection
branch or control class is represented in the map.

## T-279-1: Make decoder control coverage auditable

**User Story:** As a security reviewer, I want each decoder rejection control to
point at an explicit negative test so that malformed-input coverage can be
audited without reading the whole decoder by hand.

**Requirements:**

- R1: Inventory the active decoder controls in
  `crates/echo-scene-codec/src/cbor.rs`, including max-count checks, trailing
  bytes, truncation handling, version rejection, invalid enum tags, definite
  array lengths, hash-length checks, and option-shape checks.
- R2: Extend `docs/determinism/sec-claim-map.json` or a sibling control map so
  every inventory row points at at least one concrete negative test.
- R3: Add missing negative tests before claiming a control is covered.
- R4: Keep the existing G2 test-ID existence check and add an exhaustiveness
  check over the maintained inventory if the inventory becomes machine-readable.

**Acceptance Criteria:**

- [x] AC1: SEC-001 through SEC-005 have named negative tests in
      `echo-scene-codec`.
- [x] AC2: CI verifies every mapped test ID still exists.
- [ ] AC3: Every active decoder rejection control is listed in an auditable
      inventory.
- [ ] AC4: Every inventory row maps to a concrete negative test or carries an
      explicit unresolved status.
