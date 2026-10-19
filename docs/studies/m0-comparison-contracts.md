<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Study: M0 comparison contracts and their regressions

**Status:** study. It records proposed comparison contracts, maps them to
source at `6a7c62ff`, and adds regressions over existing behaviour. Owner
adoption of the contracts is not established. No production code, wire
format, root calculation or validity rule changes.
**Author:** Claude (Anthropic), for James Ross.
**Source:** milestone M0 in *Echo: Causal Commitments and Authenticated WARP
Readings*, revision 2, and §§2–6 of *Echo comparison contracts: consolidated
recommendation* (ChatGPT, 2026-10-18). Both are filed in the Reader vault.
Test identifiers T07, T14, T27–T34 follow that packet; T07 and T14 are
unchanged here.

## Result in one paragraph

Equal application records and equal v1 state roots do not decide whether
an accepted Action may still commit. The new regression holds one Action
undecided on three hosts that reach identical records. It obstructs only on
the host where its target was written and restored. A control host advanced
by the same number of ticks commits it. For admission, each API reads a
different basis. Witnessed submission ignores staging. Staging distinguishes
the same ticket from a different one, and checks occurrence material before
it returns a duplicate. Routing, inbox policy and the submission allocator
change results at equal committed state. A mutation matrix shows which tests
catch which mistakes. Two allocator-ordering mistakes are caught only by the
new T34 case.

## Contracts and where Echo implements them

Each row is one comparison. Equality of the listed basis is sufficient for
equality of the listed result; it is not a claim of minimality.

| Comparison | Result compared | Basis that must be equal | Source at `6a7c62ff` |
| --- | --- | --- | --- |
| Action preparation | Prepared or obstructed, the obstruction kind, the patch | Invocation and admission evidence, current basis, application reading, retained origin, the origin-to-parent interval, installed meaning, owner, policies | `prepare_operation_v1` (`echo_operation.rs:5075`); `echo_operation_origin_corroborated_v1` (`:3928`); `scoped_action_basis_holds_v1` (`:3844`); owner check by `Arc::ptr_eq` (`:154`) |
| Witnessed submission | Variant and fields, including the generation; the admission-state transition | Routing, coverage, original identity, occurrence bindings, inherited prefixes, committed ingress, inbox policy, witnessed submissions, allocator | `submit_intent_inner` (`coordinator.rs:3409`); `record_witnessed_submission` (`:4203`) |
| Ticketed staging | `Staged`, `Duplicate` or the error, with fields; the staging and inbox transition | Witnessed submission and retained envelope, occurrence bindings, ticket records, the head inbox's pending set, underlying ingest | `ingest_ticketed_invocation_inner` (`coordinator.rs:3537`); `ingest` (`:3742`); `HeadInbox::ingest` (`head_inbox.rs:1108`) |

Order matters inside the submission path, and the regressions pin it:

1. The committed-ingress duplicate check comes before the policy check.
2. The policy check (`would_accept`, `head_inbox.rs:1136`, which is a kind
   filter only) comes before the witnessed-duplicate check.
3. The witnessed-duplicate check comes before allocation.

In staging, the occurrence check comes before the staged-duplicate return.

## Regressions added

