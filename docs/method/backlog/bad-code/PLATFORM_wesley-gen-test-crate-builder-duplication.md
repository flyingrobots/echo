<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# `wesley-gen` test consumer-crate builders are ~40-line copy-paste

Legend: `PLATFORM`

## The smell

`crates/echo-wesley-gen/tests/generation.rs` has four consumer-crate
builder functions:

- `write_basic_generated_crate`
- `write_consumer_smoke_crate`
- `write_contract_host_smoke_crate`
- `write_optic_binding_smoke_crate`

Each one writes a `Cargo.toml` + `src/lib.rs` + `src/generated.rs` and
uses essentially the same boilerplate:

- the same `[workspace]` isolation block
- the same `path = "{}"` dep declaration pattern for
  `echo-wasm-abi` and `echo-registry-api`
- the same `default-features = false, features = ["alloc"]` switching
  on a `no_std` flag
- the same `serde` dep with no_std/std variants
- the same `#![no_std] / extern crate alloc;` vs `mod generated;`
  lib.rs branching

All four were also keyed off `std::process::id()` for the crate dir
until the `chore/test-loop-speedup` PR landed; the PID segment is now
inert (the shared `CARGO_TARGET_DIR` made it moot) but still litters
the disk during a test run.

## Why this matters

- A new generated-output test path means a fifth copy of the same
  boilerplate. Easy to drift one config knob (a feature flag, a
  serde version, a path) silently.
- The PID-segment leftovers create + remove directories every test
  for no benefit now that the target dir is shared elsewhere.
- The PR #383 speedup work touched all four sites identically when
  adding `.env("CARGO_TARGET_DIR", ...)` — every future change to
  the harness will pay the same cost.

## Fix shape

Extract a single `ConsumerCrateBuilder` (or a free function with an
options struct) that owns:

- the `[workspace]` boilerplate
- the dep declarations (no_std vs std switching)
- the lib.rs shim selection
- the crate-dir layout (dropping the PID segment changes path
  semantics and is safe only if deterministic unique labels or
  explicit tempdir guarantees preserve parallel-test isolation)

Each call site then declares only what's unique to it: the label,
the no_std flag, the generated source, and any extra deps.

This is a structural cleanup whose path-layout change is
behavior-preserving only when uniqueness and isolation invariants
are retained. Dropping the PID segment can otherwise change
collision behavior under parallel runs or label reuse. After the
extraction, adding a fifth generated-output test path should be a
~5-line call, not a ~40-line copy of the previous one.

## Out of scope here

- Sharing `CARGO_TARGET_DIR` (already landed in PR #383).
- The `[workspace]` isolation itself, which is required because the
  generated crates must not pull in the parent workspace's
  `Cargo.lock`. Keep that constraint.
