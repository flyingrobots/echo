#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

set -euo pipefail

script_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="${ECHO_REPO_ROOT:-$(cd -- "${script_root}/.." && pwd)}"

failures=0

fail() {
  echo "FAIL: $*" >&2
  failures=$((failures + 1))
}

require_file() {
  local label="$1"
  local file="$2"
  if [[ ! -f "$file" ]]; then
    fail "${label}: missing file ${file}"
  fi
}

require_literal() {
  local label="$1"
  local file="$2"
  local literal="$3"
  if [[ ! -f "$file" ]]; then
    fail "${label}: missing file ${file}"
    return
  fi
  if ! grep -Fq -- "$literal" "$file"; then
    fail "${label}: missing literal: ${literal}"
  fi
}

bearing="${repo_root}/docs/BEARING.md"
workitems="${repo_root}/docs/WorkItems.md"
sequencing="${repo_root}/docs/design/work-item-sequencing-and-prioritization.md"
wal_design="${repo_root}/docs/design/causal-wal-end-to-end.md"

require_file "BEARING signpost" "$bearing"
require_file "WorkItems inventory" "$workitems"
require_file "sequencing guide" "$sequencing"
require_file "causal WAL design" "$wal_design"

issue_url="https://github.com/flyingrobots/echo/issues/521"
wal_design_path="docs/design/causal-wal-end-to-end.md"

require_literal "BEARING links WAL/WSC issue" "$bearing" "$issue_url"
require_literal "BEARING links causal WAL design" "$bearing" "$wal_design_path"
require_literal "WorkItems links WAL/WSC issue" "$workitems" "$issue_url"
require_literal "sequencing links WAL/WSC issue" "$sequencing" "$issue_url"

require_literal \
  "sequencing names WAL as durable commit authority" \
  "$sequencing" \
  "WAL bytes are the durable commit authority."
require_literal \
  "sequencing names graph facts as projected evidence" \
  "$sequencing" \
  "WARP graph WAL nodes are projected evidence facts;"
require_literal \
  "sequencing names WSC evidence posture" \
  "$sequencing" \
  "WSC carries or references that evidence;"
require_literal \
  "sequencing names recovery bootstrap source" \
  "$sequencing" \
  "recovery bootstraps from WAL root or storage manifest material"

require_literal \
  "WAL design has WAL/WSC projection section" \
  "$wal_design" \
  "## WAL Projection Into The WARP Graph And WSC"
require_literal \
  "WAL design names durable commit authority" \
  "$wal_design" \
  "WAL bytes are the durable commit authority."
require_literal \
  "WAL design names graph facts as evidence" \
  "$wal_design" \
  "WARP graph facts track WAL segment evidence."
require_literal \
  "WAL design names WSC export posture" \
  "$wal_design" \
  "WSC serializes graph facts and may bundle or reference WAL bytes."
require_literal \
  "WAL design says locators are not causal identity" \
  "$wal_design" \
  "that locator is not causal identity"
require_literal \
  "WAL design names recovery bootstrap source" \
  "$wal_design" \
  "configured WAL root or storage manifest"
require_literal "WAL design names ref-only WSC" "$wal_design" "Ref-only WSC"
require_literal "WAL design names self-contained WSC" "$wal_design" "Self-contained WSC"
require_literal "WAL design names CAS-addressed WSC" "$wal_design" "CAS-addressed WSC"
require_literal "WAL design says records are recorded" "$wal_design" "Records are recorded."
require_literal \
  "WAL design says transactions are committed" \
  "$wal_design" \
  "Transactions are committed."
require_literal "WAL design says segments are sealed" "$wal_design" "Segments are sealed."
require_literal \
  "WAL design says graph facts are projected evidence" \
  "$wal_design" \
  "Graph WAL facts are projected evidence."
require_literal \
  "WAL design says commit boundary remains authority" \
  "$wal_design" \
  "The WAL commit boundary remains the authority."

if [[ "$failures" -ne 0 ]]; then
  exit 1
fi

echo "check-wal-wsc-doctrine: passed"
