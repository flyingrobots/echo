<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Footprint Honesty Rewrite Proof Slice

- Lane: `up-next`
- Legend: `PLATFORM`
- Rank: `1`

## Why now

Echo now closes the default public native rule seam and states that
user-authored rewrite logic must arrive as Wesley-compiled declarative IR.

That doctrine still needs one compile-checked proof slice showing that the
generated Rust boundary can actually prevent dishonest footprint use before
runtime.

## Hill

Echo consumes one Wesley-generated Rust rewrite boundary whose shape makes
undeclared graph access impossible or a compile-time failure.

The proof slice should stay narrow:

- one mutation rewrite with one declared footprint
- one valid implementation
- one invalid implementation that fails to compile

## Done looks like

- Echo can compile one proof slice against a Wesley-generated bounded Rust
  rewrite API
- the valid fixture compiles cleanly
- the invalid fixture fails because the generated surface does not expose an
  undeclared capability
- runtime footprint guards remain in place as second-line defense rather than
  the only honesty proof

## Repo Evidence

- `docs/invariants/DECLARATIVE-RULE-AUTHORSHIP.md`
- `crates/echo-wesley-gen`
- `docs/method/backlog/up-next/PLATFORM_continuum-proof-family-runtime-cutover.md`
- `scripts/tests/declarative_rule_authorship_invariant_test.sh`
