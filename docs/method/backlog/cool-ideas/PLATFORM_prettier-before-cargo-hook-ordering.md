<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Pre-commit hook should run prettier before cargo verify

Legend: `PLATFORM`

## What hurts

The PR #383 auto-stage fix already eliminated the most painful case
(prettier rewrites → abort → manual restage → cargo verify again from
scratch). But the underlying ordering is still backwards.

Current hook order:

1. cargo fmt check (auto-stage if rewrites)
2. toolchain pin check
3. PRNG / task list / lockfile guards
4. **`scripts/verify-local.sh pre-commit`** ← can take minutes cold
5. SPDX guard
6. **markdown prettier + lint** ← cheap, runs LAST

The asymmetry: the expensive cargo step runs first, and the cheap
prettier-and-markdownlint step runs last. If a commit only touches
markdown but a previous touch left the cargo cache cold, you pay the
full cargo verify cost for nothing. If your commit touches both Rust
and markdown and prettier rewrites the markdown, the auto-stage path
now means you only pay cargo once — but that one pay is still cold
because cargo ran before prettier had a chance to decide whether the
hook can short-circuit.

## The flip

Run the cheap checks first, expensive last:

1. toolchain / PRNG / lockfile guards (already cheap)
2. **prettier auto-format + markdownlint** ← move up
3. cargo fmt auto-stage
4. `scripts/verify-local.sh pre-commit`
5. SPDX guard

This costs nothing when cargo would have run anyway (the user is
committing Rust changes and the cache happens to be cold). It saves
the entire cargo cost when:

- the user is committing only markdown changes
- the user is committing markdown + Rust changes and prettier is
  going to fail in a way the user wants to see before they wait for
  cargo

It also makes the hook's failure modes more user-friendly: prettier
errors arrive in seconds instead of after a multi-minute cargo build.

## Risk

- The current order may be intentional if cargo fmt sometimes
  rewrites markdown-adjacent files (e.g. `include_str!` doctests).
  Worth a quick audit of which files cargo fmt touches before
  flipping.
- If prettier auto-stages markdown that's then expected by a
  downstream cargo check (unlikely, but possible if a markdown file
  is included via `include_str!`), the flip needs care.

## Expected delta

For markdown-only commits on a cold cargo cache: minutes → seconds.
For mixed Rust + markdown commits where prettier rewrites: same wall
time as today, but earlier visibility of formatting errors. For
Rust-only commits: no change.

## Related

- `docs/method/backlog/cool-ideas/PLATFORM_wesley-gen-test-loop-speedup.md`
  (the original speedup backlog, win #5 — this is the spiritual
  successor now that win #5 partially landed in PR #383).
