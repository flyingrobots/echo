<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Frozen legacy causal WAL

A filesystem WAL written by Echo's own writer **before the separated causal
identity recipe existed** (#885). Its causal requests are retained as
`EINGR002` and `EINGR003` and identified by the original recipe,
`"ingress:causal:v2\0" || ...`. Readers must keep accepting these bytes and
their identities exactly as written, and a resend of either request in the
separated form must resolve to its original submission.

**Do not regenerate these files with a newer writer.** They exist to prove that
a reader accepts what an earlier writer actually wrote; bytes produced by the
current encoder cannot show that.

| File                                          | Bytes | SHA-256                                                            |
| --------------------------------------------- | ----- | ------------------------------------------------------------------ |
| `segments/segment-00000000000000000001.ecwal` | 7594  | `95a762b44e674e7612ea74aff516034cdab192c8700c6c75d0a76b6386894457` |
| `writer-epochs.ecwal`                         | 231   | `bdef26980af969e059f52c3a212ade94cc26329083020b8bd1119313877efc61` |

## Provenance

Written on 2026-10-16 by `main` at `0512e9d2` (after #884, before #885), in the
guarded container, by a throwaway test added to
`crates/warp-core/tests/trusted_runtime_host_loop_tests.rs` and not committed:

- the reference host (`runtime()`, `empty_engine()`) with a filesystem WAL and
  the reference contract package;
- one WAL-acknowledged causal EINT intent, `causal_eint_envelope(lane,
[causal_receipt_ref(lane, "legacy-parent", 41)])`, staged with
  `admission_ticket(61)` and decided by `run_until_idle(4)`: submission
  `2348e63b4cd2b630e659d9bcac2e5456d56c35c7a5f04f717d07820a113888f7`, ingress
  `811015cf74795d042fc637d41d506024f538206586707d533fede984a2dddcbf`;
- one WAL-acknowledged causal Continuum occurrence, `pack_intent_v1(MUTATION_OP_ID,
b"amount=9")` with the same parent and `ContinuumOccurrence::new([7; 32],
[0xa7; 32])`, left pending: submission
  `8879b31036123e5b70962a068958f51e379750627e13abecf5b585d43fdd3f63`, ingress
  `4af4cf66ff6f479386420b7d1df87e762ee5964323d402b3f378d007381acf8e`.

The runtime lock file `writer-epoch.lock` is not retained; the WAL recreates it.

Used by `causal_requests_accepted_before_the_separated_recipe_survive_restart_and_resend`.
