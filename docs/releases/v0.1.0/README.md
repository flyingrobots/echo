<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo v0.1.0 Release Feature Inventory

Status: release feature inventory.

This document enumerates the feature set intended for Echo `v0.1.0`. It is not
the final tag announcement and does not replace the release plan, backlog lane,
or executable release witnesses.

Primary references:

- [Echo v0.1.0 release plan](../../design/v0.1.0-release-plan.md)
- [Current bearing](../../BEARING.md)
- [Local contract host quickstart](../../quickstart-local-contract-host.md)
- [v0.1.0 backlog lane](../../method/backlog/v0.1.0/)

## Release Definition

Echo `v0.1.0` is the first usable deterministic local contract-host release.

The release bar is:

```text
A developer can build and run a small Wesley-compiled Echo application locally,
without privileged tick authority, and can submit intents, observe outcomes,
query bounded readings, retain evidence, and replay the result deterministically
using documented, versioned APIs.
```

This release is scoped to local WARP contract hosting. It does not claim the
full Continuum transport system, settlement shells, adversarial replica import,
protected social lane policy, or the full observer-rights revelation lattice.

## Authority Guarantees

These guarantees are release features, not implementation details:

- application code cannot tick;
- application code cannot access trusted runtime control;
- submit/admit APIs do not execute synchronously;
- `AdmissionTicket` is not `TickReceipt`;
- `AdmissionTicket` is not execution;
- query observers are read-only;
- mutation handlers run only during scheduler-owned execution;
- retry is explicit new causal input, not hidden runtime behavior;
- wall-clock cadence is host/runtime-owner policy, not semantic history.

## Feature Inventory

| Feature                             | Release status                 | What ships                                                                                                                                                                                   |
| :---------------------------------- | :----------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Deterministic runtime kernel        | Complete                       | `WorldlineRuntime`, `SchedulerCoordinator::super_tick(...)`, and `Engine::commit_with_state(...)` provide scheduler-owned deterministic execution.                                           |
| Runtime-owned ticks                 | Complete                       | Application ingress never commands scheduler ticks. Trusted runtime control owns tick cadence and until-idle policy.                                                                         |
| Witnessed intent submission         | Release feature                | Accepted app ingress becomes Echo-owned submission history with stable identity and duplicate posture. Durable host storage remains the release-hardening target.                            |
| Admission evidence ladder           | Complete local proof           | Basis, aperture, budget, runtime support, invocation admission, scheduler admission, scheduler work candidate, law witness, and `AdmissionTicket` are distinct evidence steps.               |
| Ticketed runtime ingress            | Complete local proof           | Admitted work enters runtime ingress only through explicit runtime-owner authority and does not tick synchronously.                                                                          |
| Installed contract package registry | Complete local proof           | Generated contract packages install as bounded artifacts with package identity, schema hash, artifact hash, codec identity, supported operation ids, mutation handlers, and query observers. |
| Versioned package compatibility     | Complete local proof           | Package install rejects Echo ABI, Wesley generator, helper API, codec, registry layout, schema, artifact, and footprint drift before runtime-visible work or reads.                          |
| Scheduler-owned mutation dispatch   | Complete local proof           | Installed mutation handlers execute only during scheduler-owned ticks and never through application dispatch.                                                                                |
| Footprint conflict handling         | Complete local proof           | Conflicts are explicit tick outcomes with blocker attribution. Retry remains a new explicit causal act.                                                                                      |
| Receipt correlation                 | Complete local proof           | Scheduler-owned tick receipts correlate back to ticketed ingress, admission ticket digest, and witnessed submission id.                                                                      |
| Intent outcome observation          | Complete local proof           | Core can report unknown, pending, applied, rejected, and obstructed intent outcomes without exposing scheduler control.                                                                      |
| QueryView observer bridge           | Complete local proof           | `QueryView` / `Query` observations route to installed contract query observers when a matching observer is registered.                                                                       |
| Wesley mutation helpers             | Complete local proof           | `echo-wesley-gen --contract-host` emits std-only mutation rule helpers for the contract-host seam.                                                                                           |
| Wesley query observer helpers       | Complete local proof           | `echo-wesley-gen --contract-host` emits read-only typed query observer helper constructors.                                                                                                  |
| Contract-aware receipts             | Complete local proof           | Mutation receipt correlations can cite package id, schema hash, artifact hash, codec identity, operation id, and compatibility metadata.                                                     |
| Contract-aware readings             | Complete local proof           | `ReadingEnvelope` evidence for contract reads carries query package evidence and observer identity.                                                                                          |
| Bounded query readings              | Complete local proof           | Query readings can be identified by query id, vars digest, basis, aperture, observer plan, and installed contract evidence.                                                                  |
| Contract obstruction taxonomy       | Complete local boundary        | Unsupported operation, unsupported query, admission obstruction, runtime fault, missing retention, stale basis, residual reading, and budget-exceeded posture are named generically.         |
| Semantic retention above CAS        | Complete local proof           | `RetainedBlobIndex` and retained evidence refs separate byte identity from semantic contract coordinates.                                                                                    |
| Retained evidence lookup            | Complete local proof           | Contract artifacts, receipts, witness refs, reading payloads, reading envelopes, and observer artifacts can be named by retained evidence coordinates.                                       |
| Missing-retention posture           | Complete local proof           | Missing retained material returns typed posture instead of empty success or fake cache hits.                                                                                                 |
| Reference trusted host loop         | Complete local proof           | `TrustedRuntimeHost` owns package install, ingress staging, scheduler passes, until-idle policy, and observation service access.                                                             |
| App-safe local host handle          | Complete local proof           | `TrustedRuntimeApp` can submit intents and observe outcomes/readings without package-install, tick, ingress-staging, or fault-recovery authority.                                            |
| App-safe JS/WASM/browser surface    | Release feature                | `warp-wasm` is the active browser/WASM boundary. Published JS/WASM/browser packages must expose only app-safe submit/observe/read APIs and must not leak trusted runtime control.            |
| Runtime-local fault quarantine      | Complete runtime-local posture | Scoped internal faults quarantine the culprit head, while unrelated heads remain eligible. Unscoped faults block runtime work until trusted recovery.                                        |
| External consumer-shaped fixture    | Complete local proof           | A hot-text-shaped contract fixture proves mutation, overlap conflict, bounded `QueryView`, retained evidence, and replay without importing app nouns into Echo core.                         |
| Local replay witness                | Complete local proof           | `cargo xtask test-slice contract-path-release` proves the installed contract path, reference host loop, and serious external consumer fixture.                                               |
| Release quickstart                  | Initial executable baseline    | `docs/quickstart-local-contract-host.md` documents the local release witness and app/host authority split.                                                                                   |
| Determinism and performance gates   | Release gate                   | CI and local gates cover deterministic math, materialization determinism, DIND/replay paths, decoder security, reproducible WASM builds, rustdoc, clippy, and benchmark regression checks.   |

