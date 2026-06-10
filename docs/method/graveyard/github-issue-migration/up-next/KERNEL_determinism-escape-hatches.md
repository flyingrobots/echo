<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# KERNEL - Determinism escape hatches audit and closure

Echo's browser/WASM boundary currently looks cleaner than the native rule
surface. A targeted audit found no obvious JS callback injection hole in the
public wasm ABI, but it did confirm one real determinism escape hatch inside
the kernel: host-authored native rewrite code still runs directly on the
deterministic path.

This cycle should turn that audit into repo truth and close the most dangerous
gaps instead of leaving the findings trapped in chat.

Current findings:

- **No obvious wasm callback hole.**
  `echo-wasm-abi::KernelPort` is byte-oriented and callback-free. The current
  host boundary exposes explicit requests such as `dispatch_intent(...)`,
  `observe(...)`, `observe_neighborhood_site(...)`, and
  `observe_neighborhood_core(...)` rather than host-supplied executable hooks.
  `warp-wasm` routes through an installed `KernelPort` object and does not
  currently expose `js_sys::Function`, `Closure<...>`, timer callbacks, or
  host-injected executable law.

- **The real hole is the native rule API.**
  `warp-core::rule::RewriteRule` still carries executable `MatchFn` and
  `ExecuteFn` function pointers. The engine invokes those pointers directly in
  both serial and parallel execution paths. They are plain `fn` pointers rather
  than captured closures, which is better than arbitrary callback objects, but
  they still allow host-authored Rust code to touch ambient state unless the
  project treats this API as trusted/bootstrap-only.

- **Worker count is an ambient knob, but not yet a proven semantic break.**
  `warp-core::engine_impl` reads `ECHO_WORKERS` and falls back to
  `available_parallelism()`. The audit did not find evidence that different
  worker counts change committed state on their own: rewrites are grouped in
  canonical order, parallel deltas are merged canonically, and the parallel
  executor already tests all policies against a serial oracle. Still, ambient
  worker selection widens the blast radius if rule code itself is impure.

- **Unordered containers exist, but the critical ordering surfaces inspected in
  this audit are already being canonicalised.**
  `HashMap`, `HashSet`, and `FxHashMap` still appear in engine and scheduler
  internals, but the audited hot paths sort, dedupe, or route through `BTree*`
  structures before producing committed state. This is a residual risk area, not
  a confirmed active break from this pass.

- **Materialization catches one class of nondeterminism, but not all of it.**
  The materialization bus is order-independent and rejects duplicate emissions,
  explicitly calling out unordered-source iteration as a structural bug. That is
  good. But it only catches some bad rule behavior. It cannot make an impure
  rule deterministic if the rule computes different payloads or subkeys from
  time, randomness, environment, or other ambient process state.

Implications:

- The wasm/browser host bridge is not currently the main determinism problem.
- The determinism boundary is still vulnerable anywhere Echo accepts
  host-authored executable rewrite logic.
- The current policy doctrine in `admission.rs` is directionally right: hosts
  should choose deterministic engine-defined policy by reference, not inject
  executable admission law.
- The long-term closure target should be explicit: user-authored rewrite logic
  should enter Echo only as Wesley-compiled declarative IR, not as arbitrary
  native callbacks or handwritten executable rule code.

What should happen next:

1. Write down, in design/invariant form, that `RewriteRule` is a
   trusted/bootstrap-only API until a declarative or sandboxed replacement
   exists.
2. Add a DET-critical audit or release gate that rejects obvious ambient-state
   access on rule paths:
    - time
    - randomness
    - filesystem/network
    - ambient environment
    - unordered-container-driven emission without canonicalization
3. Freeze the intended replacement path:
    - **primary target:** Wesley-compiled declarative IR as the only
      user-authored rewrite source admitted to Echo
    - **non-goal:** arbitrary host-authored native execution law on the
      deterministic path
    - **optional transitional tool:** a deterministic sandboxed rule language,
      only if it still compiles down to the same lawful substrate and does not
      reopen executable host-side escape hatches
4. Keep the current worker-count and parallel-policy tests, but treat them as
   necessary supporting evidence rather than sufficient proof of safety.

Related:

- `crates/warp-core/src/rule.rs`
- `crates/warp-core/src/engine_impl.rs`
- `crates/warp-core/src/parallel/exec.rs`
- `crates/warp-core/src/admission.rs`
- `crates/echo-wasm-abi/src/kernel_port.rs`
- `crates/warp-wasm/src/lib.rs`
- `crates/warp-core/src/materialization/emission_port.rs`
- `crates/warp-core/src/materialization/bus.rs`
