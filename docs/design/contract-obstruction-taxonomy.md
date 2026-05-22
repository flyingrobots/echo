<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Contract Obstruction Taxonomy

Status: accepted local boundary.

## Purpose

Contract-hosted applications need typed obstruction posture before Echo exposes
more polished product-facing APIs. A caller should not infer application
behavior from generic strings, internal runtime errors, or empty successful
readings.

## Boundary

`warp-core` now exposes:

- `ContractObstructionKind`;
- `ContractObstructionSubject`;
- `ContractObstruction`;
- deterministic classifiers from current observation and runtime errors.

The taxonomy names generic contract-hosting posture only. It does not import
application-domain obstruction types into Echo core.

## Initial Kinds

- `UnsupportedOperation`
- `UnsupportedQuery`
- `AdmissionObstruction`
- `RuntimeFault`
- `MissingRetention`
- `StaleBasis`
- `ResidualReading`
- `BudgetExceeded`

## Authority Rule

Runtime faults remain runtime faults. They do not become lawful domain
rejections. Query residual posture remains read-side posture. It does not imply
application mutation or scheduler progress.

## Non-Goals

- Do not replace all existing error enums in this slice.
- Do not expose scheduler control through product-facing obstruction APIs.
- Do not add application-specific obstruction names to `warp-core`.
- Do not turn missing retention into empty successful reads.
