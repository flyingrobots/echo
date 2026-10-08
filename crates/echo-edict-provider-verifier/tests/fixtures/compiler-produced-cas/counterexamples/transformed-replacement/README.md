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

To reproduce, run the parent fixture's `build.py` in the guarded worker with the
pinned compiler and candidate provider, then replace its generated
`update-cell.edict` with this source and submit the same public JSONL application
build again. The parent lawpack/configuration bytes and digest are unchanged.
The correct result must preserve the authored effect argument or return an
explicit unsupported-semantics refusal. The original direct-input source remains
the positive control. Add the executable refusal/correctness regression before
implementing the provider fix; do not turn the retained acceptance into a golden
expectation.
