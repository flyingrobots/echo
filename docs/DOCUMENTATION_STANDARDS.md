<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Documentation Standards

**Status:** Current project policy for new and substantially changed
documentation.
**Supersedes:** [ADR 0012: Repository Knowledge Model After Method](adr/0012-repository-knowledge-model.md)

Echo documentation is part of the engineering contract. Its job is to make
the current system, its governing decisions, and the evidence for those claims
findable without reconstructing a private chronology.

This policy adapts the structure of Colorful Language's
[documentation standard](https://github.com/flyingrobots/colorful-language/blob/09dc60a9f23834f8511bcb3067cb1ac4393fae8a/docs/DOCUMENTATION_STANDARDS.md)
to Echo's existing knowledge model. Echo keeps its established split among
architecture, specifications, invariants, topics, and executable witnesses. It
does not adopt Colorful's roadmap, goalpost, or topic test-plan machinery.

## Core rules

1. Give every durable claim one canonical owner.
2. Give every page one primary reader job.
3. Name pages after concepts, contracts, or reader tasks, not sequence slots.
4. State relationships explicitly; directory order and filename order carry no
   architectural meaning.
5. Keep current truth, planned work, historical reasoning, and executable
   evidence distinguishable.
6. Link strong claims to code, tests, specifications, invariants, or retained
   historical decisions.

## Corpus map

| Location                                                   | Job                                                                 |
| ---------------------------------------------------------- | ------------------------------------------------------------------- |
| `README.md`, `GUIDE.md`                                    | Project entrances and supported routes into deeper documentation.   |
| `docs/README.md`                                           | Documentation spine and routing index.                              |
| `docs/topics/`                                             | Living explanations of durable Echo concepts and boundaries.        |
| `docs/architecture/`                                       | Current cross-subsystem architecture and accepted boundary designs. |
| `docs/spec/`                                               | Normative protocols, encodings, and conformance contracts.          |
| `docs/invariants/`                                         | Compact laws that implementations must preserve.                    |
| `docs/determinism/`                                        | Determinism policy, hazards, and evidence contracts.                |
| `docs/adr/`                                                | Closed archive of numbered historical decision records.             |
| GitHub Issues, Projects, pull requests, and review threads | Live work, plans, priority, blockers, and status.                   |
| `CHANGELOG.md`                                             | Externally meaningful behavior that shipped.                        |
| Git history                                                | Removed material and the exact evolution of checked-in documents.   |

Do not create a second current reference merely because another directory or
document type is convenient. Link to the canonical owner and add only the
reader-specific context the new page needs.

## Page jobs

A page should primarily help its reader do one of these jobs:

- learn a concept or boundary;
- perform a supported task;
- look up an exact contract;
- understand a decision and its tradeoffs;
- troubleshoot an observable failure;
- change the implementation and verify the result.

A page may link across jobs, but it should not become a tutorial, reference,
roadmap, architecture guide, and historical diary at once.

### Topics

A topic describes the current conceptual model. It states the boundary,
invariants, ownership, and evidence anchors needed to understand that concept.
Update it in the same change that changes the boundary.

Topics do not own live implementation queues. A limitation may be stated as a
current fact; the work to change it belongs in GitHub.

### Architecture

An architecture page explains current cross-subsystem structure or an accepted
boundary design. When an accepted design is not implemented, the page must say
so prominently and link to the GitHub owner for implementation state. It must
not describe planned behavior as existing runtime behavior.

### Specifications and invariants

Specifications define conformance. Invariants state compact laws. They are not
explanatory essays or implementation plans. Examples that form part of a
contract should be executable or backed by exact fixtures when practical.

## Durable decisions

A durable decision changes a long-lived boundary, identity, format, invariant,
authority split, compatibility promise, or recovery law. Record the decision
where a future reader will look for the concept:

- update the owning topic for a conceptual boundary;
- update the owning architecture page for a cross-subsystem boundary;
- update the owning specification or invariant for a normative contract;
- add a semantically named `rationale.md` beside a larger concept when the
  tradeoffs would otherwise overwhelm its current reference.

Do not allocate a number merely to prove that a decision happened. The numbered
ADR sequence in `docs/adr/` is a closed historical archive. Existing ADRs remain
valuable evidence and may be linked, refined, or superseded, but new durable
decisions use semantic names in their owning current-documentation area.

### Relationship contract

When a decision relates materially to another decision, include the applicable
relationship near the top of the owning document:

- **Supersedes:** the named older decision no longer governs the stated scope.
- **Superseded by:** the named newer decision now governs the stated scope.
- **Refines:** this decision adds precision without replacing the older one.
- **Depends on:** this decision requires another decision or contract to hold.
- **Related:** the documents illuminate the same boundary but neither governs
  the other.

Use descriptive links, not bare identifiers. A supersession must be recorded in
both directions so readers entering through either document can follow it.
Absence of a relationship line means no such relationship is claimed; it does
not mean “whatever has the larger number wins.”

## Current truth, plans, and history

Living references describe current implementation truth or clearly labeled
accepted contracts. GitHub owns change-local plans and status. Git history owns
the exact old text.

Do not check in backlogs, cycle packets, retrospectives, review transcripts,
status ledgers, or roadmap checklists. A short checked-in redirect may remain
when an old stable path must route readers to its current owner.

Historical reasoning must not masquerade as current behavior. Mark retained
historical documents clearly and link to the current owner that supersedes or
refines them.

## Evidence and citations

Strong claims should point to the smallest durable witness that establishes
them:

- source or public API for ownership and shape;
- tests, fixtures, or golden vectors for behavior;
- a specification or invariant for normative law;
- an accepted architecture page for an unimplemented boundary contract;
- a historical decision for retained reasoning.

Source links support an explanation; they do not replace one. Prefer
repository-relative links for checked-in sources. Use stable external
permalinks when the exact outside revision matters.

Never claim that a command, test, visual inspection, review, or runtime path was
verified when it was not run or observed.

## Examples and safety

Examples must use supported behavior and enough context to interpret them.
Separate copyable commands from expected output. Do not put shell prompts in a
copyable command block.

Put warnings before destructive, privileged, costly, or irreversible commands.
State the scope and consequence, and provide a safer check or recovery route
when one exists.

## Writing and structure

- Lead with the result, decision, warning, or essential condition.
- Prefer exact Echo terms and define unfamiliar ones at first use.
- Use active voice when it clarifies ownership.
- Use prose for causality and tradeoffs, lists for parallel facts, and tables
  for genuinely two-dimensional comparisons.
- Use descriptive link text rather than “here” or a bare path.
- Treat length and style metrics as editorial signals, not universal merge
  gates.

## Maintenance loop

For a meaningful change:

1. Identify the canonical owner of the affected claim.
2. Name the smallest executable witness when behavior changes.
3. Update design rationale only when the tradeoff needs durable explanation.
4. Implement and validate the change.
5. Update the current owner after the behavior or accepted contract changes.
6. Add explicit decision relationships when governance changed.
7. Update `docs/README.md` when a durable route was added or moved.
8. Keep live follow-up work in GitHub.

## Review checklist

Before calling a documentation change done, verify that:

- the page has one primary reader job;
- the durable claim has one canonical owner;
- current behavior and accepted-but-unimplemented design are distinguishable;
- plans and status remain in GitHub;
- durable decision relationships are explicit and bidirectional when they
  supersede;
- strong claims have appropriate evidence anchors;
- new durable pages are linked from `docs/README.md`;
- internal links resolve;
- Markdown and whitespace checks pass.

The objective is not uniform paperwork. The objective is a corpus in which a
reader can find what governs a concept, why it governs, what it replaced, and
what proves it without decoding a global number line.
