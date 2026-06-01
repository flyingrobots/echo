<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# xtask main.rs is a god file

Status: active bad-code note. `xtask/src/main.rs` is still the only source file
under `xtask/src/` and is roughly 7.8k lines; argument definitions, command
dispatch, GitHub/PR helpers, benchmark tooling, DIND tooling, docs linting, and
Method formatting are still mixed together.

`xtask/src/main.rs` is a single file with all subcommands inlined.
It's past the point where this is reasonable. Extract into modules:
`bench.rs`, `dags.rs`, `pr.rs`, `dind.rs`, `docs.rs`, `method.rs`.

The method integration is already thin (just CLI parsing + formatting)
but the rest of the subcommands have substantial logic mixed with
argument definitions.
