#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

readonly legacy_adrs=(
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
)

readonly current_adrs=(
  "0012-repository-knowledge-model.md"
  "0013-echo-continuum-authority-boundary.md"
  "0014-generated-rule-authorship-and-footprints.md"
  "0015-registry-provider-host-boundary.md"
  "0016-continuum-transport-identity.md"
  "0017-universal-little-endian-codec.md"
  "0018-sessions-causal-posture-and-authority.md"
  "0019-bunny-owns-reusable-geometry.md"
)

readonly collided_paths=(
  "docs/adr/0001-repository-knowledge-model.md"
  "docs/adr/0002-echo-continuum-authority-boundary.md"
  "docs/adr/0003-generated-rule-authorship-and-footprints.md"
  "docs/adr/0004-registry-provider-host-boundary.md"
  "docs/adr/0005-continuum-transport-identity.md"
  "docs/adr/0006-universal-little-endian-codec.md"
  "docs/adr/0007-sessions-causal-posture-and-authority.md"
  "docs/adr/0008-bunny-owns-reusable-geometry.md"
)

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

failures=0
fail() {
  echo "adr-namespace: $*" >&2
  failures=$((failures + 1))
}

for basename in "${legacy_adrs[@]}" "${current_adrs[@]}"; do
  path="docs/adr/${basename}"
  [[ -f "$path" ]] || fail "missing canonical record ${path}"
done

for path in "${collided_paths[@]}"; do
  if [[ -e "$path" ]]; then
    fail "collided ADR path still exists: ${path}"
  fi

  if git grep -Fq -- "$path" -- . \
    ':(exclude)tests/docs/test_adr_namespace.sh'; then
    fail "live reference still uses collided ADR path: ${path}"
  fi
done

while IFS= read -r path; do
  basename="${path##*/}"
  case "$basename" in
    ADR-[0-9][0-9][0-9][0-9]-*.md)
      id="${basename#ADR-}"
      id="${id%%-*}"
      ;;
    [0-9][0-9][0-9][0-9]-*.md)
      id="${basename%%-*}"
      ;;
    *)
      fail "non-canonical ADR filename: ${path}"
      continue
      ;;
  esac
  printf '%s|%s\n' "$id" "$path" >>"${tmp_dir}/records"
done < <(find docs/adr -maxdepth 1 -type f -name '*.md' ! -name README.md | sort)

if [[ -s "${tmp_dir}/records" ]]; then
  cut -d'|' -f1 "${tmp_dir}/records" | sort >"${tmp_dir}/actual-ids"
  duplicate_ids="$(uniq -d "${tmp_dir}/actual-ids")"
  [[ -z "$duplicate_ids" ]] || fail "duplicate ADR IDs: ${duplicate_ids//$'\n'/, }"
else
  : >"${tmp_dir}/actual-ids"
  fail "no canonical ADR records found"
fi

for number in {1..19}; do
  printf '%04d\n' "$number"
done >"${tmp_dir}/expected-ids"

if ! diff -u "${tmp_dir}/expected-ids" "${tmp_dir}/actual-ids" >"${tmp_dir}/id-diff"; then
  cat "${tmp_dir}/id-diff" >&2
  fail "ADR IDs must be the unique contiguous range 0001 through 0019"
fi

for basename in "${legacy_adrs[@]}" "${current_adrs[@]}"; do
  count="$(grep -Foc -- "(${basename})" docs/adr/README.md || true)"
  [[ "$count" == "1" ]] || fail "README must link ${basename} exactly once (found ${count})"
done

while IFS= read -r link; do
  [[ -f "docs/adr/${link}" ]] || fail "README link does not resolve: docs/adr/${link}"
done < <(perl -ne 'while (/\]\(([^)#]+\.md)(?:#[^)]*)?\)/g) { print "$1\n" }' docs/adr/README.md)

for basename in "${current_adrs[@]}"; do
  id="${basename%%-*}"
  path="docs/adr/${basename}"
  if [[ -f "$path" ]] && ! grep -Eq -- "^# ADR ${id}:" "$path"; then
    fail "H1 number does not match filename: ${path}"
  fi
done

if ((failures > 0)); then
  echo "adr-namespace: ${failures} violation(s)" >&2
  exit 1
fi

echo "adr-namespace: canonical IDs 0001 through 0019 are unique and indexed"
