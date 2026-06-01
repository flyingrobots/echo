<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# METHOD

A backlog, a loop, and honest bookkeeping.

Adopted from [flyingrobots/method](https://github.com/flyingrobots/method).

## Principles

### Stances

**The agent and the human sit at the same table.** They see different
things. Both are named in every design. Both must agree before work
ships.

**Default to building the agent surface first** — it is the foundation
the human experience stands on. If the work is human-first exploratory
design, say so in the design doc.

**Agent surfaces must be explicit and inspectable.** If work is
agent-mediated, say what is agent-generated, why it exists, what
evidence it relies on, and what action it expects next.

**The filesystem is the database.** A directory is a priority. A
filename is an identity. Moving a file is a decision. `ls` is the
query.

**Process should be calm.** No sprints. No velocity. No burndown. A
backlog tiered by judgment, and a loop for doing it well.

### Design constraints

**Meaning must survive without decoration.** If the work only makes
sense with color, layout, motion, or shared visual context, the design
is unfinished. Rich interaction is valuable, but the underlying truth
must stand on its own.

**Accessibility is a product concern, not a fallback string path.**
Designs must name the linear reading model and reduced-complexity
experience, not assume the default operator.

**Localization is not translation after the fact.** Wording, wrapping,
formatting, and directionality are design constraints from the start.
Prefer logical `start`/`end` thinking over hardcoded left/right
assumptions.

### Quality gates

**Everything traces to a playback question.** If you cannot say which
question your work answers, you are drifting. Stop. Reconnect to the
design, or change it.

**Tests are the executable spec.** Design names the hill and the
playback questions. Tests prove the answers. No ceremonial prose
between intent and proof.

**If a claimed result cannot be reproduced, it is not done.**
Witnesses are not victory photos. They are rerunnable proof.

---

## Structure

```text
docs/
  method/
    backlog/                      legacy workspace marker only; no live cards
    graveyard/github-issue-migration/
                                    archived pre-migration backlog cards
    legends/                        named domains
    retro/<cycle>/retro.md          closed-cycle retrospectives
    guide.md                        operator advice and non-doctrinal practice notes
    process.md                      how cycles run
  design/
    <cycle>/<task>.md               cycle design docs
    *.md                            living documents
```

---

## Signposts

METHOD expects a few bounded repo-level signposts. They summarize the
state of the repo; they do not create commitments.

| Signpost                | Role                                                                     |
| ----------------------- | ------------------------------------------------------------------------ |
| `README.md`             | The project root. What Echo is and how to build it.                      |
| `docs/BEARING.md`       | Current direction, last shipped cycle, and tensions at cycle boundaries. |
| `docs/method/README.md` | The operating doctrine and tracker shape (this file).                    |

---

## Backlog

GitHub Issues are the live backlog. Labels are the index. Historical
filesystem backlog cards were migrated to GitHub Issues on 2026-06-01 and
archived under `docs/method/graveyard/github-issue-migration/`.

The `docs/method/backlog/` directory remains only as a compatibility marker
for legacy `cargo xtask method ...` workspace discovery. Do not add new live
work cards there.

### Inbox

Anyone — human or agent — opens an issue at any time. A sentence is enough.
No legend, no scope, no ceremony. Capture it. Keep moving. The inbox is
processed during maintenance by applying or changing Method labels.

### Lanes

| Label             | Purpose                       |
| ----------------- | ----------------------------- |
| `lane:inbox`      | Unprocessed.                  |
| `lane:asap`       | Pull into a cycle soon.       |
| `lane:up-next`    | Next in line.                 |
| `lane:cool-ideas` | Not commitments.              |
| `lane:bad-code`   | It works, but it bothers you. |
| `lane:release`    | Release-bar work.             |

Each live work issue should carry exactly one lane label unless it is being
actively re-triaged.

### Naming

Use sane issue titles and Method labels. Apply a `legend:*` label when the
work belongs to a named domain. Branch names should be lowercase slugs of the
issue title.

```text
Issue: Writer head registry
Labels: lane:asap, legend:kernel
Branch: writer-head-registry
```

### Visibility

GitHub Issues must not hide executable subtasks that need independent
scheduling or dependency tracking. If an issue discovers a sequence of
implementation slices, promote those slices into visible backlog cards and
connect them with `Depends on:` links.

An issue may remain as an index for a design packet or hill, but that index must
not be the only place executable work exists.

### Promoting

When an issue is pulled into a cycle, create or update the design doc and keep
the issue as the public tracker:

```text
issue #123 Writer head registry -> docs/design/<cycle>/writer-head-registry.md
```

The issue links to the design packet. The design packet links back to the
issue. Work does not hide in two unlinked places.

### Commitment

Pull it and you own it — "you" meaning the named sponsors (human and
agent) in the design doc. It does not go back.

- **Finish** — hill met.
- **Pivot** — end early, write the retro. Remaining work re-enters
  GitHub Issues as a new item with fresh scope.

### Maintenance

End of cycle:

- Process inbox. Promote, flesh out, or delete if it is not useful
  current work.
- Re-prioritize. What you learned changes what matters.
- Clean up. Merge duplicates, kill the dead.

Do not reorganize mid-cycle.

### Cycle types

Same loop regardless:

- **Feature** — design, test, build, ship.
- **Design** — the deliverable is docs, not code.
- **Debt** — pull from `bad-code/`. The hill is "this no longer
  bothers us."

---

## Legends

A named domain that spans many cycles. Legends organize attention, not
timelines — they are reference frames, not milestones. A legend never
starts or finishes. It describes what it covers, who cares, what
success looks like, and how you know.

A `legend:*` label marks issue domain so GitHub queries reveal domain load at a glance. Legends live in `docs/method/legends/` as standalone documents.

The current legends in this repo are:

- `KERNEL` — core simulation engine: WARP graph rewrites, scheduling,
  deterministic commit, tick patches, parallel execution.
- `MATH` — deterministic math and geometry: IEEE 754 canonicalization,
  trig oracle, collision, broad phase.
- `PLATFORM` — tooling and infrastructure: WASM, xtask CLI, CI,
  benchmarks, CAS, Wesley integration.
- `DOCS` — documentation: guides, specs, living docs, course material.
  Keeping what Echo says about itself honest.

---

## Cycles

A cycle is a unit of shipped work. Design, implementation,
retrospective. Numbered sequentially, starting at `0001`.

### Size

A cycle has no prescribed duration. It should be small enough that a
failed one teaches more than it costs. If you cannot describe the hill
in one sentence, the cycle is too big. Split it.

### The loop

0. **Pull** — choose a GitHub Issue. Create a branch named from the issue
   title, or `cycle/<id>-<slug>` when the work is explicitly cycle-shaped.
   Create or update the design packet under `docs/design/<cycle>/` and link
   it to the issue. You are now committed. All cycle work happens on this
   branch.

1. **Design** — write a design doc from the template at
   `docs/method/design-template.md`. Required sections:
    - **Title and legend** — cycle number, name, legend link.
    - **Why this cycle exists** — motivation and context.
    - **Depends on** — explicit dependency chain (or "nothing").
    - **Human users / jobs / hills** — who benefits, what they do,
      one-sentence hill from the human perspective.
    - **Agent users / jobs / hills** — same, from the agent
      perspective.
    - **Human playback** — concrete walk-through scenario proving
      the human hill.
    - **Agent playback** — concrete walk-through scenario proving
      the agent hill.
    - **Implementation outline** — numbered steps of what the code
      (or docs) will do.
    - **Tests to write first** — the RED phase, named in the design.
    - **Risks / unknowns** — what might go wrong.
    - **Postures** — accessibility, localization, agent
      inspectability. If not relevant, say so explicitly. Silence
      is not a position.
    - **Non-goals** — what this cycle will not do.

2. **RED** — write failing tests. Playback questions become specs.
   Default to agent surface first.

3. **GREEN** — make them pass.

4. **Playback** — produce a witness. The agent answers agent
   questions. The human answers user questions. Write it down.

    The **witness** is the concrete artifact — test output, transcript,
    screenshot, recording — that shows both answers. No clear yes means
    no. If the witness cannot be reproduced from committed commands,
    inputs, or mechanisms, the answer is still no. If the hill claims
    accessibility, localization, or agent-facing explainability, witness
    those paths too.

5. **Close** — write the retro and witness packet on the branch.
    - Drift check (mandatory). Undocumented drift is the only true
      failure mode.
    - New debt to `bad-code/`.
    - Cool ideas to `cool-ideas/`.
    - Backlog maintenance.

    Closing the cycle packet does not mean `main` has accepted it yet.

6. **PR / review** — push the `cycle/<id>` branch and open a PR to
   `main`. The PR contains the full cycle packet: design doc,
   implementation, tests, retro, and witness. Review the full cycle
   packet until merge or rejection.

7. **Ship sync on `main`** — after merge, update repo-level ship
   surfaces such as `docs/BEARING.md`, `CHANGELOG.md`, and release
   notes when the cycle changes them.

    Releases happen when externally meaningful behavior changes. Not
    every cycle is a release. Ship sync only happens on merged `main`
    state, not branch-local closeout state.

### Disagreement at playback

Both sponsors must say yes. When they disagree:

1. Name the disagreement in the witness. What does the agent see that
   the human does not, or vice versa?
2. If the gap is scoping — the hill was met but the answer is
   unsatisfying — the cycle is **partial**. Merge what is honest.
   Write the retro. File a new backlog item for the remainder.
3. If the gap is correctness — one sponsor believes the work is
   wrong — do not merge it. Return to RED or GREEN. If the work is
   abandoned instead of fixed, close the cycle as **not met** and write
   the retro.

The human does not automatically override the agent. The agent does
not automatically override the human. The design doc is the tiebreaker:
does the witness answer the playback questions or not?

### Outcomes

- **Hill met** — close the packet, review it, merge it, then ship sync.
- **Partial** — close the packet honestly, merge only what is honest,
  and let the retro explain the gap.
- **Not met** — cycle still concludes. Write the retro. A failed
  cycle with a good retro beats a successful one with no learnings. A
  failed cycle does not need to merge to end honestly.

Every cycle ends with a retro. Success is not required.

---

## Coordination

METHOD is designed for a solo developer working with an agent. It
scales to a team without adding meetings, roles, or synchronization
ceremonies. The mechanism is passive legibility.

### The filesystem is the coordination layer

If you can answer these questions by reading the repo, you do not need
a standup:

- What is everyone working on? → active design docs in `docs/design/`
  that do not have a matching `docs/method/retro/<cycle>/retro.md`
- What is committed? → each design doc names its sponsors and hill
- What is next? → open GitHub Issues labeled `lane:asap`
- What closed, failed, or drifted? → `ls docs/method/retro/`
- What was deleted? → the audit entry or git history for the decision

If any of these are unclear, the docs are incomplete. Fix the docs,
not the process.

### BEARING.md

A single living document at `docs/BEARING.md`. One page, updated at
cycle boundaries — not mid-cycle. It answers three questions:

1. **Where are we going?** — the current priority (legend, theme, or
   plain English).
2. **What just shipped?** — last completed cycle, one line.
3. **What feels wrong?** — known tensions, open questions, gut
   feelings that do not yet have backlog items.

`BEARING.md` is a signpost, not a status report. It summarizes
direction; it does not create commitments, replace backlog items, or
record decisions that belong in design docs, retros, or the backlog.

### Conflict at the tracker

Two people pulling conflicting `lane:asap` issues is a design-doc problem, not a process problem. Active design docs are visible through the repo itself. If your hill contradicts an active cycle's hill, you should see it at step 1. Resolve it there or file it as a tension in `docs/BEARING.md`.

### What this does not add

No standups. No syncs. No status emails. No sprint planning. No retro
meetings. The retro is a document, not a ceremony. The repo is the
single source of truth. Read it.

---

## Rejected Work

Rejected work does not get a live museum directory. If the reason
matters to current operators, capture it in the active design, retro,
audit entry, or GitHub Issue that made the decision, then delete the
stale file. Git history is the archive.

---

## Flow

```text
idea -> GitHub Issue lane:inbox -> lane:cool-ideas -> lane:up-next -> lane:asap
  -> issue-named branch off main
  -> design/<cycle>/  (committed)
  -> RED -> GREEN -> playback (witness)
  -> retro/<cycle>/   (cycle packet closed on branch)
  -> push cycle/<id>, PR to main
  -> ship sync on main (BEARING / CHANGELOG / release when meaningful)
```

---

## Tooling

METHOD operations in this repo are performed via `cargo xtask`. The
following commands are implemented:

| Command                            | Purpose                                                            |
| ---------------------------------- | ------------------------------------------------------------------ |
| `cargo xtask method status`        | Summarize legacy filesystem lanes, active cycles, and legend load. |
| `cargo xtask method status --json` | Emit the same status report for agents and tooling.                |

Legacy filesystem-backlog commands may still exist for compatibility, but new
work is tracked in GitHub Issues. Do not use `cargo xtask method inbox`,
`matrix`, `dag`, `frontier`, `critical-path`, or `check-dag` as the
source of truth for live work after the GitHub Issues migration.

The following commands are planned but **not yet implemented**:

| Command                            | Purpose                                                          |
| ---------------------------------- | ---------------------------------------------------------------- |
| `cargo xtask method pull <item>`   | Promote a backlog item into the next numbered cycle.             |
| `cargo xtask method close [cycle]` | Write a retro and create its `witness/` directory.               |
| `cargo xtask method drift [cycle]` | Check active cycle playback questions against test descriptions. |

GitHub issue creation, labeling, and branch naming are now the primitives for
work tracking. Filesystem backlog commands are historical compatibility
surfaces until the embedded METHOD crate is retired or taught to speak GitHub
Issues directly.

---

## What this system does not have

No milestones. No velocity. No ticket numbers. No required meetings.

The issue tracker is tiered by lane labels. Choice within a lane is judgment at pull time. Coordination is reading the repo plus the GitHub Issues surface. That is enough.

---

## Naming conventions

| Convention           | Example                          | When                       |
| -------------------- | -------------------------------- | -------------------------- |
| `ALL_CAPS.md`        | `BEARING.md`                     | Signpost — root or `docs/` |
| `lowercase.md`       | `guide.md`                       | Everything else            |
| `<LEGEND>_<name>.md` | `KERNEL_writer-head-registry.md` | Backlog with legend        |
| `<name>.md`          | `debt-scheduler-god-module.md`   | Backlog without legend     |
| `<cycle>/`           | `0001-docs-audit/`               | Cycle directory            |
