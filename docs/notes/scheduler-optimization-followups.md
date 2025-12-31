<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Scheduler Optimization Follow-up Tasks

This document contains prompts for future work addressing gaps identified during the scheduler radix optimization session.

---

## Prompt 1: Testing & Correctness Validation

**Prompt for next session:**

> "I need comprehensive testing to validate that our hybrid scheduler (comparison sort for n ≤ 1024, radix sort for n > 1024) produces **identical deterministic results** to the original BTreeMap implementation. Please:
>
> 1. **Property-Based Tests**: Implement proptest-based fuzzing that:
>    - Generates random sequences of `enqueue()` calls with varied scope hashes, rule IDs, and insertion orders
>    - Runs both the current hybrid scheduler and a reference BTreeMap implementation
>    - Asserts that `drain_in_order()` returns **exactly the same sequence** from both implementations
>    - Tests across the threshold boundary (900-1100 elements) to catch edge cases
>    - Includes adversarial inputs: all-same scopes, reverse-sorted scopes, partially overlapping scopes
>
> 2. **Determinism Regression Tests**: Create explicit test cases that would break if we lost determinism:
>    - Same input in different order should produce same drain sequence
>    - Tie-breaking on nonce must be consistent
>    - Last-wins dedupe must be preserved
>    - Cross-transaction stability (GenSet generation bumps don't affect ordering)
>
> 3. **Threshold Boundary Tests**: Specifically test n = 1023, 1024, 1025 to ensure no ordering discontinuity at the threshold
>
> 4. **Add to CI**: Ensure these tests run on every commit to catch future regressions
>
> The goal is **100% confidence** that we haven't introduced any ordering divergence from the original BTreeMap semantics. Location: `crates/warp-core/src/scheduler.rs` and new test file `crates/warp-core/tests/scheduler_determinism.rs`"

---

## Prompt 2: Radix Sort Deep Dive

**Prompt for next session:**

> "Please examine `crates/warp-core/src/scheduler.rs` and provide a **comprehensive technical explanation** of the radix sort implementation, suitable for documentation or a blog post. Specifically explain:
>
> 1. **Why 20 passes?**
>    - We have 32 bytes (scope_be32) + 4 bytes (rule_id) + 4 bytes (nonce) = 40 bytes total
>    - Each pass handles 16 bits = 2 bytes
>    - Therefore: 40 bytes / 2 bytes per pass = 20 passes
>    - Show the pass sequence: nonce (2 passes), then rule_id (2 passes), then scope_be32 (16 passes, big-endian)
>
> 2. **Why 16-bit digits instead of 8-bit?**
>    - Trade-off: 8-bit = 256-entry histogram (1KB × 20 = 20KB zeroing), but 40 passes required
>    - 16-bit = 65,536-entry histogram (256KB × 20 = 5MB zeroing), but only 20 passes
>    - Performance analysis: At n=10k, memory bandwidth vs pass count break-even
>    - Document why we chose 16-bit for this use case (memory is cheap, passes are expensive for our data sizes)
>
> 3. **Why LSD (Least Significant Digit) instead of MSD?**
>    - LSD is stable and always takes exactly k passes (k = number of digits)
>    - MSD requires recursive partitioning and doesn't maintain insertion order for ties
>    - We need stability for nonce tie-breaking
>
> 4. **Memory layout and thin/fat separation:**
>    - Why we separate `RewriteThin` (sorting keys) from `fat: Vec<Option<P>>` (payloads)
>    - Cache locality during sorting
>    - Handle indirection mechanism
>
> 5. **The histogram counting algorithm:**
>    - Two-pass per digit: count occurrences, then exclusive prefix sum to get write indices
>    - Why we zero `counts16` before each pass
>    - How the scratch buffer enables in-place-like behavior
>
> Add this explanation as inline comments in `scheduler.rs` and/or as a new doc file at `docs/notes/radix-sort-internals.md`. Include diagrams (Mermaid or ASCII art) showing the pass sequence and memory layout."

---

## Prompt 3: Document Assumptions & Arbitrary Decisions

**Prompt for next session:**

> "Please review the scheduler optimization implementation and create comprehensive documentation explaining decisions that may appear arbitrary or require platform-specific validation. Create `docs/notes/scheduler-implementation-notes.md` covering:
>
> 1. **The 1024 threshold choice:**
>    - Empirically determined on M1 Mac (Apple Silicon)
>    - Based on when 5MB zeroing cost becomes negligible relative to comparison sort overhead
>    - **Platform dependency**: Intel x86 may have different optimal threshold due to:
>      - Different memory bandwidth characteristics
>      - Different cache sizes (L1/L2/L3)
>      - Different CPU instruction latencies
>    - **Validation needed**: Benchmark on Intel/AMD x86_64, ARM Cortex-A series, RISC-V
>    - **Potential solution**: Make threshold configurable via feature flag or runtime detection
>
> 2. **16-bit radix digit size:**
>    - Assumes 256KB zeroing is acceptable fixed cost
>    - Alternative: 8-bit digits (20KB zeroing, 40 passes) might win on memory-constrained systems
>    - Alternative: 32-bit digits (16GB histogram!) is obviously wrong, but why? Document the analysis.
>    - **Question**: Did we test 12-bit digits (4KB histogram, ~27 passes)? Should we?
>
> 3. **FxHasher (rustc-hash) choice:**
>    - Fast but non-cryptographic
>    - Assumes no adversarial input targeting hash collisions
>    - **Risk**: Pathological inputs could cause O(n²) behavior in the HashMap
>    - **Mitigation**: Could switch to ahash or SipHash if collision attacks are a concern
>
> 4. **GenSet generation counter wraparound:**
>    - What happens when `gen: u32` overflows after 4 billion transactions?
>    - Currently unhandled - assumes no single engine instance lives that long
>    - **Validation needed**: Add a debug assertion or overflow handling
>
> 5. **Comparison sort choice (sort_unstable_by):**
>    - Why unstable sort is acceptable (we have explicit nonce tie-breaking in the comparator)
>    - Why not pdqsort vs other algorithms? (It's already Rust's default)
>
> 6. **Scope hash size (32 bytes = 256 bits):**
>    - Why this size? Comes from BLAKE3 output
>    - Radix pass count directly depends on this
>    - If we ever change hash algorithm, pass count must be recalculated
>
> For each decision, document:
> - **Rationale**: Why we chose this
> - **Assumptions**: What must be true for this choice to be correct
> - **Risks**: What could go wrong
> - **Validation needed**: What tests/benchmarks would increase confidence
> - **Alternatives**: What we considered but rejected, and why"

---

## Prompt 4: Worst-Case Scenarios & Mitigations

**Prompt for next session:**

> "Please analyze the hybrid scheduler implementation to identify **worst-case scenarios** and design mitigations with empirical validation. Focus on adversarial inputs and edge cases where performance or correctness could degrade:
>
> 1. **Adversarial Hash Inputs:**
>    - **Scenario**: All scopes hash to values with identical high-order bits (e.g., all start with 0x00000000...)
>    - **Impact**: Radix sort doesn't partition until late passes, cache thrashing
>    - **Test**: Generate 10k scopes with only low-order byte varying
>    - **Mitigation**: Document that this is acceptable (real hashes distribute uniformly), or switch to MSD radix if detected
>
> 2. **Threshold Boundary Oscillation:**
>    - **Scenario**: Input size oscillates around 1024 (e.g., 1000 → 1050 → 980 → 1100)
>    - **Impact**: Algorithm selection thrashing, icache/dcache pollution
>    - **Test**: Benchmark repeated cycles of 1000/1050 element drains
>    - **Mitigation**: Add hysteresis (e.g., switch at 1024 going up, 900 going down)
>
> 3. **FxHashMap Collision Attack:**
>    - **Scenario**: Malicious input with (scope, rule_id) pairs engineered to collide in FxHasher
>    - **Impact**: HashMap lookups degrade to O(n), enqueue becomes O(n²)
>    - **Test**: Generate colliding inputs (requires reverse-engineering FxHash)
>    - **Mitigation**: Switch to ahash (DDoS-resistant) or document trust model
>
> 4. **Memory Exhaustion:**
>    - **Scenario**: Enqueue 10M+ rewrites before draining
>    - **Impact**: 5MB × 20 = 100MB scratch buffer, plus thin/fat vectors = potential OOM
>    - **Test**: Benchmark memory usage at n = 100k, 1M, 10M
>    - **Mitigation**: Add early drain triggers or pool scratch buffers across transactions
>
> 5. **Highly Skewed Rule Distribution:**
>    - **Scenario**: 99% of rewrites use rule_id = 0, remainder spread across 1-255
>    - **Impact**: First rule_id radix pass is nearly no-op, wasted cache bandwidth
>    - **Test**: Generate skewed distribution, measure vs uniform distribution
>    - **Mitigation**: Skip radix passes if variance is low (requires online detection)
>
> 6. **Transaction Starvation:**
>    - **Scenario**: Transaction A enqueues 100k rewrites, transaction B enqueues 1 rewrite
>    - **Impact**: B's single rewrite pays proportional cost in GenSet conflict checking
>    - **Test**: Benchmark two-transaction scenario with 100k vs 1 rewrites
>    - **Mitigation**: Per-transaction GenSet or early-out if footprint is empty
>
> For each scenario:
> 1. **Create a benchmark** in `crates/warp-benches/benches/scheduler_adversarial.rs`
> 2. **Measure degradation** compared to best-case (e.g., how much slower?)
> 3. **Implement mitigation** if degradation is >2x
> 4. **Re-benchmark** to prove mitigation works
> 5. **Document** in `docs/notes/scheduler-worst-case-analysis.md` with graphs
>
> The goal is to **quantify** our worst-case behavior and provide **evidence** that mitigations work, not just intuition."

---

## Alternatives Considered

During the optimization process, we evaluated several alternative approaches before settling on the current hybrid radix sort implementation:

### 1. **Pure Comparison Sort (Status Quo)**
- **Approach**: Keep BTreeMap-based scheduling
- **Pros**:
  - Already implemented and tested
  - Simple, no custom sort logic
  - Good for small n
- **Cons**:
  - O(n log n) complexity
  - 44% slower at n=1000 than hybrid
  - Doesn't scale to n=10k+
- **Why rejected**: Performance target (60 FPS = 16.67ms frame budget) requires sub-millisecond scheduling at n=1000+. BTreeMap doesn't meet this at scale.

---

### 2. **Pure Radix Sort (No Threshold)**
- **Approach**: Always use 20-pass radix sort, no comparison fallback
- **Pros**:
  - Simpler code (no branching)
  - Perfect O(n) scaling
  - Excellent at large n
- **Cons**:
  - 91x slower at n=10 (687µs vs 7.5µs)
  - Fixed 5MB zeroing cost dominates small inputs
  - Real games have variable rewrite counts per frame
- **Why rejected**:
  - Most frames have <100 rewrites, paying huge penalty for rare large frames is unacceptable
  - "Flat green line" in benchmarks (see `docs/benchmarks/BEFORE.webp`)
  - Cannot justify 91x regression for 90% of frames to optimize 10% of frames

---

### 3. **8-bit Digit Radix Sort**
- **Approach**: Use 256-entry histogram (1KB) with 40 passes instead of 16-bit/20 passes
- **Pros**:
  - Only 20KB zeroing overhead vs 5MB
  - Could lower threshold to ~128
  - Better cache locality (256 entries fit in L1)
- **Cons**:
  - Double the number of passes (40 vs 20)
  - Each pass has loop overhead, random access patterns
  - More opportunities for branch misprediction
- **Why rejected**:
  - Preliminary analysis suggested memory bandwidth not the bottleneck, pass count is
  - At n=10k, memory cost (5MB) is amortized, but 20 extra passes are not
  - Rust's `sort_unstable` is *extremely* optimized; hard to beat with more passes
  - Would need empirical benchmarking to prove 8-bit is better (didn't have time)

---

### 4. **Active-Bucket Zeroing**
- **Approach**: Only zero histogram buckets that were non-zero after previous pass
- **Pros**:
  - Could save 15-20% at large n by avoiding full 256KB zeroes
  - Maintains 16-bit digit performance
- **Cons**:
  - Requires tracking which buckets are "dirty"
  - Extra bookkeeping overhead (bitmap? linked list?)
  - Complexity increase
  - Benefit only at n > 10k
- **Why rejected**:
  - Premature optimization - current implementation meets performance targets
  - Complexity/benefit ratio not compelling
  - Can revisit if profiling shows zeroing is bottleneck at scale
  - User's philosophy: "golden path happens 90% of the time"

---

### 5. **Cross-Transaction Buffer Pooling**
- **Approach**: Reuse `scratch` and `counts16` buffers across multiple `drain_in_order()` calls
- **Pros**:
  - Amortizes allocation cost across multiple frames
  - Reduces memory allocator pressure
  - Could enable per-thread pools for parallelism
- **Cons**:
  - Requires lifetime management (who owns the pool?)
  - Breaks current simple API (`drain_in_order()` is self-contained)
  - Unclear benefit (allocations are fast, we care about compute time)
- **Why rejected**:
  - No evidence allocation is bottleneck (Criterion excludes setup with `BatchSize::PerIteration`)
  - Complexity without measured gain
  - Would need profiling to justify

---

### 6. **Rule-Domain Optimization**
- **Approach**: If `rule_id` space is small (<256), skip high-order rule_id radix pass
- **Pros**:
  - Saves 1 pass for common case (most games have <100 rules)
  - Simple optimization (if `max_rule_id < 256`, skip pass)
- **Cons**:
  - Requires tracking max rule_id dynamically
  - Saves ~5% total time (1/20 passes)
  - Adds conditional logic to hot path
- **Why rejected**:
  - Marginal gain (~5%) not worth complexity
  - Pass overhead is cheap relative to histogram operations
  - User constraint: "one dude, on a laptop" - optimize high-value targets first

---

### 7. **MSD (Most Significant Digit) Radix Sort**
- **Approach**: Sort high-order bytes first, recursively partition
- **Pros**:
  - Can early-out if data is already partitioned
  - Potentially fewer passes for sorted data
- **Cons**:
  - Not stable (requires explicit tie-breaking logic)
  - Variable number of passes (hard to predict performance)
  - Recursive implementation (cache unfriendly)
  - Complex to implement correctly
- **Why rejected**:
  - LSD radix guarantees exactly 20 passes (predictable performance)
  - Stability is critical for nonce tie-breaking
  - Our data is random (graph hashes), no sorted patterns to exploit
  - Complexity not justified by speculative gains

---

### 8. **Hybrid with Multiple Thresholds**
- **Approach**: Three-way split: comparison (<256), 8-bit radix (256-4096), 16-bit radix (>4096)
- **Pros**:
  - Theoretically optimal for all input sizes
  - Could squeeze out extra 5-10% in 100-1000 range
- **Cons**:
  - Three codepaths to maintain
  - Two threshold parameters to tune
  - Cache pollution from three different algorithms
  - Testing complexity (need coverage at both boundaries)
- **Why rejected**:
  - Diminishing returns - hybrid with single threshold already meets targets
  - User's philosophy: "good enough for golden path"
  - Engineering time better spent on other features
  - Premature optimization

---

## Summary: Why Hybrid Radix at 1024?

The current implementation (comparison sort for n ≤ 1024, 16-bit radix for n > 1024) was chosen because:

1. **Meets performance targets**: 44% speedup at n=1000, perfect O(n) at scale
2. **Simple**: One threshold, two well-understood algorithms
3. **Robust**: Rust's `sort_unstable` is battle-tested, radix is deterministic
4. **Measurable**: Clear boundary at 1024 makes reasoning about performance easy
5. **Good enough**: Covers 90% golden path, doesn't over-optimize edge cases

Alternative approaches either:
- Sacrificed small-n performance (pure radix)
- Added complexity without measured gains (active-bucket zeroing, pooling)
- Required more tuning parameters (multi-threshold hybrid)
- Didn't align with user's resource constraints (one person, hobby project)

The guiding principle: **"Ship what works for real use cases, iterate if profiling shows a better target."**
