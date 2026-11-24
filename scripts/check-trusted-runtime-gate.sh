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
# The probes build from a temporary directory outside the repository, which
# loses `rust-toolchain.toml`. Resolve the pinned toolchain here and carry it.
if [ -z "${RUSTUP_TOOLCHAIN:-}" ] && command -v rustup >/dev/null 2>&1; then
  RUSTUP_TOOLCHAIN="$(rustup show active-toolchain | awk 'NR==1 {print $1}')"
  export RUSTUP_TOOLCHAIN
fi

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
# Intended to be exhaustive over every public item that is gated on
# `native_rule_bootstrap`, alone or jointly with `trusted_runtime`. To extend it
# after adding such an item, list it with `grep -rn native_rule_bootstrap
# crates/warp-core/src`, add one line here, and confirm the script passes.
# `ContractQueryObserver`, `InstalledContractPackage` and
# `ProviderContractPackageInstallerV1` are deliberately not probed: they are
# exported without a native gate and are inert data or sealed-trait types. Every
# call that registers or installs one is gated and probed above.
native_probes=(
  "engine_register_rule|fn _p() { let _ = warp_core::Engine::register_rule; }"
  "engine_register_contract_package|fn _p() { let _ = warp_core::Engine::register_contract_package; }"
  "engine_register_query_observer|fn _p() { let _ = warp_core::Engine::register_contract_query_observer; }"
  "host_register_contract_package|fn _p() { let _ = warp_core::TrustedRuntimeHost::register_contract_package; }"
  "runtime_ingest_installed_contract|fn _p() { let _ = warp_core::WorldlineRuntime::ingest_installed_contract_invocation; }"
  "rule_type_pattern_graph|use warp_core::PatternGraph;"
  "rule_type_rewrite_rule|use warp_core::RewriteRule;"
  "rule_type_rule_executor|use warp_core::RuleExecutor;"
  "build_engine|use warp_core::build_engine;"
  "engine_installed_contract_mutation_package_id|fn _p() { let _ = warp_core::Engine::installed_contract_mutation_package_id; }"
  "engine_installed_contract_mutation_evidence|fn _p() { let _ = warp_core::Engine::installed_contract_mutation_evidence; }"
  "engine_installed_contract_query_package_id|fn _p() { let _ = warp_core::Engine::installed_contract_query_package_id; }"
  "engine_installed_provider_mutation_package_id|fn _p() { let _ = warp_core::Engine::installed_provider_contract_mutation_package_id; }"
  "engine_installed_provider_mutation_evidence|fn _p() { let _ = warp_core::Engine::installed_provider_contract_mutation_evidence_v1; }"
  "engine_installed_provider_package|fn _p() { let _ = warp_core::Engine::installed_provider_contract_package; }"
  "engine_installed_provider_package_by_reference|fn _p() { let _ = warp_core::Engine::installed_provider_contract_package_by_reference; }"
  "runtime_ingest_provider_contract|fn _p() { let _ = warp_core::WorldlineRuntime::ingest_provider_contract_invocation_v1; }"
  "host_admit_provider_package|fn _p() { let _ = warp_core::TrustedRuntimeHost::admit_provider_contract_package_v1; }"
  "host_install_provider_package|fn _p() { let _ = warp_core::TrustedRuntimeHost::install_admitted_provider_contract_package_v1_trusted; }"
  "host_stage_installed_submission|fn _p() { let _ = warp_core::TrustedRuntimeHost::stage_installed_contract_submission; }"
  "host_admit_installed_submission|fn _p() { let _ = warp_core::TrustedRuntimeHost::admit_installed_contract_submission; }"
  "host_stage_provider_submission|fn _p() { let _ = warp_core::TrustedRuntimeHost::stage_provider_contract_submission_v1; }"
  "host_admit_provider_submission|fn _p() { let _ = warp_core::TrustedRuntimeHost::admit_provider_contract_submission_v1; }"
  "app_submit_contract_inverse|fn _p() { let _ = warp_core::TrustedRuntimeApp::submit_contract_inverse_with_runtime_wal_ack; }"
  "type_echo_config|use warp_core::EchoConfig;"
  "fn_run_pair_determinism|use warp_core::run_pair_determinism;"
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
