<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# PR #141 — Comment Burn-Down

PR: [#141](https://github.com/flyingrobots/echo/pull/141)

## Purpose & Retention

This file is a PR-scoped, action-oriented index of review threads → fixing SHAs.

- Canonical design decisions belong in `docs/decision-log.md`.
- After PR #141 merges, this file may be deleted or moved to `docs/legacy/` if it remains useful as a historical artifact.

## Snapshot (2025-12-28)

- Head branch: `echo/wasm-spec-000-scaffold`
- Base branch: `main`
- Head commit (at last update): `46bc079`
- Latest CodeRabbit review commit: `933239a` (review submitted 2025-12-28)

### Extraction (paginated, per EXTRACT-PR-COMMENTS procedure)

```bash
gh api --paginate repos/flyingrobots/echo/pulls/141/comments > /tmp/pr141-review-comments.json
gh api --paginate repos/flyingrobots/echo/issues/141/comments > /tmp/pr141-issue-comments.json
```

- PR review comments (inline): 96 total
  - Top-level: 61
  - Replies: 35
- Issue comments (conversation): 1 (CodeRabbit rate-limit / other-bot note; non-actionable)

## Buckets (Top-Level Review Comments)

Notes:

- `P0` == CodeRabbit “🔴 Critical” (blockers).
- Many comments are “stale” in GitHub terms (carried forward across commits); each item below was verified against current code/docs before action.
- Some CodeRabbit comments include a built-in “✅ Confirmed …” marker; many do not. This file is the canonical burn-down record for PR #141.

### P0 — Blockers

- [x] [r2645857657](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857657) `crates/echo-wasm-bindings/src/lib.rs` — Only log rewrites for successful mutations (no-op history is a semantic violation). Fixed in `7825d81`.
- [x] [r2645857663](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857663) `crates/echo-wasm-bindings/src/lib.rs` — Prevent dangling edges: validate `from`/`to` nodes exist before connecting. Fixed in `7825d81`.
- [x] [r2645857667](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857667) `crates/echo-wasm-bindings/src/lib.rs` — Do not record `DeleteNode` rewrites when the node does not exist. Fixed in `7825d81`.
- [x] [r2645857670](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857670) `crates/echo-wasm-bindings/src/lib.rs` — Remove `.unwrap()` from WASM boundary; avoid panics and deprecated serde helpers. Fixed in `7825d81`.

- [x] [r2612251496](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251496) `docs/METHODOLOGY.md` — Remove/clarify phantom crate (`crates/echo-kernel`) in the methodology diagram. Fixed in `cfe9270`.
- [x] [r2612251499](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251499) `docs/METHODOLOGY.md` — Mark hosted spec domains and completion-hash certification as planned (not implemented yet). Fixed in `cfe9270`.
- [x] [r2612251505](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251505) `docs/METHODOLOGY.md` — Definition of Done must include the repo’s quality gates (tests, docs, clippy, docs-guard, SPDX, fmt). Fixed in `cfe9270` + `641e482`.

- [x] [r2645857677](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857677) `docs/decision-log.md` — Remove duplicate decision-log row (keep the authoritative combined entry). Fixed in `641e482`.
- [x] [r2645857683](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857683) `docs/jitos/spec-0000.md` — Fix incorrect `crate::rmg_core::*` example imports (use external `rmg_core` crate paths). Fixed in `cf286e9`.
- [x] [r2612251514](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251514) `docs/tasks.md` — Remove duplicate contradictory task entries. Fixed in `cfe9270`.

- [x] [r2645857694](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857694) `specs/spec-000-rewrite/Cargo.toml` — CodeRabbit claimed `edition = "2024"` is invalid; it is valid under the repo toolchain (`rust-toolchain.toml` pins Rust 1.90.0) and the crate declares `rust-version = "1.85.0"` (see `f70ba94`). No code change required.

### P1 — Major

- [x] [r2649699435](https://github.com/flyingrobots/echo/pull/141#discussion_r2649699435) `crates/echo-session-ws-gateway/src/main.rs` — Add negative tests for frame parsing (partial header/body, too-small, payload-too-large). Fixed in `46bc079`.
- [x] [r2649699436](https://github.com/flyingrobots/echo/pull/141#discussion_r2649699436) `crates/echo-wasm-abi/src/lib.rs` — Remove vestigial `#[serde_as]` usage (no annotations present). Fixed in `46bc079`.
- [x] [r2649699438](https://github.com/flyingrobots/echo/pull/141#discussion_r2649699438) `crates/echo-wasm-bindings/README.md` — Document API surface with explicit type signatures. Fixed in `46bc079`.

- [x] [r2612251468](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251468) `crates/echo-session-client/src/lib.rs` — Classify protocol errors by code so session-level errors become `Global` notifications. Fixed in `12ecd95`.
- [x] [r2612251472](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251472) `crates/echo-session-ws-gateway/Cargo.toml` — Upgrade `axum`/`axum-server` to compatible, modern versions. Fixed in `89c2bb1`.
- [x] [r2612251488](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251488) `crates/echo-session-ws-gateway/src/main.rs` — Don’t swallow task errors; improve logging for debuggability. Fixed in `89c2bb1`.
- [x] [r2612251492](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251492) `crates/echo-session-ws-gateway/src/main.rs` — DRY: factor duplicate frame-length arithmetic into a helper. Fixed in `89c2bb1`.
- [x] [r2612251482](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251482) `crates/echo-session-ws-gateway/src/main.rs` — Cap the frame accumulator to prevent DoS via malformed streams. Fixed in `89c2bb1`.

- [x] [r2645857640](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857640) `crates/echo-wasm-abi/Cargo.toml` — Declare MSRV for edition-2024 crates. Fixed in `2431e9f`.
- [x] [r2645857649](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857649) `crates/echo-wasm-abi/src/lib.rs` — Expand rustdoc: intent, invariants, and examples for public types. Fixed in `2431e9f`.
- [x] [r2645857654](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857654) `crates/echo-wasm-bindings/src/lib.rs` — Expand `DemoKernel` rustdoc to document intent and invariants. Fixed in `95f8eda` (and tightened in `7825d81`).

- [x] [r2645857687](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857687) `docs/jitos/spec-0000.md` — Replace deprecated serde-on-`JsValue` helpers; keep WASM boundary panic-free. Fixed in `7825d81` + `cf286e9`.

### P2 — Minor

- [x] [r2649699430](https://github.com/flyingrobots/echo/pull/141#discussion_r2649699430) `crates/echo-session-client/src/lib.rs` — Strengthen test to assert full notification structure (kind/title/body), not just scope. Fixed in `46bc079`.
- [x] [r2649699439](https://github.com/flyingrobots/echo/pull/141#discussion_r2649699439) `crates/echo-wasm-bindings/src/lib.rs` — Make `add_node` a no-op on duplicate ids to avoid clobbering + semantic ambiguity; add regression test. Fixed in `46bc079`.
- [x] [r2649699447](https://github.com/flyingrobots/echo/pull/141#discussion_r2649699447) `docs/notes/pr-141-comment-burn-down.md` — Replace bare URL with a Markdown link (MD034). Fixed in `46bc079`.
- [x] [r2649699453](https://github.com/flyingrobots/echo/pull/141#discussion_r2649699453) `docs/notes/pr-141-comment-burn-down.md` — Capitalize “Markdown” (proper noun). Fixed in `46bc079`.
- [x] [r2649699463](https://github.com/flyingrobots/echo/pull/141#discussion_r2649699463) `specs/spec-000-rewrite/Cargo.toml` — Replace invalid categories (keep `wasm`, swap out `gui`/`education` for valid crates.io slugs). Fixed in `46bc079`.
- [x] [r2649699470](https://github.com/flyingrobots/echo/pull/141#discussion_r2649699470) `specs/spec-000-rewrite/spec.md` — Fix MD022 (blank line after headings). Fixed in `46bc079`.

- [x] [r2612251521](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251521) `README.md` — Remove trailing whitespace / tighten formatting. Fixed in `cf286e9`.
- [x] [r2645857690](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857690) `README.md` — Add alt text to images. Fixed in `cf286e9`.
- [x] [r2612251524](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251524) `README.md` — Resolve Markdown formatting nits in the referenced section. Fixed in `cf286e9`.

- [x] [r2612251540](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251540) `WASM-TASKS.md` — Fix heading spacing. Fixed in `6238c98`.
- [x] [r2612251473](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251473) `crates/echo-session-ws-gateway/README.md` — Add missing blank lines around headings/fences. Fixed in `6238c98`.
- [x] [r2612251477](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251477) `crates/echo-session-ws-gateway/src/main.rs` — Add a timeout to UDS connect to avoid hanging forever. Fixed in `89c2bb1`.

- [x] [r2645857679](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857679) `docs/execution-plan.md` — Add verifiable evidence pointers (commit SHAs / branch notes) to completion claims. Fixed in `641e482`.
- [x] [r2645857680](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857680) `docs/jitos/spec-0000.md` — Improve Markdown spacing/readability (MD022). Fixed in `cf286e9`.
- [x] [r2612251509](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251509) `docs/spec-concurrency-and-authoring.md` — Add missing blank lines around fences. Fixed in `6238c98`.
- [x] [r2612251512](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251512) `docs/spec-concurrency-and-authoring.md` — Clarify that `echo::delay()`/`echo::emit()` are Echo host functions (not built-in Rhai). Fixed in `6238c98`.

### P3 — Trivial

- [x] [r2649699428](https://github.com/flyingrobots/echo/pull/141#discussion_r2649699428) `crates/echo-session-client/src/lib.rs` — Extract the `>= 400` scope threshold into a named constant. Fixed in `46bc079`.
- [x] [r2649699432](https://github.com/flyingrobots/echo/pull/141#discussion_r2649699432) `crates/echo-session-ws-gateway/README.md` — Add language to fenced code blocks (MD040). Fixed in `46bc079`.
- [x] [r2649699434](https://github.com/flyingrobots/echo/pull/141#discussion_r2649699434) `crates/echo-session-ws-gateway/src/main.rs` — Add rustdoc for JS-ABI constants (frame structure intent). Fixed in `46bc079`.
- [x] [r2649699437](https://github.com/flyingrobots/echo/pull/141#discussion_r2649699437) `crates/echo-wasm-abi/src/lib.rs` — Broaden serialization tests to cover all `SemanticOp` variants. Fixed in `46bc079`.
- [x] [r2649699441](https://github.com/flyingrobots/echo/pull/141#discussion_r2649699441) `crates/echo-wasm-bindings/tests/api_tests.rs` — “Edge-case coverage significantly improved” (ack; no action required). Fixed in `46bc079`.
- [x] [r2649699442](https://github.com/flyingrobots/echo/pull/141#discussion_r2649699442) `docs/jitos/spec-0000.md` — Remove interactive “Which one do you want me to generate next?” prompt from the spec doc. Fixed in `46bc079`.
- [x] [r2649699444](https://github.com/flyingrobots/echo/pull/141#discussion_r2649699444) `docs/notes/pr-141-comment-burn-down.md` — Explain relationship to `docs/decision-log.md` and define a retention policy. Fixed in `46bc079`.
- [x] [r2649699466](https://github.com/flyingrobots/echo/pull/141#discussion_r2649699466) `specs/spec-000-rewrite/index.html` — Add an explicit note about keeping CSS inline for Phase 0 (extraction planned later). Fixed in `46bc079`.
- [x] [r2649699471](https://github.com/flyingrobots/echo/pull/141#discussion_r2649699471) `WASM-TASKS.md` / `docs/tasks.md` — Add automated enforcement for “task lists must not contradict themselves” (pre-commit + CI). Fixed in `46bc079`.

- [x] [r2612251483](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251483) `crates/echo-session-ws-gateway/src/main.rs` — Avoid immediate ping tick (let handshake settle). Fixed in `89c2bb1`.
- [x] [r2645857635](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857635) `crates/echo-session-ws-gateway/src/main.rs` — Log rejected Origin values for debugging. Fixed in `89c2bb1`.

- [x] [r2645857642](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857642) `crates/echo-wasm-abi/Cargo.toml` — Pin dependencies to minor versions for reproducibility. Fixed in `2431e9f`.
- [x] [r2645857643](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857643) `crates/echo-wasm-abi/README.md` — Fix heading spacing. Fixed in `2431e9f`.

- [x] [r2645857651](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857651) `crates/echo-wasm-bindings/README.md` — Fix Markdown formatting / align exposed API docs. Fixed in `cf286e9`.
- [x] [r2645857656](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857656) `crates/echo-wasm-bindings/src/lib.rs` — Reorder ops to mutate, then log (future-proof history consistency). Fixed in `95f8eda`.
- [x] [r2645857675](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857675) `crates/echo-wasm-bindings/tests/api_tests.rs` — Add tests for error/no-op paths and boundary conditions. Fixed in `7825d81`.

- [x] [r2612251529](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251529) `specs/spec-000-rewrite/index.html` — Remove orphaned `#app` mount node. Fixed in `f70ba94`.
- [x] [r2645857695](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857695) `specs/spec-000-rewrite/spec.md` — Replace “to add” with an explicit Phase-0 win condition. Fixed in `cf286e9`.
- [x] [r2612251537](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251537) `specs/spec-000-rewrite/src/lib.rs` — Remove redundant `#[allow(missing_docs)]` when the item is documented. Fixed in `f70ba94`.
- [x] [r2612251535](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251535) `specs/spec-000-rewrite/src/lib.rs` — Same redundancy: doc comment + `#[allow(missing_docs)]`. Fixed in `f70ba94`.

### PX — Agent Artifacts (Codex connector bot)

- [x] [r2612244537](https://github.com/flyingrobots/echo/pull/141#discussion_r2612244537) Backend disconnect should stop ping loop. Fixed earlier in `970a4b5` (and refined in `89c2bb1`).
- [x] [r2612244530](https://github.com/flyingrobots/echo/pull/141#discussion_r2612244530) Gate Spec-000 wasm entrypoint/deps so host builds stay green. Fixed earlier in `2fec335`.
