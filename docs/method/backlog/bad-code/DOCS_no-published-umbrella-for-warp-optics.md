<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# No published umbrella names the WARP optics ecosystem

Status: bad code (well, bad documentation, which is bad code in
disguise).

## Where

- `github.com/flyingrobots` profile (no umbrella README; just a list of
  repositories)
- `docs/architecture/there-is-no-graph.md` (contains the ecosystem
  framing — buried)
- Each repository's own README (introduces itself, doesn't introduce
  its neighbors)

## Smell

The frame that connects Echo, Wesley, jedit, git-warp, Graft, WARPDrive,
warp-ttd, Bijou, Alfred, ninetails, xyph, and Think exists. It lives in
the table in `there-is-no-graph.md` under "Runtimes As Optics," about
120 lines deep into an architecture note inside one of the projects it
catalogs:

```text
| Runtime or tool | WARP optic role                                          |
| --------------- | -------------------------------------------------------- |
| Echo            | live execution and deterministic admission optic         |
| git-warp        | Git-backed causal persistence optic                      |
| Wesley          | semantic/compiler optic over authored contract history   |
| warp-ttd        | historical inspection and causal forensics optic         |
| Graft           | governed aperture and support-obligation optic           |
| WARPDrive       | POSIX/FUSE materialization and write-back optic          |
| jedit           | human-facing console that hosts readings, lanes, and ... |
```

A first-time visitor to `github.com/flyingrobots` cannot see this table.
They see ~20 repository cards, each a sibling, each appearing to be of
equal status. They have to do detective work to find out that:

- Echo is the foundational runtime
- Wesley is the meta-compiler everything depends on for codec parity
- jedit is the reference consumer
- WARPDrive is the planned compatibility layer
- The other names are siblings, experiments, or building blocks

## Why it matters

The work is good. The story that explains why it's good is invisible to
exactly the audience most likely to evaluate it: an outsider arriving
through GitHub's profile page.

Concretely:

1. **Under-adoption.** Each repo's README has to re-explain the
   ecosystem context to a reader who lands cold. Most don't, because
   it would balloon every README. The result is that each repo looks
   smaller and more isolated than it is.
2. **Recruiter friction.** Someone evaluating the portfolio sees a
   list, not a system. Twelve OSS projects look like dabbling; one
   ecosystem of twelve interrelated optics looks like vision.
3. **Contributor friction.** A potential contributor doesn't know
   where to start, what's load-bearing, what's a sketch, or what
   depends on what.
4. **Decision drift.** Without a published frame, design decisions in
   one repo can pull against the ecosystem's stated philosophy
   without anyone noticing. The umbrella story is governance, not
   just marketing.

## Suggested fix

This is a 200-word problem with two viable shapes.

### Shape A — `flyingrobots/.github` profile README

GitHub renders `<org>/.github/profile/README.md` (or the user-level
equivalent) as the front page of the profile. Drop in:

- One paragraph: "I build WARP-shaped systems. Here's what that means."
- The optics table from `there-is-no-graph.md`, with each row linked
  to its repo
- Three "what to look at first" repos for different audiences
  (recruiters, contributors, the curious)
- A status legend (foundational / consumer / experimental / archived)
  next to each project

Estimated: 200-300 words. Half a day. Largest blocker is deciding the
status labels honestly.

### Shape B — `flyingrobots/platform` umbrella repo

A dedicated repo whose README is the ecosystem story, with a
dependency diagram (Mermaid), per-project status, the WARP philosophy
in 500 words, and links into each project's deeper docs. Optionally
hosts a static site (VitePress?) that aggregates docs from each repo.

Estimated: a week to scaffold and write. Better for long-term but
heavier upfront.

**Recommendation: ship Shape A first.** It costs a half-day and solves
80% of the problem. Shape B emerges naturally if Shape A starts
attracting traffic.

## Risk: don't over-engineer this

The temptation will be to design a beautiful documentation system. The
correct first move is **a profile README with a table and status
labels**. Ship that. See if anyone notices. Iterate from feedback, not
from imagined needs.

## Surface when

- A new repo is added to the portfolio (it's the moment "wait, what
  context do I drop this in?" becomes loudest)
- Anyone asks "where should I start looking at your work?"
- Before any external talk / publication mentioning multiple projects
- Before WARPDrive goes from vapor to v0.0.1 — the umbrella story is
  what justifies WARPDrive existing at all
