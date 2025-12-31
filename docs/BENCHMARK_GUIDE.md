<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# How to Add Benchmarks to Echo

This guide covers Echo's gold standard for benchmarking: **Criterion + JSON artifacts + D3.js dashboard integration**.

## Philosophy

Benchmarks in Echo are not just about measuring performance—they're about:
- **Empirical validation** of complexity claims (O(n), O(m), etc.)
- **Regression detection** to catch performance degradation early
- **Professional visualization** so anyone can understand performance characteristics
- **Reproducibility** with statistical rigor (confidence intervals, multiple samples)

## Prerequisites

- Familiarity with [Criterion.rs](https://github.com/bheisler/criterion.rs)
- Understanding of the component you're benchmarking
- Clear hypothesis about expected complexity (O(1), O(n), O(n log n), etc.)

## Step-by-Step Guide

### 1. Create the Benchmark File

Create a new benchmark in `crates/warp-benches/benches/`:

```rust
// crates/warp-benches/benches/my_feature.rs
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use warp_core::*; // Import what you need

fn bench_my_feature(c: &mut Criterion) {
    let mut group = c.benchmark_group("my_feature");

    // Configure measurement
    group.sample_size(50);           // Statistical samples
    group.measurement_time(std::time::Duration::from_secs(8));

    // Test multiple input sizes to validate complexity
    for &n in &[10, 100, 1_000, 3_000, 10_000, 30_000] {
        // Set throughput for per-operation metrics
        group.throughput(Throughput::Elements(n as u64));

        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            // Setup (outside timing)
            let data = create_test_data(n);

            // Measured operation
            b.iter(|| {
                let result = my_feature(black_box(&data));
                black_box(result); // Prevent optimization
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_my_feature);
criterion_main!(benches);
```

**Key Points:**
- Use `black_box()` to prevent compiler from optimizing away benchmarked code
- Test multiple input sizes (at least 5-6 points) to validate complexity claims
- Set `Throughput` to get per-operation metrics
- Keep setup outside the timing closure

### 2. Register in Cargo.toml

Add to `crates/warp-benches/Cargo.toml`:

```toml
[[bench]]
name = "my_feature"
harness = false  # Required for Criterion
```

### 3. Run the Benchmark

```bash
# Run just your benchmark
cargo bench -p warp-benches --bench my_feature

# Results go to: target/criterion/my_feature/{n}/new/estimates.json
```

Verify the JSON artifacts exist:
```bash
ls -la target/criterion/my_feature/*/new/estimates.json
```

### 4. Integrate with Dashboard

#### 4a. Add to `docs/benchmarks/index.html`

Find the `GROUPS` array and add your benchmark:

```javascript
const GROUPS = [
    // ... existing benchmarks ...
    {
        key: 'my_feature',                    // Must match group name
        label: 'My Feature Description',      // Display name
        color: '#7dcfff',                     // Hex color (pick unique)
        dash: '2,6'                           // Line style: null or '2,6' or '4,4' or '8,4'
    },
];
```

**Color Palette (already used):**
- `#bb9af7` - Purple (snapshot_hash)
- `#9ece6a` - Green (scheduler_drain)
- `#e0af68` - Yellow (scheduler_enqueue)
- `#f7768e` - Red (scheduler_drain/drain)
- `#7dcfff` - Cyan (reserve_independence)

**Pick a new color or use available:**
- `#ff9e64` - Orange
- `#73daca` - Teal
- `#c0caf5` - Light blue

**Dash Patterns:**
- `null` - Solid line
- `'2,6'` - Short dashes (dotted)
- `'4,4'` - Medium dashes
- `'8,4'` - Long dashes

#### 4b. Add to `scripts/bench_bake.py`

Find the `GROUPS` list and add your benchmark:

```python
GROUPS = [
    # ... existing benchmarks ...
    ("my_feature", "My Feature Description"),
]
```

### 5. Generate the Dashboard

```bash
# Full workflow: run benchmarks + bake inline HTML + open
make bench-bake

# This will:
# 1. Run all benchmarks
# 2. Collect JSON artifacts from target/criterion/
# 3. Bake them into docs/benchmarks/report-inline.html
# 4. Open in your browser
```

Alternative workflows:
```bash
# Live dashboard (fetches from target/criterion/)
make bench-serve  # http://localhost:8000/docs/benchmarks/

# Just open the baked report (no rebuild)
make bench-open-inline
```

### 6. Verify Dashboard Integration

Open the dashboard and check:

- [ ] Your benchmark appears as a new line on the chart
- [ ] Color and dash pattern are distinct from other lines
- [ ] Legend shows correct label
- [ ] Hovering over points shows values
- [ ] Stat card displays mean and confidence intervals
- [ ] Line shape validates your complexity hypothesis
  - Linear on log-log = O(n)
  - Constant horizontal = O(1)
  - Quadratic curve = O(n²)

### 7. Document Your Benchmark

Create `docs/benchmarks/MY_FEATURE_BENCHMARK.md`:

```markdown
# My Feature Benchmark

## Overview

Brief description of what you're measuring and why.

## What Was Added

### Benchmark Implementation
- File: `crates/warp-benches/benches/my_feature.rs`
- Measures: [specific metric]
- Input sizes: 10, 100, 1K, 3K, 10K, 30K
- Key design choices: [why you set it up this way]

### Dashboard Integration
- Color: [color code]
- Line style: [dash pattern]
- Label: [display name]

## Results

| Input Size (n) | Mean Time | Per-Operation | Throughput |
|----------------|-----------|---------------|------------|
| 10             | X.XX µs   | XXX ns        | X.XX M/s   |
| 100            | X.XX µs   | XXX ns        | X.XX M/s   |
| 1,000          | XXX µs    | XXX ns        | X.XX M/s   |
| 3,000          | X.XX ms   | X.XX µs       | XXX K/s    |
| 10,000         | XX.X ms   | X.XX µs       | XXX K/s    |
| 30,000         | XX.X ms   | X.XX µs       | XXX K/s    |

### Analysis

**Key Findings:**
- [Your complexity claim]: O(n), O(m), O(1), etc.
- [Evidence]: Per-operation time remains constant / grows linearly / etc.
- [Comparison]: If expected O(n²), we'd see XXX scaling but actual is YYY

**Validation:**
- ✅ Hypothesis confirmed: [why]
- ⚠️  Caveats: [what this doesn't test]

## Running the Benchmark

```bash
# Quick test
cargo bench -p warp-benches --bench my_feature

# Full dashboard
make bench-bake
```

## Interpretation

### What This Proves
✅ [Your claims backed by data]

### What This Doesn't Prove
⚠️  [Limitations and future work]

## Related Documentation
- [Related files and docs]
```

## Quality Standards

### Benchmark Code Quality

- [ ] **Statistical rigor**: 50+ samples, 8s measurement time
- [ ] **Multiple input sizes**: At least 5-6 data points
- [ ] **Proper use of `black_box()`**: Prevent unwanted optimization
- [ ] **Clean setup/teardown**: Only measure what matters
- [ ] **Realistic workloads**: Test actual use cases, not synthetic edge cases
- [ ] **Comments**: Explain WHY you're measuring this way

### Dashboard Integration Quality

- [ ] **Unique visual identity**: Distinct color + dash pattern
- [ ] **Clear labeling**: Legend text explains what's measured
- [ ] **Data integrity**: JSON artifacts exist for all input sizes
- [ ] **Visual validation**: Line shape matches expected complexity

### Documentation Quality

- [ ] **Context**: Why this benchmark exists
- [ ] **Results table**: Actual numbers with units
- [ ] **Analysis**: Interpretation of results vs hypothesis
- [ ] **Honest caveats**: What's NOT proven
- [ ] **Related docs**: Links to implementation and related docs

## Common Pitfalls

### Pitfall 1: Forgetting `harness = false`

**Symptom:** `cargo bench` runs but shows "0 tests, 0 benchmarks"

**Fix:** Add `harness = false` to `[[bench]]` entry in Cargo.toml

### Pitfall 2: Group Name Mismatch

**Symptom:** Dashboard shows "No data" for your benchmark

**Fix:** Ensure `benchmark_group("name")` in Rust matches `key: 'name'` in index.html

### Pitfall 3: Compiler Optimizes Away Your Code

**Symptom:** Benchmark shows impossibly fast times (nanoseconds for complex operations)

**Fix:** Wrap inputs and outputs with `black_box()`:
```rust
b.iter(|| {
    let result = my_function(black_box(&input));
    black_box(result);
});
```

### Pitfall 4: Measuring Setup Instead of Operation

**Symptom:** Benchmark times include allocation, I/O, or other setup

**Fix:** Move setup outside the timing closure:
```rust
// WRONG
b.iter(|| {
    let data = create_test_data(n);  // Measured!
    process(data)
});

// RIGHT
let data = create_test_data(n);  // Not measured
b.iter(|| {
    process(black_box(&data))
});
```

### Pitfall 5: Not Testing Enough Input Sizes

**Symptom:** Can't validate complexity claims (2 points can't distinguish O(n) from O(n²))

