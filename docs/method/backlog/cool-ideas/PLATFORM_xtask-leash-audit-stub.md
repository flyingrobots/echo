<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# `xtask leash-audit` (minimum viable): print the leash state, don't transition yet

Legend: `PLATFORM`

## The argument for building it _now_

The convention card
`docs/method/backlog/cool-ideas/METHOD_leash-files.md` deferred the
full `xtask leash-audit` machinery (trigger/symbol/status
transitions, grep against repo state, automatic graveyard moves)
under the "you only need a Tool when you have two of the thing"
rule.

That rule is correct in steady state. But the 2026-05-30 pre-Phase-2
review surfaced a sharper observation: **Phase 2 GREEN is precisely
the moment the first leash should fire.** If the audit machinery
does not exist when that moment arrives, the jedit session-port
scaffold becomes barnacle by default.

A _minimum_ version of `xtask leash-audit` would prevent that
default without committing to the full state-machine yet.

## Minimum viable shape

A single subcommand that reads every `docs/method/backlog/leash/
*.md` frontmatter and prints a structured table.

```text
$ cargo xtask leash-audit
Active leashes (1):

  ┌────────────────────────────────────────────────────────────┐
  │ scaffold: JeditWorldlineSessionPort                        │
  │ repo:     jedit                                            │
  │ trigger:  echo cycle 0025-sessions-as-causal-contexts      │
  │           phase Phase 2 GREEN                              │
  │ status:   active                                           │
  │                                                            │
  │ symbols still present in jedit (per declared grep):        │
  │   ✓ JeditWorldlineSessionPort         (4 hits in src/)     │
  │   ✓ JeditWorldlineSessionNotRegistered (3 hits in src/)    │
  │   ✓ JeditWorldlineId                  (7 hits)             │
  │   ✓ JeditTransportSeam                (5 hits)             │
  │   ✓ jeditSessionPort                  (8 hits in src+spec) │
  │   ✓ createInMemoryJeditWorldlineSessionPort (2 hits)       │
  │   ✓ installed-jedit-eint-bridge       (3 hits)             │
  └────────────────────────────────────────────────────────────┘
```

What it does NOT do (yet):

- Transition statuses automatically (`active → triggered →
deleted`). Phase 2 GREEN will fire the first trigger manually;
  the state machine grows out of that real example.
- Walk `docs/method/retro/<cycle>/` to detect closed cycles.
  Future work after the first transition is recorded by hand.
- Open PRs to move resolved leashes into graveyard. Adds blast
  radius; ship the audit first.
- Cross-repo execution. The minimum version runs against one repo
  at a time. The leash file declares its target `repo`; the audit
  treats that as a label, not a remote operation.

## Why this size

- Reading frontmatter and grepping symbols is ~100 lines of
  xtask code; the state machine is the part that grows over time.
- Printing the table is enough to make the leash visible at the
  right cadence (CI, weekly review, cycle close).
- A useful tool ships before the trigger fires; the missing
  features get added when their absence hurts.

## When the full machinery becomes useful

Specifically: when there are at least two active leashes _and_ at
least one of them has crossed its trigger. At that point the
manual "did X close? have the symbols disappeared?" check is paying
real cost and the state-machine investment is justified.

Until then: the minimum version is the leash, the leash is the
minimum version.

## Trigger / acceptance

Resolve this card when:

1. `cargo xtask leash-audit` reads every
   `docs/method/backlog/leash/*.md` frontmatter and prints the
   active leashes with per-symbol grep hit counts.
2. The grep is run against the leash's declared `repo` working
   tree (resolved via a configurable repo-root map, or against the
   current working directory when no map is provided).
3. The command exits non-zero if any leash references a `repo`
   that cannot be resolved, so misconfiguration is visible.

## Companion

- `docs/method/backlog/cool-ideas/METHOD_leash-files.md` — the
  convention this audit walks.
- `jedit/docs/method/backlog/leash/jedit-session-port.md` — the
  first leash, and the one that will make this tool
  immediately useful.
