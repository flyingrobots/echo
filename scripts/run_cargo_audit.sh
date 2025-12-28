#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

set -euo pipefail

# Single source of truth for cargo-audit policy flags.
# Keep in sync with deny.toml advisory ignores (when applicable).

cargo audit --deny warnings \
  --ignore RUSTSEC-2024-0436 \
  --ignore RUSTSEC-2024-0370 \
  --ignore RUSTSEC-2021-0127
