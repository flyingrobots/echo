<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# WARP Optics

Echo's public surfaces follow one shape:

```text
explicit causal basis or site
+ bounded aperture
+ named law
+ support, capability, budget, and evidence posture
-> witnessed hologram
```

## Ontology

There is witnessed causal history. WARP optics chart it. Holograms witness those
charts. Materialized graphs, files, editor buffers, UI state, and debug views are
observer-relative readings; they are not the substrate ontology.

An optic therefore cannot be specified by a query noun alone. Its identity and
result must account for:

- the explicit causal basis or site resolved for the invocation;
- the bounded region it may observe or affect;
- the law under which admission or revelation occurs;
- the caller's capabilities and the runtime's support posture;
- the applicable resource budget; and
- the evidence retained for success, rejection, obstruction, or absence.

## Authority Boundaries

- Application-authored intent does not tick the runtime.
- Admission tickets are not execution receipts.
- Trusted runtime control owns scheduler opportunities and recovery actions.
- Query observers are read-only and execute against a resolved basis.
- Transport arrival is not semantic history; lawful Echo acceptance is.
- Retry is a new explicit causal act, never a hidden loop.
- Application nouns belong in authored contracts and generated adapters, not in
  Echo core.

## Runtime Shape

Generated packages register operation and query identities, handlers, observer
plans, and declared footprints. Echo resolves support and authority, stages
ticketed ingress, runs scheduler-owned ticks, emits receipt correlations, and
returns bounded reading envelopes. Retained artifacts are addressed by both
content identity and semantic coordinates; one must not impersonate the other.

The central implementation seams are:

- package verification and registration in
  `crates/warp-core/src/contract_registry.rs` and
  `crates/warp-core/src/engine_impl.rs`;
- public application and trusted-host DTOs in `crates/echo-wasm-abi`;
- retained content and semantic lookup in `crates/echo-cas`; and
- causal WAL/WSC evidence in `crates/warp-core`.

## Non-Regressions

- No synchronous execution hidden inside application dispatch.
- No application-controlled tick, WAL, package-install, or recovery authority.
- No graph, file, or UI reading promoted into source-of-truth state.
- No broad ambient query without an explicit basis and aperture.
- No success posture when required evidence is missing, redacted, corrupt, or
  outside budget.
