#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

set -euo pipefail

# Wrapper to check SPDX compliance on all files (e.g. for CI/pre-push)
# or specific files if arguments are passed.

ROOT=$(git rev-parse --show-toplevel)
SCRIPT="$ROOT/scripts/ensure_spdx.sh"

if [[ ! -x "$SCRIPT" ]]; then
  chmod +x "$SCRIPT"
fi

if [[ $# -gt 0 ]]; then
  "$SCRIPT" --check "$@"
else
  "$SCRIPT" --check --all
fi