<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Frozen `EPRV0002` WAL

A filesystem WAL written by Echo's own writer **before commit binding existed**
(#873, #882). Its one scheduler tick is attributed but not commit-bound: the
retained tick record is `EPRV0002` and the tick's commit hash is v2. Readers
must keep accepting these bytes exactly as written.

**Do not regenerate these files with a newer writer.** They exist to prove that
a reader accepts what an earlier writer actually wrote; bytes produced by the
current encoder cannot show that.

| File | Bytes | SHA-256 |
| --- | --- | --- |
| `segments/segment-00000000000000000001.ecwal` | 4940 | `f9fd809e035f01563fb013eab5aac959ddd748837f690cb9a1a6ba97494edcc5` |
| `writer-epochs.ecwal` | 231 | `bdef26980af969e059f52c3a212ade94cc26329083020b8bd1119313877efc61` |

## Provenance

Written on 2026-10-15 by `main` at `f5c5abf1` (after #876, #877 and #878; before
#882), in the guarded container, by a throwaway test added to
`crates/warp-core/tests/provider_contract_admission_tests.rs`:

- the provider fixture host (`provider_runtime_host`) with a filesystem WAL;
- the default provider proposal (`proposal(RELEASE_DIGEST, PACKAGE_ARTIFACT_SHA256)`), admitted and installed;
- one WAL-acknowledged EINT intent, `pack_intent_v1(generated::OPERATION_ID, b"frozen-eprv0002")`, admitted with `admit_provider_contract_submission_v1`;
- `run_until_idle(4)`.

The submission ID it decided is
`e6f10fe39e0b55b4d4a0764cfed1794d885ea30a1541a1d263f1eb6d653c679b`. The runtime
lock file `writer-epoch.lock` is not retained; the WAL recreates it.

Used by `a_frozen_eprv0002_wal_recovers_extends_with_bound_ticks_and_restarts`.
