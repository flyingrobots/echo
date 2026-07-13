#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

readonly required_docs=(
  "docs/adr/README.md"
  "docs/adr/0001-repository-knowledge-model.md"
  "docs/adr/0002-echo-continuum-authority-boundary.md"
  "docs/adr/0003-generated-rule-authorship-and-footprints.md"
  "docs/adr/0004-registry-provider-host-boundary.md"
  "docs/adr/0005-continuum-transport-identity.md"
  "docs/adr/0006-universal-little-endian-codec.md"
  "docs/adr/0007-sessions-causal-posture-and-authority.md"
  "docs/adr/0008-bunny-owns-reusable-geometry.md"
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
