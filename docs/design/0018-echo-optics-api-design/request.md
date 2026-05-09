<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Optics API Design Request

This file archives the source prompt for Echo's first-class Optics API design.
The design doc and backlog tasks for this body of work should use this request
as the controlling reference.

````text
Yep — use this instead:

You are working in the Echo repository.
Your task is to design Echo’s first-class Optics API and API surfaces.
This is not a jedit design task. jedit may be used only as an example consumer to validate ergonomics.
# Goal
Design a generic Echo Optic model for bounded, capability-scoped, coordinate-anchored observation and intent dispatch over Echo causal history.
An Echo Optic must support future consumers such as editors, debuggers, inspectors, replay tools, import/export flows, and retained reading caches.
# Core Doctrine
Use this rule:
```text
Optic reads.
Intent proposes.
Echo admits.
Receipt witnesses.

An optic is not a mutable handle.

An optic is:

Optic = capability + focus + coordinate + projection law + intent family

An optic does not mutate its subject. It names:

1. a lawful way to observe a focused projection
2. a lawful family of intents that may be submitted against that projection

Optic writes are never setters. They are intent dispatches against an explicit causal basis.

Required Conceptual Model

Design optics over:

* worldlines
* strands
* braids
* coordinates/frontiers
* retained readings
* cached readings
* observer apertures
* witness-backed projections

An optic read should:

choose aperture
-> slice causal history
-> lower under law
-> witness
-> retain if needed
-> emit observer-relative reading

An optic intent dispatch should:

construct intent
-> validate capability
-> validate causal basis
-> apply/admit under contract law
-> emit tick/admission result
-> emit receipt/witness

Required API Surface

Design the smallest useful Echo Optic API.

It should include generic concepts such as:

* EchoOptic
* OpticId
* OpticFocus
* OpticAperture
* EchoCoordinate
* ReadingEnvelope
* ObserveOpticRequest
* DispatchOpticIntentRequest
* IntentDispatchResult
* OpticCapability
* OpticObstruction
* ReadIdentity
* WitnessBasis
* ProjectionVersion
* ReducerVersion

Sketch possible interfaces in the repo’s preferred language/style.

Do not make GraphQL the core runtime API. GraphQL-like examples may be included only as adapter illustrations.

Required Semantics

Reads

An optic read must be bounded.

A read must name:

* optic id
* focus
* aperture
* causal coordinate/frontier
* projection law/version
* reducer law/version where relevant
* witness basis
* read identity
* residual or obstruction posture
* bounds/budget posture

Reads must not secretly fall back to full materialization.

If evidence is missing, return an explicit obstruction.

Writes / Intent Dispatch

The optic must not expose setters.

The write-side surface should be named something like:

* dispatchOpticIntent
* submitOpticIntent
* proposeIntent

Do not call it set.

An intent dispatch must name:

* optic id
* base coordinate/frontier
* intent family
* subject/focus
* actor/cause
* capability basis
* admission law
* resulting tick/receipt/admission posture

If the base coordinate is stale, the API must not silently mutate current state.

It may:

* reject
* obstruct
* stage
* preserve plurality
* require rebase
* admit under explicitly named law

Admission Outcomes

Keep admission outcomes typed and explicit:

* Admitted
* Staged
* Plural
* Conflict
* Obstructed

Do not collapse this into:

* Ok/Err
* boolean success
* string status
* latest-writer-wins
* hidden host-time ordering

Cached / Retained Readings

Design how optics interact with retained readings and echo-cas.

A retained reading needs both:

* content hash
* semantic coordinate / read identity

The CAS hash names bytes.

The read identity names the question those bytes answer.

A cached reading remains valid only for the coordinate, witness basis, projection version, reducer version, aperture, rights, and budget posture it names.

New ticks should create new frontiers, not mutate old readings.

Live Tail Honesty

Do not allow an optic read to return a stale checkpoint hash as if it identified the live result.

Honest options:

* reduce the live tail under bounded witness basis
* return a read identity naming checkpoint basis plus tail witness set
* return slice hash or witness-set hash with explicit meaning
* fail closed with obstruction or missing-basis posture

Attachments / Recursive Boundaries

Optics should treat attachments as explicit aperture boundaries.

Default readings should expose attachment refs or obstruction posture.

Recursive descent into attachments must be explicit and capability/budget/law checked.

Non-Goals

Do not design:

* a global graph API
* a global mutable state API
* a file-handle API
* direct setters
* hidden materialization fallback
* a sync daemon
* git-warp dependency
* proof system implementation
* GraphQL-first runtime substrate
* host-bag abstractions like RuntimeFacade, ObservationManager, UniversalMaterializer, or GraphLikeRuntimeAdapter

Deliverables

Produce:

1. Design Summary

Explain the ideal Echo Optics API.

Include:

* what an optic is
* what an optic is not
* read semantics
* intent dispatch semantics
* coordinate/frontier behavior
* cached reading behavior
* live-tail honesty
* admission outcomes
* attachment boundaries
* capability model

2. Proposed Types / Interfaces

Draft concrete type/interface sketches for the Optics API.

Include:

* optic descriptor
* open optic request/result
* observe request/result
* intent dispatch request/result
* reading envelope
* read identity
* witness basis
* obstruction model
* admission outcome model
* retained reading key
* capability model

3. API Surface Proposal

Propose the smallest initial public API surface.

Prefer something like:

openOptic
closeOptic
observeOptic
dispatchOpticIntent
retainReading
revealReading

But challenge this list if better names or smaller cuts exist.

Clearly separate:

Optic observes.
Admission admits.
Retention retains.
Plumber maintains.
Debug explains.

4. Compatibility With Existing Echo Doctrine

Explain how this design aligns with:

* witnessed causal history as substrate truth
* observer-relative readings
* bounded replay/reveal
* suffix admission
* tick receipts as holographic witnesses
* echo-cas as retention, not ontology
* Echo as a peer Continuum runtime, not a git-warp subordinate

5. Test Strategy

Propose tests proving:

* optic reads name causal basis
* optic reads are bounded
* missing evidence obstructs instead of materializing everything
* cached readings are keyed by read identity, not just content hash
* live-tail reads do not reuse stale checkpoint hashes
* intent dispatch requires explicit base coordinate
* stale base coordinate does not silently mutate current state
* admission outcomes stay typed
* attachment descent is explicit
* plumber/debug APIs do not become hidden fallbacks

6. Backlog Tasks

Produce a METHOD-friendly backlog task series.

Each task must include:

* title
* goal
* files likely touched
* acceptance criteria
* non-goals
* test expectations

Prefer small slices.

Suggested order:

1. Add doctrine/design packet for Echo Optics.
2. Define core optic nouns and IDs.
3. Define ReadingEnvelope and ReadIdentity.
4. Define WitnessBasis and retained reading key.
5. Define optic obstruction/admission result families.
6. Define openOptic / closeOptic request models.
7. Define observeOptic model with bounds and aperture.
8. Define dispatchOpticIntent model with explicit base coordinate.
9. Add stale-basis obstruction tests.
10. Add cached-reading identity tests.
11. Add live-tail hash honesty tests.
12. Add attachment boundary/descent placeholder model.
13. Add narrow fake/example optic implementation for one simple contract.
14. Add adapter notes for future editor/debugger/replay consumers.

Output Format

Use:

# Echo Optics API Design
## Summary
## Core Doctrine
## Optic Model
## Public API Surface
## Types And Interfaces
## Read Semantics
## Intent Dispatch Semantics
## Admission Outcomes
## Cached And Retained Readings
## Live Tail Honesty
## Attachments And Recursive Apertures
## Capability Model
## Relationship To Existing Echo Doctrine
## Test Strategy
## Backlog
### TASK-001: ...
### TASK-002: ...

Be strict.

Reject direct mutation.

Reject global graph/state APIs.

Reject broad host-bag abstractions.

Keep the design small, typed, bounded, causal, capability-scoped, and testable.
````
