<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# PR #141 — Comment Burn-Down

PR: https://github.com/flyingrobots/echo/pull/141

## Snapshot (2025-12-28)

- Head branch: `echo/wasm-spec-000-scaffold`
- Base branch: `main`
- Latest head commit (at extraction): `83576fc`
- Extracted via `gh`:
  - PR review comments (inline): 30
  - Issue comments (conversation): 1 (CodeRabbit “other bot detected” note)
  - Reviews: 3 (2× CodeRabbit “CHANGES_REQUESTED”, 1× Codex connector “COMMENTED”)

## Buckets

> Notes:
>
> - `stale=true` means the comment was created on an earlier commit and carried forward by GitHub (verify current code state before acting).
> - `latest=false` means the comment is on an older commit (still actionable if the underlying issue remains).

### P0 — Blockers

- [ ] [r2612251496](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251496) docs/METHODOLOGY.md:70 — Phantom crate in methodology diagram: `echo-kernel` does not exist. (author coderabbitai[bot], commit 83576fc, stale=true, latest=true)
- [ ] [r2612251499](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251499) docs/METHODOLOGY.md:82 — The Completion Hash certification workflow and spec-001.jitos.dev domain are unimplemented aspirations, not current reality. (author coderabbitai[bot], commit 83576fc, stale=true, latest=true)
- [ ] [r2612251505](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251505) docs/METHODOLOGY.md:99 — Critical omission: Definition of Done ignores mandatory quality gates. (author coderabbitai[bot], commit 83576fc, stale=true, latest=true)
- [ ] [r2612251514](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251514) docs/tasks.md:10 — BLOCKER: Duplicate and contradictory task entries. (author coderabbitai[bot], commit 83576fc, stale=true, latest=true)

### P1 — Major

