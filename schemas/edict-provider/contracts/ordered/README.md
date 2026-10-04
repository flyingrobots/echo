<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Ordered Edict contract publication

This is the complete Apache-2.0 contract-pack pair from Edict commit
[`2405a550e93e1e97fff640caa44bbd0f65ffff3c`](https://github.com/flyingrobots/edict/commit/2405a550e93e1e97fff640caa44bbd0f65ffff3c),
under upstream `fixtures/provider-contracts/v1/`. The owning change is
[Edict #219](https://github.com/flyingrobots/edict/pull/219), which must pass
upstream review and land before this dependent change is ready to merge.

The CDDL SHA-256 is
`82273f3ea016a421c881f15b0fd451802205903ac9177bac8accbf3173f66d2c`;
the exact manifest SHA-256 is
`6303668861667a30418870ef25e5f169017905ae1f9d261451ba298120afdd9d`.
Neither file may be edited independently. Replace both only from one explicit
upstream publication and update its digest pins and this provenance together.

Consumers must call `admit_provider_contract_pack_for_publication_v1` with
`ProviderContractPublicationV1::OrderedInstructions`. The original
`admit_provider_contract_pack_v1` still admits only the existing
[pure-binding publication](../v1/README.md). The manifest coordinate and API
remain `v1`; the explicit selector and digests distinguish the publications.
The size ceiling is publication-specific, checked before JSON parsing.

The new schema describes the `orderedTargetIrArtifact` envelope and requires
an `executionOrder` array. Schema validity alone does not prove that this array
is a permutation, that its data dependencies hold, or that an Echo interpreter
supports its effects. This admission path does not select an ordered executable
profile or change any Jedit producer pin.

[Echo #734](https://github.com/flyingrobots/echo/issues/734) owns this admission
boundary. [Echo #684](https://github.com/flyingrobots/echo/issues/684) retains the
subsequent lowering, independent verification, and stateful execution work.

## Public compiler witness

From the repository root, the COPY-based Docker witness is:

```sh
docker build -f scripts/consumer-witnesses/ordered-publication.Dockerfile \
  -t echo-ordered-publication-witness .
docker run --rm echo-ordered-publication-witness
```

The image builds pinned Edict `2405a550e93e1e97fff640caa44bbd0f65ffff3c`,
fetches Jim's authored read/guard probe at
`19edb6fba94a8fea2dea63aa2f05cffc3e084f97`, and retains the old Echo provider
at `49e9efb68001dfd78563d18bac9359a87671e431` as a negative control. It also
uses the current Echo library to generate and digest-admit a disposable
provider candidate with this explicit schema publication. Existing checked
provider artifacts and Jedit producer pins are not rewritten.

The same public JSONL lawpack/application build crosses three boundaries:

1. The original v1 adapter refuses the read-result guard during target lowering.
2. A disposable v2 lawpack selection reaches the old provider's schema refusal.
3. Replacing only that provider with the ordered-publication candidate reaches
   `ProviderLowererRefused: UnsupportedSemantics` at `core.echo-pure-operation`.

Every refusal must leave the application output empty. The source body remains
unchanged; only the lawpack import digest changes after its experimental v2
selection. The v2 reference binds the exact compiler CDDL for this experiment,
not a released Echo v2 semantic contract. The success marker is
`ORDERED_PUBLICATION_REACHED_SEMANTIC_REFUSAL`.

This is real compiler/provider boundary evidence, not successful stateful
lowering or execution. The probe's `snapshot-read-probe` intrinsic remains
unsupported. No executable package is synthesized from a schema or oracle,
and no native Jim planner is used.
