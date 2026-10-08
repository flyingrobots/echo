<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Source-function Edict contract publication

This complete Apache-2.0 contract pair was copied from Edict commit
[`acd71fce03d2b5fc6d78c2a92ca922a45da6bac2`](https://github.com/flyingrobots/edict/commit/acd71fce03d2b5fc6d78c2a92ca922a45da6bac2),
under `fixtures/provider-contracts/source-functions-v1/`, for
[Edict #226](https://github.com/flyingrobots/edict/issues/226). The pair is byte-identical at the
merged compiler revision
[`01161c1745baad0d713234ba1a671b9a26923aa4`](https://github.com/flyingrobots/edict/commit/01161c1745baad0d713234ba1a671b9a26923aa4).

The CDDL contains 34,460 bytes with SHA-256
`e484eabd615584a38cb57454b52747786f2314c135c13f99da6f6bae619a709f`.
The manifest contains 75,989 bytes with SHA-256
`aebc2e4133407ae433c18b55421a89bf15dc5d489368f82ce4874f948d4f9c79`.
Keep the pair byte-for-byte intact. Replace both only from one explicit upstream
publication, updating the digest pins and provenance together.

Select this pair with `admit_provider_contract_pack_for_publication_v1` and
`ProviderContractPublicationV1::SourceFunctions`. The unchanged manifest API and
coordinate do not select a publication: exact digests and the explicit enum
variant do. Admission checks the 75,989-byte ceiling before parsing JSON,
embedded schema/resource consistency, provenance, and both pinned raw digests.
It performs no discovery or automatic upgrade.

The [pure-binding](../v1/README.md) and
[ordered-instruction](../ordered/README.md) pairs and selectors remain separate.
The original admission API and checked artifact/package generators still select
pure bindings. This schema accepts function-free Core and adds an optional,
nonempty `functions` map whose entries carry typed parameters, a return type,
ordered pure bindings, and a result. Old publications reject that new map.
Omitting an empty map preserves the old canonical shape.

Schema admission does not establish call authority, lexical scope, totality,
combined call depth, resource bounds, or executable support. The provider and
its independent verifier must validate source-owned definitions against the
exact authenticated Core, separately from imported lawpack authority. Runtime
support and limits belong to the
[application contract hosting contract](../../../../docs/architecture/application-contract-hosting.md).

The isolated `tests/edict-provider-host-v1` suite retains its frozen
`2e3f52f9e6d615f96eb594a40126e223a9253d98` compiler/host and exact compatibility
corpus. Source-function consumer checks use the separately pinned public
compiler, whose application build invokes both provider components through its
actual host. Record that compiler revision and the exact emitted package/report
bytes for runtime checks. Preserve the frozen suite's broader refusal and replay
checks when refreshing the provider components.

## Explicit candidate generation

The [candidate generator](../../../../crates/echo-wesley-gen/examples/source_functions_publication_witness.rs)
selects this pair, generates provider artifacts with the normal owned library,
assembles the exact checked lowerer/verifier component bytes, and digest-admits
the complete package before writing a new destination directory. It does not
replace the existing checked pure-binding corpus or choose a Jedit producer pin.
Run it only after the component and asset refresh described in the
[generator operating instructions](../../../../crates/echo-wesley-gen/README.md#refreshing-source-bound-provider-publication).

Inside the admitted guarded Docker worker, using its accounted Cargo target:

```sh
cargo +1.96.0 run --locked -p echo-wesley-gen \
  --example source_functions_publication_witness -- /owned-data/source-functions-provider
```

The new directory must be inside the runner's guarded data root. The Docker
presence check does not enforce shared storage budgets, free-space floors,
exclusive ownership, CPU/memory ceilings, log caps, or process cleanup; the
outer runner must enforce them. No new worker or Cargo cache is required.

Changing generator source changes its exact source identity and therefore
regenerated provenance and provider-package identity. Preserving old contract
publication bytes does not mean those newly generated package identities stay
unchanged. Retain an exact old provider package for compatibility controls.
Component reproducibility remains subject to the independent designated-build
policy in [the component contract](../../components/v1/README.md).
