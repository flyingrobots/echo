<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Per-test timing history: never discover a 96-second test by vibes again

Legend: `PLATFORM`

## The pain we just lived

The `test_toy_contract_no_std_generated_output_checks_in_consumer_crate`
integration test took ~96 seconds per run for many cycles before
anyone felt it strongly enough to act. That test is now ~1.14s after
the speedup work in PR #383, but the discovery path was wrong:

> 96-second test → multiple hours of compounded irritation → rage fix

That is barbarism. The cost should have been visible in numbers
before it became rage fuel.

(To be fair: rage fuel did ship PR #383. Don't make this a
recurring shipping mechanism.)

## The proposal

Every test run records per-test wall-clock to a structured log:

```text
target/test-timing-history.jsonl
```

One JSON object per test result, append-only:

```json
{
    "ts": "2026-05-31T02:02:29Z",
    "repo": "echo",
    "branch": "cycle/0025-sessions-as-causal-contexts",
    "test_binary": "echo-wesley-gen-generation",
    "test": "test_toy_contract_no_std_generated_output_checks_in_consumer_crate",
    "duration_ms": 1140,
    "status": "pass"
}
```

Capture happens via a `cargo test` wrapper or a libtest reporter
hook (structured libtest JSON output is a nightly/unstable path;
on stable, use a wrapper strategy that records per-test timings
from supported output/reporting surfaces). On Node side (jedit), wrap the
`node --test` JSON reporter.

The JSONL log is gitignored — it is local history, not shared
state.

## The query surface

A new `xtask slow-tests` subcommand reads the log and surfaces
anomalies:

```text
xtask slow-tests --top 20                 # slowest tests overall
xtask slow-tests --branch current         # slowest on current branch
xtask slow-tests --since 7d               # slowest in the last week
xtask slow-tests --regressed-since 7d     # got slower vs prior baseline
xtask slow-tests --test <name> --history  # timing history for one test
```

The `--regressed-since` query is the load-bearing one. It catches
the case where a test was fast yesterday and is slow today —
exactly the signal that almost-shipped #383 weeks earlier than it
did.

## Why this matters

- Cost surfacing is the only sustainable defense against test-loop
  decay. Optimizing once is not enough; you have to notice when it
  starts decaying again.
- The dev-loop is now fast (PR #383). Locking it in needs the
  visibility layer; otherwise the next slow test enters as
  background friction and only escapes at rage-fuel volume.
- Cost: one JSONL file per repo + a few hundred lines of xtask
  code. Lower than the wins it protects.

## Out of scope here

- A CI pipeline that fails on regression. Local visibility first.
  CI gating is the natural step after a useful baseline exists.
- A flamegraph / span tracer for slow tests. Different tool; this
  card is about discovery, not root-cause attribution.

## Trigger / acceptance

Resolve this card when:

1. `cargo test` runs in echo produce / update `target/test-timing-
history.jsonl` automatically (via wrapper or libtest reporter).
2. `node --test` runs in jedit do the same.
3. `xtask slow-tests --top 20` returns the 20 slowest test
   recordings, sorted by duration.
4. `xtask slow-tests --regressed-since 7d` returns tests whose
   median duration over the past 7d exceeded their prior-7d median
   by some configurable threshold.

## Companion

`docs/method/backlog/cool-ideas/PLATFORM_wesley-gen-test-loop-speedup.md`
— this card is the regression-defense layer for that speedup work.
The speedup made the loop fast; this card keeps it that way.
