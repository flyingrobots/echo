<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Docs Lifecycle

Echo documentation is part of the runtime architecture. It must tell readers
what is authoritative now without deleting the evidence that explains how the
system arrived there.

This policy defines how design docs accumulate, how their current claims become
domain documentation, and how superseded material is archived.

## Authority Model

Echo docs have several different jobs. Do not make one file do all of them.

| Role      | Purpose                                       | Normal Location                      |
| --------- | --------------------------------------------- | ------------------------------------ |
| Canonical | Current behavior, invariants, and APIs        | `docs/architecture/`, `docs/spec/`   |
| Design    | Active proposal, decision path, tradeoffs     | `docs/design/`                       |
| Evidence  | Historical proof, retros, audits, raw output  | `docs/method/`, `docs/audits/`       |
| Procedure | How contributors operate the repo             | `docs/procedures/`, `docs/workflows` |
| Archive   | Superseded source material retained for audit | Archive/graveyard namespaces         |

The public reader path should prefer canonical docs. Design and evidence docs
remain valuable, but they must not silently compete with canonical docs for
authority.

## Lifecycle States

Every design doc is in exactly one lifecycle state:

| State        | Meaning                                                      |
| ------------ | ------------------------------------------------------------ |
| `active`     | Still being evaluated or implemented                         |
| `accepted`   | Decision accepted, implementation or docs absorption pending |
| `absorbed`   | Current truth merged into canonical domain docs              |
| `superseded` | Replaced by a newer design, spec, architecture doc, or code  |
| `archived`   | Retained only as historical evidence                         |

The lifecycle state is about authority, not age. A recent design can be
superseded. An old spec can remain canonical.

## Required Design Header

New or materially edited design docs should start with a status block after the
title:

```md
Status: active
Domain: observation
Canonical output: docs/spec/SPEC-0004-worldlines-playback-truthbus.md
Supersedes: none
Superseded by: none
Evidence: cargo xtask test-slice observation
```

Use `Canonical output` to name the domain doc that will absorb the current
truth. If the design is exploratory and has no destination yet, write
`Canonical output: pending`.

## Domain Areas

Every feature or architectural theme should converge into one canonical domain
area. The first pass domain areas are:

| Domain               | Canonical Output Pattern                                        |
| -------------------- | --------------------------------------------------------------- |
| Runtime carrier      | `docs/spec/warp-core.md`, carrier-specific specs                |
| Admission/scheduler  | scheduler, tick, settlement, and receipt specs                  |
| Observation/readings | observation, reading envelope, WARP view protocol specs         |
| Contract hosting     | application contract hosting and Wesley boundary docs           |
| Retention/WSC        | WSC, retained readings, CAS, and semantic lookup docs           |
| Continuum            | witnessed suffix, import/export, and transport docs             |
| Determinism          | deterministic math, DIND, release policy, benchmark evidence    |
| Product integrations | jedit, WARPDrive, and other application-facing integration docs |
| Contributor workflow | procedures, validation, review, and docs lifecycle docs         |

If a design doc spans multiple domains, pick a primary canonical output and
cross-link the secondary outputs.

## Accumulation Process

Design docs are allowed to accumulate while a feature is uncertain. They should
capture problem framing, rejected alternatives, constraints, evidence, and open
questions. They should not pretend to be current product documentation.

Use this loop:

1. **Open design**: create or update a design packet in `docs/design/` with
   `Status: active` and a named `Canonical output`.
2. **Build evidence**: land implementation, tests, fixtures, demos, or docs
   that prove the claim.
3. **Accept decision**: when the design direction is chosen, set
   `Status: accepted` and list the exact evidence.
4. **Absorb current truth**: update the canonical domain doc so a new reader
   can learn the current system without reading the whole design thread.
5. **Retire design authority**: set the design doc to `absorbed`,
   `superseded`, or `archived`.

The output of design work is not another permanent reader-path design doc. The
output is an updated canonical domain doc plus retained decision evidence.

## Absorption Rules

A design doc is ready to be marked `absorbed` when:

- the canonical domain doc contains the current invariant or behavior;
- the canonical domain doc links back to the design only for rationale;
- executable evidence is named in the design, PR, changelog, or canonical doc;
- the design doc no longer needs to appear in the public "Start Here" path.

Absorption should be a summary, not a paste. Preserve the canonical rule,
surface API, invariants, and operational examples. Leave obsolete debate,
temporary slice planning, and raw logs in the design/evidence record.

## Supersession Rules

Mark a design `superseded` when a newer design, spec, implementation, or
architectural decision replaces its claims.

The superseded doc must point to the replacement:

```md
Status: superseded
Superseded by: docs/spec/warp-core.md#runtime-owned-ticks
```

Do not delete superseded docs during ordinary feature work. Deletion is only
appropriate when the material is duplicate, generated, or migrated with an
inspectable mapping.

## Archive Rules

Archive material when it is useful as evidence but harmful as reader-path truth.

Good archive candidates:

- old implementation plans after their claims are absorbed;
- Method backlog exports already migrated to GitHub Issues;
- raw witness output and review artifacts;
- design packets whose only current value is historical rationale;
- docs that describe removed surfaces or retired terminology.

Archive moves must preserve findability:

- leave a short tombstone or status header when the old path has live inbound
  links;
- update `docs/index.md` if the old doc was listed there;
- run dead-reference checks for touched docs;
- prefer one archive move per domain slice instead of broad unrelated sweeps.

## Public Docs Map Rules

`docs/index.md` is not an inventory. It is the reader path.

Only list a design doc in the public docs map when it is active, currently
load-bearing, and there is no canonical domain doc yet. Once absorbed, remove
it from the public path and let the canonical doc carry the current claim.

The docs map should answer:

- What should I believe now?
- Where is the canonical model?
- What evidence proves the claim?
- Where do contributors go to operate safely?

It should not answer:

- What files exist?
- What plans ever existed?
- What did every prior slice discuss?

## PR Checklist

For any PR that creates, materially edits, or relies on a design doc:

- [ ] The design doc has a lifecycle status.
- [ ] The primary domain area is named.
- [ ] The canonical output is named or explicitly `pending`.
- [ ] If implementation landed, current behavior was absorbed into a canonical
      domain doc or a follow-up issue was opened.
- [ ] Superseded docs point to their replacement.
- [ ] `docs/index.md` links only to current reader-path docs.
- [ ] Dead-reference checks pass for touched markdown.

## Anti-Patterns

- Treating `docs/design/` as the public manual.
- Adding a new design doc when a canonical domain doc update would be clearer.
- Leaving accepted designs in the reader path after the feature ships.
- Archiving by age instead of authority.
- Deleting historical evidence without a migration map.
- Hiding current behavior in PR prose without updating canonical docs.

## Minimal Slice

When in doubt, do the smallest lawful docs lifecycle slice:

1. Pick one feature/domain.
2. Name the current canonical doc.
3. Absorb the current invariant into that doc.
4. Mark one or more design docs `absorbed` or `superseded`.
5. Update `docs/index.md` only if the reader path changed.
6. Run markdownlint and dead-reference checks.
