<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Experimental application root

The `application_root_shadow` feature keeps an incremental authenticated
application reading beside the legacy state root. It is experimental and off
by default. Enabling it changes no state root, receipt, patch, WAL or wire
format. It does not select a new commitment profile for admitted histories;
the separate `application_root_profile` feature does, per worldline, as
described in [Experimental commitment profile](#experimental-commitment-profile).
The legacy commitment contract remains in
[merkle-commit.md](../spec/merkle-commit.md).

## Scope and identity

The profile identifier is `echo.application-root.radix.v0`. The reading covers
every node, edge and attachment record in every stored warp, including
unreachable records. Boundary ports and warp-instance headers are excluded.
Outgoing-edge ranges describe sets of edge records, each of which has its own
leaf; a range is not a separate record leaf.

`application_record_key` hashes the record domain, kind and slot identity.
`application_record_bytes` encodes node types, edge endpoints and types, and
atom or portal attachments. `application_value_digest` hashes those bytes in
the value domain. The compressed binary radix trie hashes leaves and branches
under separate domains; each branch records the first differing key bit.
The key set determines the trie shape. The exact byte definitions and proof
codec are owned by
[`application_root.rs`](../../crates/warp-core/src/application_root.rs).

Immutable trie nodes share storage through `Arc`. Cloning a root handle takes
constant time and retains the earlier version while that handle is held.
`application_opening` produces membership or absence evidence for a slot;
`verify_application_opening` validates its canonical encoding against the
supplied root and slot, returning the value digest for membership or `None`
for absence. A consumer can compare that digest with the digest of its record
bytes. These openings prove this experimental reading, not admission or
causal-history validity.

## Maintaining the reading

Mutable graph access marks the retained trie stale. `application_root()`
answers a stale reading by a complete rebuild. A commit with a known write
set advances the trie using declared `out_slots` plus every slot written by
its operations, as defined in [generated rule authorship](GeneratedRules.md).
Changing a whole warp instance uses a rebuild.

The runtime coordinator, executable-operation patch application and retained
patch replay update the reading at their commit boundaries. Rollback markers,
replay checkpoints and forks retain the corresponding root handle. The runtime
commit guard preserves that handle across reconstruction of the worldline
state. `application_root_is_incremental()` exposes whether the retained trie
is current; `application_root_rebuilt()` independently rebuilds its root.

With `delta_validate`, each incremental advance asserts agreement with a
complete rebuild. Benchmark validation also requires recorded incremental/rebuild
and live/replay parity to both succeed. Missing or failed parity makes a
completed round invalid even when `delta_validate` is disabled, so the
[scene-fixture runner](BenchmarkSceneFixtures.md) cannot summarize that round
as a valid timing.

## Executable witnesses

- `application_root` unit tests cover incremental updates against independent
  builds, retained old versions, openings and malformed evidence.
- `worldline_state` tests cover ordinary and structural patches, untracked
  mutation fallback, and rollback.
- The coordinator test covers runtime commits, replay and a forked strand.
- `application_root_shadow_tests` checks F5 incremental/rebuild and replay
  parity, plus legacy-root replay agreement. Debug builds run tier S; release
  builds run tiers S and M.
- `scene_fixture_tests` includes canaries for missing or failed parity and is
  scheduled with the shadow feature without `delta_validate`.

These shadow witnesses do not establish a production commitment format.
WAL and wire formats retain their existing meaning.

## Experimental commitment profile

The `application_root_profile` feature (experimental, off by default, implies
`application_root_shadow`) lets a worldline commit its application reading
instead of the legacy reachable-state root. `CommitmentProfile` is either
`LegacyReachableV1` (`echo.state-root.legacy-reachable.v1`, the default) or
`ApplicationRadixV0` (`echo.state-root.application-radix.v0`). The profile is
chosen with `WorldlineState::with_commitment_profile` before the worldline is
registered. Nothing infers a profile from a root's bytes.

An `ApplicationRadixV0` worldline commits
`H("echo.state-root.application-radix.v0" || application_root)` as its
`state_root`. The tag separates this value from every legacy root, so no
historical `state_root` is reinterpreted. Commit hash v2 and v3 recipes are
unchanged; they bind whichever root the profile commits. A
`LegacyReachableV1` worldline's roots, commit hashes and receipts are
unchanged, and with the feature off every path below uses the legacy recipe.

The profile travels with the retained trie, so rollback markers, replay bases
and checkpoints, forks and the runtime commit guard carry it. The runtime
commit advances the trie from the commit's complete write set and commits its
tagged root without walking the graph. Replay, readings at earlier
coordinates, import candidates, playback cursors, provenance initial
boundaries and checkpoint restores verify under the worldline's own recipe
(`recomputed_state_root`, `state_root_of`). A history replayed or recovered
under the other profile fails as a root mismatch.

Limits of this profile:

- It is part of the host's worldline registration and is not written to the
  WAL, provenance codecs or WSC. A mismatch is a root mismatch, not a typed
  profile error; a durable descriptor would need a codec version and owner
  approval of the domain strings.
- It binds only the application reading. It is not a boundary seal over the
  history root, and ports and warp-instance headers remain outside the
  reading.
- There is no migration from a legacy history and no cross-host exchange
  witness.

Witnesses: the coordinator test that commits, replays and forks under the
profile; `application_root_profile_tests` (F5 validates under both profiles,
which commit different roots); `trusted_runtime_host_loop_tests` (WAL recovery
succeeds only under the registering profile); and
`executable_operation_pipeline_tests` (Actions commit and recover under the
profile, including stale-basis corroboration of the genesis origin).
