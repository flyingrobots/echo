<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# `claude-think --remember`: semantic restore + ritualize the surface

Legend: `PLATFORM`

## The pain we just lived

The 2026-05-30 echo/jedit merge session ended with 764 Think entries,
multiple handoff breadcrumbs, and context recovery by archaeology.
The agent's session-start ritual at the time relied on
`claude-think --recent`, which returns chronological tail — useful
for "what just happened" but useless for "what did past-me decide
about Session admission gates."

`claude-think --remember [query]` already exists and does semantic
recall (verified by running `--help` after the user flagged the
misunderstanding). The current surface:

```text
claude-think --remember [--brief] [--limit=N] [query]
```

This card is therefore two things, not one:

1. A small enhancement proposal to extend the existing surface.
2. A ritualization proposal so the existing surface gets used.

## Enhancement proposal

Add filter flags that the current surface lacks:

- `--since <duration>` using canonical ISO 8601 durations (e.g.
  `--since P7D`, `--since PT24H`) so a session-pickup query can
  scope to recent work without re-ranking ancient entries.
- `--repo <name>` to match entries that mention a particular repo
  in their body or were captured while that repo was the cwd. (The
  receipt store may already carry this metadata; if so, this is a
  filter, not a new field.)
- `--branch <name>` analogous to `--repo`, useful when several
  branches per repo are live.

Acceptance criteria for the enhancement:

```text
claude-think --remember "0025 sessions"
claude-think --remember --since P14D "jedit EINT cutover"
claude-think --remember --repo echo --since P7D "dev-loop speedup"
```

Output for each match:

- entry id
- timestamp
- semantic rank (or just sorted by relevance)
- short excerpt
- inferred repo / branch if known

## Ritualization proposal

The agent's `~/.claude/CLAUDE.md` currently recommends
`claude-think --recent` as the session-start restore. That should
become `claude-think --remember "<branch or cycle name>"` because:

- `--recent` returns chronological tail and gets diluted fast on an
  active project.
- `--remember "<branch>"` lands the agent on entries that mention
  the work it is actually about to do.
- The same call form works for handoff resume (`--remember "echo
0025 phase 2"`) and for hot-restart after a session-limit reset
  (`--remember "$(git rev-parse --abbrev-ref HEAD)"`).

The agent has already captured a feedback memory
(`feedback-verify-tool-surface`) noting this lesson. Ritualizing
the canonical command in CLAUDE.md closes the loop.

## Why this matters

- The 2026-05-30 session would have skipped ~5 minutes of "where
  was I?" archaeology with one `--remember` query at the start.
- Multiply that across every session resumed cold and the savings
  are non-trivial.
- Adding `--since` / `--repo` filters is cheap compared to the
  alternative (each agent invocation paginating through 764 entries
  scoring by tf-idf or similar).

## Out of scope here

- Building the semantic-search backend. The existing
  implementation already does this; the enhancement only adds
  filters on top.
- Cross-machine sync of the Think store. Different concern.

## Trigger / acceptance

Resolve this card when:

1. `claude-think --remember --since <duration>` accepts a single
   canonical duration format (ISO 8601) and filters correctly.
2. `claude-think --remember --repo <name>` filters by inferred
   repo metadata.
3. `~/.claude/CLAUDE.md` updates the session-start ritual to
   `claude-think --remember "<context>"`.
