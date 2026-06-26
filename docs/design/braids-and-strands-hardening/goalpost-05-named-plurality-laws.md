<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Goalpost 5: Named Plurality Laws

Status: implemented.

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

## Implementation Design

`PluralityLawRef` is the named law reference:

```text
PluralityLawFamily
+ PluralityLawName
+ version
```

Law names cannot be the all-zero digest, and law versions start at 1. Existing
braid settlement policy ids map through the fallible
`PluralityLawRef::settlement_policy(...)` constructor, preserving the current
retained policy identity while making the law family and version explicit in
replay. Braid shell assembly and validation reject all-zero policy ids before a
retained shell can claim a named law reading. Collapse-derived shells map their
`collapse_policy` id through `PluralityLawRef::collapse_policy(...)`, so replay
and audit report collapse readings as `PluralityLawFamily::Collapse` instead of
settlement-law readings.

`PluralityLawFamily` is core-generic: settlement, collapse,
conflict-preserving, quorum, authority, and adapter-provided. Adapter-provided
families are scoped by `AuthorityDomainRef`, so Echo core can route
domain-specific laws without importing application-domain law nouns.

`PluralityLawCard` is the machine-readable Law Card. It binds a law reference,
required support/evidence facts, emitted artifact or reading classes, concealed
material classes, and `PluralityLawEvidencePosture`. Requirements, emissions,
and concealments are sorted and deduplicated before card identity is computed,
so caller vector order is not part of law identity.

`PluralityLawRegistry` registers Law Cards deterministically by
`PluralityLawRef`. Duplicate registration returns
`PluralityLawRegistryError::DuplicateLaw`. Execution authorization returns
`PluralityLawAuthorization` for registered laws and typed
`PluralityLawObstruction` for unsupported or unauthorized execution.

`PluralityLawReading` binds the law reference, retained support digest,
witness receipt, evidence posture, and disclosure budget into a witnessed
reading digest. `BraidShellReplay` now carries the settlement `law_ref`, and
`BraidShellAudit` carries the full `law_reading` so retained braid readings
state which named law interpreted plurality.

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
