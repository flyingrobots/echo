# MWMR Concurrency Mini‑Epic Roadmap (Footprints, Reserve Gate, Telemetry)

Status: Active • Owner: rmg-core • Created: 2025-10-27

 
## Outcomes
- Enforce MWMR determinism via independence checks (footprints + ports + factor masks).
- Keep the hot path zero‑overhead (compact u32 rule ids; domain‑separated family ids only at boundaries).
- Prove commutation with property tests (N‑permutation) and add basic telemetry for conflict rates.

---

 
## Phase 0.5 — Foundations (Done / In‑Progress)
- [x] Footprint type with ports and factor mask (IdSet/PortSet; deterministic intersects)
- [x] RewriteRule surface extended with `compute_footprint`, `factor_mask`, `ConflictPolicy`
- [x] PendingRewrite carries `footprint` + `phase`
- [x] Property test: 2 independent motion rewrites commute (equal snapshot hash)
- [x] Spec doc: `docs/spec-mwmr-concurrency.md`

---

 
## Phase 1 — Reservation Gate & Compact IDs
- [x] CompactRuleId(u32) and rule table mapping family_id → compact id (in Engine)
- [x] DeterministicScheduler::reserve(tx, &mut PendingRewrite) → bool (active frontier per tx)
- [x] Engine commit() wires the reserve gate (execute only Reserved rewrites)
- [x] Feature‑gated JSONL telemetry (reserved/conflict) with timestamp, tx_id, short rule id
- [ ] Use CompactRuleId in PendingRewrite and internal execution paths (leave family id for ordering/disk/wire)

---

 
## Phase 2 — Proof & Performance
- [ ] Property test: N‑permutation commutation (N = 3..6 independent rewrites)
- [ ] Reserve gate smoke tests (same PortKey ⇒ conflict; disjoint ports ⇒ reserve)
- [ ] Criterion bench: independence checks (10/100/1k rewrites) — target < 1 ms @ 100
- [ ] Telemetry counters per tick (conflict_rate, retry_count, reservation_latency_ms, epoch_flip_ms)
- [ ] Add Retry with randomized backoff (behind flag) once telemetry lands; keep default Abort

---

 
## Phase 3 — Rule Identity & Hot‑Load
- [x] build.rs generates const family id for `rule:motion/update` (domain‑separated)
- [ ] Generalize generator (src/gen/rule_ids.rs) and runtime assert test to catch drift
- [ ] Lua FFI registration: `register_rule{name, match, exec, ?id, ?revision}`; engine computes if omitted
- [ ] Revision ID = blake3("rule-rev:<lang>:canon-ast-v1" || canonical AST bytes)

---

 
## Phase 4 — Storage & Epochs (Scoping/Design)
- [ ] Offset‑graph arena + mmap view (zero‑copy snapshots)
- [ ] Double‑buffered planes (attachments/skeleton), lazy epoch flips, grace‑period reclamation
- [ ] Optional Merkle overlays for partial verification

---

 
## Guardrails & Invariants
- Deterministic planning key = (scope_hash, family_id); execution may be parallel, ordering stays stable.
- Footprint independence order: factor_mask → ports → edges → nodes; fail fast on ports.
- Keep |L| ≤ 5–10; split rules or seed from rare types if larger.
- Never serialize CompactRuleId; boundary formats carry family id + (optional) revision id.

---

 
## Telemetry (dev feature)
- Events: `reserved`, `conflict` (ts_micros, tx_id, rule_id_short)
- Counters per tick: conflict_rate, retry_count, reservation_latency_ms, epoch_flip_ms, bitmap_blocks_checked

---

 
## Links
- Spec: `docs/spec-mwmr-concurrency.md`
- Tests: `crates/rmg-core/tests/footprint_independence_tests.rs`, `crates/rmg-core/tests/property_commute_tests.rs`
- Engine: `crates/rmg-core/src/engine_impl.rs`, `crates/rmg-core/src/scheduler.rs`
- Build: `crates/rmg-core/build.rs`