**Fix:** Test at least 5-6 input sizes spanning 3+ orders of magnitude (10, 100, 1K, 10K, etc.)

## Advanced Topics

### Comparing Against Baselines

To measure improvement over an old implementation:

1. Keep old implementation in benchmark with `_baseline` suffix
2. Run both benchmarks
3. Add both to dashboard as separate lines
4. Document the improvement factor

### Per-Component Breakdown

To measure multiple phases of a process:

```rust
let mut group = c.benchmark_group("my_feature");

// Total time
group.bench_function("total", |b| { /* ... */ });

// Individual phases
group.bench_function("phase_1", |b| { /* ... */ });
group.bench_function("phase_2", |b| { /* ... */ });
```

Dashboard supports hierarchical groups: `my_feature/phase_1`

### Stress Testing

For finding performance cliffs, extend input sizes:

```rust
for &n in &[10, 100, 1_000, 10_000, 100_000, 1_000_000] {
    // ...
}
```

May need to increase `measurement_time` for large inputs.

## Makefile Reference

```bash
make bench-report      # Run benches + serve + open dashboard
make bench-bake        # Run benches + bake inline HTML + open
make bench-serve       # Serve dashboard at http://localhost:8000
make bench-open-inline # Open baked report without rebuilding
```

## CI Integration (Future)

Currently benchmarks run manually. To add CI gating:

1. Baseline results in version control
2. Regression check comparing to baseline
3. Fail CI if performance degrades >10%

See TODO in `crates/warp-benches/benches/scheduler_drain.rs:11`.

## Questions?

- Check existing benchmarks in `crates/warp-benches/benches/`
- Read [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- Look at `docs/benchmarks/RESERVE_BENCHMARK.md` for a complete example

## Checklist

Before considering your benchmark "done":

- [ ] Rust benchmark file created with proper Criterion setup
- [ ] Registered in `Cargo.toml` with `harness = false`
- [ ] Runs successfully: `cargo bench -p warp-benches --bench my_feature`
- [ ] JSON artifacts generated in `target/criterion/`
- [ ] Added to `docs/benchmarks/index.html` GROUPS array
- [ ] Added to `scripts/bench_bake.py` GROUPS list
- [ ] Dashboard displays line with unique color/dash pattern
- [ ] Results validate complexity hypothesis
- [ ] Documentation created in `docs/benchmarks/`
- [ ] Results table with actual measurements
- [ ] Analysis explains findings and caveats
