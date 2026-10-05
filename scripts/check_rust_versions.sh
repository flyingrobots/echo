#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
workspace_manifest="$repo_root/Cargo.toml"
toolchain_file="$repo_root/rust-toolchain.toml"
policy_file="$repo_root/scripts/rust-msrv-policy.tsv"

fail() {
  echo "Error: $*" >&2
  exit 1
}

[[ -f "$workspace_manifest" ]] || fail "workspace manifest missing: $workspace_manifest"
[[ -f "$toolchain_file" ]] || fail "toolchain file missing: $toolchain_file"
[[ -f "$policy_file" ]] || fail "MSRV policy file missing: $policy_file"

# This validates reviewed declarations, not the dependency graph or compiler
# compatibility. A path-to-version inventory prevents blanket MSRV exceptions.
# Bash indexed arrays keep this guard usable with macOS's system Bash as well.
policy_keys=()
policy_versions=()
policy_count=0
while read -r key version extra || [[ -n "$key$version$extra" ]]; do
  [[ -z "$key" || "$key" == \#* ]] && continue
  [[ -n "$version" && -z "$extra" ]] || fail "malformed MSRV policy row: $key"
  [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "invalid MSRV policy version: $key $version"
  case "$key" in
    toolchain|workspace|xtask/Cargo.toml|tests/edict-provider-host-v1/Cargo.toml) ;;
    crates/*/Cargo.toml|specs/*/Cargo.toml)
      [[ "$key" != *"/../"* && "$key" != *"/./"* && "$key" != *"//"* ]] || fail "invalid MSRV policy path: $key"
      ;;
    *) fail "unknown MSRV policy key: $key" ;;
  esac
  for ((previous_index = 0; previous_index < policy_count; previous_index++)); do
    [[ "${policy_keys[$previous_index]}" != "$key" ]] || fail "duplicate MSRV policy entry: $key"
  done
  policy_keys[$policy_count]="$key"
  policy_versions[$policy_count]="$version"
  policy_count=$((policy_count + 1))
done < "$policy_file"

policy_value() {
  local wanted="$1" index
  for ((index = 0; index < policy_count; index++)); do
    if [[ "${policy_keys[$index]}" == "$wanted" ]]; then
      printf '%s\n' "${policy_versions[$index]}"
      return 0
    fi
  done
  return 1
}

expected_toolchain="$(policy_value toolchain)" || fail "toolchain missing from MSRV policy"
expected_workspace="$(policy_value workspace)" || fail "workspace default missing from MSRV policy"

read_toolchain_channel() {
  awk -F'"' '
    /^[[:space:]]*\[/ { in_toolchain = ($0 ~ /^[[:space:]]*\[toolchain\][[:space:]]*(#.*)?$/) }
    in_toolchain && /^[[:space:]]*channel[[:space:]]*=[[:space:]]*"/ { print $2; exit }
  ' "$1"
}

toolchain_channel="$(read_toolchain_channel "$toolchain_file")"
[[ "$toolchain_channel" == "$expected_toolchain" ]] || fail "toolchain does not match MSRV policy ($toolchain_channel != $expected_toolchain)"

workspace_version="$(awk '
  BEGIN { in_section = 0 }
  /^[[:space:]]*\[workspace\.package\][[:space:]]*$/ { in_section = 1; next }
  in_section && /^[[:space:]]*\[[^]]+][[:space:]]*$/ { in_section = 0 }
  in_section && /^[[:space:]]*rust-version[[:space:]]*=/ {
    if (match($0, /"[^"]+"/)) {
      print substr($0, RSTART + 1, RLENGTH - 2)
      exit
    }
  }
' "$workspace_manifest")"
[[ "$workspace_version" == "$expected_workspace" ]] || fail "workspace default does not match MSRV policy ($workspace_version != $expected_workspace)"

for ((index = 0; index < policy_count; index++)); do
  key="${policy_keys[$index]}"
  version="${policy_versions[$index]}"
  [[ "$version" == "$expected_workspace" || "$version" == "$expected_toolchain" ]] || fail "unsupported MSRV policy version: $key $version"
  case "$key" in
    toolchain|workspace) continue ;;
  esac
  [[ -f "$repo_root/$key" ]] || fail "manifest missing for MSRV policy entry: $key"
done

manifests=()
manifest_count=0
# Include nested manifests, the build driver, and the independent host witness.
# Their presence must be accounted for explicitly in the policy inventory.
for root in "$repo_root/crates" "$repo_root/specs"; do
  if [[ -d "$root" ]]; then
    while IFS= read -r manifest; do
      manifests[$manifest_count]="$manifest"
      manifest_count=$((manifest_count + 1))
    done < <(find "$root" -name Cargo.toml -print | sort)
  fi
done
for path in xtask/Cargo.toml tests/edict-provider-host-v1/Cargo.toml; do
  if [[ -f "$repo_root/$path" ]]; then
    manifests[$manifest_count]="$repo_root/$path"
    manifest_count=$((manifest_count + 1))
  fi
done
[[ $manifest_count -gt 0 ]] || fail "no package manifests found"

for manifest in "${manifests[@]}"; do
  relative="${manifest#"$repo_root/"}"
  expected="$(policy_value "$relative")" || fail "unregistered manifest in MSRV policy: $relative"
  if [[ "$relative" == tests/edict-provider-host-v1/Cargo.toml ]]; then
    nested_toolchain="${manifest%Cargo.toml}rust-toolchain.toml"
    [[ -f "$nested_toolchain" ]] || fail "toolchain file missing: $nested_toolchain"
    nested_channel="$(read_toolchain_channel "$nested_toolchain")"
    [[ "$nested_channel" == "$expected" ]] || fail "toolchain does not match MSRV policy: $nested_toolchain ($nested_channel != $expected)"
  fi
  version="$(awk '
    /^[[:space:]]*\[/ { in_package = ($0 ~ /^[[:space:]]*\[package\][[:space:]]*(#.*)?$/) }
    in_package && /^[[:space:]]*rust-version[[:space:]]*=[[:space:]]*"/ {
      if (match($0, /"[^"]+"/)) {
        print substr($0, RSTART + 1, RLENGTH - 2)
        exit
      }
    }
  ' "$manifest")"
  if [[ -z "$version" ]]; then
    inherited="$(awk '
      /^[[:space:]]*\[/ { in_package = ($0 ~ /^[[:space:]]*\[package\][[:space:]]*(#.*)?$/) }
      in_package && /^[[:space:]]*rust-version\.workspace[[:space:]]*=[[:space:]]*true/ { print "yes"; exit }
    ' "$manifest")"
    [[ -n "$inherited" ]] || fail "rust-version missing: $relative"
    version="$workspace_version"
  fi
  [[ "$version" == "$expected" ]] || fail "rust-version mismatch: $relative declares $version; policy requires $expected"
done

printf 'OK: %s package MSRVs match explicit policy; general toolchain %s, workspace default %s\n' \
  "$manifest_count" "$expected_toolchain" "$expected_workspace"
