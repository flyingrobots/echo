<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# JSON output mode for method crate

Add `--json` flag to `cargo xtask method status` (and future method
subcommands) so agents can consume structured output without parsing
plain text.

Requires adding `serde` + `serde_json` to the `method` crate.
