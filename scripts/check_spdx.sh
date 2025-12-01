#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

set -euo pipefail

# Wrapper to check SPDX compliance on all files (e.g. for CI/pre-push)
# or specific files if arguments are passed.
#
# Flags:
#   --repair   : Run ensure_spdx.sh in repair mode (no --check)
#   [files...] : Check/repair specific files
#   (default)  : Check all files

ROOT=$(git rev-parse --show-toplevel)
SCRIPT="$ROOT/scripts/ensure_spdx.sh"

if [[ ! -x "$SCRIPT" ]]; then
  chmod +x "$SCRIPT"
fi

REPAIR_MODE=0
ARGS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repair) REPAIR_MODE=1; shift ;;
    *) ARGS+=("$1"); shift ;;
  esac
done

if [[ "$REPAIR_MODE" -eq 1 ]]; then
  if [[ ${#ARGS[@]} -gt 0 ]]; then
    "$SCRIPT" --all "${ARGS[@]}"
  else
    "$SCRIPT" --all
  fi
else
  if [[ ${#ARGS[@]} -gt 0 ]]; then
    "$SCRIPT" --check "${ARGS[@]}"
  else
    "$SCRIPT" --check --all
  fi
fi
