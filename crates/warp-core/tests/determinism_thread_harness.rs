// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::unwrap_used
)]
//! Thread-count determinism harness tests for M003.
//!
//! The report is rendered as deterministic JSON text by hand. Do not use
//! `serde_json` in `warp-core`; JSON here is a diagnostic test artifact, not a
//! causal encoding boundary.

mod common;

use std::fmt::Write as _;

use common::{hex32, parallel_harness, ParallelScenario, ParallelTestHarness};
use warp_core::math::scalar::F32Scalar;
use warp_core::math::Scalar;
use warp_core::{compute_commit_hash_v2, Hash};

#[cfg(feature = "det_fixed")]
use warp_core::math::scalar::DFix64;

const WORKERS_UNDER_TEST: &[usize] = &[1, 2, 4, 8];

#[derive(Clone, Copy)]
enum ScalarBackend {
    F32,
    #[cfg(feature = "det_fixed")]
    DFix64,
}

impl ScalarBackend {
    fn name(self) -> &'static str {
        match self {
            Self::F32 => "F32Scalar",
            #[cfg(feature = "det_fixed")]
            Self::DFix64 => "DFix64",
        }
    }

    fn digest(self, tick: u64) -> Hash {
        match self {
            Self::F32 => scalar_digest::<F32Scalar>(tick),
            #[cfg(feature = "det_fixed")]
            Self::DFix64 => scalar_digest::<DFix64>(tick),
        }
    }
}

#[derive(Clone, Copy)]
struct DivergenceHook {
    tick: u64,
    workers: usize,
}

#[derive(Clone)]
struct TickWitness {
    state_root: Hash,
    patch_digest: Hash,
    commit_id: Hash,
    scalar_digest: Hash,
}

#[derive(Clone)]
struct TickComparison {
    tick: u64,
    workers: usize,
    state_root_match: bool,
    patch_digest_match: bool,
    commit_id_match: bool,
    scalar_digest_match: bool,
    baseline_state_root: Hash,
    actual_state_root: Hash,
    baseline_commit_id: Hash,
    actual_commit_id: Hash,
}

impl TickComparison {
    fn deterministic(&self) -> bool {
        self.state_root_match
            && self.patch_digest_match
            && self.commit_id_match
            && self.scalar_digest_match
    }

    fn mismatch_fields(&self) -> Vec<&'static str> {
        let mut fields = Vec::new();
        if !self.state_root_match {
            fields.push("state_root");
        }
        if !self.patch_digest_match {
            fields.push("patch_digest");
        }
        if !self.commit_id_match {
            fields.push("commit_id");
        }
        if !self.scalar_digest_match {
            fields.push("scalar_digest");
        }
        fields
    }
}

struct DeterminismReport {
    scenario: &'static str,
    scalar_backend: &'static str,
    baseline_workers: usize,
    worker_counts: Vec<usize>,
    tick_count: u64,
    comparisons: Vec<TickComparison>,
}

impl DeterminismReport {
    fn divergence_count(&self) -> usize {
        self.comparisons
            .iter()
            .filter(|comparison| !comparison.deterministic())
            .count()
    }

    fn first_divergence(&self) -> Option<&TickComparison> {
        self.comparisons
            .iter()
            .find(|comparison| !comparison.deterministic())
    }

