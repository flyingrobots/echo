<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Reference Trusted Runtime Host Loop

Status: implemented local reference boundary.

This packet records the reference local host loop for the v0.1.0 contract path.
The loop is deliberately boring: it names the trusted runtime-owner role that
already existed implicitly in tests and wires it through a small wrapper. It is
not a daemon, not wall-clock semantics, and not an application tick API.

## Claim

Application code can submit canonical intent material and observe outcomes or
readings through an app-facing handle, while the trusted host owns:

- generated package installation;
- ticketed runtime ingress staging;
- scheduler-owned tick passes;
- until-idle policy;
- query service access;
- future trusted fault recovery.

The app-facing handle exposes no package installation, no ticketed ingress
staging, no `super_tick`, no scheduler pass, and no fault recovery authority.

## Implemented Surface

`warp-core` now provides:

- `TrustedRuntimeHost`, gated behind the trusted runtime and native bootstrap
  features;
- `TrustedRuntimeApp`, the app-facing submit/observe/query handle;
- `TrustedRuntimeHost::install_contract_package(...)`;
- `TrustedRuntimeHost::stage_installed_contract_submission(...)`;
- `TrustedRuntimeHost::tick_once(...)`;
- `TrustedRuntimeHost::run_until_idle(...)`;
- `TrustedRuntimeHostRunReport`, which records scheduler passes and committed
  step count.

The host initializes provenance from registered runtime worldlines, owns the
engine and runtime, and uses existing `SchedulerCoordinator::super_tick(...)`
for scheduler-owned execution.

## Authority Boundary

```text
application
-> TrustedRuntimeApp::submit_intent(...)
-> witnessed submission handle

trusted runtime host
-> installs package
-> stages ticketed ingress
-> runs scheduler-owned ticks
-> app observes outcome or bounded query reading
```

The host loop does not make application dispatch synchronous. A submitted
intent remains pending until trusted runtime-owned ingress staging and
scheduler-owned tick execution decide it.

## Non-Goals

- No production daemon.
- No wall-clock cadence semantics.
- No hidden retry.
- No app-controlled tick, scheduler pass, or trusted recovery.
- No new admission law.
- No dynamic plugin loading.

## Evidence

- `cargo test -p warp-core --features "native_rule_bootstrap trusted_runtime" --test trusted_runtime_host_loop_tests`

The witness installs a generated-style package, submits through the app handle,
stages ticketed ingress through the host, runs until idle, observes an applied
intent outcome, and queries through the read-only observer service with package
evidence.
