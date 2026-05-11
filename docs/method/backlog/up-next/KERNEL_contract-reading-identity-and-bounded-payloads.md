<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Contract Reading Identity And Bounded Payloads

Status: planned kernel/runtime implementation.

Depends on:

- [Contract QueryView observer bridge](../asap/PLATFORM_contract-queryview-observer-bridge.md)
- [Contract-aware receipts and readings](./KERNEL_contract-aware-receipts-and-readings.md)
- [0018 - Contract-Hosted File History Substrate](../../../design/0018-contract-hosted-file-history-substrate/design.md)

## Why now

Contract observations need honest identities and bounded payload behavior before
large-file consumers can trust them.

## What it should look like

Reading identity should include:

- observed coordinate;
- contract family, artifact, and schema identity;
- query op id;
- vars digest;
- observer/read law version where available;
- aperture or budget request;
- witness refs;
- budget, rights, residual, plurality, conflict, or obstruction posture.

## Acceptance criteria

- Same query, same basis, same vars, and same observer law produce the same
  reading identity.
- Schema, op id, vars, basis, or aperture changes produce different identity.
- A bounded text-window fixture returns only the requested aperture.
- Payload size is bounded by the request or posture reports budget limitation.
- Unsupported or stale basis returns obstruction.

## Non-goals

- Do not canonicalize text-editor state in Echo core.
- Do not require full payload materialization before identity can be computed.
- Do not use CAS content hash alone as reading identity.
