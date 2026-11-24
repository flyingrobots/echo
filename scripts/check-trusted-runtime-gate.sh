#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#
# Feature-matrix and reachability check for the `trusted_runtime` gate.
#
# 1. Builds warp-core with neither feature, `trusted_runtime` alone, and both.
# 2. Compiles an external consumer against `trusted_runtime` alone. The host,
#    WAL and recovery types must be reachable. Every native-rule registration
#    entry point must be unreachable. Each probe is also compiled under both
#    features, so a probe that fails only because it is wrong cannot pass.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
cd "$ROOT"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"

for features in "" "trusted_runtime" "native_rule_bootstrap,trusted_runtime"; do
  echo "== build warp-core features=[${features}]"
  cargo check -p warp-core --lib --no-default-features ${features:+--features "$features"}
done

mkdir -p "$work/probe/src/bin"
write_manifest() {
  cat >"$work/probe/Cargo.toml" <<MANIFEST
[package]
name = "trusted-runtime-gate-probe"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
warp-core = { path = "$ROOT/crates/warp-core", default-features = false, features = [$1] }

[workspace]
MANIFEST
}

# name|body: the body must compile when the symbol is reachable.
host_probes=(
  "host|use warp_core::{TrustedRuntimeHost, TrustedRuntimeHostError};"
  "wal|use warp_core::{TrustedRuntimeWal, TrustedRuntimeWalConfig, TrustedRuntimeWalError, TrustedRuntimeWalRecovery};"
)
native_probes=(
  "engine_register_rule|fn _p() { let _ = warp_core::Engine::register_rule; }"
  "engine_register_contract_package|fn _p() { let _ = warp_core::Engine::register_contract_package; }"
  "engine_register_query_observer|fn _p() { let _ = warp_core::Engine::register_contract_query_observer; }"
  "host_register_contract_package|fn _p() { let _ = warp_core::TrustedRuntimeHost::register_contract_package; }"
  "runtime_ingest_installed_contract|fn _p() { let _ = warp_core::WorldlineRuntime::ingest_installed_contract_invocation; }"
  "rule_types|use warp_core::{PatternGraph, RewriteRule, RuleExecutor};"
  "build_engine|use warp_core::build_engine;"
)

probe_builds() { # <features> <name> <body>
  write_manifest "$(printf '"%s"' "${1//,/\",\"}")"
  printf '%s\nfn main() {}\n' "$3" >"$work/probe/src/bin/$2.rs"
  (cd "$work/probe" && cargo check --quiet --bin "$2" 2>"$work/err.txt")
}

fail=0
for entry in "${host_probes[@]}"; do
  name="${entry%%|*}"; body="${entry#*|}"
  if probe_builds "trusted_runtime" "$name" "$body"; then
    echo "ok   $name reachable under trusted_runtime alone"
  else
    echo "FAIL $name must be reachable under trusted_runtime alone"; cat "$work/err.txt"; fail=1
  fi
done
for entry in "${native_probes[@]}"; do
  name="${entry%%|*}"; body="${entry#*|}"
  if ! probe_builds "native_rule_bootstrap,trusted_runtime" "$name" "$body"; then
    echo "FAIL probe $name is invalid: it does not compile under both features"; cat "$work/err.txt"; fail=1
    continue
  fi
  if probe_builds "trusted_runtime" "$name" "$body"; then
    echo "FAIL $name is reachable under trusted_runtime alone"; fail=1
  else
    echo "ok   $name unreachable under trusted_runtime alone"
  fi
done
exit "$fail"
