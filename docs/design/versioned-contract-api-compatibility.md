<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Versioned Contract And API Compatibility

Status: implemented local package-boundary slice.

This packet records the v0.1.0 compatibility boundary for generated contract
packages. The goal is narrow: Echo should reject a generated package that does
not fit the host runtime before the package can install mutation handlers,
install query observers, stage runtime-visible work, or answer accepted reads.

## Claim

Generated contract packages now carry and verify the compatibility identity
needed by the local Echo contract-host seam:

- Echo contract ABI version;
- Wesley generator version;
- contract-host helper API version;
- registry layout version;
- codec identity;
- schema hash;
- package artifact hash;
- per-operation footprint certificate identity.

The package boundary rejects drift as `ContractArtifactRejection` instead of
accepting ambiguous generated code and discovering the mismatch later through
malformed runtime ingress, missing handlers, or misleading readings.

## Boundary

The compatibility check is host/runtime-owner policy. It does not make
application payloads valid, does not authorize execution, does not grant query
rights, and does not turn package metadata into semantic reading identity.

The verified identity is evidence metadata:

```text
package install policy
-> generated registry metadata
-> verified installed package record
-> receipt and reading contract evidence
```

Runtime-visible work still requires witnessed submission, admission evidence,
ticketed runtime ingress, and a scheduler-owned tick. Readings still require a
registered read-only query observer and bounded observation request.

## Implemented Surface

- `echo-registry-api` exposes `ECHO_CONTRACT_ABI_VERSION` and
  `CONTRACT_HOST_HELPER_API_VERSION`.
- `RegistryInfo` includes Echo ABI, Wesley generator, and helper API versions.
- `ContractArtifactVerificationPolicy` requires those expected versions.
- `verify_contract_artifact(...)` rejects Echo ABI, Wesley generator, and helper
  API drift before weaker schema, codec, or footprint checks can pass.
- `echo-wesley-gen --contract-host` emits generated constants for the same
  compatibility identity and uses them in the generated registry provider.
- `warp-core` retains verified compatibility metadata in
  `ContractEvidenceIdentity`.
- Installed contract readings and receipt correlations can cite the verified
  compatibility metadata that produced them.

## Invariants

- Unsupported compatibility is rejected at package install, not at scheduler
  work time.
- Generated compatibility metadata is not caller testimony.
- Receipts and readings may cite compatibility identity, but that citation is
  not execution authority.
- A CAS content hash is still byte identity only; compatibility metadata does
  not become semantic lookup identity.
- Echo core remains generic. Wesley-generated code owns application nouns.

## Evidence

- `cargo test -p echo-registry-api`
- `cargo test -p echo-wesley-gen --test generation`
- `cargo test -p warp-core --features native_rule_bootstrap --test installed_contract_registry_tests`
- `cargo test -p warp-core --features "native_rule_bootstrap host_test" --test installed_contract_intent_pipeline_tests`
- `cargo check -p echo-wasm-abi`

## Remaining Release Work

This slice is not release automation. Later v0.1.0 work still needs:

- stable package publishing metadata;
- release notes that name the supported compatibility set;
- adapter-level compatibility reporting for product-facing clients;
- migration policy for future incompatible contract-host seams.
