<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Continuum Contract Artifact Interchange

Status: cool idea, future protocol lane.

Depends on:

- [Contract strands and counterfactuals](../up-next/KERNEL_contract-strands-and-counterfactuals.md)
- [Witnessed suffix admission shells](../asap/PLATFORM_witnessed-suffix-admission-shells.md)
- external Continuum protocol publication work

## Why later

Echo should prove local Wesley-compiled contract hosting before protocolizing
contract artifacts across runtimes.

Once local hosting, receipts, readings, CAS retention, and contract strands are
real, Continuum can define how sibling runtimes exchange contract-shaped
causal artifacts.

## What it should look like

Continuum artifacts should be able to name:

- contract family
- schema hash
- intent or observer kind
- basis and frontier
- payload hash
- receipt refs
- witness refs
- reading refs
- admission posture
- retained artifact hashes

Sibling runtimes should exchange protocol-shaped causal artifacts, not runtime
internals.

## Acceptance criteria

- One Echo contract receipt exports as a Continuum-shaped artifact.
- A second runtime or verifier can inspect the artifact family identity without
  understanding Echo internals.
- Import uses witnessed suffix admission law instead of state snapshot sync.
- Missing contract family or schema support yields obstruction, not silent
  downgrade.

## Non-goals

- Do not start before local contract hosting is real.
- Do not require proof-carrying execution.
- Do not design all network transport.
- Do not collapse contract artifacts into generic state snapshots.
