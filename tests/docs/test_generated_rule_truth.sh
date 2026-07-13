#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

topic="docs/topics/GeneratedRules.md"
adr="docs/adr/0014-generated-rule-authorship-and-footprints.md"
wesley_fixture="crates/echo-wesley-gen/tests/generation.rs"
edict_bridge="crates/warp-core/src/edict_target_ir.rs"

fail() {
  echo "generated-rule-truth: $*" >&2
  exit 1
}

require_literal() {
  local file="$1"
  local literal="$2"
  grep -Fq -- "$literal" "$file" || fail "${file} is missing: ${literal}"
}

reject_literal() {
  local file="$1"
  local literal="$2"
  if grep -Fq -- "$literal" "$file"; then
    fail "${file} retains inaccurate claim: ${literal}"
  fi
}

require_literal "$wesley_fixture" ".register_rule(increment_contract_rule("
require_literal "$edict_bridge" "It is not general Edict"
require_literal "$edict_bridge" "bundle admission, target plugin dispatch, or scheduler counterfactual"

require_literal "$topic" 'Wesley currently emits raw `RewriteRule` builders'
require_literal "$topic" 'does not emit an `InstalledContractPackage`'
require_literal "$topic" 'The Edict bridge is fixture-only'
require_literal "$topic" 'does not admit a package or execute scheduler work'
require_literal "$topic" '`native_rule_bootstrap` is a Cargo feature gate and repository policy boundary'
require_literal "$topic" 'It is not an access-control or security seal'

require_literal "$adr" 'The feature is a policy and compatibility boundary, not an access-control seal.'

reject_literal "$topic" "The supported flow is:"
reject_literal "$topic" "The missing Edict bridge is a generator"
reject_literal "$adr" '`native_rule_bootstrap` remains sealed'
reject_literal "$adr" "without the sealed feature"

echo "generated-rule-truth: current Wesley, Edict, and feature boundaries are explicit"
