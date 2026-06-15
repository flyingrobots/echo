<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Goalpost 5: Named Plurality Laws

Status: planned.

Roadmap:
[`../braids-and-strands-roadmap.md`](../braids-and-strands-roadmap.md)

## Decision Summary

Echo will make retained plurality interpretation explicit through named,
versioned law machinery. Echo core provides the law registry shape and
obstruction posture; adapter-provided law families carry domain-specific
meaning outside core.

## Invariant

Retained plural claims are never interpreted by hidden caller policy. A
witnessed reading names the law, law version, evidence posture, and support
used to interpret plurality.

## Sponsored Human

A maintainer wants future plurality and collapse behavior to be reviewable as
law, not scattered caller logic.

## Sponsored Agent

An agent needs machine-readable Law Cards and typed obstruction evidence so it
can inspect whether a reading was authorized, supported, and produced under
the intended law.

## Scope

This goalpost includes:

- plurality law registry shape;
- Law Cards;
- law name and version binding in witnessed readings;
- adapter-provided law family routing;
- unsupported or unauthorized law obstruction evidence.

## Non-Goals

This goalpost does not include:

- application-domain law nouns in Echo core;
- hiding plurality interpretation inside callers;
- executing laws before replay and witness boundaries exist;
- collapsing braided strands into merge semantics.

## Slices

| Slice  | Work                                | Witness                                 |
| ------ | ----------------------------------- | --------------------------------------- |
| GP5-S1 | Define plurality law registry shape | compile/API tests for law registration  |
| GP5-S2 | Add machine-readable Law Cards      | schema or fixture validation            |
| GP5-S3 | Bind law name/version into readings | witnessed reading identity tests        |
| GP5-S4 | Route adapter-provided law families | no-app-nouns guard plus adapter fixture |
| GP5-S5 | Add law obstruction evidence        | unsupported/unauthorized law tests      |

## Acceptance

- A retained braid reading states which law interpreted plurality.
- Two laws over the same retained support produce distinct witnessed readings.
- Unsupported or unauthorized law execution yields typed obstruction evidence.
- Echo core keeps application nouns out while allowing adapters to provide
  domain-specific law families.
