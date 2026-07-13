#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

readonly required_docs=(
  "docs/adr/README.md"
  "docs/adr/0012-repository-knowledge-model.md"
  "docs/adr/0013-echo-continuum-authority-boundary.md"
  "docs/adr/0014-generated-rule-authorship-and-footprints.md"
  "docs/adr/0015-registry-provider-host-boundary.md"
  "docs/adr/0016-continuum-transport-identity.md"
  "docs/adr/0017-universal-little-endian-codec.md"
  "docs/adr/0018-sessions-causal-posture-and-authority.md"
  "docs/adr/0019-bunny-owns-reusable-geometry.md"
  "docs/adr/0020-retained-reading-storage-and-proof-boundary.md"
  "docs/topics/README.md"
  "docs/topics/RuntimeAuthority.md"
  "docs/topics/StrandsAndBraids.md"
  "docs/topics/Obstructions.md"
  "docs/topics/RuntimeConstellation.md"
  "docs/topics/WarpOptics.md"
  "docs/topics/GeneratedRules.md"
)

readonly forbidden_process_paths=(
  "METHOD.md"
  "docs/method"
  "docs/design"
  "docs/BEARING.md"
  "docs/WorkItems.md"
  "docs/workflows.md"
  "docs/procedures"
  "docs/technical-teardown.md"
  "backlog"
  "crates/method"
  "scripts/check-append-only.js"
  "scripts/check_task_lists.sh"
  "scripts/generate_evidence.cjs"
  "scripts/tests/check_task_lists_test.sh"
  "scripts/validate_claims.cjs"
  "docs/determinism/CLAIM_MAP.yaml"
  "docs/determinism/DETERMINISM_CLAIMS_v0.1.md"
  "docs/determinism/RELEASE_POLICY.md"
  "docs/determinism/sec-claim-map.json"
  "schemas/runtime/README.md"
  "docs/architecture/WARP_DRIFT.md"
  "docs/architecture/continuum-foundations.md"
  "docs/architecture/wsc-verkle-ipa-retained-readings.md"
  "docs/benchmarks/RESERVE_BENCHMARK.md"
  "docs/benchmarks/parallelism-study.md"
  ".github/workflows/refresh-dependency-dags.yml"
  "docs/assets/dags"
  "scripts/dag-utils.js"
  "scripts/generate-dependency-dags.js"
  "scripts/open_dependency_dags_pr.sh"
  "scripts/parse-tasks-dag.js"
  "scripts/scaffold-community.sh"
  "scripts/tests/parse-tasks-dag.test.js"
  "tests/hooks/test_dependency_dags.sh"
)

readonly stale_process_anchor_files=(
  "scripts/tests/fixed_timestep_invariant_test.sh"
  "scripts/tests/declarative_rule_authorship_invariant_test.sh"
  "scripts/tests/strand_contract_invariant_test.sh"
  "crates/warp-core/tests/strand_contract_tests.rs"
  "crates/warp-core/src/revelation.rs"
  "crates/warp-core/src/braid_shell.rs"
)

failures=0
for required_doc in "${required_docs[@]}"; do
  if [[ ! -f "${required_doc}" ]]; then
    echo "knowledge-model: missing ${required_doc}" >&2
    failures=$((failures + 1))
  fi
done

for forbidden_path in "${forbidden_process_paths[@]}"; do
  if [[ -e "${forbidden_path}" ]]; then
    echo "knowledge-model: forbidden process artifact ${forbidden_path}" >&2
    failures=$((failures + 1))
  fi
done

if grep -Eq 'CLAIM_MAP\.yaml|sec-claim-map\.json' .github/workflows/det-gates.yml; then
  echo "knowledge-model: determinism workflow references a deleted status map" >&2
  failures=$((failures + 1))
fi

if grep -Eq '(DET|SEC|REPRO|PRF)-[0-9]{3}' .github/workflows/det-gates.yml; then
  echo "knowledge-model: determinism workflow emits undefined numeric claim IDs" >&2
  failures=$((failures + 1))
fi

for evidence_gate_claim in \
  'Evidence artifact presence' \
  'required_artifacts=(' \
  'needs.decoder-security.result' \
  'gathered-artifacts/perf-artifacts/perf-report.json' \
  'gathered-artifacts/static-inspection/static-inspection.log' \
  'Missing or empty artifact'; do
  if ! grep -Fq -- "${evidence_gate_claim}" .github/workflows/det-gates.yml; then
    echo "knowledge-model: determinism artifact gate missing: ${evidence_gate_claim}" >&2
    failures=$((failures + 1))
  fi
done

if grep -Fq -- "scaffold-community" det-policy.yaml; then
  echo "knowledge-model: determinism policy names a retired process scaffolder" >&2
  failures=$((failures + 1))
fi

if grep -Fq -- "Recommended Next Benches" docs/benchmarks/scheduler-performance-warp-core.md; then
  echo "knowledge-model: scheduler benchmark guide contains a work queue" >&2
  failures=$((failures + 1))
fi

