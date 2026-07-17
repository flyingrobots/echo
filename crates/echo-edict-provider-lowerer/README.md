<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Edict Provider Lowerer

`echo-edict-provider-lowerer` implements Echo's exact
`edict:target-provider/lowerer@1.0.0` component for the first checked provider
closure. Its production boundary is pure: callers supply canonical Core, the
exact target profile, the complete semantic closure, requested output roles,
and response limits through the frozen Edict WIT request.

The `0.1.0` Rust source crate is enabled and configured for publication, and its
source archive is self-contained. It packages the four exact admitted provider
resources needed by this first closure, while a repository-owned witness requires
those package-local bytes to remain identical to the checked generated corpus.
Registry publication exposes the pure native model and a reproducible component
source build; the frozen WIT component remains the provider ABI, and neither
distribution channel grants Echo runtime authority. Release
`echo-edict-canonical 0.1.0` first; after that dependency is visible in the
registry index, the lowerer's full package/publish gate becomes executable and
must run from the same clean revision.

The first supported operation is the mutating `a.b@1.t` compatibility fixture.
The lowerer emits canonical `edict.target-ir.artifact/v1` bytes whose decoded
inner domain is `echo.span-ir/v1`. It accepts only the exact Core module
coordinate, local intent key, input/output type bindings, Echo DPO target
profile, and #652 lawpack, authority, and lowerability identities. Rebound
operations, authored optics that this crossing cannot yet discharge, changed
type bindings or definitions, evaluation budgets, unsupported Core ABI, target
profiles, semantics, and undeclared, mismatched, duplicate, or unsorted
output-role claims produce typed provider refusals. The
one-effect closure requires exactly the compiler-owned input, effect-result, and
obstruction declarations, then validates pre-effect, obstruction-arm, and
post-effect scope by their complete identities before cloning expressions into
Target IR. It accepts only an empty input-constraint set and the reviewed
zero-argument `domain.WriteRejected` obstruction constructor. Effect inputs and
intent results admit no call-expression callee in this closure; later
constraint, constructor, or call semantics require explicit lowering laws. Reads remain
unsupported and fail closed; the lowerer never represents a read as a synthetic
mutation.

The native model accepts exact sorted subsets of three output declarations:
`generated.echo-dpo` / `GeneratedArtifact` / `echo.generated-artifact/v1`,
`review.echo-dpo` / `ReviewPayload` / `echo.review-payload/v1`, and
`target-ir.echo-dpo` / `TargetIr` / `edict.target-ir.artifact/v1`. The generated
and review envelopes use `generated/echo_dpo.rs` and `review/echo_dpo.json`.
The generated envelope binds profile `echo.dpo.registration/v1`, operation
`a.b@1.t`, and the exact Rust source; the permanently non-authoritative review
names the exact generated artifact as its subject. `echo.span-ir/v1` is the
semantic Target IR coordinate, while `edict.target-ir.artifact/v1` is the
artifact output and digest-framing domain. Likewise, `echo.dpo.bundle/v1` is a
target-bundle profile, not an Edict contract-bundle occurrence.

Final `edict.bundle.semantic/v1` and `edict.bundle.release/v1` identities are
supplied only after assembly. Generated `bind_contract_bundle` checks their
typed SHA-256 form and domains, expected-versus-actual bundle digests, and the
operation coordinate, Target IR, Echo ABI, helper API,
provider/input/output/effect-failure/obstruction schemas,
target/generated/operation profiles, and abstract footprint obligation and
algebra identities. Every framed identity is compared as a complete
coordinate/domain/digest proposition. The footprint binding does not invent a concrete static
read/write set. `bind_contract_bundle` remains a pure equality and consistency
preflight: it does not authenticate the expected pin, admit or install a
package, or confer registration or runtime authority. This behavior currently
belongs to the native model. The canonical generated profile/package now carry
the Echo-owned `echo.semantic-operation-id.fnv1-32/v1` law and exact persisted
id for `a.b@1.t`; generation refuses both Echo protocol-reserved ids
(`u32::MAX` scheduler control and `u32::MAX - 1` witnessed suffix import) and
package-local collisions without salting or probing. The generated CDDL bounds
the numeric application-id domain, while semantic generation independently
recomputes the law and checks collision freedom; schema admission alone proves
neither proposition. Emitted source carries public expected constants for that
exact law and id without re-deriving them, requires both as untrusted bundle
claims, and refuses a disagreement before its private-state registration
descriptor exposes the matched id claim.

