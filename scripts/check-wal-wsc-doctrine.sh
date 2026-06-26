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

reject_literal_anywhere() {
  local label="$1"
  local literal="$2"
  shift 2

  local file
  for file in "$@"; do
    if [[ ! -f "$file" ]]; then
      fail "${label}: missing file ${file}"
      continue
    fi
    if grep -Fq -- "$literal" "$file"; then
      fail "${label}: rejected stale literal still present in ${file}: ${literal}"
    fi
  done
}

bearing="${repo_root}/docs/BEARING.md"
workitems="${repo_root}/docs/WorkItems.md"
sequencing="${repo_root}/docs/design/work-item-sequencing-and-prioritization.md"
wal_design="${repo_root}/docs/design/causal-wal-end-to-end.md"
wal_doctrine="${repo_root}/docs/design/wal-wsc-durability-roadmap.md"
wal_topic="${repo_root}/docs/topics/WAL.md"
release_contract="${repo_root}/docs/releases/echo-1.0-contract.md"

require_file "BEARING signpost" "$bearing"
require_file "Work tracking boundary" "$workitems"
require_file "GitHub-native sequencing doctrine" "$sequencing"
require_file "causal WAL design" "$wal_design"
require_file "WAL/WSC durability doctrine" "$wal_doctrine"
require_file "WAL topic" "$wal_topic"
require_file "Echo 1.0 release contract" "$release_contract"

project_url="https://github.com/users/flyingrobots/projects/15"
issue_521_url="https://github.com/flyingrobots/echo/issues/521"
issue_584_url="https://github.com/flyingrobots/echo/issues/584"
issue_585_url="https://github.com/flyingrobots/echo/issues/585"
issue_588_url="https://github.com/flyingrobots/echo/issues/588"
issue_589_url="https://github.com/flyingrobots/echo/issues/589"
issue_591_url="https://github.com/flyingrobots/echo/issues/591"
release_contract_path="docs/releases/echo-1.0-contract.md"
wal_design_path="docs/design/causal-wal-end-to-end.md"
wal_doctrine_path="docs/design/wal-wsc-durability-roadmap.md"

require_literal "BEARING links Continuum Stack Convergence Project" "$bearing" "$project_url"
require_literal "BEARING links release contract" "$bearing" "$release_contract_path"
require_literal "BEARING links WAL/WSC issue" "$bearing" "$issue_521_url"
require_literal "BEARING links causal WAL design" "$bearing" "$wal_design_path"
require_literal "BEARING links WAL/WSC doctrine" "$bearing" "$wal_doctrine_path"

require_literal "Work boundary links Continuum Stack Convergence Project" "$workitems" "$project_url"
require_literal "Work boundary links Release Bar" "$workitems" "$issue_584_url"
require_literal "Work boundary links WAL/WSC issue" "$workitems" "$issue_521_url"
require_literal "Work boundary links release contract" "$workitems" "$release_contract_path"
require_literal "Work boundary links WAL/WSC doctrine" "$workitems" "$wal_doctrine_path"
require_literal \
  "Work boundary names legacy method backlog marker" \
  "$workitems" \
  "contains only \`.gitkeep\`; live backlog moved to GitHub"
require_literal \
  "Work boundary says close on evidence" \
  "$workitems" \
  "Close issues only when their executable exit criteria have passed"

reject_literal "Work boundary removes audit date" "$workitems" "Last audited:"
reject_literal "Work boundary removes source audit table" "$workitems" "| Open count/status |"
reject_literal "Work boundary removes progress bars" "$workitems" "Progress bars from the current work stream"
reject_literal "Work boundary removes current batch" "$workitems" "Current batch status"
reject_literal "Work boundary removes issue inventory section" "$workitems" "## v0.1.0 Lane"
reject_literal "Work boundary removes stale ASAP backlog links" "$workitems" "](method/backlog/asap/"
reject_literal \
  "Work boundary removes stale WAL/WSC backlog link" \
  "$workitems" \
  "method/backlog/v0.1.0/PLATFORM_wal-wsc-storage-relationship.md"
reject_literal "Work boundary removes stale up-next backlog links" "$workitems" "](method/backlog/up-next/"
reject_literal "Work boundary removes stale inbox backlog links" "$workitems" "](method/backlog/inbox/"
reject_literal "Work boundary removes stale bad-code backlog links" "$workitems" "](method/backlog/bad-code/"
reject_literal "Work boundary removes stale cool-ideas backlog links" "$workitems" "](method/backlog/cool-ideas/"

require_literal "sequencing links Continuum Stack Convergence Project" "$sequencing" "$project_url"
require_literal "sequencing links release contract" "$sequencing" "../releases/echo-1.0-contract.md"
require_literal "sequencing links WorkItems boundary" "$sequencing" "../WorkItems.md"
require_literal "sequencing links WAL/WSC issue" "$sequencing" "$issue_521_url"
require_literal "sequencing links WAL/WSC doctrine" "$sequencing" "$wal_doctrine_path"
require_literal \
  "sequencing uses single milestone doctrine" \
  "$sequencing" \
  "Use one \`Echo 1.0\` milestone per participating repository."
require_literal \
  "sequencing uses native dependency doctrine" \
  "$sequencing" \
  "Use native \`blocked by\` and \`blocking\` relationships for sequencing."
require_literal \
  "sequencing rejects custom repository field" \
  "$sequencing" \
  "Repository is native metadata. Do not create a custom Repository field."
