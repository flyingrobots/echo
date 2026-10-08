# Reproduce the projected CAS compiler witness

Run `build.py` inside the guarded Echo worker with a pinned Edict compiler binary,
its SHA-256, the candidate provider package and a fresh output directory. The
recipe invokes the public JSONL application-build command; it does not synthesize
Core, Target IR, a package or a verifier report.

`seed/` preserves the canonical hello-echo lawpack inputs from Edict commit
`3f81f759e921a69b04fe8cf8e62e62f8f3dc7b7e`. The recipe deterministically derives a
CAS lawpack with an explicit `Bytes<exact=32>` expected digest, replace write class,
CAS intrinsic and input binding. It recomputes the affected export, adapter and
manifest references, then fills the authored source template's lawpack digest.
Keep BlobIds are not used as attachment-value digests.

The retained build used the pinned Edict compiler from source commit
`2405a550e93e1e97fff640caa44bbd0f65ffff3c` and executable SHA-256
`3b082d61c8cd23b0f917efb55c4eebd54e75c0df5d31c72f90ad856853678bf5`.
`built/` preserves the authored source with its exact digest, the public compiler's
successful status, and exact package/report bytes as lowercase hex. The report
accepts this package. This fixture proves compiler/provider production.
`crates/warp-core/tests/edict_projected_cas_tests.rs` uses these exact bytes for
separate scheduler and fresh-host WAL recovery witnesses, including pending
Actions, committed results and stale-digest obstructions. It checks typed target
bytes directly because the fixture target is detached from the reachable root.
