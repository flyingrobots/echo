<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Suspicious Repository Surfaces

For each of the following, please investigate:

1. What uses it?
2. Why is it here?
3. Is it needed?
4. Can it be removed?
5. Are there alternatives?
6. Are there any security concerns?
7. Are there any performance concerns?
8. Are there any maintainability concerns?
9. How would removing it affect the project's invariants?
10. How does it interact with other parts of the codebase?
11. Recommendation: keep, remove, or refactor

The following are probably trash:

.dx-debug/
.venv/
apps/ttd-app/
blog/

The following are suspicious:

crates/echo-config-fs/
crates/echo-dry-tests/
crates/echo-graph/
crates/echo-runtime-schema/
crates/echo-scene-codec/
crates/echo-scene-port/
crates/echo-session-proto/
crates/echo-session-ws-gateway/assets/vendor/

docs/.vitepress/
docs/archive/
docs/book/
docs/man/
docs/theory/

docs/macros.tex
docs/ref.bib

docs/workflows.md

node_modules/
packages/
schemas/runtime/\*

scripts/hooks/
scripts/tests/
scirpts/ in general, i suppose

specs/spec-000-rewrite/
tests/hooks/

xtask/src/main.rs

.ban-globals-allowlist
.ban-nondeterminism-allowlist
.ban-unordered-abi-allowlist

audit.toml
CLAUDE.md
deny.toml
Makefile
plawright.config.ts

The following should probably be factored out into a different repo:

crates/method/

crates/echo-ttd/
crates/echo-wesley-gen/
crates/ttd-browser/
crates/ttd-manifest/
crates/ttd-protocol-rs/
