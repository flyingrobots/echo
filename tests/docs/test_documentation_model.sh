#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

# This is an immutable archive inventory, not an allocator. Array order carries
# no architectural meaning; current relationships live in named documents.
readonly archived_records=(
  "ADR-0001-warp-two-plane-skeleton-and-attachments.md"
  "ADR-0002-warp-instances-descended-attachments.md"
  "ADR-0003-Materialization-Bus.md"
  "ADR-0004-No-Global-State.md"
  "ADR-0005-Physics.md"
  "ADR-0006-Ban-Non-Determinism.md"
  "ADR-0007-BOAW-Storage.md"
  "ADR-0008-Worldline-Runtime-Model.md"
  "ADR-0009-Inter-Worldline-Communication.md"
  "ADR-0010-observational-seek-and-administrative-rewind.md"
  "ADR-0011-explicit-observation-contract.md"
  "0012-repository-knowledge-model.md"
  "0013-echo-continuum-authority-boundary.md"
  "0014-generated-rule-authorship-and-footprints.md"
  "0015-registry-provider-host-boundary.md"
  "0016-continuum-transport-identity.md"
  "0017-universal-little-endian-codec.md"
  "0018-sessions-causal-posture-and-authority.md"
  "0019-bunny-owns-reusable-geometry.md"
  "0020-retained-reading-storage-and-proof-boundary.md"
  "0021-public-optic-observation-boundary.md"
  "0022-application-requested-causal-anchor-admission.md"
  "0023-admitted-executable-operation-packages.md"
  "0024-anchored-node-creation-from-absence.md"
  "0025-scheduler-owned-executable-operation-actions.md"
  "0026-durable-external-action-settlement.md"
)

failures=0
fail() {
  echo "documentation-model: $*" >&2
  failures=$((failures + 1))
}

for required_path in \
  docs/DOCUMENTATION_STANDARDS.md \
  docs/README.md \
  docs/topics/README.md \
  docs/adr/README.md; do
  [[ -f "$required_path" ]] || fail "missing policy owner ${required_path}"
done

if ! grep -Fq -- 'closed historical archive' docs/DOCUMENTATION_STANDARDS.md; then
  fail "documentation policy does not close the numbered ADR archive"
fi

if ! grep -Fq -- 'Do not allocate a new numbered ADR.' AGENTS.md; then
  fail "agent policy still permits numbered ADR allocation"
fi

if grep -ERiq --exclude-dir=adr \
  '(requires?|create|write|add|allocate) (a |an )?(new |separate )?ADR' \
  docs; then
  fail "current documentation still routes a durable decision into a new ADR"
fi

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

printf '%s\n' "${archived_records[@]}" | sort >"${tmp_dir}/expected"
find docs/adr -maxdepth 1 -type f -name '*.md' ! -name README.md \
  -exec basename {} \; | sort >"${tmp_dir}/actual"

if ! diff -u "${tmp_dir}/expected" "${tmp_dir}/actual" >"${tmp_dir}/archive-diff"; then
  cat "${tmp_dir}/archive-diff" >&2
  fail "numbered ADR archive differs from its closed inventory"
fi

for basename in "${archived_records[@]}"; do
  count="$(grep -Foc -- "(${basename})" docs/adr/README.md || true)"
  [[ "$count" == "1" ]] || \
    fail "archive index must link ${basename} exactly once (found ${count})"

  case "$basename" in
    ADR-[0-9][0-9][0-9][0-9]-*.md)
      id="${basename#ADR-}"
      id="${id%%-*}"
      expected_heading="# ADR-${id}:"
      ;;
    [0-9][0-9][0-9][0-9]-*.md)
      id="${basename%%-*}"
      expected_heading="# ADR ${id}:"
      ;;
    *)
      fail "unexpected historical filename shape: ${basename}"
      continue
      ;;
  esac

  if ! grep -Fq -- "$expected_heading" "docs/adr/${basename}"; then
    fail "historical heading does not match filename: docs/adr/${basename}"
  fi
done

while IFS= read -r link; do
  [[ -f "docs/adr/${link}" ]] || \
    fail "archive index link does not resolve: docs/adr/${link}"
done < <(perl -ne 'while (/\]\(([^)#]+\.md)(?:#[^)]*)?\)/g) { print "$1\n" }' docs/adr/README.md)

if ! grep -Eq -- '^- \*\*Status:\*\* Superseded$' \
  docs/adr/0012-repository-knowledge-model.md; then
  fail "historical repository knowledge model is not marked superseded"
fi

if ! grep -F -- '(0012-repository-knowledge-model.md)' docs/adr/README.md | \
  grep -Fq -- '| Superseded'; then
  fail "archive index does not mark repository knowledge model superseded"
fi

if ((failures > 0)); then
  echo "documentation-model: ${failures} violation(s)" >&2
  exit 1
fi

echo "documentation-model: semantic policy current; numbered ADR archive closed and indexed"