if grep -Eq 'RESERVE_BENCHMARK\.md|reserve_independence|MY_FEATURE_BENCHMARK\.md' \
  docs/benchmarks/BENCHMARK_GUIDE.md; then
  echo "knowledge-model: benchmark guide contains a status-report template or dead example" >&2
  failures=$((failures + 1))
fi

if grep -Eq \
  '^- \[ \] Added to .*GROUPS array|^- \[ \] Added to .*BENCH_CORE_GROUP_KEYS|^- \[ \] Dashboard displays line' \
  docs/benchmarks/BENCHMARK_GUIDE.md; then
  echo "knowledge-model: benchmark checklist makes the core dashboard unconditional" >&2
  failures=$((failures + 1))
fi

for benchmark_checklist_claim in \
  'Selected report surface is registered and renders the benchmark' \
  'Core-dashboard benchmarks are added to both'; do
  if ! grep -Fq -- "${benchmark_checklist_claim}" docs/benchmarks/BENCHMARK_GUIDE.md; then
    echo "knowledge-model: benchmark checklist missing: ${benchmark_checklist_claim}" >&2
    failures=$((failures + 1))
  fi
done

if grep -Fq -- \
  "A retained-reading proof envelope names" \
  docs/adr/0020-retained-reading-storage-and-proof-boundary.md; then
  echo "knowledge-model: ADR 0020 implies an unsupported proof envelope exists" >&2
  failures=$((failures + 1))
fi

if ! grep -Fq -- \
  "Any proof-bearing retained-reading envelope, when supported, must name" \
  docs/adr/0020-retained-reading-storage-and-proof-boundary.md; then
  echo "knowledge-model: ADR 0020 is missing the future proof-envelope contract" >&2
  failures=$((failures + 1))
fi

if stale_process_anchors="$({
  rg -n \
    'cycle 0003|cycle 0004|cycle 0012|design packet 0026' \
    "${stale_process_anchor_files[@]}"
} 2>/dev/null)"; then
  echo "knowledge-model: source comments cite deleted process artifacts" >&2
  echo "${stale_process_anchors}" >&2
  failures=$((failures + 1))
fi

if grep -Eq \
  'graph-rewrite simulation engine|ScenePort|TTD Port|State is a typed, directed multigraph|WARP Graphs' \
  ARCHITECTURE.md; then
  echo "knowledge-model: root architecture page teaches the superseded graph/port model" >&2
  failures=$((failures + 1))
fi

if grep -Eq 'FixedTrig|State is a finite directed multigraph' ADVANCED_GUIDE.md; then
  echo "knowledge-model: advanced guide teaches superseded or fabricated doctrine" >&2
  failures=$((failures + 1))
fi

for superseded_root_doc in ARCHITECTURE.md ADVANCED_GUIDE.md; do
  if ! grep -Fq -- "Superseded" "${superseded_root_doc}" || \
    ! grep -Fq -- "docs/README.md" "${superseded_root_doc}"; then
    echo "knowledge-model: ${superseded_root_doc} lacks an honest supersession route" >&2
    failures=$((failures + 1))
  fi
done

if grep -Fq -- 'Graph is truth' AGENTS.md || \
  grep -Eq 'Current architectural truth.*`ARCHITECTURE\.md`' AGENTS.md; then
  echo "knowledge-model: AGENTS promotes superseded graph or root-document authority" >&2
  failures=$((failures + 1))
fi

if ! grep -Fq -- 'Witnessed causal history is truth' AGENTS.md; then
  echo "knowledge-model: AGENTS lacks the trace derivation authority rule" >&2
  failures=$((failures + 1))
fi

if grep -Eq 'T2000 on|README Brag|PATHS_DEFAULT=|PATTERNS=\(' \
  docs/adr/ADR-0006-Ban-Non-Determinism.md; then
  echo "knowledge-model: ADR 0006 contains an embedded implementation packet" >&2
  failures=$((failures + 1))
fi

if grep -Eq \
  '^## (12\) Migration plan|14\) Tests|Commit-ready Rust test skeletons|Next: wiring strategy|Sequencing|The one warning)|Phase 6B COMPLETE|^- \[x\]' \
  docs/adr/ADR-0007-BOAW-Storage.md; then
  echo "knowledge-model: ADR 0007 contains an implementation diary or test-plan packet" >&2
  failures=$((failures + 1))
fi

if grep -Eq \
  '^## (Implementation Plan|Key Files \(Observed State|Gameplay and Non-Debug Use Cases|Test Requirements|Document Governance)|critical path|8-step plan|crates/ttd-browser' \
  docs/adr/ADR-0008-Worldline-Runtime-Model.md; then
  echo "knowledge-model: ADR 0008 contains a dated plan, status ledger, or work governance" >&2
  failures=$((failures + 1))
fi

if grep -Eq \
  '^## (Implementation Considerations|Test Requirements|Document Governance)|^### (Near-term|Mid-term|Later)' \
  docs/adr/ADR-0009-Inter-Worldline-Communication.md; then
  echo "knowledge-model: ADR 0009 contains a roadmap, test plan, or work governance" >&2
  failures=$((failures + 1))
