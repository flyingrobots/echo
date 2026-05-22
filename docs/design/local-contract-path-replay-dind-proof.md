<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Local Contract Path Replay And DIND Proof

Status: implemented narrow release witness.

This packet records the local v0.1.0 contract-path replay witness. The full DIND
suite remains the broader determinism gate. This slice adds a narrow,
documented `test-slice` entry that proves the local contract-host path without
requiring developers to run every deterministic scenario during normal
iteration.

## Claim

The v0.1.0 local contract path has a named release witness:

```bash
cargo xtask test-slice contract-path-release
```

That witness runs the explicit tests that prove:

- installed contract mutation dispatch happens only through scheduler-owned
  ticks;
- unsupported package operations do not become runtime-visible work;
- conflict rejection is final for that tick attempt;
- witnessed submissions replay with stable identity;
- the installed contract pipeline replays to the same receipt and outcome;
- a generic external fixture retains reading and receipt evidence;
- the reference trusted host loop owns tick authority;
- the serious external-consumer-shaped fixture exercises mutation, conflict,
  QueryView reading, and semantic retention.

## Boundary

This is a local release witness, not distributed replica proof. It does not
implement settlement shells, adversarial import, or the full observer-rights
revelation lattice.

The witness is intentionally explicit. It names exact Cargo test targets so
local iteration does not accidentally compile every integration test binary.

## Evidence

- `cargo test -p xtask test_slice_contract_path_release_stays_explicit`
- `cargo xtask test-slice contract-path-release --dry-run`
- `cargo xtask test-slice contract-path-release`

## Remaining Release Work

Later release work still needs the broader DIND/reproducibility gates to run in
CI and the release candidate, but this slice gives the local app contract path a
small, repeatable proof command.
