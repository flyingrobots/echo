<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# GitHub-Native Work Sequencing

Status: stable operating doctrine.

Live sequencing belongs in GitHub, not in repository roadmaps. This document
records how Echo work is ordered without duplicating current Project state.

The live control surface is the Echo 1.0 Convergence Project:

<https://github.com/users/flyingrobots/projects/14>

The release constitution is
[`docs/releases/echo-1.0-contract.md`](../releases/echo-1.0-contract.md).
The work-tracking boundary is [`docs/WorkItems.md`](../WorkItems.md).

## Native GitHub Structure

Use one `Echo 1.0` milestone per participating repository.

Use parent and sub-issues for hierarchy:

```text
Release Bar
└── Gate
    └── Capability epic or PR-sized work item
```

Do not add deeper operational nesting unless GitHub issue relationships become
harder to read without it.

Use native `blocked by` and `blocking` relationships for sequencing. Do not
encode blockers in custom Project fields.

The repository that owns the contract owns the parent issue. Consumer
implementations become sub-issues in their owning repositories.

## Project Fields

Keep native fields visible:

- Status
- Type
- Repository
- Milestone
- Parent issue
- Sub-issue progress
- Assignees
- Linked pull request
- Reviewers

Use only these custom fields for Echo 1.0 planning:

| Field    | Values                                                              |
| -------- | ------------------------------------------------------------------- |
| Track    | Durability, Continuum, Suffix Exchange, Edict, Jedit, Docs, Release |
| Goalpost | GP0, GP1, GP2, GP3, GP4, GP5, GP6                                   |
| Target   | 1.0, Deferred, Research                                             |
| Risk     | Low, Medium, High, Critical                                         |
| Proof    | Missing, Unit, Integration, Conformance, Network, Release           |
| Slice    | S, M, Needs decomposition                                           |

Repository is native metadata. Do not create a custom Repository field.

An item with Slice `Needs decomposition` is not ready implementation work. It
must be split until the remaining work is S or M.

## Proof Policy

Close issues because their exit criteria passed, not because code merged.

Every release gate issue must include:

- Contract
- Owner
- Executable check
- Required evidence
- Negative cases
- Compatibility set
- Pass rule

Every gate pass rule must be binary. Avoid "mostly", "sufficient", "solid
enough", or "looks good" as release criteria.

Proof values mean:

- Missing: no executable proof is linked.
- Unit: local unit proof only.
- Integration: multi-component proof without conformance or network boundary.
- Conformance: profile or protocol conformance report.
- Network: real network-boundary proof.
- Release: release-candidate or compatibility-set proof.

Default release views must filter to Target `1.0`.

## WAL/WSC Sequencing Doctrine

WAL bytes are the durable commit authority.

WARP graph WAL nodes are projected evidence facts; they are not recovery
authority.

WSC carries or references that evidence; transport arrival is not semantic Echo
history.

Recovery bootstraps from WAL root or storage manifest material, not from
pre-existing graph facts.

The WAL/WSC doctrine is tracked by
[#521 WAL/WSC Storage Relationship](https://github.com/flyingrobots/echo/issues/521)
and recorded in
[`docs/design/wal-wsc-durability-roadmap.md`](wal-wsc-durability-roadmap.md).

## Operating Guardrails

Do not place live planning, issue inventories, progress bars, current batches,
or mutable release status into repository docs.

Do not multiply milestones to represent cross-repository architecture. Use the
Project Goalpost field for that.

Do not duplicate native GitHub metadata with custom fields.

Do not let research work appear in the default Echo 1.0 view.

Do not place long-lived personal tokens in repositories for Project
automation. Persistent automation should use a GitHub App with Project
permissions.
