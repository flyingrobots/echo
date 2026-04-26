<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Enforce Echo design vocabulary

Status: active cool idea. Echo has `docs/guide/course/glossary.md` and docs
linting, but no glossary/terminology enforcement gate. Keep this as a bounded
docs-tooling task, not as a new vocabulary source of truth.

Echo uses WARP terms informally across docs and code comments. A glossary test
would catch terminology drift in teaching docs, specs, and comments. The
authoritative vocabulary should remain in the live docs it checks, starting with
`docs/guide/course/glossary.md`.
