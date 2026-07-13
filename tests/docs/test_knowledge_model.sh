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
  "scripts/tests/check_task_lists_test.sh"
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

if ((failures > 0)); then
  echo "knowledge-model: ${failures} violation(s)" >&2
  exit 1
fi

echo "knowledge-model: all required documents present"
