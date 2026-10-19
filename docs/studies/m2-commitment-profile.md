<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Study: M2, slice 1 — an opt-in application-root commitment profile

**Status:** experimental, off by default (`application_root_profile`, which
implies M1's `application_root_shadow`). Worldlines default to the legacy
profile, whose roots, commit hashes and receipts are unchanged. This is a
first slice of M2, not all of it; see *What M2 still needs*. Owner adoption
of the profile, its domain strings and any durable descriptor is not
established.
**Author:** Claude (Anthropic), for James Ross.
**Source:** milestone M2 and §§8.5, 9 and 15.1 of *Echo: Causal Commitments
and Authenticated WARP Readings*, revision 2.
**Builds on:** M1 (#922), which builds on #918 and #919.

## Result

On F5 tier M (5,000 appends into a growing graph), the same workload under
the application profile:

| Tier | Profile | Append p50 | Per append | Replay p50 |
| --- | --- | --- | --- | --- |
| S | legacy-reachable-v1 | 16.56 ms | 0.0906 ms | 10.34 ms |
| S | application-radix-v0 | 9.34 ms | 0.0441 ms | 2.71 ms |
| M | legacy-reachable-v1 | 5,630.16 ms | 1.0993 ms | 5,551.82 ms |
| M | application-radix-v0 | 272.04 ms | 0.0542 ms | 99.17 ms |

Appending is 20.7 times faster and replay 56 times faster in tier M; both
rounds validate under the fixture's own outcome checks. Same host, same
branch, same build, five rounds each
([output](m2-commitment-profile/bench.txt)).

## What the slice does

- **An explicit profile per worldline.** `CommitmentProfile` is
  `LegacyReachableV1` (default) or `ApplicationRadixV0`, chosen with
  `WorldlineState::with_commitment_profile` before registration. It travels
  with the M1 trie, so every path that carries the trie carries the profile:
  rollback markers, replay checkpoints and bases, forks, and the runtime
  commit guard. Nothing infers a profile from a root's bytes (revision 2,
  §15.1).
- **A tagged root.** An application-profile worldline's `state_root` is
  `H("echo.state-root.application-radix.v0" || application_root)`. The tag
  keeps it domain-separated from every legacy root, so no historical
  `state_root` field is ever reinterpreted. Commit hash recipes v2 and v3 are
  unchanged; they bind whichever root the profile commits.
- **No graph walk.** The engine's runtime commit advances the worldline's
  trie from the commit's complete write set and commits its root, instead of
  `compute_state_root_with`. The executable-operation (Action) path already
  reads `WorldlineState::state_root`, which is profile-aware.
- **Verification under the profile.** Replay, readings at earlier
  coordinates, import-candidate checks, playback cursors, provenance initial
  boundaries and checkpoint restores compute the worldline's own recipe
  (`recomputed_state_root`, `state_root_of`) where they used the legacy
  function. With the feature off, each of these is the legacy function.

## Evidence

| Test | Shows |
| --- | --- |
| `coordinator::…::application_profile_commits_the_tagged_application_root_and_verifies_only_under_it` | Four contract commits: each committed root is the tagged application root and never the legacy root; the initial boundary is committed under the profile; replay verifies under the profile and fails under the legacy one; a fork inherits the profile and the root |
| `application_root_profile_tests::f5_validates_under_the_application_profile` | F5 validates under both profiles with the fixture's own outcome, replay and earlier-coordinate checks; the two profiles commit different roots for the same graph |
| `trusted_runtime_host_loop_tests::application_profile_history_recovers_only_under_its_profile` | A filesystem-WAL history recovers when the host registers the same profile and refuses to recover under the legacy profile |
| `executable_operation_pipeline_tests::application_profile_actions_commit_and_recover` | 70 Actions built at the genesis boundary commit across two ticks (the scoped stale-basis exception corroborates the genesis origin under the profile), and filesystem-WAL recovery agrees |
| Full suite with `application_root_profile,delta_validate` | Every legacy test still passes with the feature compiled in; see Results |

## What M2 still needs

These are gates the revision assigns to M2 that this slice does not meet,
and decisions that are not mine to make:

1. **A durable profile descriptor.** The profile is part of the host's
   worldline registration, like the initial state; it is not written to the
   WAL, provenance codecs or WSC. A mismatch is detected (replay and recovery
   fail), but the failure is a root mismatch, not a typed "unknown or
   mismatched profile" error. A descriptor needs a codec version and owner
   approval of the domain strings.
2. **The boundary seal.** Revision 2 §8.5 specifies a seal binding the
   application and history roots, receipt core, parents, coordinates and
   profiles, with a cross-reading validation rule. This slice binds only the
   application reading, through the existing v2/v3 commit hashes. The
   history reading is M3/M4 work.
3. **Ports and warp-instance headers** remain outside the application
   reading.
4. **Migration.** There is no path from a legacy history to the new profile;
   §15.1's migration-boundary rules apply when one is wanted.
5. **Exchange.** Import of application-profile histories across hosts uses
   the same checks, but no cross-host exchange test exists.

## Results

| Check | Result |
| --- | --- |
| Full `warp-core` suite with `native_rule_bootstrap,trusted_runtime,host_test,application_root_profile,delta_validate` | 2,019 passed; the only failure is the long-`TMPDIR` socket test that also fails on `main` ([results](m2-commitment-profile/full-suite-delta-validate.txt)). Legacy worldlines are the default, so this is the legacy suite with the feature compiled in, plus the new tests. |
| `executable_operation_pipeline_tests`, `application_root_profile_tests`, `trusted_runtime_host_loop_tests` with `delta_validate` (after adding the Action test) | 49, 1 and 70 passed ([results](m2-commitment-profile/targeted-tests.txt)) |
| `application_root_profile_tests` in release (tiers S and M) | Passes |
| Clippy `-D warnings`: profile feature (`--lib --tests --benches`), legacy `--all-targets` with `delta_validate`, default library | Pass |
| `cargo fmt --all -- --check` | Pass |

Finding along the way: test files that put `#![cfg(feature = …)]` on the
whole crate lose their crate docs when the feature is off, and CI's
`--all-targets` clippy (`-D missing_docs`) rejects them. Both new test files
gate their items instead; the M1 branch received the same fix.
