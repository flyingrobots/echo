<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo 1.0 Release Contract

This document is a release constitution, not a live roadmap. It records the
promises that must be proven before Echo 1.0 can ship. It must not contain
dates, percentages, live task lists, current branch state, issue inventories, or
work-in-progress status.

Live release state belongs in GitHub:

- Continuum Stack Convergence Project:
  <https://github.com/users/flyingrobots/projects/15>
- Echo 1.0 Release Bar:
  [#584](https://github.com/flyingrobots/echo/issues/584)
- Echo 1.0 milestone:
  <https://github.com/flyingrobots/echo/milestone/31>

The Project is the cross-repository control surface. Issues and pull requests
are the work. Executable evidence is the proof. This file records Echo's binary
release bar inside the larger Continuum stack convergence.

## Release Gates

Echo 1.0 is blocked until all three release gates are closed by
evidence-bearing pull requests:

| Gate | Issue                                                                                              | Contract                                                                                |
| ---- | -------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| A    | [#585 Gate A - Continuum Participant Conformance](https://github.com/flyingrobots/echo/issues/585) | Echo is a lawful Continuum participant.                                                 |
| B    | [#591 Gate B - Networked Causal Suffix Exchange](https://github.com/flyingrobots/echo/issues/591)  | Echo and `git-warp` exchange witnessed causal suffixes over a real network boundary.    |
| D    | [#588 Gate D - Release Integrity](https://github.com/flyingrobots/echo/issues/588)                 | The release candidate is reproducible, packaged, documented, compatible, and auditable. |

The release bar issue [#584](https://github.com/flyingrobots/echo/issues/584)
depends on all three gate issues. Gate D depends on Gates A and B, because a
release candidate may only be cut from the demonstrated compatibility set.

## Definitions

Continuum participation status attained means Echo publishes a versioned
participant descriptor, declares its capability and profile posture, supports
registration, admission, invocation, and obstruction receipt chains, and
publishes a conformance report tied to exact commits.

Networked causal suffix exchange means Echo and `git-warp` exchange witnessed
causal suffixes in both directions over a real network boundary, survive
disconnect and resume, deterministically admit or reject duplicate, reordered,
stale, and tampered suffixes, and prove reconstructed state from received
history.

Release integrity means Echo ships only from a clean-room reproducible build,
an exact cross-repository compatibility set, verified package and documentation
artifacts, upgrade and rollback evidence, and the closed evidence from all
other gates.

A proof packet is a downloadable or inspectable evidence bundle containing the
repository SHAs, dependency versions, workflow runs, conformance results,
network traces, admission receipts, invocation receipts, artifact digests,
negative-test output, and release manifest entries required by the relevant
gate.

A compatibility set is the pinned multi-repository state used to build,
exercise, and release Echo 1.0. It must include Echo, `git-warp`, Continuum, and
every artifact schema or protocol version needed to verify the release gates.

## Non-Goals

This contract does not schedule work, estimate completion, count open issues,
track current status, or replace GitHub Issues, Projects, milestones, pull
requests, or evidence artifacts.

Echo 1.0 does not require every research idea, demo surface, browser surface,
or deferred platform feature to ship. Work outside Target `1.0` must not appear
in default release views.

Echo core must not absorb application nouns. Application nouns belong in
authored contracts, compiled artifacts, adapters, fixtures, and external
applications.

State sync is not sufficient for Gate B. The release proof must be witnessed
causal suffix exchange with deterministic admission and rejection behavior.

Edict and `jedit` compatibility work does not gate Echo 1.0. A particular
application, authoring language, or generated rule package may prove downstream
compatibility independently, but it is not part of Echo's binary release bar.

Independently green repositories are not sufficient for Gate D. Echo 1.0 is one
demonstrated compatibility set, not several unrelated successful CI runs.

## Compatibility Policy

Gate D must produce a machine-readable release manifest named
`echo-convergence.lock` or an equivalent checked release artifact. The manifest
must pin:

- Echo commit and release candidate version.
- `git-warp` commit and suffix-exchange protocol version.
- Continuum commit and participant profile version.
- Dependency versions and any generated-package versions actually included in
  the release candidate.
- Workflow runs, proof-packet digests, and release-candidate artifact digests.

No release candidate may be generated from unpinned repository state,
unversioned protocol posture, or implicit local working directories.

## Evidence Requirements

Gate A requires a versioned participant descriptor, declared capability/profile
posture, registration receipt chain, admission receipt chain, invocation
receipt chain, exercised obstruction vocabulary, and a published conformance
report tied to exact commits.

Gate B requires bidirectional network exchange, disconnect and resume,
duplicate/reordered/stale/tampered suffix cases, deterministic admission or
rejection, reconstructed state proof from received history, and an evidence
bundle containing both peers' identities and commit SHAs.

Gate D requires a clean-room reproducible build, exact cross-repository
compatibility set, upgrade and rollback tests, package and documentation
verification, all three gate issues closed by evidence-bearing PRs, and a
release candidate generated only from the pinned compatibility set.

## Pass Rule

Echo 1.0 is releasable only when every release gate returns a binary pass from
its executable check, every required proof packet exists and is linked from the
owning issue, every negative case is exercised, every compatibility coordinate
is pinned, and the release candidate is generated from the pinned compatibility
set.

There is no partial pass for Echo 1.0.
