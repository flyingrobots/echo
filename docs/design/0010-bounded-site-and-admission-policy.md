<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0010 - Bounded Site and Admission Policy

Cycle: `0010-bounded-site-and-admission-policy`  
Legend: `KERNEL`  
Source backlog item: `docs/method/backlog/up-next/KERNEL_bounded-site-and-admission-policy.md`

## Sponsors

- Human: Runtime / ontology architect
- Agent: Kernel implementation agent

## Hill

Echo gets one explicit admission-side noun for the site over which claims are
judged, and one explicit policy story for how `super_tick()` decides lawful
results. The runtime keeps its single-tick law, but stops scattering policy
truth across unrelated structs and comments.

## Playback Questions

### Human

- [ ] Can we point to one explicit site noun and explain what part of local
      graph truth is being judged during a tick, compare, or settlement step?
- [ ] Can a host choose policy without being allowed to inject bespoke causal
      law into the engine?

### Agent

- [ ] Can I explain `super_tick()` as admission over ingress claims at bounded
      sites under explicit policy?
- [ ] Can I map existing Echo policy fragments into one admission-law family
      without pretending the runtime is a reducer-only machine?

## Accessibility and Assistive Reading

- Linear truth / reduced-complexity posture: the packet should answer four
  questions in order: what a bounded site is, what policy governs admission,
  what lawful outcomes exist, and what shell/witness layering follows.
- Non-visual or alternate-reading expectations: the runtime noun stack must be
  reconstructible from text without assuming diagrams or debugger UI.

## Localization and Directionality

- Locale / wording / formatting assumptions: the terms remain ASCII-first and
  stable enough for code, logs, tests, and generated debug output.
- Logical direction / layout assumptions: this packet defines runtime law, not
  screen layout or observer-product presentation.

## Agent Inspectability and Explainability

- What must be explicit and deterministic for agents: the admission site, the
  governing policy identity, the lawful outcome family, and the distinction
  between witness core and shell.
- What must be attributable, evidenced, or governed: when policy changes causal
  meaning, the engine must be able to name that policy; observer-side
  summarisation must not masquerade as canonical admission.

## Why this cycle exists

Echo already has most of the raw pieces:

- content-addressed ingress
- one `super_tick()` path
- `policy_id` in engine/patch identity
- `ChannelPolicy`, `ConflictPolicy`, `HeadEligibility`, `PlaybackMode`, and
  `RetentionPolicy`
- settlement reason classes and neighborhood publication

What Echo lacks is one explicit admission-law object model that ties those
pieces together.

Without that model, the runtime risks staying correct in implementation while
remaining blurry in doctrine:

- policy exists, but as fragments
- sites exist, but under several overlapping names
- outcomes exist, but not yet as one explicit admission algebra
- shells exist, but are too easily mistaken for the witness itself

## Design decision

Echo should state its central runtime step as:

**`super_tick()` performs admission over ingress claims at bounded sites under
explicit engine-defined policy, yielding lawful outcomes plus witness-bearing
shells.**

## Core laws

### 1. Admission site law

Echo needs one explicit admission-side noun:

- `BoundedSite`

`BoundedSite` is Echo's formalisation of focal closure on the admission side.
It names the local site over which coexistence or obstruction is judged.

This is not a new competing geometry. It is the place where Echo cashes out the
existing ideas already present in:

- footprint
- affect / affected region
- reintegration boundary
- optional derived neighborhood publication

### 2. Site structure law

`BoundedSite` should be rich enough to explain lawful judgement and small enough
to avoid becoming a second ontology of the whole graph.

At minimum it should carry or derive:

- the direct claim footprint
- the locally affected region needed for admission
- the reintegration boundary needed for later comparison or settlement

Neighborhood publication may be derived from the same underlying truth, but it
is not the authoritative site noun for admission.

### 3. Policy law

Admission policy in Echo is engine-defined law.

The allowed model is:

- Echo defines deterministic policy families
- hosts select, parameterise, or reference them
- policy identity is explicit when it changes published causal meaning

The forbidden model is:

- hosts injecting bespoke executable admission law and still claiming that the
  result is ordinary Echo truth

### 4. Outcome law

Echo should standardise one admission outcome family that spans local tick work
and the later braid/settlement story:

- `Derived`
- `Plural`
- `Conflict`
- `Obstruction`

This does not require every subsystem to emit the same struct today. It does
require the runtime and docs to stop speaking as if every unresolved situation
were merely failure or residue.

### 5. Shell law

Echo should preserve the distinction between:

- the lawful result
- the witness of lawfulness
- the shell or hologram used to carry that result onward

`TickReceipt` is one shell family.
Settlement publications and later transport artefacts may be other shell
families.

The runtime should not pretend that every witness-bearing publication is
secretly the same receipt type.

### 6. Single-tick admission law

This packet does not introduce a second execution model.

All state change still happens under the existing single-tick law:

- claims enter through ingress
- Echo routes and orders them deterministically
- `super_tick()` performs the admission step
- new graph truth and shell artefacts are emitted

This packet only makes the law explicit and names the missing site/policy
objects honestly.

## Existing policy surfaces to unify

The raw policy substrate already exists in Echo. The work is to align and name
it coherently:

- `policy_id` in engine/patch identity
- `ChannelPolicy`
- `ConflictPolicy`
- `HeadEligibility`
- `PlaybackMode`
- `RetentionPolicy`
- settlement reason classes

## First implementation cut

The first cut does not need to rewrite the whole engine.

It should:

1. introduce an explicit `BoundedSite` noun
2. restate `super_tick()` in docs and code comments as admission over claims at
   bounded sites under explicit policy
3. introduce or stabilise one admission outcome family
4. keep existing policy fragments, but route them through one clearer
   admission-law story

## Non-goals

- replacing Echo's scheduler with a different execution model
- freezing one cross-engine site struct
- making settlement or braid publication authoritative in the same cut
- inventing host-authored policy plugins

## Backlog Context

- [0006 - Echo Continuum alignment](./0006-echo-continuum-alignment.md)
- [0007 - Braid geometry and neighborhood publication](./0007-braid-geometry-and-neighborhood-publication.md)
- [0008 - Strand settlement](./0008-strand-settlement.md)
- [0009 - Strand runtime graph ontology](./0009-strand-runtime-graph-ontology.md)
- External reference: Continuum `0020 - Shared Admission and Policy Publication`
