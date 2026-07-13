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
  ".github/workflows/refresh-dependency-dags.yml"
  "docs/assets/dags"
  "scripts/dag-utils.js"
  "scripts/generate-dependency-dags.js"
  "scripts/open_dependency_dags_pr.sh"
  "scripts/parse-tasks-dag.js"
  "scripts/tests/parse-tasks-dag.test.js"
  "tests/hooks/test_dependency_dags.sh"
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

if ((failures > 0)); then
  echo "knowledge-model: ${failures} violation(s)" >&2
  exit 1
fi

echo "knowledge-model: all required documents present"