- [ ] [r2612251468](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251468) crates/echo-session-client/src/lib.rs:219 — Scope classification must distinguish session-level errors from decode failures using error codes. (author coderabbitai[bot], commit 83576fc, stale=true, latest=true)
- [ ] [r2612251472](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251472) crates/echo-session-ws-gateway/Cargo.toml:17 — Upgrade both axum and axum-server to compatible versions; axum 0.6 + axum-server 0.5 are severely outdated. (author coderabbitai[bot], commit 83576fc, stale=true, latest=true)
- [ ] [r2612244537](https://github.com/flyingrobots/echo/pull/141#discussion_r2612244537) crates/echo-session-ws-gateway/src/main.rs:0 — <sub><sub>![P1 Badge](https://img.shields.io/badge/P1-orange?style=flat)</sub></sub>  Handle backend disconnects to stop ping loop (author chatgpt-codex-connector[bot], commit 35b06d7, stale=false, latest=false)
- [ ] [r2612251488](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251488) crates/echo-session-ws-gateway/src/main.rs:0 — All task errors silently swallowed — debugging nightmare. (author coderabbitai[bot], commit 35b06d7, stale=false, latest=false)
- [ ] [r2612251482](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251482) crates/echo-session-ws-gateway/src/main.rs:179 — Unbounded accumulator — DoS vector if upstream sends malformed data. (author coderabbitai[bot], commit 83576fc, stale=true, latest=true)
- [ ] [r2612251492](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251492) crates/echo-session-ws-gateway/src/main.rs:314 — Duplicated frame-length calculation logic — DRY violation. (author coderabbitai[bot], commit 83576fc, stale=true, latest=true)
- [ ] [r2645857640](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857640) crates/echo-wasm-abi/Cargo.toml:6 — Edition 2024 requires Rust 1.85+ — MSRV not documented anywhere. (author coderabbitai[bot], commit 83576fc, stale=false, latest=true)
- [ ] [r2645857649](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857649) crates/echo-wasm-abi/src/lib.rs:85 — Public API rustdoc is superficial — missing invariants, usage guidance, and field semantics. (author coderabbitai[bot], commit 83576fc, stale=false, latest=true)
- [ ] [r2645857654](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857654) crates/echo-wasm-bindings/src/lib.rs:19 — Insufficient documentation for public API struct. (author coderabbitai[bot], commit 83576fc, stale=false, latest=true)
- [ ] [r2612244530](https://github.com/flyingrobots/echo/pull/141#discussion_r2612244530) specs/spec-000-rewrite/src/lib.rs:9 — <sub><sub>![P1 Badge](https://img.shields.io/badge/P1-orange?style=flat)</sub></sub>  Add wasm target gating/deps for spec-000 crate (author chatgpt-codex-connector[bot], commit 83576fc, stale=true, latest=true)

### P2 — Minor

- [ ] [r2612251521](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251521) README.md:11 — Fix trailing whitespace. (author coderabbitai[bot], commit 83576fc, stale=true, latest=true)
- [ ] [r2612251524](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251524) README.md:81 — Address markdown formatting issues. (author coderabbitai[bot], commit 83576fc, stale=true, latest=true)
- [ ] [r2612251540](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251540) WASM-TASKS.md:41 — Address markdown heading spacing throughout. (author coderabbitai[bot], commit 83576fc, stale=true, latest=true)
- [ ] [r2612251473](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251473) crates/echo-session-ws-gateway/README.md:15 — Markdown lint violations: missing blank lines around heading and code fence. (author coderabbitai[bot], commit 83576fc, stale=true, latest=true)
- [ ] [r2612251477](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251477) crates/echo-session-ws-gateway/src/main.rs:145 — No timeout on Unix socket connect — can hang indefinitely. (author coderabbitai[bot], commit 83576fc, stale=true, latest=true)
- [ ] [r2612251509](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251509) docs/spec-concurrency-and-authoring.md:33 — Minor: Add blank line before code fence. (author coderabbitai[bot], commit 83576fc, stale=true, latest=true)
- [ ] [r2612251512](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251512) docs/spec-concurrency-and-authoring.md:41 — Clarify that `echo::delay()` and `echo::emit()` are custom Rust-registered functions, not built-in Rhai constructs. (author coderabbitai[bot], commit 83576fc, stale=true, latest=true)

### P3 — Trivial

- [ ] [r2612251483](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251483) crates/echo-session-ws-gateway/src/main.rs:0 — Ping loop fires immediately on first tick — client gets ping before handshake settles. (author coderabbitai[bot], commit 35b06d7, stale=false, latest=false)
- [ ] [r2645857635](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857635) crates/echo-session-ws-gateway/src/main.rs:296 — Origin check is secure but consider logging rejected origins for debugging. (author coderabbitai[bot], commit 83576fc, stale=false, latest=true)
- [ ] [r2645857642](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857642) crates/echo-wasm-abi/Cargo.toml:17 — Loose version specifiers — pin to minor versions for reproducibility. (author coderabbitai[bot], commit 83576fc, stale=false, latest=true)
- [ ] [r2645857643](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857643) crates/echo-wasm-abi/README.md:13 — Missing blank lines around headings — degrades readability. (author coderabbitai[bot], commit 83576fc, stale=false, latest=true)
- [ ] [r2645857651](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857651) crates/echo-wasm-bindings/README.md:11 — Markdown formatting violations — heading spacing and trailing whitespace. (author coderabbitai[bot], commit 83576fc, stale=false, latest=true)
- [ ] [r2645857656](https://github.com/flyingrobots/echo/pull/141#discussion_r2645857656) crates/echo-wasm-bindings/src/lib.rs:55 — Consider operation ordering for future extensibility. (author coderabbitai[bot], commit 83576fc, stale=false, latest=true)
- [ ] [r2612251529](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251529) specs/spec-000-rewrite/index.html:0 — Orphaned `#app` div — Leptos mounts to `<body>`, not this element. (author coderabbitai[bot], commit 35b06d7, stale=false, latest=false)
- [ ] [r2612251537](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251537) specs/spec-000-rewrite/src/lib.rs:0 — Same redundancy: doc comment + `#[allow(missing_docs)]`. (author coderabbitai[bot], commit 35b06d7, stale=false, latest=false)
- [ ] [r2612251535](https://github.com/flyingrobots/echo/pull/141#discussion_r2612251535) specs/spec-000-rewrite/src/lib.rs:15 — Redundant `#[allow(missing_docs)]` — there's a doc comment right above it. (author coderabbitai[bot], commit 83576fc, stale=true, latest=true)

