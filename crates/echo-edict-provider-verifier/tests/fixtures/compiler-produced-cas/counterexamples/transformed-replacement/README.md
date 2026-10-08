<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Authored replacement ignored by the accepted package

This is retained failing evidence for the blocking argument-binding finding on
[PR 776](https://github.com/flyingrobots/echo/pull/776#issuecomment-6055363576),
not an accepted language/runtime contract or a positive golden fixture.

The source calls the imported effect with `message: "forced"`, while its returned
record independently uses `input.message`. The pinned public compiler documented
in the parent fixture accepted this source using the provider components at
`7bda832c15960e6ddd54e0d6c1fa91a079b9b20f`. The exact emitted executable package
and accepted verification report are preserved here as lowercase hex.

The emitted mutation binding is `application_input_replacement_path = ["message"]`.
That selects the caller's field rather than the authored effect argument. The
observed failure is successful production/verification of this mismatched
binding. Execution of this counterexample package has not been performed.

The corrected providers explicitly refuse this source because their bounded
mutation profiles support only the declared application input as the effect
argument. The regression
`providers_refuse_effect_inputs_not_equal_to_the_declared_application_argument`
checks lowerer and verifier refusals independently for both mutation profiles.
Its first run failed because the lowerer accepted a transformed argument; after
the fix the verifier executable suite passed 70 tests and the lowerer suite 13.

To reproduce the public compiler refusal, run the command in the parent README
with `--source-template` pointing to this `update-cell.edict` and
`--expect-refusal ProviderLowererRefused`. The corrected component pair produced
that structured refusal with compiler exit code 2 and no published CBOR outputs.
The original direct-input positive control still emitted byte-identical package
and report artifacts. These checks establish production/refusal; they do not
claim execution of the historical erroneous package or final PR approval.