| Test | File | Claim |
| --- | --- | --- |
| `equal_application_records_do_not_decide_a_held_actions_eligibility` | `executable_operation_pipeline_tests.rs` | Three hosts, one Action accepted at tick 0 and held dormant on a second writer head. The intervals are idle, write-and-restore of the target, and write-and-restore of an unrelated node. Records, roots and request bytes are equal before the decision. Outcomes: commit, `BasisChanged`, commit. |
| `t27_equal_admission_references_give_equal_results` | `admission_comparison_tests.rs` | With equal reference tuples, seven requests across both APIs give equal results and equal transitions. Each request's expected variant is asserted, so equality is not vacuous. |
| `t34_accepted_unstaged_changes_the_submission_result` | same | Same committed and even resulting state; `Duplicate` versus `Accepted`. Comparing transitions alone misses it. |
| `t34_staged_unticked_changes_only_the_staging_result` | same | The witnessed layers are equal and only staging differs. Submission agrees. Staging gives `Duplicate` for the same ticket and `TicketedIngressAlreadyStaged` for another; the unstaged runtime stages. |
| `t34_occurrence_binding_precedes_duplicate_returns` | same | A bound occurrence key refuses other material. Restaging with changed occurrence material refuses before the staged-duplicate return, even with the same ticket. |
| `t34_routing_and_policy_change_results_at_equal_committed_state` | same | Different default writers give different submission identities. A kind filter gives `RejectedByPolicy`. |
| `t34_allocation_exhaustion_is_part_of_the_submission_basis` | same | At `IngressSubmissionGeneration::MAX`, a new submission refuses with `IntentSubmissionGenerationOverflow`. A known one still returns its duplicate and still stages. |
| `t34_persistence_image_restores_witnessed_but_not_staged_work` | same | The witnessed-submission persistence image restores the witnessed layer only, as its documentation says. The restored runtime stages where the original reports a duplicate. |

The reference tuple (`AdmissionReference`) and the application-record tuple
are test-only. No production `AdmissionBasis` digest is introduced. Results
are compared through their `Debug` form because `RuntimeError` has no
equality.

T28 is withdrawn. The restart obligation is Echo's existing
`filesystem_runtime_wal_recovers_replayable_provenance_after_restart`
(`trusted_runtime_host_loop_tests.rs:965`), which needs `host_test`. For the
proposed T30, assertion 2 should compare named application projections, not
universal execution. The held-Action regression is the counterexample to the
universal form.

## Mutation matrix

Each mutant plants one mistake the contracts warn against. The witness set
is the `warp-core` library unit tests plus four integration targets
(`executable_operation_pipeline_tests`, `admission_comparison_tests`,
`inbox`, `trusted_runtime_host_loop_tests`), with features
`native_rule_bootstrap,trusted_runtime,host_test`. The baseline passes 1,023
tests.

| Mutant | Mistake | Tests that fail |
| --- | --- | --- |
| `value_only` | A restored value corroborates its origin; interval writes are ignored. | `a_restored_value_does_not_corroborate_its_origin_and_recovery_agrees` (existing); the held-Action regression (new) |
| `no_scoped_exception` | Strict whole frontier: any tick advance obstructs. | Three existing tests (`budget_deferred_…`, `action_recovery_bounds_…`, `independent_actions_beyond_one_tick_…`); the held-Action regression (new) |
| `staging_occurrence_late` | Staging returns its duplicate before checking occurrence material. | `coordinator::tests::restaging_with_changed_occurrence_material_is_a_conflict` (existing); T34 occurrence case (new) |
| `submit_allocates_before_dedupe` | Submission allocates before returning a known duplicate. | T34 allocation case only (new) |
| `allocate_before_dedupe` | The recorder allocates before returning an existing record, which staging reaches. | T34 allocation case only (new) |

The mutants are in [`mutants.py`](m0-comparison-contracts/mutants.py), with one
pass/fail listing per run beside it. Each mutant is one exact textual
replacement, restored after its run. The `allocate_before_dedupe` listing is
from the rerun after the test change described next.

The first T34 allocation case missed the recorder mutant: submission
deduplicates before it reaches the recorder, so the mutant did not change
submission. Staging at the last generation was then added, and both
allocator mutants fail.

## Observations not turned into claims

- `HeadInbox::set_policy` (`head_inbox.rs:1296`) removes pending envelopes
  that the new policy rejects. It does not touch ticketed-staging records.
  No public path I found reaches it on a registered head, so this is
  recorded as a source observation, not a defect.
- Recovery of staged-but-unticked work through the runtime WAL was not run
  here. The persistence image's limit is documented behaviour, not a
  recovery finding.

## Reproduction

```sh
cargo +1.96.0 test --locked -p warp-core \
  --features native_rule_bootstrap,trusted_runtime,host_test \
  --test executable_operation_pipeline_tests --test admission_comparison_tests
```

Runs used the pinned 1.96 toolchain, `CARGO_INCREMENTAL=0`, a bounded
scratch target and the workstation `host/heavy-work` lock.
