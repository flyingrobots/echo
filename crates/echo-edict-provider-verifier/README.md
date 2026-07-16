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
component runs; decoding this native model's result is never admission.
The `wasm32` guest adapter vendors Edict's exact frozen
`edict:target-provider/verifier@1.0.0` WIT world and performs only exhaustive
transport-to-model conversion. Its reproducibly built 183,513-byte checked
component has SHA-256
`61c833dddb1919a4b92b55b984baf01116b82f6b7d6dc23760b7ecba01dc52c9`.
Component identity and admitted host replay remain separate propositions: the
pinned Edict host preflights the request artifacts and declared output schema,
invokes the checked component, then admits and manifests each returned accepted
or rejected report. It preserves an unsupported output-role overclaim as a typed
refusal without a response or manifest and replays all three completed outcome
classes identically in independent fresh stores.

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
report alone does not identify the Core, target profile, or semantic closure;
the digest-locked package assembled by the next campaign goalpost binds those
inputs and the verifier component together. Neither artifact grants Echo
runtime installation, execution, or consequence authority.
