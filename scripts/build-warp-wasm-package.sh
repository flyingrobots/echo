#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
WARP_WASM_DIR="${WARP_WASM_DIR:-${REPO_ROOT}/crates/warp-wasm}"

if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "error: wasm-pack is required to build crates/warp-wasm/pkg" >&2
  exit 127
fi

cd "${WARP_WASM_DIR}"
wasm-pack build --target bundler --out-dir pkg --out-name rmg_wasm -- --features engine
