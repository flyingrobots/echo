<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Kitten Sessions And Delegated Authority

**Status:** exploratory design note, not an accepted decision. Unlike the
files in `docs/adr/`, this is not settled architecture — it is a forward
pointer for whoever eventually designs the session/authority extension this
note describes. Treat ADR 0018 ("Sessions as Causal Posture and Authority")
as the accepted precedent this would extend, not replace.

## Context

The Profunctor Optics constitution (the cross-project architecture
document covering Xyph, Echo, Continuum, WARP DRIVE, and related
components) introduces a **Kitten** as the canonical active participant:
"an Observer equipped for active participation." Every Kitten owns or
references a `KittenSession`:

```text
KittenSession {
  kitten_identity
  observer_session_ref
  navigator_position
  current_intent_refs
  delegated_capability_refs
  active_obligation_refs
  update_state_or_memory_root
  continuation_profile
}
```

`ObserverSession` (ADR 0018's existing session-as-causal-posture concept)
answers _how_ a participant observes. `KittenSession` answers _who_ is
actively participating through that observation geometry, and critically
keeps `observer_session_ref` and `delegated_capability_refs` as separate
fields — a Kitten acting on another's behalf holds a _reference_ to that
other's session plus an explicit statement of what was delegated to it, not
a copy or substitution of the grantor's identity.

## Where this is already almost wired

`ObservationRequest` (`crates/echo-wasm-abi/src/kernel_port.rs`) already has
real fields that a `KittenSession`-shaped caller would populate meaningfully:

```rust
pub struct ObservationRequest {
    pub coordinate: ObservationCoordinate,
    pub frame: ObservationFrame,
    pub projection: ObservationProjection,
    pub observer_plan: ReadingObserverPlan,
    pub observer_instance: Option<ObserverInstanceRef>,
    pub budget: ObservationReadBudget,
    pub rights: ObservationRights,
}
```

Every caller observed so far (including WARP DRIVE's G2/G3 gates via
`ObservationRequest::builtin_one_shot()`) fills `observer_instance`,
`budget`, and `rights` with the most degenerate possible values: `None`,
`UnboundedOneShot`, and `KernelPublic`. The shape for a real session/budget/
rights posture exists; nothing currently populates it with anything other
than a placeholder.

There is currently **no** field on `ObservationRequest` (or anywhere else
in the observation path) corresponding to the constitution's
`law_coordinate` or `policy_coordinate`. Real law/policy types exist
elsewhere in this crate (`PluralityLawRef`, `SettlementPolicy`,
`AuthorityPolicy`, `AdmissionPolicyRef`), but none of them are wired to a
read.

## The authority-and-time-travel question

If a caller ever holds a `KittenSession` whose delegated authority is
narrower than what the grantor held at some earlier coordinate, does
reading that earlier coordinate through Causal Travel resurrect the wider
authority?

No. Authority observed at a historical coordinate is a _Then-known_ fact
(honest history), never a _Now-known_ exercisable grant. Admission of any
new proposal must resolve the admitting authority at the coordinate of
admission — the current frontier, whatever it is — never at the coordinate
a prior reading was taken from. This is the same law already governing
stale-basis writes: a proposal built from a wider-authority historical
reading, submitted after that authority narrowed, should receive the same
typed obstruction a stale-basis write receives, not a silent pass-through.

## Open questions

- Does a `KittenSession`'s `delegated_capability_refs` track the grantor's
  _current_ authority continuously (shrinking the instant the grantor's
  does), or is a delegation a discrete grant fixed at issuance that
  requires separate, explicit revocation? Both are lawful; this repo
  hasn't picked one.
- Would `law_coordinate`/`policy_coordinate` become new fields on
  `ObservationRequest` directly, or would they resolve through
  `observer_instance`/`rights` indirectly?
- Is `ObserverInstanceRef` (already `Option`-typed on `ObservationRequest`)
  the right seam to grow into a full `KittenSession` reference, or does
  that deserve its own type?

## Related

- `docs/adr/0018-sessions-causal-posture-and-authority.md` — the accepted
  precedent this would extend.
- `docs/adr/0013-echo-continuum-authority-boundary.md`
- `docs/topics/RuntimeAuthority.md`
- Companion note in `warp-drive`:
  `docs/method/backlog/cool-ideas/GATE_warp-drive-as-app-kitten.md`