require_literal \
  "sequencing rejects large ready slices" \
  "$sequencing" \
  "An item with Slice \`Needs decomposition\` is not ready implementation work."
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
  "Recovery bootstraps from WAL root or storage manifest material"

reject_literal "sequencing removes update date" "$sequencing" "Last updated:"
reject_literal "sequencing removes current chunk" "$sequencing" "## Current Chunk"
reject_literal "sequencing removes sprint schedule" "$sequencing" "Sprint A:"
reject_literal "sequencing removes release-proof issue table" "$sequencing" "## Release-Proof Sequence"

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

require_literal "release contract has title" "$release_contract" "# Echo 1.0 Release Contract"
require_literal "release contract rejects live roadmap role" "$release_contract" "not a live roadmap"
require_literal "release contract links Continuum Stack Convergence Project" "$release_contract" "$project_url"
require_literal "release contract links Release Bar" "$release_contract" "$issue_584_url"
require_literal "release contract links Gate A" "$release_contract" "$issue_585_url"
require_literal "release contract links Gate B" "$release_contract" "$issue_591_url"
require_literal "release contract links Gate C" "$release_contract" "$issue_589_url"
require_literal "release contract links Gate D" "$release_contract" "$issue_588_url"
require_literal "release contract names release manifest" "$release_contract" "echo-convergence.lock"
require_literal \
  "release contract requires proof packets" \
  "$release_contract" \
  "A proof packet is a downloadable or inspectable evidence bundle"
require_literal \
  "release contract pins compatibility set" \
  "$release_contract" \
  "A compatibility set is the pinned multi-repository state"
require_literal \
  "release contract forbids independent green repos" \
  "$release_contract" \
  "Independently green repositories are not sufficient for Gate D."
require_literal \
  "release contract has binary pass rule" \
  "$release_contract" \
  "There is no partial pass for Echo 1.0."

reject_literal "release contract removes update date" "$release_contract" "Last updated:"
reject_literal "release contract removes progress percentages" "$release_contract" "Progress:"
reject_literal "release contract removes current status" "$release_contract" "Current status"

require_literal "WAL doctrine has title" "$wal_doctrine" "# WAL/WSC Durability Doctrine"
require_literal "WAL doctrine rejects live roadmap role" "$wal_doctrine" "This document is not the live roadmap."
require_literal "WAL doctrine links Continuum Stack Convergence Project" "$wal_doctrine" "$project_url"
require_literal "WAL doctrine links WAL/WSC issue" "$wal_doctrine" "$issue_521_url"
require_literal "WAL doctrine links suffix exchange gate" "$wal_doctrine" "$issue_591_url"
require_literal \
  "WAL doctrine names durable commit authority" \
  "$wal_doctrine" \
  "WAL bytes are the durable commit authority."
require_literal \
  "WAL doctrine names graph facts as projected evidence" \
  "$wal_doctrine" \
  "WARP graph WAL nodes are projected evidence facts."
require_literal "WAL doctrine names WSC evidence posture" "$wal_doctrine" "WSC carries or references that evidence."
require_literal \
  "WAL doctrine names recovery bootstrap source" \
  "$wal_doctrine" \
  "Recovery bootstraps from WAL root or storage manifest material"
require_literal \
  "WAL doctrine requires crash-point matrix" \
  "$wal_doctrine" \
  "Defined crash-point matrix passes."
require_literal \
  "WAL doctrine requires deterministic recovery" \
  "$wal_doctrine" \
  "Recovery is deterministic."
require_literal \
  "WAL doctrine requires retained restart evidence" \
  "$wal_doctrine" \
  "Retained evidence survives restart."
require_literal \
  "WAL doctrine requires idempotent duplicate replay" \
  "$wal_doctrine" \
  "Duplicate replay is idempotent."
require_literal \
  "WAL doctrine requires corrupt evidence rejection" \
  "$wal_doctrine" \
  "Corrupt or incomplete evidence is deterministically rejected."
require_literal \
  "WAL doctrine requires CI recovery artifacts" \
  "$wal_doctrine" \
  "Required recovery artifacts are emitted by CI."

reject_literal "WAL doctrine removes roadmap issue map" "$wal_doctrine" "## Roadmap Issue Map"
reject_literal "WAL doctrine removes 30-slice tracker" "$wal_doctrine" "durable 30-slice tracker"
reject_literal "WAL doctrine removes active packet status" "$wal_doctrine" "Status: active roadmap packet."
reject_literal "WAL doctrine removes update date" "$wal_doctrine" "Last updated:"
reject_literal "WAL doctrine removes goalpost sections" "$wal_doctrine" "## Goalpost "
reject_literal "WAL doctrine removes current PR tracking" "$wal_doctrine" "https://github.com/flyingrobots/echo/pull/582"

durability_claim_docs=(
  "$bearing"
  "$workitems"
  "$sequencing"
  "$wal_design"
  "$wal_doctrine"
  "$wal_topic"
  "$release_contract"
)

reject_literal_anywhere \
  "durability docs reject missing filesystem runtime WAL witness claim" \
  "filesystem runtime WAL witness is missing" \
  "${durability_claim_docs[@]}"
reject_literal_anywhere \
  "durability docs reject premature WSC import authority" \
  "WSC import recovery is authoritative without WAL-backed validation" \
  "${durability_claim_docs[@]}"
reject_literal_anywhere \
  "durability docs reject posture-only retained payload recovery" \
  "retained payload recovery can rely on posture-only refs" \
  "${durability_claim_docs[@]}"

if [[ "$failures" -ne 0 ]]; then
  exit 1
fi

echo "check-wal-wsc-doctrine: passed"
