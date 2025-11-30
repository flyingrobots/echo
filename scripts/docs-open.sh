#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
set -euo pipefail
URL="${1:-http://localhost:5173/}"
case "${OSTYPE:-}" in
  darwin*) open "$URL" ;;
  linux*)  xdg-open "$URL" >/dev/null 2>&1 || echo "Open: $URL" ;;
  msys*|cygwin*|win32*) cmd.exe /c start "" "$URL" ;;
  *) echo "Open: $URL" ;;
esac

