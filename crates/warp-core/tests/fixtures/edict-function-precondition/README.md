<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Function-precondition publication fixture

`source/FunctionPrecondition.edict` declares a source helper
`span(start, end) = end - start` with the precondition `start <= end`. The
intent calls it under its own `where input.start <= input.end`, which proves the
call. The program selects `echo.dpo@1` and shares the `edict-effect-program`
lawpack.

The `effect_program_publication` example accepts this directory with
`--name precondition`. It builds the lawpack, pins the import digest, and builds
the application twice through the checked provider package. The five retained
`precondition-*.hex` outputs were written by that producer with Amblicit
`ce2ad2f5faed31147dc9cb5db05353d60b68396d`, which contains the precondition
compiler half (amblicit#264); they are not authored Target IR. The producer also
requires the predecessor provider at `a9792ddc` to refuse the same source with
`ArtifactSchemaMismatch`, because its contract pack predates preconditions.
`scripts/verify-edict-provider-host-v1.sh` reruns the producer with `--check`.

The runtime witness
`crates/warp-core/tests/executable_operation_pipeline/public_precondition_effect.rs`
pins the package identity and checks results, the `where.0` refusal for
`start > end`, and the exact admission policy. Forged Core whose call does not
establish the precondition is covered by the shared function-precondition
corpus in all three judgments.