    fn to_json(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "{{");
        let _ = writeln!(out, "  \"scenario\": \"{}\",", self.scenario);
        let _ = writeln!(out, "  \"scalar_backend\": \"{}\",", self.scalar_backend);
        let _ = writeln!(out, "  \"baseline_workers\": {},", self.baseline_workers);
        let _ = writeln!(
            out,
            "  \"worker_counts\": {},",
            json_usize_array(&self.worker_counts)
        );
        let _ = writeln!(out, "  \"tick_count\": {},", self.tick_count);
        let _ = writeln!(out, "  \"divergence_count\": {},", self.divergence_count());
        match self.first_divergence() {
            Some(first) => {
                let _ = writeln!(out, "  \"first_divergence\": {{");
                let _ = writeln!(out, "    \"tick\": {},", first.tick);
                let _ = writeln!(out, "    \"workers\": {},", first.workers);
                let _ = writeln!(
                    out,
                    "    \"fields\": {}",
                    json_str_array(&first.mismatch_fields())
                );
                let _ = writeln!(out, "  }},");
            }
            None => {
                let _ = writeln!(out, "  \"first_divergence\": null,");
            }
        }
        let _ = writeln!(out, "  \"comparisons\": [");
        for (idx, comparison) in self.comparisons.iter().enumerate() {
            let comma = if idx + 1 == self.comparisons.len() {
                ""
            } else {
                ","
            };
            let _ = writeln!(out, "    {{");
            let _ = writeln!(out, "      \"tick\": {},", comparison.tick);
            let _ = writeln!(out, "      \"workers\": {},", comparison.workers);
            let _ = writeln!(
                out,
                "      \"deterministic\": {},",
                comparison.deterministic()
            );
            let _ = writeln!(
                out,
                "      \"state_root_match\": {},",
                comparison.state_root_match
            );
            let _ = writeln!(
                out,
                "      \"patch_digest_match\": {},",
                comparison.patch_digest_match
            );
            let _ = writeln!(
                out,
                "      \"commit_id_match\": {},",
                comparison.commit_id_match
            );
            let _ = writeln!(
                out,
                "      \"scalar_digest_match\": {},",
                comparison.scalar_digest_match
            );
            let _ = writeln!(
                out,
                "      \"baseline_state_root\": \"{}\",",
                hex32(&comparison.baseline_state_root)
            );
            let _ = writeln!(
                out,
                "      \"actual_state_root\": \"{}\",",
                hex32(&comparison.actual_state_root)
            );
            let _ = writeln!(
                out,
                "      \"baseline_commit_id\": \"{}\",",
                hex32(&comparison.baseline_commit_id)
            );
            let _ = writeln!(
                out,
                "      \"actual_commit_id\": \"{}\"",
                hex32(&comparison.actual_commit_id)
            );
            let _ = writeln!(out, "    }}{comma}");
        }
        let _ = writeln!(out, "  ]");
        let _ = writeln!(out, "}}");
        out
    }
}

fn json_usize_array(values: &[usize]) -> String {
    let mut out = String::from("[");
    for (idx, value) in values.iter().enumerate() {
        if idx > 0 {
            out.push_str(", ");
        }
        let _ = write!(out, "{value}");
    }
    out.push(']');
    out
}

fn json_str_array(values: &[&str]) -> String {
    let mut out = String::from("[");
    for (idx, value) in values.iter().enumerate() {
        if idx > 0 {
            out.push_str(", ");
        }
        let _ = write!(out, "\"{value}\"");
    }
    out.push(']');
    out
}

fn scenario_name(scenario: ParallelScenario) -> &'static str {
    match scenario {
        ParallelScenario::Small => "Small",
        ParallelScenario::ManyIndependent => "ManyIndependent",
        ParallelScenario::ManyConflicts => "ManyConflicts",
        ParallelScenario::DeletesAndAttachments => "DeletesAndAttachments",
        ParallelScenario::PrivacyClaims => "PrivacyClaims",
    }
}

fn scalar_digest<S: Scalar>(tick: u64) -> Hash {
    let t = S::from_f32((tick as f32) + 1.25);
    let scale = S::from_f32(0.5);
    let bias = S::from_f32(3.0);
    let (sin, cos) = t.sin_cos();
    let value = ((t + sin) * (cos + bias)) / (scale + S::one());

    let mut hasher = blake3::Hasher::new();
    hasher.update(b"echo:test:thread-determinism:scalar-digest:v1\0");
    hasher.update(&tick.to_le_bytes());
    hasher.update(&value.to_f32().to_bits().to_le_bytes());
    hasher.finalize().into()
}

fn make_witness(
    raw: &common::ParallelExecResult,
    parent: Option<Hash>,
    scalar_backend: ScalarBackend,
    tick: u64,
) -> TickWitness {
    let parents = parent.into_iter().collect::<Vec<_>>();
    let commit_id = compute_commit_hash_v2(&raw.state_root, &parents, &raw.patch_digest, 0);
    TickWitness {
        state_root: raw.state_root,
        patch_digest: raw.patch_digest,
        commit_id,
        scalar_digest: scalar_backend.digest(tick),
    }
}

fn make_comparison(
    tick: u64,
    workers: usize,
    baseline: &TickWitness,
    actual: &TickWitness,
) -> TickComparison {
    TickComparison {
        tick,
        workers,
        state_root_match: baseline.state_root == actual.state_root,
        patch_digest_match: baseline.patch_digest == actual.patch_digest,
        commit_id_match: baseline.commit_id == actual.commit_id,
        scalar_digest_match: baseline.scalar_digest == actual.scalar_digest,
        baseline_state_root: baseline.state_root,
        actual_state_root: actual.state_root,
        baseline_commit_id: baseline.commit_id,
        actual_commit_id: actual.commit_id,
    }
}

