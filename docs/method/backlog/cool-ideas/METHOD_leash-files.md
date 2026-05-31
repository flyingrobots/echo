<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Leash files: a structural lane for temporary scaffolds

Legend: `METHOD`

## The problem

Cross-repo or cross-cycle work routinely produces _temporary scaffolds_:
client-side mirrors of unfinished engine APIs, transitional adapters,
"good for now" boundary shims. They are deliberate decisions, not
mistakes. The failure mode is not introducing them — it is forgetting
to delete them when their reason expires.

The natural pressure on a working scaffold is to accrete features,
become depended on, and resist removal. Prose-only backlog cards
("we'll remember to delete this") rely on humans being the system
that nags. Humans are goldfish with keyboards.

The 2026-05-30 jedit PR #33 review-resolution session ended with
exactly this concern about `JeditWorldlineSessionPort` and friends
("Temporary things love to buy furniture. Don't let it."). The
prototype response — `jedit/docs/method/backlog/asap/sessions-migration.md`
— is prose-heavy and human-friendly but not _machine-naggable_.

## The shape

Promote a new backlog lane:

```text
docs/method/backlog/leash/<scaffold-slug>.md
```

Each leash file is a small structured record naming a scaffold, the
event that should free it, and the symbols whose disappearance is the
deletion proof. The lane sits alongside `asap/`, `bad-code/`,
`cool-ideas/`, and `graveyard/`; it does not replace any of them. A
leash and a prose `asap/` card cross-reference each other when both
exist.

## Required frontmatter

```yaml
---
scaffold: JeditWorldlineSessionPort
repo: jedit
introduced_by:
    pr: 33
    merged_sha: 90245c73
    date: 2026-05-30
reason: |
    Engine-side Session/WorldlineId types do not exist yet; the client
    carries a transitional mirror so the EINT cutover can ship without
    blocking on echo cycle 0025 Phase 2.
deletion_trigger:
    repo: echo
    cycle: "0025-sessions-as-causal-contexts"
    phase: "Phase 2 GREEN"
symbols:
    - JeditWorldlineSessionPort
    - JeditTransportSeam
    - createInMemoryJeditWorldlineSessionPort
    - installed-jedit-eint-bridge
status: active
companion_cards:
    - jedit/docs/method/backlog/asap/sessions-migration.md
---
```

Required fields: `scaffold`, `repo`, `introduced_by`, `reason`,
`deletion_trigger`, `symbols`, `status`. Optional but encouraged:
`companion_cards`, `notes` body section after the frontmatter for
context.

`status` is one of `active`, `triggered`, `deleted`, `escalated`.

## Lifecycle

1. **`active`** — the scaffold is in production, the trigger has not
   fired. The leash exists. No action.
2. **`triggered`** — the deletion trigger condition has been met
   (e.g. the named cycle/phase closed). Tooling moves the file into
   `triggered` and surfaces it on the next cycle-close summary.
3. **`deleted`** — the named symbols are gone from the repo. Tooling
   confirms via grep and moves the leash file into
   `docs/method/graveyard/` with the deletion SHA appended.
4. **`escalated`** — the deletion trigger fired but the symbols
   still exist after a defined grace window. Tooling promotes the
   leash to `asap/` and tags the next cycle.

## What tooling has to do

A single script (proposed: `xtask leash-audit`) runs in CI / locally:

- Parse every `docs/method/backlog/leash/*.md` frontmatter.
- For each `active` leash:
    - If the `deletion_trigger.cycle` has closed (look at
      `docs/method/retro/<cycle>/`), promote to `triggered`.
- For each `triggered` leash:
    - Grep the leash's repo for every entry in `symbols`. If zero
      hits, promote to `deleted` and move to graveyard. If any hits,
      surface as a warning.
- For `triggered` leashes older than the configured grace window
  (default: 1 cycle), promote to `escalated`.

The script is dumb on purpose. The hard work is keeping the leash
records honest, not the matching.

## Why this is the right size of mechanism

- It does not introduce a new project-management tool. It is a
  backlog lane, same as every other lane.
- It does not require humans to remember anything. The trigger
  condition is encoded; tooling notices.
- It scales: a repo with five active leashes still produces a
  five-line audit summary.
- It composes with `asap/`: prose explanations stay where prose
  explanations belong; structural records stay where structural
  records belong.
- It composes with `graveyard/`: completed leashes become a
  historical record of "we said this was temporary, here is the SHA
  that proved it."

## Prototype

The first concrete leash, landing alongside this card, is in the
jedit repo at:

```text
jedit/docs/method/backlog/leash/jedit-session-port.md
```

It names `JeditWorldlineSessionPort` + `JeditTransportSeam` +
`createInMemoryJeditWorldlineSessionPort` + the installed EINT
bridge as scaffolds whose deletion is bound to Echo cycle 0025
Phase 2 GREEN.

## Not in scope here

- The `xtask leash-audit` implementation. Land the convention
  first; build the audit when the second leash file gets created
  (the universal "you only need a Tool when you have two of the
  thing" rule).
- Cross-repo automation that opens deletion PRs automatically. That
  is the natural next step but adds blast radius; ship the manual
  audit first.

## Companion memory

The 2026-05-30 session captured the leash-pattern motivation as a
project memory in the agent's own store; see
`project-jedit-session-port-leash` in the agent's memory index. That
memory and this card are the same idea at two altitudes: the memory
keeps the agent honest in casual touches of the codebase; this card
keeps METHOD honest at the cycle/process level.
