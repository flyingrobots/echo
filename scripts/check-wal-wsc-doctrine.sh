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

wal_topic="${repo_root}/docs/topics/WAL.md"
runtime_authority="${repo_root}/docs/topics/RuntimeAuthority.md"
continuum_transport="${repo_root}/docs/architecture/continuum-transport.md"
release_contract="${repo_root}/docs/releases/echo-1.0-contract.md"

require_file "WAL topic" "$wal_topic"
require_file "runtime authority topic" "$runtime_authority"
require_file "Continuum transport architecture" "$continuum_transport"
require_file "Echo 1.0 release contract" "$release_contract"

require_literal "WAL topic names durable commit authority" "$wal_topic" "WAL bytes are the durable commit authority."
require_literal "WAL topic names graph facts as projected evidence" "$wal_topic" "Graph facts are projected evidence."
require_literal "WAL topic names WSC evidence posture" "$wal_topic" "carries or references that evidence"
require_literal "WAL topic rejects locator identity" "$wal_topic" "A WAL path is a storage locator, not causal identity."
require_literal "WAL topic names recovery bootstrap source" "$wal_topic" "projected WAL root or storage manifest"
require_literal "WAL topic says records are recorded" "$wal_topic" "Records are recorded."
require_literal "WAL topic says transactions are committed" "$wal_topic" "Transactions are committed."
require_literal "WAL topic says segments are sealed" "$wal_topic" "Segments are sealed."
require_literal "WAL topic requires deterministic recovery" "$wal_topic" "Recovery is deterministic"
require_literal "WAL topic requires crashpoint coverage" "$wal_topic" "The defined crashpoint matrix passes"
require_literal "WAL topic requires idempotent duplicate replay" "$wal_topic" "Duplicate replay is idempotent."
require_literal "WAL topic requires corrupt evidence rejection" "$wal_topic" "Corrupt or incomplete evidence is deterministically rejected."
require_literal "WAL topic requires retained restart evidence" "$wal_topic" "Retained evidence survives restart"
require_literal "WAL topic requires CI recovery artifacts" "$wal_topic" "Required recovery artifacts are emitted by CI."

require_literal "runtime authority keeps ticks host-owned" "$runtime_authority" "create a tick, or command a tick"
require_literal "runtime authority distinguishes admission from execution" "$runtime_authority" "An admission ticket witnesses lawful eligibility. It is not execution."
require_literal "runtime authority treats retry as a causal act" "$runtime_authority" "Retry is a new explicit causal act."

require_literal "Continuum transport treats import as admission" "$continuum_transport" "Import is ordinary admission at a distance."

project_url="https://github.com/users/flyingrobots/projects/15"
issue_584_url="https://github.com/flyingrobots/echo/issues/584"
issue_585_url="https://github.com/flyingrobots/echo/issues/585"
issue_588_url="https://github.com/flyingrobots/echo/issues/588"
issue_589_url="https://github.com/flyingrobots/echo/issues/589"
issue_591_url="https://github.com/flyingrobots/echo/issues/591"

require_literal "release contract has title" "$release_contract" "# Echo 1.0 Release Contract"
require_literal "release contract rejects live roadmap role" "$release_contract" "not a live roadmap"
require_literal "release contract links Continuum Stack Convergence Project" "$release_contract" "$project_url"
require_literal "release contract links Release Bar" "$release_contract" "$issue_584_url"
require_literal "release contract links Gate A" "$release_contract" "$issue_585_url"
require_literal "release contract links Gate B" "$release_contract" "$issue_591_url"
require_literal "release contract links Gate D" "$release_contract" "$issue_588_url"
require_literal "release contract decouples Edict compatibility" "$release_contract" 'Edict and `jedit` compatibility work does not gate Echo 1.0.'
reject_literal_anywhere "release contract rejects Edict release gate" "$issue_589_url" "$release_contract"
reject_literal_anywhere "release contract rejects retired Gate C" "Gate C" "$release_contract"
require_literal "release contract names release manifest" "$release_contract" "echo-convergence.lock"
require_literal "release contract requires proof packets" "$release_contract" "A proof packet is a downloadable or inspectable evidence bundle"
require_literal "release contract pins compatibility set" "$release_contract" "A compatibility set is the pinned multi-repository state"
require_literal "release contract forbids independent green repos" "$release_contract" "Independently green repositories are not sufficient for Gate D."
require_literal "release contract has binary pass rule" "$release_contract" "There is no partial pass for Echo 1.0."

durability_claim_docs=(
  "$wal_topic"
  "$runtime_authority"
  "$continuum_transport"
  "$release_contract"
)

reject_literal_anywhere "durability docs reject missing filesystem runtime WAL witness claim" "filesystem runtime WAL witness is missing" "${durability_claim_docs[@]}"
reject_literal_anywhere "durability docs reject premature WSC import authority" "WSC import recovery is authoritative without WAL-backed validation" "${durability_claim_docs[@]}"
reject_literal_anywhere "durability docs reject posture-only retained payload recovery" "retained payload recovery can rely on posture-only refs" "${durability_claim_docs[@]}"

if [[ "$failures" -ne 0 ]]; then
  exit 1
fi

echo "check-wal-wsc-doctrine: passed"
