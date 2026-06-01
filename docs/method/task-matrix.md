<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# METHOD Task Matrix

Status: retired by the GitHub Issues migration on 2026-06-01.

This document used to be generated from `docs/method/backlog/**`. That
filesystem backlog is no longer the live work tracker. Echo now tracks Method
backlog work in GitHub Issues, with Method lanes represented by labels.

The historical filesystem cards were migrated in
[issue #390](https://github.com/flyingrobots/echo/issues/390) and archived under
[`docs/method/graveyard/github-issue-migration/`](graveyard/github-issue-migration/).
The archive mapping from old card path to migrated issue lives in
[`docs/method/graveyard/github-issue-migration/README.md`](graveyard/github-issue-migration/README.md).

## Current Tracking Surface

Use GitHub Issues, not this file, for active work tracking:

- [ASAP lane](https://github.com/flyingrobots/echo/issues?q=is%3Aissue%20is%3Aopen%20label%3Alane%3Aasap)
- [Up-next lane](https://github.com/flyingrobots/echo/issues?q=is%3Aissue%20is%3Aopen%20label%3Alane%3Aup-next)
- [Release lane](https://github.com/flyingrobots/echo/issues?q=is%3Aissue%20is%3Aopen%20label%3Alane%3Arelease)
- [Inbox lane](https://github.com/flyingrobots/echo/issues?q=is%3Aissue%20is%3Aopen%20label%3Alane%3Ainbox)
- [Cool-ideas lane](https://github.com/flyingrobots/echo/issues?q=is%3Aissue%20is%3Aopen%20label%3Alane%3Acool-ideas)
- [Bad-code lane](https://github.com/flyingrobots/echo/issues?q=is%3Aissue%20is%3Aopen%20label%3Alane%3Abad-code)
- [Work in progress](https://github.com/flyingrobots/echo/issues?q=is%3Aissue%20is%3Aopen%20label%3Awork-in-progress)

## Retired Matrix Semantics

The old matrix encoded file-local dependency hints among backlog cards. Those
hints remain preserved in the archived card text and in the migrated issue
bodies, but this Markdown matrix must not be treated as canonical after the
migration.

For new work:

- create or update a GitHub Issue;
- apply exactly one Method lane label unless the work is deliberately being
  re-triaged;
- use `work-in-progress` only for active work;
- express dependencies with issue links, issue relationships, and design-doc
  references rather than resurrecting `docs/method/backlog/**` rows.