## Developer-Facing Path

At `v0.1.0`, the intended local developer path is:

1. Author a GraphQL contract.
2. Generate Rust helpers, codecs, registry metadata, mutation helpers, and query
   observer helpers with Wesley.
3. Install the generated contract package into Echo.
4. Submit canonical intent bytes through an app-safe API.
5. Let the trusted runtime host stage admitted work and run scheduler-owned
   ticks.
6. Observe the submitted intent as pending, applied, rejected, obstructed, or
   unknown.
7. Query bounded readings through read-only `QueryView` observers.
8. Inspect retained receipts, reading envelopes, witnesses, package identity,
   and retained payload refs.
9. Replay the local contract path deterministically.

## Browser And WASM Release Bar

The legacy `ttd-browser` crate is not part of the release surface. The browser
and WASM direction for `v0.1.0` is `warp-wasm` plus the shared
`echo-wasm-abi` boundary.

If JS/WASM/browser packages ship as part of `v0.1.0`, they must satisfy the same
authority bar as the Rust local host path:

- expose app-facing submit/observe/query APIs;
- hide trusted runtime control;
- hide scheduler tick/step/start/run-until-idle authority;
- preserve canonical intent and query encoding;
- preserve compatibility checks for generated packages;
- return evidence-bearing outcomes and readings rather than bare state;
- pass reproducible WASM build gates.

## Release Witnesses

The main local release witness is:

```bash
cargo xtask test-slice contract-path-release --dry-run
cargo xtask test-slice contract-path-release
```

The witness covers:

- installed contract pipeline replay;
- reference trusted runtime host loop;
- serious external-consumer-shaped contract fixture.

Broader release validation should also keep the existing CI gates green:

- deterministic math guards;
- materialization determinism;
- DIND replay checks;
- decoder security tests;
- reproducible WASM builds;
- rustdoc warnings;
- clippy lanes;
- scheduler benchmark regression gates.

## Explicit Non-Goals

The following are not `v0.1.0` release features:

- full Continuum replica transport/import;
- settlement shells and adversarial import;
- Verkle/IPA proof-carrying readings;
- full observer-rights/revelation governance;
- dynamic plugin loading;
- streaming subscriptions;
- hidden retry queues;
- full `jedit` product integration;
- Graft live automation;
- generic braid/counterfactual system;
- full social/speculative lane policy;
- durable scheduler fault provenance.

## Release Candidate Criteria

A `v0.1.0` release candidate exists when:

- all required feature clusters have merged tests;
- the external consumer proof passes on a clean checkout;
- the quickstart is executable from scratch;
- the authority-boundary audit is green;
- replay/DIND proof is green;
- package and version metadata are stable;
- JS/WASM/browser packages, if published, expose only app-safe authority;
- no P1 backlog item targets the `v0.1.0` feature bar.
