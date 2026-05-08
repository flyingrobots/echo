<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Continuum Contract Artifact Interchange

Status: cool idea, future protocol lane.

Depends on:

- [Contract strands and counterfactuals](../up-next/KERNEL_contract-strands-and-counterfactuals.md)
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
- schema codec, schema hash algorithm, and schema hash
- intent or observer kind
- basis and frontier
- payload codec, payload hash algorithm, and payload hash
- receipt refs as codec/hash tuples
- witness refs as codec/hash tuples
- reading refs as codec/hash tuples
- admission posture
- retained artifact codec/hash tuples

Sibling runtimes should exchange protocol-shaped causal artifacts, not runtime
internals.

Every encoded field that contributes to identity needs a canonical byte
serialization rule and stable interoperable codec/hash identifiers such as
multicodec and multihash labels. Semantic names like "schema hash" are not
enough for cross-runtime verification.

## Acceptance criteria

- One Echo contract receipt exports as a Continuum-shaped artifact.
- A second runtime or verifier can inspect the artifact family identity without
  understanding Echo internals.
- Import is not treated as generic state snapshot sync; it preserves contract
  artifact identity and admission posture.
- Missing contract family or schema support is `obstructed`, not silent
  downgrade.

## Non-goals

- Do not start before local contract hosting is real.
- Do not require proof-carrying execution.
- Do not design all network transport.
- Do not collapse contract artifacts into generic state snapshots.
