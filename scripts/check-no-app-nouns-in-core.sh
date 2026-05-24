#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

FORBIDDEN_PATTERNS=(
  'Stack Witness'
  'createBuffer'
  'replaceRange'
  'textWindow'
  'TextBufferOptic'
  'jedit'
)

CORE_SOURCE_DIRS=(
  "$ROOT_DIR/crates/warp-core/src"
  "$ROOT_DIR/crates/warp-wasm/src"
  "$ROOT_DIR/crates/echo-wasm-abi/src"
)

# Scope is intentional: production core source must stay generic. Tests and
# docs may still carry app-shaped fixtures as external-consumer examples.
matches=0
for pattern in "${FORBIDDEN_PATTERNS[@]}"; do
  if rg -n --fixed-strings "$pattern" "${CORE_SOURCE_DIRS[@]}"; then
    matches=1
  fi
done

if [[ "$matches" -ne 0 ]]; then
  echo "Echo production core source contains app-specific fixture nouns." >&2
  echo "Keep app nouns in authored contracts, generated adapters, or app repos." >&2
  exit 1
fi
