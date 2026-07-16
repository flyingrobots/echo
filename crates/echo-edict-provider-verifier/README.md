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
transport-to-model conversion. Reproducible component packaging and admitted
host replay remain separate gates.

For this first one-operation closure, the exact checked target profile, exact
lowerability facts, and exact `echo.dpo@1.replace` intrinsic jointly bind the
`precommit-atomic` guard posture and `echo.dpo.footprint/v1` algebra identity. The
current Target IR has no independent footprint field and its requirements list
is empty, so this crate does not claim a general guard-order or footprint-
expression proof. The component packaging gate must additionally reproduce
the profile's resource-reference digests from the checked intrinsic,
footprint, cost, obstruction, and operation-profile resource bytes.

A verifier report's proposition is deliberately narrow: the fixed verifier
accepted or rejected the exact Target IR reference named by that report. The
report alone does not identify the Core, target profile, or semantic closure;
the digest-locked package assembled by the next campaign goalpost binds those
inputs and the verifier component together. Neither artifact grants Echo
runtime installation, execution, or consequence authority.
