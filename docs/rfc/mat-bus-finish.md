<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
<!-- markdownlint-disable MD024 -->
# RFC: MaterializationBus Completion

**Status:** Draft
**Date:** 2026-01-17
**Branch:** `materialization-bus`
**Depends on:** ADR-0003-Materialization-Bus

## Summary

This RFC completes the MaterializationBus implementation with three deliverables:

1. **EmissionPort trait** — Hexagonal boundary for rule emissions
2. **ReduceOp enum** — Built-in deterministic reduce operations (no user functions)
3. **Cross-platform determinism tests** — GitHub Actions + DIND harness

---

## 1. EmissionPort Trait (Hexagonal Architecture)

### Problem

The current plan passes `&MaterializationBus` directly to rule executors. This:

- Couples rules to concrete implementation
- Exposes internal `EmitKey` construction to callers
- Makes testing harder (can't mock the bus)
- Violates hexagonal/ports-and-adapters principles

### Solution

Introduce an `EmissionPort` trait as the driven port. Rules depend on the trait; the engine provides a scoped adapter.

```rust
// crates/warp-core/src/materialization/emission_port.rs

/// Driven port for rule emissions (what rules see).
///
/// Rules emit to channels via this trait. The engine provides a scoped
/// implementation that automatically constructs EmitKeys from execution context.
pub trait EmissionPort {
    /// Emit data to a channel.
    ///
    /// The implementation handles EmitKey construction. Callers only provide
    /// channel and payload.
    fn emit(&self, channel: ChannelId, data: Vec<u8>);

    /// Emit with explicit subkey (for multi-emission rules).
    ///
    /// Use when a single rule invocation needs to emit multiple values to
    /// the same channel. The subkey disambiguates emissions.
    fn emit_with_subkey(&self, channel: ChannelId, subkey: u32, data: Vec<u8>);
}
```

### Scoped Adapter

The engine creates a `ScopedEmitter` for each rule execution:

```rust
// crates/warp-core/src/materialization/scoped_emitter.rs

/// Scoped adapter that auto-fills EmitKey from execution context.
///
/// Created by the engine for each rule invocation. Captures the scope hash
/// and rule ID, preventing rules from forging keys.
pub struct ScopedEmitter<'a> {
    bus: &'a MaterializationBus,
    scope_hash: Hash,
    rule_id: u32,
}

impl<'a> ScopedEmitter<'a> {
    /// Create a new scoped emitter for a rule execution.
    pub fn new(bus: &'a MaterializationBus, scope_hash: Hash, rule_id: u32) -> Self {
        Self { bus, scope_hash, rule_id }
    }
}

impl EmissionPort for ScopedEmitter<'_> {
    fn emit(&self, channel: ChannelId, data: Vec<u8>) {
        let key = EmitKey::new(self.scope_hash, self.rule_id);
        self.bus.emit(channel, key, data);
    }

    fn emit_with_subkey(&self, channel: ChannelId, subkey: u32, data: Vec<u8>) {
        let key = EmitKey::with_subkey(self.scope_hash, self.rule_id, subkey);
        self.bus.emit(channel, key, data);
    }
}
```

### Engine Integration

```rust
// In Engine::execute_rule() or similar

let emitter = ScopedEmitter::new(&self.bus, scope_node.hash(), rule.id());
rule.execute(context, &emitter)?;
```

### Testing

Rules can be tested with a mock port:

```rust
#[cfg(test)]
struct MockEmissionPort {
    emissions: RefCell<Vec<(ChannelId, Vec<u8>)>>,
}

impl EmissionPort for MockEmissionPort {
    fn emit(&self, channel: ChannelId, data: Vec<u8>) {
        self.emissions.borrow_mut().push((channel, data));
    }
    // ...
}
```

### Duplicate EmitKey Rejection

**Policy: Reject duplicate (channel, EmitKey) pairs. Always.**

If a rule emits twice to the same channel with the same EmitKey, the bus returns
`DuplicateEmission` error. This catches rules that iterate non-deterministic
sources (e.g., `HashMap`) without proper subkey differentiation.

```rust
/// Error returned when the same (channel, EmitKey) is emitted twice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateEmission {
    pub channel: ChannelId,
    pub key: EmitKey,
}

impl MaterializationBus {
    /// Emit data to a channel. Returns error if key already exists.
    pub fn emit(
        &self,
        channel: ChannelId,
        key: EmitKey,
        data: Vec<u8>,
    ) -> Result<(), DuplicateEmission> {
        use std::collections::btree_map::Entry;

        let mut pending = self.pending.borrow_mut();
        let channel_map = pending.entry(channel).or_default();

        match channel_map.entry(key) {
            Entry::Vacant(e) => {
                e.insert(data);
                Ok(())
            }
            Entry::Occupied(_) => Err(DuplicateEmission { channel, key }),
        }
    }
}
```

**Why reject even if payloads are identical?**

Allowing "identical payload = OK" encourages sloppy code that emits redundantly.
Then someone changes a field and tests fail mysteriously. Rejecting always forces
rule authors to think: "Am I iterating deterministically? Do I need unique subkeys?"

### Files to Create/Modify

| File                                     | Action                                              |
| ---------------------------------------- | --------------------------------------------------- |
| `src/materialization/emission_port.rs`   | **Create** — trait definition                       |
| `src/materialization/scoped_emitter.rs`  | **Create** — adapter implementation                 |
| `src/materialization/mod.rs`             | **Modify** — export new types                       |
| `src/materialization/bus.rs`             | **Modify** — add DuplicateEmission, update emit()   |
| `src/engine.rs` (or equivalent)          | **Modify** — create ScopedEmitter per rule          |

---

## 2. ReduceOp Enum (Built-in Deterministic Ops)

### Problem

The current `ChannelPolicy::Reduce { join_fn_id }` design assumes a join function registry where users register merge functions by ID. This is a determinism landmine:

- User functions may not be commutative/associative
- Function lookup adds indirection and potential for error
- Can't verify correctness at compile time
- Opens door to non-deterministic user code

### Solution

Replace `join_fn_id` with a closed enum of built-in reduce operations.

**IMPORTANT: Not all reduce ops are commutative.** They fall into two categories:

| Category                | Ops                                    | Property                                         |
| ----------------------- | -------------------------------------- | ------------------------------------------------ |
| **Commutative Monoids** | `Sum`, `Max`, `Min`, `BitOr`, `BitAnd` | Order doesn't matter: `a ⊕ b = b ⊕ a`            |
| **Order-Dependent**     | `First`, `Last`, `Concat`              | Deterministic via EmitKey order, NOT commutative |

Both categories are **deterministic** (same inputs → same output), but only commutative ops are **permutation-invariant** at the value level. Order-dependent ops rely on the canonical EmitKey ordering.

```rust
// crates/warp-core/src/materialization/reduce_op.rs

/// Built-in reduce operations for channel coalescing.
///
/// # Algebraic Categories
///
/// **Commutative monoids** (permutation-invariant):
/// - `Sum`, `Max`, `Min`, `BitOr`, `BitAnd`
/// - Result is identical regardless of emission order
///
/// **Order-dependent** (deterministic via EmitKey order):
/// - `First`, `Last`, `Concat`
/// - Result depends on canonical EmitKey ordering
/// - NOT commutative — do not claim they are!
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReduceOp {
    // ─── COMMUTATIVE MONOIDS ───────────────────────────────────────────

    /// Sum all values as little-endian u64.
    /// Empty input → `[0u8; 8]` (zero).
    Sum,

    /// Take maximum value (lexicographic byte comparison).
    /// Empty input → `[]` (empty vec).
    Max,

    /// Take minimum value (lexicographic byte comparison).
    /// Empty input → `[]` (empty vec).
    Min,

    /// Bitwise OR all values.
    /// Shorter values are zero-padded on the right.
    /// Empty input → `[]` (empty vec).
    BitOr,

    /// Bitwise AND all values.
    /// Result length = minimum input length (intersection semantics).
    /// Empty input → `[]` (empty vec).
    BitAnd,

    // ─── ORDER-DEPENDENT (NOT COMMUTATIVE) ─────────────────────────────

    /// Take first value by EmitKey order.
    /// Empty input → `[]` (empty vec).
    /// WARNING: Not commutative. Depends on canonical key ordering.
    First,

    /// Take last value by EmitKey order.
    /// Empty input → `[]` (empty vec).
    /// WARNING: Not commutative. Depends on canonical key ordering.
    Last,

    /// Concatenate all values in EmitKey order.
    /// Empty input → `[]` (empty vec).
    /// WARNING: Not commutative. Order matters for result bytes.
    Concat,
}

impl ReduceOp {
    /// Returns true if this op is a commutative monoid (permutation-invariant).
    pub const fn is_commutative(&self) -> bool {
        matches!(self, Self::Sum | Self::Max | Self::Min | Self::BitOr | Self::BitAnd)
    }
}
```

### Updated ChannelPolicy

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChannelPolicy {
    /// All emissions in EmitKey order, length-prefixed.
    #[default]
    Log,

    /// Error if more than one emission.
    StrictSingle,

    /// Reduce via built-in operation.
    Reduce(ReduceOp),
}
```

### Implementation

```rust
impl ReduceOp {
    /// Apply this reduce operation to a set of values.
    ///
    /// Values are provided in EmitKey order (required for First/Last/Concat).
    /// Returns the reduced result.
    ///
    /// # Empty Input Behavior
    ///
    /// All ops return `[]` (empty vec) on empty input, EXCEPT:
    /// - `Sum` returns `[0u8; 8]` (zero as u64 LE)
    ///
    /// This is intentional: empty input means "nothing to reduce."
    pub fn apply(self, values: impl IntoIterator<Item = Vec<u8>>) -> Vec<u8> {
        let mut iter = values.into_iter().peekable();

        // Handle empty input uniformly (except Sum)
        if iter.peek().is_none() {
            return match self {
                Self::Sum => vec![0u8; 8], // Identity for addition
                _ => Vec::new(),            // "Nothing to reduce"
            };
        }

        match self {
            // ─── COMMUTATIVE MONOIDS ───────────────────────────────────

            Self::Sum => {
                let sum: u64 = iter
                    .map(|v| {
                        let mut buf = [0u8; 8];
                        let len = v.len().min(8);
                        buf[..len].copy_from_slice(&v[..len]);
                        u64::from_le_bytes(buf)
                    })
                    .sum();
                sum.to_le_bytes().to_vec()
            }

            Self::Max => iter.max().unwrap(), // unwrap safe: checked non-empty

            Self::Min => iter.min().unwrap(), // unwrap safe: checked non-empty

            Self::BitOr => {
                iter.reduce(|acc, v| bitwise_or(&acc, &v)).unwrap()
            }

            Self::BitAnd => {
                iter.reduce(|acc, v| bitwise_and(&acc, &v)).unwrap()
            }

            // ─── ORDER-DEPENDENT (EmitKey order matters) ───────────────

            Self::First => iter.next().unwrap(), // unwrap safe: checked non-empty

            Self::Last => iter.last().unwrap(),  // unwrap safe: checked non-empty

            Self::Concat => iter.flatten().collect(),
        }
    }
}

/// Bitwise OR with zero-padding for shorter operand.
fn bitwise_or(a: &[u8], b: &[u8]) -> Vec<u8> {
    let len = a.len().max(b.len());
    let mut result = vec![0u8; len];
    for (i, byte) in result.iter_mut().enumerate() {
        let av = a.get(i).copied().unwrap_or(0);
        let bv = b.get(i).copied().unwrap_or(0);
        *byte = av | bv;
    }
    result
}

/// Bitwise AND with truncation to shorter operand (intersection semantics).
fn bitwise_and(a: &[u8], b: &[u8]) -> Vec<u8> {
    let len = a.len().min(b.len());
    (0..len).map(|i| a[i] & b[i]).collect()
}
```

### Files to Create/Modify

| File                                   | Action                                           |
| -------------------------------------- | ------------------------------------------------ |
| `src/materialization/reduce_op.rs`     | **Create** — enum + apply()                      |
| `src/materialization/channel.rs`       | **Modify** — update ChannelPolicy                |
| `src/materialization/bus.rs`           | **Modify** — call ReduceOp::apply() in finalize  |
| `tests/materialization_determinism.rs` | **Add** — reduce op tests                        |

---

## 3. Cross-Platform Determinism Tests

### Problem

MaterializationBus must produce identical output across:

- macOS (dev machines)
- Linux (CI, production)
- WASM (browser runtime)

Current tests run only on the host platform.

### Solution

Two-layer testing:

| Layer              | Environment        | Trigger                 | Purpose                        |
| ------------------ | ------------------ | ----------------------- | ------------------------------ |
| **DIND**           | Docker-in-Docker   | `cargo xtask dind-test` | Local dev, fast iteration      |
| **GitHub Actions** | Native runners     | Push/PR                 | Gate merges, real environments |

### 3.1 DIND Harness Extension

Extend existing DIND test harness to include materialization digest:

```rust
// crates/echo-dind-tests/src/lib.rs

/// Output from a determinism test run.
#[derive(Debug, Serialize, Deserialize)]
pub struct DeterminismOutput {
    /// State hash after N ticks.
    pub state_hash: String,
    /// Tick receipt hashes.
    pub receipt_hashes: Vec<String>,
    /// NEW: Materialization digest (hash of all finalized frames).
    pub materialization_digest: String,
}
```

The test runs the same scenario on:

1. Host (macOS/Linux)
2. Docker Linux container
3. WASM via wasm-pack

All three must produce identical `materialization_digest`.

### 3.2 GitHub Actions Workflow

```yaml
# .github/workflows/determinism.yml

name: Determinism

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  determinism-matrix:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: macos-latest
            target: x86_64-apple-darwin

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable
        with:
          targets: ${{ matrix.target }},wasm32-unknown-unknown

      - name: Install wasm-pack
        run: cargo install wasm-pack

      - name: Run determinism tests
        run: cargo test -p warp-core --test materialization_determinism

      - name: Run WASM determinism tests
        run: wasm-pack test --node crates/warp-core

      - name: Capture materialization digest
        id: digest
        run: |
          DIGEST=$(cargo run -p echo-dind-tests --bin capture-digest)
          echo "digest=$DIGEST" >> $GITHUB_OUTPUT

      - name: Upload digest artifact
        uses: actions/upload-artifact@v4
        with:
          name: digest-${{ matrix.os }}
          path: digest.txt

  verify-cross-platform:
    needs: determinism-matrix
    runs-on: ubuntu-latest
    steps:
      - name: Download all digests
        uses: actions/download-artifact@v4

      - name: Compare digests
        run: |
          LINUX=$(cat digest-ubuntu-latest/digest.txt)
          MACOS=$(cat digest-macos-latest/digest.txt)

          if [ "$LINUX" != "$MACOS" ]; then
            echo "DETERMINISM FAILURE: Linux and macOS produced different digests"
            echo "Linux: $LINUX"
            echo "macOS: $MACOS"
            exit 1
          fi

          echo "Cross-platform determinism verified: $LINUX"
```

### 3.3 Local DIND Command

```bash
# Run locally before pushing
cargo xtask dind-determinism

# Runs:
# 1. Native test → captures digest
# 2. Docker test → captures digest
# 3. WASM test → captures digest
# 4. Compares all three
```

### Files to Create/Modify

| File                                               | Action                                     |
| -------------------------------------------------- | ------------------------------------------ |
| `.github/workflows/determinism.yml`                | **Create** — CI workflow                   |
| `crates/echo-dind-tests/src/lib.rs`                | **Modify** — add materialization_digest    |
| `crates/echo-dind-tests/src/bin/capture-digest.rs` | **Create** — digest capture binary         |
| `xtask/src/main.rs`                                | **Modify** — add dind-determinism command  |

---

## Implementation Order

```text
Phase 1: EmissionPort (unblocks engine integration)
├── Create emission_port.rs
├── Create scoped_emitter.rs
├── Update mod.rs exports
└── Add unit tests

Phase 2: ReduceOp (completes bus semantics)
├── Create reduce_op.rs
├── Update channel.rs (ChannelPolicy)
├── Update bus.rs (finalize with reduce)
└── Add reduce tests to determinism suite

Phase 3: Cross-Platform Tests (gates merges)
├── Extend DIND harness
├── Create GitHub workflow
├── Add xtask command
└── Verify on first PR
```

## Test Plan: "SPEC is reSPECted"

Comprehensive test suite ensuring the spec cannot lie.

### Tier 1 — EmitKey Correctness + Wire Encoding

| Test                                              | What It Proves                                           |
| ------------------------------------------------- | -------------------------------------------------------- |
| `emit_key_ord_is_lexicographic_scope_rule_subkey` | Ordering matches spec                                    |
| `emit_key_wire_encoding_is_40_bytes_no_padding`   | bytes[0..32]=scope, [32..36]=rule LE, [36..40]=subkey LE |
| `emit_key_roundtrip_wire`                         | encode → decode → equals                                 |
| `emit_key_subkey_from_hash_is_deterministic`      | Same input → same u32                                    |

### Tier 2 — Bus Duplicate Rejection

| Test                                                | What It Proves                                     |
| --------------------------------------------------- | -------------------------------------------------- |
| `bus_rejects_duplicate_key_same_channel`            | (ch, key, A) then (ch, key, B) → DuplicateEmission |
| `bus_allows_same_key_different_channels`            | (ch1, key) and (ch2, key) both OK                  |
| `bus_rejects_duplicate_key_even_if_bytes_identical` | No "identical payload = OK" loophole               |

### Tier 3 — Permutation Invariance ("SPEC Police")

| Test                                            | What It Proves                     |
| ----------------------------------------------- | ---------------------------------- |
| `log_finalize_is_permutation_invariant_small_n` | All N! orderings → identical bytes |
| `bus_channel_iteration_is_canonical`            | Channels in BTreeMap order         |
| `bus_log_preserves_all_emissions_no_drops`      | count(output) == count(input)      |

### Tier 4 — ReduceOp Algebra

**Commutative ops (must be permutation-invariant):**

| Test                                        | What It Proves                 |
| ------------------------------------------- | ------------------------------ |
| `reduce_sum_commutative_associative`        | All permutations → same result |
| `reduce_max_min_are_commutative`            | Byte-lex comparison is stable  |
| `reduce_bitor_commutative_variable_length`  | Zero-padding semantics correct |
| `reduce_bitand_commutative_variable_length` | Truncation semantics correct   |

**Order-dependent ops (NOT commutative, deterministic via EmitKey):**

| Test                                        | What It Proves                 |
| ------------------------------------------- | ------------------------------ |
| `reduce_first_picks_first_in_emitkey_order` | Smallest key wins              |
| `reduce_last_picks_last_in_emitkey_order`   | Largest key wins               |
| `reduce_concat_matches_emitkey_order`       | Output = concat(sorted by key) |

**Truth serum:**

| Test                                            | What It Proves                     |
| ----------------------------------------------- | ---------------------------------- |
| `reduce_op_commutativity_table_is_honest`       | `is_commutative()` matches reality |
| `reduce_empty_input_returns_specified_identity` | Sum→[0;8], others→[]               |

### Tier 5 — Engine Integration

| Test                                             | What It Proves                   |
| ------------------------------------------------ | -------------------------------- |
| `engine_log_emissions_stable_across_apply_order` | Rewrite order doesn't matter     |
| `engine_strict_single_deterministic_failure`     | Same error signature both orders |
| `engine_reduce_sum_stable_across_apply_order`    | Reduced sum identical            |
| `engine_emits_only_post_commit`                  | Port empty before commit         |

### Tier 6 — Cross-Platform Digest

| Test                                                 | What It Proves                   |
| ---------------------------------------------------- | -------------------------------- |
| `determinism_output_includes_materialization_digest` | Harness writes digest            |
| `cross_platform_digest_matches_linux_macos_wasm`     | All platforms identical          |
| `scope_hash_is_content_hash_not_id_hash`             | Equivalent stores → same EmitKey |

---

## Open Questions

1. **WASM target for CI** — `wasm32-unknown-unknown` or `wasm32-wasi`? Recommend `unknown-unknown` for browser purity.

2. **Reduce op extensibility** — Should we ever allow user-defined reduce ops? **NO.** Use `Log` and reduce client-side.

3. **Digest algorithm** — BLAKE3 of concatenated frame bytes. Simple, no Merkle tree needed.

---

## Success Criteria

- [ ] Rules emit via `EmissionPort` trait, not direct bus access
- [ ] Duplicate (channel, EmitKey) pairs rejected with `DuplicateEmission`
- [ ] `ChannelPolicy::Reduce(ReduceOp)` replaces `join_fn_id`
- [ ] All 8 `ReduceOp` variants implemented with `is_commutative()` classification
- [ ] Empty-input behavior: Sum→[0;8], all others→[]
- [ ] All Tier 1-5 tests passing
- [ ] GitHub Actions workflow passes on PR
- [ ] `cargo xtask dind-determinism` passes locally
- [ ] Cross-platform digest match verified in CI

---

## Revision History

| Date       | Change                                                                |
| ---------- | --------------------------------------------------------------------- |
| 2026-01-17 | Initial draft                                                         |
| 2026-01-17 | Fixed ReduceOp algebra claims (First/Last/Concat are NOT commutative) |
| 2026-01-17 | Added duplicate EmitKey rejection policy                              |
| 2026-01-17 | Specified empty-input behavior (Sum→[0;8], others→[])                 |
| 2026-01-17 | Added comprehensive "SPEC is reSPECted" test plan                     |