fi

if grep -Eq '^## Appendix|README Brag|PATHS_DEFAULT=|PATTERNS=\(' \
  docs/adr/ADR-0004-No-Global-State.md; then
  echo "knowledge-model: ADR 0004 contains a copied enforcement recipe" >&2
  failures=$((failures + 1))
fi

if grep -Eq '^## Follow-ups|docs/warp-two-plane-law\.md' \
  docs/adr/ADR-0001-warp-two-plane-skeleton-and-attachments.md \
  docs/adr/ADR-0002-warp-instances-descended-attachments.md; then
  echo "knowledge-model: two-plane ADRs contain a work queue or dead invariant path" >&2
  failures=$((failures + 1))
fi

if grep -Eq \
  'Future recursion in attachments.*future work|Full descended attachments require additional design work' \
  docs/adr/ADR-0001-warp-two-plane-skeleton-and-attachments.md; then
  echo "knowledge-model: ADR 0001 treats implemented descended attachments as future work" >&2
  failures=$((failures + 1))
fi

if grep -Eq \
  'Compatibility is one phase|During ABI v1|At the start of Phase 6|deleted on schedule|Those remain later work' \
  docs/adr/ADR-0011-explicit-observation-contract.md; then
  echo "knowledge-model: ADR 0011 contains an ABI migration diary or future-work queue" >&2
  failures=$((failures + 1))
fi

if grep -Eq '^## [0-9]+\. (Audit Findings|Implementation Checklist)|Issue #[0-9]+|PR #[0-9]+' \
  docs/determinism/SPEC_DETERMINISTIC_MATH.md; then
  echo "knowledge-model: deterministic math policy contains a dated audit/status checklist" >&2
  failures=$((failures + 1))
fi

if rg -q \
  'Echo-admitted causal anchors|A causal anchor is .*Echo-admitted|Echo validates that the frontier is admitted|A later slice must|Causal anchor + = Echo admitted' \
  docs/topics/CausalAnchors.md crates/warp-core/src/causal_anchor.rs; then
  echo "knowledge-model: causal-anchor contract overclaims trusted admission" >&2
  failures=$((failures + 1))
fi

for causal_anchor_truth in \
  'canonical causal-anchor value contract' \
  'caller-provided references' \
  'No current API verifies' \
  'publishes the value under trusted runtime authority'; do
  if ! grep -Fq -- "${causal_anchor_truth}" docs/topics/CausalAnchors.md; then
    echo "knowledge-model: causal-anchor topic missing: ${causal_anchor_truth}" >&2
    failures=$((failures + 1))
  fi
done

if ! grep -Fq -- 'does not verify frontier admission' \
  crates/warp-core/src/causal_anchor.rs || \
  ! grep -Fq -- 'receipt provenance, authority, or retention' \
  crates/warp-core/src/causal_anchor.rs; then
  echo "knowledge-model: causal-anchor API docs hide the unverified boundary" >&2
  failures=$((failures + 1))
fi

if grep -Eq \
  'remaining durability gate|Track that remaining implementation work|required next hardening step' \
  docs/topics/WAL.md; then
  echo "knowledge-model: WAL topic contains a live implementation queue" >&2
  failures=$((failures + 1))
fi

if ! grep -Fq -- 'Topology operations do not currently have' \
  docs/topics/WAL.md || \
  ! grep -Fq -- 'WAL-backed accepted evidence or recoverable WSC-retained material' \
  docs/topics/WAL.md; then
  echo "knowledge-model: WAL topic hides the topology durability boundary" >&2
  failures=$((failures + 1))
fi

if grep -Eq 'current `v[0-9]+\.[0-9]+\.[0-9]+` goal|Ongoing work focuses' README.md; then
  echo "knowledge-model: README contains a live release goal or work queue" >&2
  failures=$((failures + 1))
fi

if grep -Eq \
  'renderer-agnostic engine|reproducible simulations|Temporal Tooling|will regain coverage when reintroduced|hook now aborts the commit if running `cargo fmt` would change any files' \
  CONTRIBUTING.md; then
  echo "knowledge-model: contributor guide contains obsolete framing, future work, or hook behavior" >&2
  failures=$((failures + 1))
fi

if rg -q \
  '^Legend:|^## Why this packet exists|^## (Human|Agent) users / jobs / hills|^The hill:' \
  docs/spec; then
  echo "knowledge-model: canonical specs contain Method-era packet taxonomy" >&2
  failures=$((failures + 1))
fi

if grep -Eq '^Status: (Partial|Incomplete|In progress)' \
  docs/spec/abi-golden-vectors.md; then
  echo "knowledge-model: ABI golden-vector spec contains a live completeness bit" >&2
  failures=$((failures + 1))
fi

if ((failures > 0)); then
  echo "knowledge-model: ${failures} violation(s)" >&2
  exit 1
fi

echo "knowledge-model: all required documents present"
