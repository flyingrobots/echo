<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Structured effect publication fixture

These authored inputs select the current `echo.dpo@2` profile and its exact
`echo.span-ir/v3` reference. The lawpack exports only `ensureNode`, `createNode`,
and their input/result types. The application exercises an initial effect,
a bounded loop, and both statement-conditional arms through the public compiler.

The `effect_step_publication` example accepts this directory with the output name
`structured`. It builds the lawpack, replaces the source import's digest with
the actual manifest digest, and builds the application twice. The retained
source carries the digest that producer computed,
`sha256:f50f7bf38dd9cb910ba1aa8c24908b931d3cc795fe60d05ba9c916d0baf93d7a`.
The five retained `structured-*.hex` outputs, including
`structured-executable-operation-package.hex`, were written by that producer
with Amblicit `ce2ad2f5faed31147dc9cb5db05353d60b68396d`; they are not authored
Target IR. `scripts/verify-edict-provider-host-v1.sh` reruns the producer with
`--check` and requires the same bytes.

| File | Domain-framed digest |
| --- | --- |
| `structured-executable-operation-package.hex` | `echo:operation-package:v1` `60aa9ae3bf797283e13132ff27fcea4547aa6fef900fe60b479f3e2acb2bbd4d` |

The intended runtime input sets `initial` to node A and supplies two items:
`{create: true, node: B}`, then `{create: false, node: A}`. A successful private
evaluation must buffer and complete exactly `EnsureNode(A)`, `CreateNode(B)`,
`EnsureNode(A)` in that order. Empty and one-item lists retain the corresponding
prefix. The return value is the initial ensure result, an empty record. No
graph commit, causal receipt, or WAL publication is claimed by this fixture.

`items` has maximum length two and each node payload has at most 64 atom bytes.
The configuration retains a 1,024-byte input ceiling, 64-byte result ceiling,
65,536-byte allocation ceiling, and 4,096 each for steps, read bytes, and write
bytes. The shared node shape keeps the two-item input within the input ceiling;
there is no extra unused atom-replacement payload in either branch.

The producer's own controls require the same source to be refused under
`echo.dpo@1` and under a substituted `echo.dpo@2` profile. The runtime witness
`crates/warp-core/tests/executable_operation_pipeline/public_structured_effect.rs`
owns exact package policy, list-bound refusal, and effect-order assertions.
Frozen `echo.dpo@1` fixtures are unchanged.
