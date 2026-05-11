<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# RE-030 — Converge QueryView Reads onto Optics

Legend: [RE — Runtime Engine]

## Problem

Stack Witness 0001 currently proves `textWindow` through
`observe(QueryView)` using a narrow fixture shortcut. That keeps the first
jedit-through-Echo witness small, but it also leaves query reads outside the
stronger optic surface that should eventually own read identity, aperture
identity, basis validation, rights, budgets, and capability posture.

`QueryView` should be the reading frame for a query-producing optic, not a
parallel read side door.

## Desired Shape

Model contract query reads as optic-shaped observations:

```text
contract-authored optic
  focus: worldline / basis / read identity
  aperture: QueryBytes {
    query_id
    vars_digest
  }
  artifact identity
  observer plan
  rights posture
  budget posture
  evidence posture
```

The Stack Witness 0001 `textWindow` path should eventually move from direct
`observe(QueryView)` fixture interception to an optic aperture that produces
`ReadingEnvelope + QueryBytes("hello")`.

## Non-Goals

Do not do this before the walking skeleton is merged and stable. The current
PR should only close the direct-path integrity gaps:

1. validate requested worldline,
2. validate fixture vars,
3. require committed fixture history before materializing the read.

## Why

Optics are the right long-term home for:

1. durable read identity,
2. query aperture identity,
3. basis and worldline validation,
4. capability-scoped rights,
5. budget enforcement,
6. debugger/TTD explanation.

Keeping the follow-up explicit prevents Stack Witness 0001 from fossilizing a
temporary direct `QueryView` shortcut into the permanent architecture.

## Effort

Medium — requires an optic-backed query aperture path plus migration of the
Stack Witness `textWindow` fixture tests from direct `observe(QueryView)` to
the optic read surface.
