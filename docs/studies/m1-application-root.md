<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Study: M1, an incremental application root in shadow mode

**Status:** experimental, off by default (`application_root_shadow`). It
changes no state root, receipt, patch, WAL or wire format; with the feature
off it compiles out. Owner adoption of the profile or of any later root
replacement is not established.
**Author:** Claude (Anthropic), for James Ross.
**Source:** milestone M1 in *Echo: Causal Commitments and Authenticated WARP
Readings*, revision 2. The profile is the one E1 selected (branch
`claude/e1-authenticated-maps`, study `e1-authenticated-maps.md` there).
**Depends on:** #918 (`WarpOp::extend_written_slots`, the complete write
set) and #919 (the F5 scene fixture), which this branch merges.

## What M1 asked for, and what is here

| M1 requirement (revision 2, §15) | Here |
| --- | --- |
| Add the selected experimental application dictionary beside the current graph store, with its profile explicit | `application_root.rs`: E1's `radix-v0` compressed binary radix trie, under the domain `echo.application-root.radix.v0` (`APPLICATION_ROOT_PROFILE`) |
| Derive record updates from the existing transaction/touch boundary | A commit's write set is its patch's `out_slots` plus every slot its ops write (the #918 definition); the written records are read from the committed graph |
| Include all scope members | Every node, edge and attachment record in every warp, reachable or not; boundary ports and warp-instance headers are out of this experimental scope |
| Point and absence openings | `WorldlineState::application_opening` and `verify_application_opening`, with a strict canonical codec |
| An independent complete-record rebuild | `ApplicationTrie::rebuild`, and `WorldlineState::application_root_rebuilt` |
| Exit: incremental and independent roots agree across ordinary and structural mutations, rollback, replay and forks; legacy roots identical | Tests below, plus the full suite with a per-commit parity assertion |

## Design

The trie follows the pattern Echo already uses for the retained state root:
*a commit retains what it computed; mutable access to the graph drops it.*

- `WorldlineState` holds an `ApplicationRootShadow`: an immutable,
  `Arc`-shared trie and a flag saying whether it describes the current graph.
  A version is a root handle, so cloning is constant time, old versions stay
  readable while held, and nothing needs a lock.
- `warp_state_mut` marks the trie not current. A commit that knows its write
  set advances it incrementally: the runtime commit in `SchedulerCoordinator`,
  `apply_patch_journaled` (executable-operation Actions), and
  `WorldlineTickPatchV1::apply_to_worldline_state` (replay). Anything else
  leaves it stale, and `application_root()` then answers by a full rebuild,
  so correctness never depends on a hook having run.
- An op that changes a whole warp instance has no slot-addressable write
  set; the advance rebuilds instead.
- Rollback markers, replay checkpoints and forks clone the field with the
  rest of the state, so a restored frontier gets back its old trie version.
- The runtime commit guard rebuilds `WorldlineState` from parts on every
  commit. It carries the trie through: marked not current on success (the
  coordinator advances it), unchanged on error (the graph is restored
  exactly).

## Evidence

**Unit and integration tests** (all with `application_root_shadow`):

| Test | Shows |
| --- | --- |
| `application_root::tests::incremental_updates_match_rebuilds_and_old_versions_survive` | 200 random batches of inserts, replacements and deletes over 300 keys; each version equals an independent build, and every old version still has its hash afterwards |
| `application_root::tests::openings_verify_and_refuse_mutations` | Membership and absence openings verify; every single-bit flip and an appended byte are refused; an absence opening does not transfer to a present key |
| `worldline_state::…::application_root_follows_ordinary_and_structural_patches` | Node, attachment and edge upserts, attachment replacement, edge deletion, node deletion with its attachment, portal opening, and warp-instance deletion (rebuild path), all through patches that declare nothing in `out_slots`; an untracked mutation leaves the trie stale and is still answered correctly; the legacy root is unchanged by unreachable records while the application root changes |
| `worldline_state::…::application_root_rolls_back_with_its_frontier` | A rollback marker restores the pre-attempt application root |
| `coordinator::…::application_root_tracks_runtime_commits_forks_and_replay` | Four contract commits through `super_tick` stay incremental and exact; provenance replay reproduces the live root; a forked strand's root equals the source's and its own rebuild |
| `application_root_shadow_tests::f5_application_root_is_incremental_exact_and_replayable` | F5 tiers S and M: the live root is incremental and equals its rebuild, replay reproduces it, and the legacy replay root still equals the live one |

**Full suite with per-commit parity.** With `delta_validate`, every advance
asserts that the incremental root equals a full rebuild. See Results.

**Benchmark.** The F5 `scene_fixtures` bench (tiers S and M, seed 7, five
rounds), same host, same branch, feature off and on:

| Tier | Feature | Append p50 | Per append | Replay p50 |
| --- | --- | --- | --- | --- |
| S | off | 14.64 ms | 0.0786 ms | 8.29 ms |
| S | on | 16.89 ms | 0.0911 ms | 10.25 ms |
| M | off | 5,530.93 ms | 1.0740 ms | 5,674.56 ms |
| M | on | 5,513.33 ms | 1.0714 ms | 5,564.61 ms |

In tier M the shadow costs less than run-to-run variation. In tier S it adds
about 12.5 µs per append (16%) and 24% to replay, where the legacy root is
cheap and the trie's fixed per-commit work shows.

The first measured version cost 9.4 ms per append in tier M (47 s per
round): the commit guard's `from_parts` rebuilt the trie on every commit.
A diagnostic run confirmed every coordinator advance was incremental, which
located the rebuild in the guard. Carrying the trie through the guard fixed
it.

## What M1 does not do

The legacy state root is still computed every tick; it is 93–96% of F5-M's
append time. M1 only shows that the replacement is exact and cheap to
maintain. Using it is M2: a versioned root/seal format, replay and recovery
of new-profile histories, and continued byte-for-byte verification of old
ones.

## Results

| Check | Result |
| --- | --- |
| Full `warp-core` suite, features `native_rule_bootstrap,trusted_runtime,host_test,application_root_shadow,delta_validate` (every advance asserted equal to a full rebuild) | 2,016 passed. No parity assertion fired. Two non-passes: the long-`TMPDIR` socket failure (`special_files_are_rejected_before_a_read_attempt`, also failing on `main`), and `application_root_shadow_tests`, which I stopped because tier M in a debug build with per-commit rebuilds was too slow ([results](m1-application-root/full-suite-delta-validate.txt)) |
| That test, rerun after limiting debug builds to tier S, with `delta_validate` | Passes ([output](m1-application-root/f5-test-debug-delta-validate.txt)) |
| The same test in release, tiers S and M | Passes |
| Unit tests (`--lib application_root`) | 5 passed |
| `cargo clippy -p warp-core --features native_rule_bootstrap,trusted_runtime,host_test,application_root_shadow --lib --tests -- -D warnings`, default-feature library clippy, `cargo fmt --all -- --check`, the three ban scripts | Pass |
| Bench, feature off / on / on before the guard fix | [off](m1-application-root/bench-feature-off.txt), [on](m1-application-root/bench-feature-on.txt), [before the fix](m1-application-root/bench-feature-on-before-guard-fix.txt) |

CI gains a test step (the unit tests and the F5 integration test with
`delta_validate`) and a clippy step for the feature.