fn run_report(
    scenario: ParallelScenario,
    scalar_backend: ScalarBackend,
    tick_count: u64,
    worker_counts: &[usize],
    hook: Option<DivergenceHook>,
) -> DeterminismReport {
    let harness = parallel_harness();
    let base = harness.build_base_snapshot(scenario);
    let mut comparisons = Vec::new();

    for &workers in worker_counts {
        let mut baseline_parent = None;
        let mut actual_parent = None;

        for tick in 0..tick_count {
            let ingress = harness.make_ingress(scenario, tick);
            let baseline_raw = harness.execute_parallel(&base, &ingress, tick, 1);
            let mut actual_raw = harness.execute_parallel(&base, &ingress, tick, workers);

            if hook.is_some_and(|hook| hook.tick == tick && hook.workers == workers) {
                actual_raw.patch_digest[0] ^= 0x80;
            }

            let baseline = make_witness(&baseline_raw, baseline_parent, scalar_backend, tick);
            let actual = make_witness(&actual_raw, actual_parent, scalar_backend, tick);
            comparisons.push(make_comparison(tick, workers, &baseline, &actual));

            baseline_parent = Some(baseline.commit_id);
            actual_parent = Some(actual.commit_id);
        }
    }

    DeterminismReport {
        scenario: scenario_name(scenario),
        scalar_backend: scalar_backend.name(),
        baseline_workers: 1,
        worker_counts: worker_counts.to_vec(),
        tick_count,
        comparisons,
    }
}

#[test]
fn reports_zero_divergences_for_f32_core_scenarios() {
    for scenario in [
        ParallelScenario::Small,
        ParallelScenario::ManyIndependent,
        ParallelScenario::ManyConflicts,
    ] {
        let report = run_report(scenario, ScalarBackend::F32, 4, WORKERS_UNDER_TEST, None);
        let json = report.to_json();

        assert_eq!(report.divergence_count(), 0, "{json}");
        assert!(json.contains("\"scalar_backend\": \"F32Scalar\""));
        assert!(json.contains("\"worker_counts\": [1, 2, 4, 8]"));
        assert!(json.contains("\"state_root_match\": true"));
        assert!(json.contains("\"commit_id_match\": true"));
    }
}

#[test]
fn zero_ticks_report_is_trivially_deterministic() {
    let report = run_report(
        ParallelScenario::Small,
        ScalarBackend::F32,
        0,
        WORKERS_UNDER_TEST,
        None,
    );
    let json = report.to_json();

    assert_eq!(report.divergence_count(), 0);
    assert!(report.comparisons.is_empty());
    assert!(json.contains("\"tick_count\": 0"));
    assert!(json.contains("\"comparisons\": ["));
}

#[test]
fn ordering_break_hook_reports_first_divergence() {
    let report = run_report(
        ParallelScenario::ManyIndependent,
        ScalarBackend::F32,
        4,
        WORKERS_UNDER_TEST,
        Some(DivergenceHook {
            tick: 2,
            workers: 4,
        }),
    );
    let json = report.to_json();
    let first = report
        .first_divergence()
        .expect("ordering hook should force divergence");

    assert!(report.divergence_count() >= 1, "{json}");
    assert_eq!(first.tick, 2);
    assert_eq!(first.workers, 4);
    assert!(first.mismatch_fields().contains(&"patch_digest"));
    assert!(first.mismatch_fields().contains(&"commit_id"));
    assert!(json.contains("\"first_divergence\": {"));
    assert!(json.contains("\"fields\": [\"patch_digest\", \"commit_id\"]"));
}

#[cfg(feature = "det_fixed")]
#[test]
fn reports_zero_divergences_for_dfix64_backend() {
    let report = run_report(
        ParallelScenario::ManyIndependent,
        ScalarBackend::DFix64,
        4,
        WORKERS_UNDER_TEST,
        None,
    );
    let json = report.to_json();

    assert_eq!(report.divergence_count(), 0, "{json}");
    assert!(json.contains("\"scalar_backend\": \"DFix64\""));
    assert!(json.contains("\"scalar_digest_match\": true"));
}
