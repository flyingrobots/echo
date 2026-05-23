<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Quickstart: Local Contract Host

Status: v0.1.0 executable quickstart baseline.

This quickstart shows the current local contract-host path. It is not a product
app, not a daemon, and not distributed Continuum transport. It is the smallest
release-grade proof that Echo can host a generated contract locally while
preserving the authority boundary:

```text
application submits
trusted runtime host authorizes scheduler opportunities
scheduler-owned ticks emit receipts
application observes outcomes and readings
```

## Run The Witness

From a clean checkout:

```bash
cargo xtask test-slice contract-path-release --dry-run
cargo xtask test-slice contract-path-release
```

The dry run prints the exact Cargo targets. The real run executes:

- installed contract pipeline replay;
- reference trusted runtime host loop;
- serious external-consumer-shaped contract fixture.

## What The Witness Proves

The witness covers the current v0.1.0 local contract path:

1. Install a generated-style package through the package boundary.
2. Submit canonical EINT bytes through an app-facing handle.
3. Keep the submission pending until trusted runtime-owned staging.
4. Stage ticketed runtime ingress through the trusted host.
5. Run scheduler-owned ticks until idle.
6. Observe applied and rejected intent outcomes.
7. Query a bounded `QueryView` reading through a read-only observer.
8. Retain reading payload and receipt evidence through semantic coordinates.
9. Replay the local installed pipeline to the same receipt and outcome.

## Application Surface

Application code should use the app-facing surface:

```rust
let mut app = host.app();
let submission = app.submit_intent(envelope)?;
let outcome = app.observe_intent_outcome(&submission.submission_id);
let reading = app.observe(query_request)?;
```

The app surface does not expose:

- package installation;
- ticketed runtime ingress staging;
- `super_tick`;
- scheduler pass or run-until-idle control;
- scheduler fault recovery authority.

## Trusted Host Surface

The trusted runtime owner uses the host surface:

```rust
host.install_contract_package(package)?;
host.stage_installed_contract_submission(submission.submission_id, &ticket)?;
host.run_until_idle(4)?;
```

That host role owns scheduler control. Wall-clock cadence is host policy; fixed
logical ticks are Echo semantic history.

## Compatibility Boundary

Generated packages must fit the runtime before they install. The package
boundary verifies:

- Echo contract ABI version;
- Wesley generator version;
- contract-host helper API version;
- registry layout version;
- codec id;
- schema hash;
- footprint certificate identity.

Unsupported compatibility fails closed at package install. It does not become
runtime-visible work or an accepted read.

## Retention Boundary

Retention uses semantic coordinates above content-only CAS:

```text
CAS hash = bytes
semantic coordinate = question those bytes answer
```

Missing retained material is an obstruction, not an empty successful reading.

## Non-Goals

- No streaming subscriptions.
- No hidden retry queue.
- No distributed replica import.
- No full observer-rights revelation lattice.
- No application-created ticks or `TickReceipt` values.
