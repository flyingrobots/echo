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
type bindings, unsupported Core ABI, target profiles, semantics, and output
roles produce typed provider refusals. Reads remain unsupported and fail
closed; the lowerer never represents a read as a synthetic mutation.

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
`x86_64-unknown-linux-gnu` builder. `audit` authenticates an existing component
without rebuilding it. `promote` requires two distinct underlying candidate
files, byte equality, the repository-approved SHA-256 identity, complete
component admission, and `--write`; G4 separately proves those candidates came
from fresh designated builds. Successful refresh uses synchronized temporary
bytes and atomic replacement, while write failure preserves the prior artifact.
`designated-build` can emit candidates but refuses the checked repository path.
Edict-host invocation evidence lives in the isolated Rust 1.94 witness under
`tests/edict-provider-host-v1/` so Wasmtime and unpublished Edict host crates do
not enter Echo's Rust 1.90 workspace dependency graph.

These bytes implement a provider translation. They do not install a package,
admit runtime authority, execute an operation, or attest an Echo consequence.
Those remain explicit Echo runtime crossings.
