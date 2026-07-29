<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Edict Provider Verifier

This crate owns Echo's pure semantic decision for Edict's frozen verifier
boundary. It compares explicit, digest-bound Edict Core and Target IR artifacts
under the checked Echo provider closure. It performs no discovery or I/O and
grants no Echo runtime authority.

The first native slice is intentionally independent from the provider lowerer.
A supported but semantically false Target IR produces a rejected verifier
report; malformed input in the selected native closure and unsupported source
semantics produce typed provider refusals. Complete structural admission for
every CDDL alternative remains the Edict host's owning-schema check before the
component runs; decoding this native model's result is never admission. Before
classifying a known but unsupported expression as semantic disagreement, the
native verifier checks its complete expression, predicate, input-constraint,
require-failure, and Core-value shape under the fixed recursion bound. The
target profile's diagnostic ABI and every emitted report consume one shared
admitted identity.

The verifier also owns the structurally separate executable-operation route. It
accepts the exact source, Core, lawpack, exports, adapter, target
configuration, Target IR, and compiler-emitted package as digest-bound inputs,
then reconstructs the expected generic package without importing or calling
the lowerer. Exact equality yields an accepted
`echo.operation-package-verifier-report/v1`; a rebound or otherwise
self-consistent package mutation yields a typed rejected report. The verifier
independently resolves source-local effect and obstruction aliases through the
exact lawpack import before comparing the canonical package. It independently
validates the compiler-owned result-projection artifact, reconstructs the
runtime expression and application-input paths, binds their identity into the
expected package, and names that exact projection in the accepted report.
Application vocabulary is treated only as opaque authored data.

The `wasm32` guest adapter vendors Edict's exact frozen
`edict:target-provider/verifier@1.0.0` WIT world and performs only exhaustive
transport-to-model conversion. Its reproducibly built 277,568-byte checked
component has SHA-256
`574073ef2281e5cbec27951d7e53d061cac27d0c84a7db1f1a252e8639ce4b6b`.
Component identity and admitted host replay remain separate propositions: the
pinned Edict host preflights the request artifacts and declared output schema,
invokes the checked component, then admits and manifests each returned accepted
or rejected report. It preserves an unsupported output-role overclaim as a typed
refusal without a response or manifest and replays all three completed outcome
classes identically in independent fresh stores and separate host processes.

For this first one-operation closure, the exact checked target profile, exact
lowerability facts, and exact `echo.dpo@1.replace` intrinsic jointly bind the
`precommit-atomic` guard posture and `echo.dpo.footprint/v1` algebra identity. The
current Target IR has no independent footprint field and its requirements list
is empty, so this crate does not claim a general guard-order or footprint-
expression proof. The crate embeds the exact generated type, intrinsic,
footprint, cost, operation-profile, obstruction, lawpack-adapter, and verifier
resources. Before comparing Core with Target IR, it reproduces their
domain-framed identities, resolves the profile and lawpack references, and
checks the complete reviewed semantic crossing. Raw byte identities remain a
separate pinned proposition, and workspace validation must still prove these
package-local copies equal the CDDL-admitted checked corpus.

A verifier report's proposition is deliberately narrow: the fixed verifier
accepted or rejected the exact Target IR reference named by that report. The
report alone does not identify the Core, target profile, or semantic closure.
For each successful invocation, the pinned Edict host's output manifest binds
the exact Core, target profile, Target IR, ordered semantic inputs, requested
output, and admitted report digest. That per-invocation envelope is distinct
from the digest-locked package assembled by the next campaign goalpost, which
binds the static inputs and verifier component together. Neither proposition
has an Echo causal site or witnessed hologram, and neither grants Echo runtime
installation, execution, or consequence authority.
