<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# DECLARATIVE-RULE-AUTHORSHIP

**Status:** Normative | **Legend:** KERNEL | **Cycle:** 0012

## Invariant

Echo's deterministic execution path admits user-authored rewrite logic only as
Wesley-compiled declarative IR. Native executable rewrite code is a
trusted/bootstrap-only engine implementation surface, not a public user
extension boundary.

For Echo, that declarative boundary is the GraphQL-authored shared family
surface: graph entities, graph rewrites, declared footprints, and any types
that cross Rust, TypeScript, WASM, process, or network boundaries. The rest of
the runtime may remain handwritten Rust as long as it does not become a shadow
authority for user-authored rewrite law.

## Rulings

The following rulings are normative. "MUST" and "MUST NOT" follow
RFC 2119 convention.

### R1 — User-authored rewrite law is declarative

User-authored rewrite logic MUST enter Echo as Wesley-compiled declarative IR.
User-authored rewrite law MUST NOT enter the deterministic path as handwritten
native callbacks, function pointers, or ad hoc executable host code.

GraphQL-authored shared families are the intended source for user-authored
graph entities, graph rewrites, declared footprints, and cross-boundary types
that Echo exposes to hosts, `warp-ttd`, or other engines.

Echo MAY keep handwritten Rust for engine internals, storage, scheduler
mechanics, and other local implementation details, but those details MUST NOT
replace the shared family as the public authoring boundary for rewrite law or
cross-boundary types.

### R1a — Declared footprints are part of the authored boundary

When a user-authored graph rewrite enters Echo through the shared family
surface, its declared footprint is part of the contract, not optional
commentary.

The long-term target is for Wesley-generated Rust surfaces to make those
capability boundaries explicit enough that dishonest footprint use becomes
structurally impossible or a compile-time failure, rather than something Echo
discovers only after arbitrary native code has already been admitted.

### R2 — Native rewrite functions are bootstrap-only

`RewriteRule`, `MatchFn`, and `ExecuteFn` are bootstrap-only trusted-code
surfaces. They MAY exist for engine internals, internal system rules,
transitional bootstrap code, and tests, but they MUST NOT be treated as the
long-term public authoring boundary for application rewrite logic.

### R3 — Host policy is selected, not authored

Hosts MAY select engine-defined deterministic policy by reference. Hosts MUST
NOT inject bespoke executable admission law into Echo's deterministic path.

### R4 — Public host boundaries remain callback-free

The browser/WASM deterministic boundary MUST remain byte-oriented and
callback-free. Public host adapters MUST NOT smuggle executable host callbacks
or host-authored closures into the deterministic kernel loop.

### R5 — Ambient-state exemptions do not legitimize native rule authorship

Allowlist exemptions in determinism scanning MUST NOT be used to legitimize
user-authored native rewrite execution on the deterministic path. If a file is
reachable from user-authored rewrite execution, nondeterministic API usage MUST
be refactored away rather than excused by policy.

### R6 — Transitional sandboxes remain subordinate to the same law

If Echo temporarily introduces a sandboxed authoring layer, that layer is
acceptable only if it compiles or lowers to the same lawful declarative
substrate and does not reopen arbitrary executable host-side escape hatches.

## Rationale

The wasm/browser boundary is not currently the main determinism hole. The real
escape hatch is the native rule API itself: Echo still runs executable matcher
and executor function pointers directly on the deterministic path.

That may be acceptable for trusted bootstrap code, but it is not an acceptable
public authoring story. As long as user-authored logic can enter as arbitrary
native code, Echo cannot fully guarantee that ambient state, hidden callbacks,
or impure helper code will stay out of deterministic execution.

The correct closure is not "be careful." The correct closure is to narrow the
user authoring boundary until it is declarative, inspectable, and compilable by
Wesley into a lawful rewrite substrate.

That does not require Wesley to compile the entire application. It requires
Wesley to own the lawful graph/rewrite boundary so Echo can keep runtime
internals handwritten without exposing arbitrary native author code on the
deterministic path.

## Consequences

- Echo may keep native `RewriteRule` internals for bootstrap and tests.
- Echo MUST move user-facing rewrite authorship toward Wesley-generated
  declarative IR instead of stabilizing native rule callbacks as product API.
- Determinism audit gates should scrutinize rule-authoring and host-boundary
  files more strictly than ordinary support code.
- A sandboxed language is acceptable only as a front-end to the same
  declarative substrate, not as a new executable bypass around it.

## Cross-references

- [0010-bounded-site-and-admission-policy](../design/0010-bounded-site-and-admission-policy.md)
- [TTD-COUNTERFACTUAL-CREATION](./TTD-COUNTERFACTUAL-CREATION.md)
- [FIXED-TIMESTEP](./FIXED-TIMESTEP.md)
- [RELEASE_POLICY](../RELEASE_POLICY.md)
- [KERNEL_determinism-escape-hatches](../method/backlog/up-next/KERNEL_determinism-escape-hatches.md)
