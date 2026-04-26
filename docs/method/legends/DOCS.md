<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# DOCS — Documentation

_Legend for keeping what Echo says about itself honest, grounded, and
useful to newcomers and agents alike._

## Goal

Every document in the repo describes implemented reality. No
aspirational specs, no stale architecture outlines, no fiction.
Newcomers can find what they need; agents can parse what they read.

This legend covers work like:

- guides and entry points (start-here, eli5, warp-primer)
- specifications for implemented features
- course material and demo walkthroughs
- docs audit and cleanup
- keeping docs in sync with code as the kernel evolves

## Human users

- newcomers trying to understand what Echo is and how it works
- James, maintaining docs accuracy as the codebase evolves
- contributors looking for the right spec before modifying code

## Agent users

- agents reading specs to understand API contracts before generating
  code
- agents answering "how does X work?" by finding the right document
- agents auditing docs for staleness or inconsistency

## Human hill

A newcomer can read `docs/guide/start-here.md` and understand what
Echo is, how it works, and where to look next — without encountering
a single document that describes something that doesn't exist.

## Agent hill

An agent can search the docs directory and trust that every spec it
finds describes implemented behavior, so it can generate correct code
without having to cross-reference git history to check if the feature
actually shipped.

## Core invariants

- No spec exists for unimplemented features. Aspirations go in the
  backlog, not in specs.
- `docs/audits/docs-inventory-2026-04-26.md` tracks the current
  five-at-a-time inventory.
- Guide entry points link to real, working content.
- Stale docs are deleted, not left to mislead. Git is the archive.

## Current cycle and backlog

- latest completed cycle: (none under METHOD yet)
- live backlog:
    - `asap/DOCS_docs-cleanup.md`
    - `asap/DOCS_cli-man-pages.md`