The generated-artifact profile now owns the exact `le-binary-v1` value-codec
claim, and the generated source implements that claim with distinct `Id`,
`Input`, and `Output` types rather than treating equal wire shapes as one
semantic type. The `Id` codec preserves raw UTF-8, enforces the authored
16-Unicode-scalar bound, and performs no normalization. Generated decoders fail
closed on malformed UTF-8, over-bound lengths or values, truncation, and
trailing bytes. Descriptor methods encode and decode the exact input/output
types and `pack_intent(...)` places the encoded input into Echo's canonical EINT
v1 envelope under the matched operation id. The EINT `vars` field is opaque to
Echo; its bytes are owned by the selected profile codec, not universally by
canonical CBOR.

After bundle binding, the descriptor can also expose a borrowed,
provider-generic `ProviderRegistryV1` and can preflight one explicit host
mutation implementation into an opaque `ProviderContractPackageProposalV1`.
That crossing compares the complete Target IR, bundle, target/generated/
operation profiles, provider and value schemas, codec, obstruction, operation
id, ABI, helper API, and footprint identities. It also binds the generated
matcher and canonical rule name. Equality of those identities detects
cross-binding; it does not prove that arbitrary host callbacks semantically
implement the operation. The proposal neither installs nor registers itself:
only a trusted Echo host may perform the later installation crossing. This
proposal constructor supports mutations and refuses a `Query`; authored reads
remain a separate bounded observer/optic path and must never be represented as
synthetic mutations.

The refreshed 189,668-byte checked lowerer component has SHA-256
`9b5ecf1c87a4a98ad3d7381d85e3625649a4f9d62f80f74b2aca6ace26be14af` and has
crossed the reproducible promotion boundary. The pinned Edict host admits its
generated envelope under the owning `generated-artifact` CDDL root, and the
isolated host fixture exercises the exact helper binding, typed codecs, EINT
packing, borrowed registry, and non-installing proposal contract. Host-side
admission of the review envelope remains an independent crossing.

The native Rust model is also the narrow unit-test boundary. A `wasm32` adapter
generated from [`wit/edict-target-provider.wit`](wit/edict-target-provider.wit)
exports the exact Component Model function and performs total conversions to
and from that model. The component imports only the frozen WIT's type closure;
it imports no filesystem, network, environment, clock, randomness, registry,
logging callback, or WASI capability.

Build and audit local component bytes from the repository root with:

```sh
cargo +1.90.0 xtask provider-lowerer-component build \
  --target-dir target/provider-lowerer-component
```

Local component construction resolves and authenticates absolute Rust 1.90.0
Cargo and compiler executables, binds the inner Cargo build to that exact
compiler, disables configured wrappers, and validates the complete module,
import/export topology, and contract attestation without claiming cross-host byte
identity. The checked artifact is built and compared exactly on the designated
`linux/amd64` Rust image pinned by OCI digest. The inner build uses a controlled
Cargo home beneath its target directory, removes ambient Cargo
profile/build/target overrides, and remaps dependency source paths to `/cargo`;
physical cache locations therefore cannot alter the component bytes.
`audit` authenticates an existing component without rebuilding it. `promote`
requires two distinct underlying candidate files, byte equality, the
repository-approved SHA-256 identity, complete component admission, and
`--write`; G4 separately proves those candidates came from independently
provisioned designated builds. Successful refresh uses synchronized temporary
bytes and atomic replacement, while write failure preserves the prior artifact.
`designated-build` can emit candidates but refuses the checked repository path.
Edict-host invocation evidence lives in the isolated Rust 1.94 witness under
`tests/edict-provider-host-v1/` so Wasmtime and unpublished Edict host crates do
not enter Echo's Rust 1.90 workspace dependency graph.

These bytes implement a provider translation. They do not install a package,
admit runtime authority, execute an operation, or attest an Echo consequence.
Those remain explicit Echo runtime crossings.
