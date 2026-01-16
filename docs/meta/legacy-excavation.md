<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Legacy Excavation Log (Placeholder)

This document is a place to record “legacy prototype” artifacts we discover (old folders, old
designs, abandoned experiments) and the decisions we make about them:

- keep concept (rewrite cleanly)
- redesign (needs rethinking)
- discard (no longer relevant)

If you add entries here, prefer linking to concrete files and capture a short “why” in the decision
log when a choice affects public surface area or determinism.

## Process (Recommended)

1. Identify a legacy artifact (folder, demo, script, or spec).
2. Summarize its intent in one sentence.
3. Decide: **keep concept**, **redesign**, or **discard**.
4. Record any determinism or public API implications.
5. Link to the replacement or follow-up issue if one exists.

## Where to Look

- Old demos or prototype subfolders.
- Archived build scripts or abandoned toolchains.
- Experimental rendering or physics integrations.
- Prototype specs that predate the Rust-first era.

## Template

| Artifact | What It Was | What We Keep | Action | Notes |
| --- | --- | --- | --- | --- |
| `path/to/thing` | (1–2 lines) | (concepts) | keep/redesign/discard | (gotchas, deps, links) |
