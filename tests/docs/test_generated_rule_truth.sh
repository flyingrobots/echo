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
readme="README.md"
jim_case="docs/case-studies/JimAndEcho.md"

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
require_literal "$topic" 'The Edict provider path authenticates exact authored semantic source'
require_literal "$topic" 'non-installing provider package proposal'
require_literal "$topic" 'Generated code cannot install itself'
require_literal "$topic" 'This is claim admission, not package-byte admission:'
require_literal "$topic" '`AdmittedProviderContractPackageV1`'
require_literal "$topic" '`native_rule_bootstrap` is a Cargo feature gate and repository policy boundary'
require_literal "$topic" 'It is not an access-control or security seal'
require_literal "$topic" 'The `footprint_enforce_release` qualification lane is not wired into CI.'
require_literal "$topic" 'No Wesley or Edict package is currently footprint-release-qualified.'

require_literal "$adr" 'The feature is a policy and compatibility boundary, not an access-control seal.'
require_literal "$adr" 'This is a qualification requirement, not a claim that the lane is currently implemented.'

reject_literal "$topic" "The supported flow is:"
reject_literal "$topic" "The missing Edict bridge is a generator"
reject_literal "$topic" "The Edict bridge is fixture-only"
reject_literal "$topic" "does not admit a package or execute scheduler work"
reject_literal "$topic" "No Wesley or Edict pack is currently release-qualified."
reject_literal "$adr" '`native_rule_bootstrap` remains sealed'
reject_literal "$adr" "without the sealed feature"

require_literal "$readme" 'The Wesley compatibility path emits raw `RewriteRule` builders and generated'
require_literal "$readme" 'not emit an `InstalledContractPackage` or exercise package verification.'
require_literal "$readme" 'The helper performs pure, fail-closed preflight across exact package'
require_literal "$readme" 'opaque, non-installing provider package proposal'
require_literal "$readme" 'The helper does not construct an `InstalledContractPackage`, register or'
require_literal "$readme" "No generated bridge yet carries either compiler path across Echo's trusted-host"
require_literal "$readme" '`AdmittedProviderContractPackageV1`'
require_literal "$readme" 'provider-native installed record remain missing.'
require_literal "$readme" 'provider-native installed record remain missing. The helper can encode typed'
require_literal "$readme" 'input and pack a canonical intent for submission through a caller-owned Echo'
require_literal "$readme" 'boundary into native scheduler execution.'
require_literal "$readme" 'The package-shaped flow below is partially implemented, but is not yet a'
reject_literal "$readme" 'It does not yet provide codec-bound invocation'
reject_literal "$readme" 'The package-shaped flow below is the target corridor, not a current end-to-end application path:'
reject_literal "$readme" 'Wesley generates type-safe helpers, codecs, registry metadata, and host adapters.'

require_literal "$jim_case" 'produce an opaque'
require_literal "$jim_case" '`ProviderContractPackageProposalV1`. The proposal retains generated registry'
require_literal "$jim_case" '`AdmittedProviderContractPackageV1`'
require_literal "$jim_case" 'A later provider-native installation crossing must'
require_literal "$jim_case" 'before execution authority exists.'
reject_literal "$jim_case" 'host.register_contract_package(jim_package)?;'
reject_literal "$jim_case" 'Let the trusted host verify, bind, and register the package'

echo "generated-rule-truth: current Wesley, Edict, and feature boundaries are explicit"
