<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Study: stale-Action corroboration read an incomplete write set

**Status:** study accompanying the fix on this branch. **Author:** Claude
(Anthropic), for James Ross. **Found by:** the M1 shadow-root probe (branch
`claude/m1-shadow-probe`, study `docs/studies/m1-shadow-root-probe.md`
there). **Rerun:** branch `claude/m1-shadow-probe-v2`.

## The defect

The scoped stale-basis exception (#837) lets an Action built at an earlier
tick commit when nothing touched its target or observed nodes in between.
"Touched" came from `extend_echo_operation_written_slots_v1`
(`echo_operation.rs`). That function collected each retained patch's
`out_slots`.

`out_slots` is the footprint-declared write set. It is not a complete record
of what a tick wrote.
- **What is missing:** runtime-ingress materialization
  (`Engine::materialize_runtime_ingress_event`) writes the event node, its
  intent attachment and its kind edge directly to the store. No rule footprint
  covers those writes.
- **Where they do appear:** the patch's ops, which come from the touch log,
  record them.
- **How large the gap is:** in F5 tier M, 14,997 of 29,994 written slots were
  missing from `out_slots`.

Those writes were invisible to corroboration. No Action exploiting this was
constructed. The undeclared writes happen once per new ingress, and
corroboration only decides cases where a value changed and was restored.

## The fix

| Item | Change |
| --- | --- |
| `WarpOp::extend_written_slots` (`tick_patch.rs`) | New. Adds the record slots an op can write. Deletions include the attachment removed with the node or edge, and a portal adds its attachment and child root. It returns `false` for warp-instance upserts and deletes, whose writes are not slot-addressable. |
| `extend_echo_operation_written_slots_v1` | Now collects `out_slots` plus the slots the ops write. An instance-level op makes the interval's writes unknown (`None`), so the whole-frontier rule applies. |
| Formats | Unchanged: patch contents, digests, roots and wire formats. |

Behaviour changes in two directions:
- **Narrower:** the exception now refuses where an undeclared write touched
  the target.
- **Stricter:** the exception is also unavailable across a tick that upserts
  or deletes a warp instance.

## Evidence

- **RED, then GREEN.** `interval_written_slots_include_writes_outside_declared_footprints`
  (`echo_operation.rs` tests) commits one real runtime-ingress tick through
  `SchedulerCoordinator::super_tick`. It asserts that the interval write set
  contains the materialized event node and its alpha attachment.
  - Without the fix: [`assertion failed: written.contains(&…SlotId::Node(event))`](corroboration-write-set/red.txt).
  - With the fix: [passes](corroboration-write-set/green.txt).
- **Full `warp-core` suite** (`native_rule_bootstrap,trusted_runtime,host_test`):
  1,942 passed and 1 failed ([results](corroboration-write-set/full-suite.txt)).
  - **The failure:** `special_files_are_rejected_before_a_read_attempt`, with
    "path must be shorter than SUN_LEN".
  - **Why it is unrelated:** the run's scratch `TMPDIR` is too long for a Unix
    socket path, and the test fails the same way with the fix stashed.
- **Lint and format:** clippy `-D warnings` passes for the library and tests
  with `native_rule_bootstrap,trusted_runtime,host_test`, and for the default
  library. fmt is clean.
- **M1 shadow rerun.** The shadow-root probe was rerun with this fix
  cherry-picked, taking its slots from the same write set. Parity with the
  independent rebuild holds through F5 tiers S and M
  ([output](corroboration-write-set/m1-shadow-after-fix.txt)).

All runs used Rust 1.96 under the workstation `host/heavy-work` lock with a
disk guard.

## Not covered

- Boundary port slots: `out_slots` may name them; the ops-derived set does
  not add any.
- Ticks rolled back by the state journal: they leave no provenance entry.
- An end-to-end Action scenario: none is known that the old set would have
  let through.
