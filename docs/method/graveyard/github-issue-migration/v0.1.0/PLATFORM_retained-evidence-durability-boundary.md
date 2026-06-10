<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Retained Evidence Durability Boundary

Status: v0.1.0 release blocker.

Depends on:

- [Contract retention and semantic lookup seams](./PLATFORM_contract-retention-and-semantic-lookup-seams.md)
- [WAL/WSC storage relationship](./PLATFORM_wal-wsc-storage-relationship.md)
- [Contract-aware receipts and readings](./KERNEL_contract-aware-receipts-and-readings.md)

## Why now

Echo can expose retained-evidence posture, but posture is not the same claim as
durable recovery. The release proof needs explicit language and tests that
distinguish "the runtime knows this reading was missing" from "the runtime can
recover this material after restart."

## Required behavior

Echo must distinguish:

- retained evidence posture: present, missing, corrupt, redacted, or obstructed
  from the current read model;
- durable recovery evidence: WAL/WSC-backed proof that the material reference,
  semantic coordinate, and commit anchor can survive restart;
- semantic lookup: the product-level coordinate used to find a retained
  reading, receipt, or artifact;
- byte identity: the content digest of retained bytes.

## Acceptance criteria

- [ ] Echo docs distinguish retained-evidence posture from durable recovery
      evidence.
- [ ] App-safe readings do not imply durable payload recovery unless backed by
      WAL/WSC evidence.
- [ ] Missing retained material returns typed obstruction or missing-retention
      posture, not empty success.
- [ ] A retained reading ref is not treated as a query identity.
- [ ] A query identity does not imply payload retention.
- [ ] Cache hits are not evidence unless the semantic coordinate and material
      digest match.

## Test plan

- Add a future retained-reading fixture proving missing payload posture is not
  reported as a successful empty read.
- Add a future restart fixture proving only WAL/WSC-backed retained refs are
  advertised as durable.
- Add a future semantic-coordinate fixture proving byte identity and semantic
  lookup identity stay separate.

## Non-goals

- Do not require full object-store retention for every local development path.
- Do not implement streaming subscriptions.
- Do not collapse query identity, retained reading identity, and payload digest
  into one noun.
