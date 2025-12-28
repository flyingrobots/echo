#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"

workspace_manifest="${repo_root}/Cargo.toml"
if [[ ! -f "$workspace_manifest" ]]; then
  echo "Error: workspace manifest not found: $workspace_manifest" >&2
  exit 1
fi

toolchain_file="${repo_root}/rust-toolchain.toml"
if [[ ! -f "$toolchain_file" ]]; then
  echo "Error: rust toolchain file not found: $toolchain_file" >&2
  exit 1
fi

toolchain_channel="$(
  awk -F'"' '
    /^[[:space:]]*channel[[:space:]]*=[[:space:]]*"/ {
      print $2
      exit
    }
  ' "$toolchain_file"
)"

if [[ -z "$toolchain_channel" ]]; then
  echo "Error: unable to parse toolchain channel from $toolchain_file" >&2
  exit 1
fi

if [[ ! "$toolchain_channel" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Error: rust-toolchain.toml channel must be a pinned semver for this guard (got: $toolchain_channel)" >&2
  exit 1
fi

workspace_package_rust_version="$(
  awk '
    BEGIN { in_section = 0 }
    /^[[:space:]]*\[workspace\.package\][[:space:]]*$/ { in_section = 1; next }
    in_section && /^[[:space:]]*\[[^]]+][[:space:]]*$/ { in_section = 0 }
    in_section && /^[[:space:]]*rust-version[[:space:]]*=/ {
      if (match($0, /"[^"]+"/)) {
        print substr($0, RSTART + 1, RLENGTH - 2)
        exit
      }
    }
  ' "$workspace_manifest"
)"

if [[ -n "$workspace_package_rust_version" && "$workspace_package_rust_version" != "$toolchain_channel" ]]; then
  echo "Error: [workspace.package] rust-version ($workspace_package_rust_version) does not match rust-toolchain.toml channel ($toolchain_channel)" >&2
  exit 1
fi

manifests=()
# Scan manifests recursively under crates/ and specs/ so nested workspace members
# cannot be silently skipped.
for root in "${repo_root}/crates" "${repo_root}/specs"; do
  if [[ -d "$root" ]]; then
    while IFS= read -r f; do
      manifests+=("$f")
    done < <(find "$root" -name Cargo.toml -print | sort)
  fi
done

if [[ "${#manifests[@]}" -eq 0 ]]; then
  echo "Error: no workspace manifests found under crates/ or specs/" >&2
  exit 1
fi

missing=()
mismatch=()
versions=()

for manifest in "${manifests[@]}"; do
  rel="${manifest#"${repo_root}/"}"

  rust_version="$(
    awk '
      /^[[:space:]]*rust-version[[:space:]]*=[[:space:]]*"/ {
        # Extract the first quoted value. This remains correct even when the
        # line has an inline comment containing additional quotes.
        if (match($0, /"[^"]+"/)) {
          print substr($0, RSTART + 1, RLENGTH - 2)
          exit
        }
      }
    ' "$manifest"
  )"

  if [[ -z "$rust_version" ]]; then
    uses_workspace="$(
      awk '
        /^[[:space:]]*rust-version\.workspace[[:space:]]*=[[:space:]]*true/ {
          print "yes"
          exit
        }
      ' "$manifest"
    )"

    if [[ -n "$uses_workspace" ]]; then
      if [[ -z "$workspace_package_rust_version" ]]; then
        missing+=("$rel (rust-version.workspace=true but [workspace.package] rust-version is missing)")
        continue
      fi

      rust_version="$workspace_package_rust_version"
    else
      missing+=("$rel")
      continue
    fi
  fi

  versions+=("$rust_version")

  if [[ "$rust_version" != "$toolchain_channel" ]]; then
    mismatch+=("$rel: $rust_version")
  fi
done

if [[ "${#missing[@]}" -ne 0 ]]; then
  echo "Error: rust-version missing from ${#missing[@]} workspace manifests:" >&2
  printf '  - %s\n' "${missing[@]}" >&2
  exit 1
fi

if [[ "${#mismatch[@]}" -ne 0 ]]; then
  echo "Error: rust-version mismatch vs rust-toolchain.toml channel ($toolchain_channel):" >&2
  printf '  - %s\n' "${mismatch[@]}" >&2
  echo "Hint: keep all workspace members aligned to avoid accidental use of newer language features in local builds." >&2
  exit 1
fi

unique_versions="$(printf '%s\n' "${versions[@]}" | sort -u)"
echo "OK: workspace rust-version matches rust-toolchain.toml ($toolchain_channel)"
while IFS= read -r version; do
  printf '  - %s\n' "$version"
done <<<"$unique_versions"
