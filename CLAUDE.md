<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo — Agent Instructions

Follow [`AGENTS.md`](AGENTS.md). It is the single source for repository
architecture, knowledge ownership, Git safety, and the executable-claim loop.

## Build and test

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
cargo xtask
```

## Determinism

Canonicalize floating-point operations per
`docs/determinism/SPEC_DETERMINISTIC_MATH.md`. Deterministic paths may not use
global mutable state, uncontrolled randomness, system time, or unordered
containers. CI enforces these constraints.
