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

reject_literal() {
  local label="$1"
  local file="$2"
  local literal="$3"
  if [[ ! -f "$file" ]]; then
    fail "${label}: missing file ${file}"
    return
  fi
  if grep -Fq -- "$literal" "$file"; then
    fail "${label}: rejected stale literal still present: ${literal}"
  fi
}

bearing="${repo_root}/docs/BEARING.md"
workitems="${repo_root}/docs/WorkItems.md"
sequencing="${repo_root}/docs/design/work-item-sequencing-and-prioritization.md"
wal_design="${repo_root}/docs/design/causal-wal-end-to-end.md"
roadmap="${repo_root}/docs/design/wal-wsc-durability-roadmap.md"

require_file "BEARING signpost" "$bearing"
require_file "WorkItems inventory" "$workitems"
require_file "sequencing guide" "$sequencing"
require_file "causal WAL design" "$wal_design"
require_file "WAL/WSC durability roadmap" "$roadmap"

issue_url="https://github.com/flyingrobots/echo/issues/521"
wal_design_path="docs/design/causal-wal-end-to-end.md"
roadmap_path="docs/design/wal-wsc-durability-roadmap.md"

require_literal "BEARING links WAL/WSC issue" "$bearing" "$issue_url"
require_literal "BEARING links causal WAL design" "$bearing" "$wal_design_path"
require_literal "BEARING links durability roadmap" "$bearing" "$roadmap_path"
require_literal "WorkItems links WAL/WSC issue" "$workitems" "$issue_url"
require_literal "WorkItems links durability roadmap" "$workitems" "$roadmap_path"
require_literal \
  "WorkItems names legacy method backlog marker" \
  "$workitems" \
  "Contains only \`.gitkeep\`; live backlog moved to GitHub Issues."
require_literal "sequencing links WAL/WSC issue" "$sequencing" "$issue_url"
require_literal "sequencing links durability roadmap" "$sequencing" "$roadmap_path"

reject_literal "WorkItems removes stale ASAP backlog links" "$workitems" "](method/backlog/asap/"
reject_literal \
  "WorkItems removes stale WAL/WSC backlog link" \
  "$workitems" \
  "method/backlog/v0.1.0/PLATFORM_wal-wsc-storage-relationship.md"
reject_literal \
  "WorkItems removes stale WSC backlog link" \
  "$workitems" \
  "method/backlog/v0.1.0/PLATFORM_wsc-causal-history-storage.md"
reject_literal \
  "WorkItems removes stale retained evidence backlog link" \
  "$workitems" \
  "method/backlog/v0.1.0/PLATFORM_retained-evidence-durability-boundary.md"
reject_literal "WorkItems removes stale up-next backlog links" "$workitems" "](method/backlog/up-next/"
reject_literal "WorkItems removes stale inbox backlog links" "$workitems" "](method/backlog/inbox/"
reject_literal "WorkItems removes stale bad-code backlog links" "$workitems" "](method/backlog/bad-code/"
reject_literal "WorkItems removes stale cool-ideas backlog links" "$workitems" "](method/backlog/cool-ideas/"

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

require_literal \
  "roadmap names durable issue map" \
  "$roadmap" \
  "## Roadmap Issue Map"
require_literal \
  "roadmap names 30-slice tracker" \
  "$roadmap" \
  "durable 30-slice tracker"
require_literal \
  "roadmap links current PR" \
  "$roadmap" \
  "https://github.com/flyingrobots/echo/pull/582"
for issue_number in {554..581}; do
  require_literal \
    "roadmap links child issue #${issue_number}" \
    "$roadmap" \
    "https://github.com/flyingrobots/echo/issues/${issue_number}"
done
require_literal \
  "roadmap names runtime WAL durable join" \
  "$roadmap" \
  "## Goalpost 1: Durable Runtime WAL Join"
require_literal \
  "roadmap names WAL evidence projection" \
  "$roadmap" \
  "## Goalpost 2: WAL Evidence Projection"
require_literal \
  "roadmap names WSC export and import" \
  "$roadmap" \
  "## Goalpost 3: WSC Causal-History Export And Import"
require_literal \
  "roadmap names retained evidence durability" \
  "$roadmap" \
  "## Goalpost 4: Retained Evidence Durability"

if [[ "$failures" -ne 0 ]]; then
  exit 1
fi

echo "check-wal-wsc-doctrine: passed"
