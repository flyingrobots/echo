#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.." || exit 1

if ! rg -q -- '- "!docs/archive/\*\*"' .coderabbit.yaml; then
  echo "CodeRabbit must ignore archived documentation: missing !docs/archive/** path filter" >&2
  exit 1
fi
