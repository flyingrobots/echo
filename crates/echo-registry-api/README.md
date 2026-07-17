<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# echo-registry-api

Provider-neutral registry vocabulary for Echo-generated helpers. The crate has
two deliberately separate surfaces:

- the Wesley/GraphQL compatibility surface (`RegistryProvider`,
  `RegistryInfo`, and `OpDef`) that an application-specific registry crate
  implements for `warp-wasm`; and
- the Edict provider surface (`ProviderRegistryV1`, `ProviderOperationV1`, and
  their identity records), which borrows exact semantic, bundle, schema,
  profile, Target IR, codec, obstruction, and footprint claims from a generated
  helper.

Neither surface gives this crate application semantics. `warp-wasm` links only
to the compatibility interface, while provider package preflight consumes the
separate borrowed Edict vocabulary without importing a GraphQL facade.

`OpDef` preserves authored operation directive metadata as JSON. Echo admission
tooling can interpret entries such as `wes_footprint`, but this crate only
carries the data so the generic runtime boundary stays application-neutral.

## Operation identifiers

`stable_semantic_operation_id_v1(...)` owns the generic, versioned derivation
from an exact semantic coordinate and query/mutation kind. The top two `u32`
values are reserved for Echo protocol envelopes: scheduler control at
`u32::MAX` and witnessed suffix import at `u32::MAX - 1`.
`is_reserved_operation_id(...)` is the shared fail-closed predicate for
generators and application-blind envelope construction. Generators must also
detect collisions across their complete operation set; a numeric schema range
cannot prove either derivation or collision freedom.

## Contract artifact verification

Hosts can call `verify_contract_artifact(...)` against a generated
`RegistryProvider` before deciding how much trust to assign to a
Wesley-generated artifact. The verification policy compares:

- Echo contract ABI version;
- Wesley generator version;
- contract-host helper API version;
- codec id;
- registry layout version;
- schema hash;
- expected per-operation footprint certificate hashes;
- optional per-operation generated artifact hashes;
- whether every mutation must carry a footprint certificate named by policy.

A policy that checks only schema, codec, and layout returns
`MetadataVerified`. The stronger `CompileTimeCertified` posture is reserved for
policies that also require mutation footprint certificates and successfully
verify the expected certificate set. Release fast paths must key off the
posture, not merely on successful metadata verification.

The verifier returns a typed `ContractArtifactRejection` on mismatch. It does
not validate application payload semantics or execute an operation; generated
application adapters still own domain validation before packing EINT bytes.

Generated compatibility metadata is install-time evidence only. It does not
grant execution authority, query rights, or scheduler control, and it does not
replace semantic reading identity.

## Provider-generic registry

`ProviderRegistryV1` is descriptive borrowed evidence retained by an opaque
provider package proposal. It contains no Wesley generator metadata, GraphQL
field vocabulary, executor capability, installation token, or runtime
authority. Each `ProviderOperationV1` binds the complete operation proposition:
semantic coordinate and kind, persisted id law and value, distinct input and
output schema/codec contracts, failure-to-obstruction mapping, Target IR,
target and generated profiles, operation profile, and abstract footprint
identity.

`ProviderRegistryV1::operation_by_id(...)` fails closed: it returns `None` when
the id is absent _or_ duplicated instead of choosing one conflicting claim.
Constructing, inspecting, or matching this registry does not authenticate a
bundle, install a package, register an operation, invoke a callback, or prove
that a host implementation has the semantics it claims.
