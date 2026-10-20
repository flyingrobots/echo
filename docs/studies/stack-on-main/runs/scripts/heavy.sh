#!/bin/bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
# Usage: heavy.sh NOTE DEADLINE_S -- cmd...  Holds host/heavy-work and runs cmd under the 15 GiB guard.
S="$(cd "$(dirname "$0")" && pwd)"; NOTE="$1"; DL="$2"; shift 3
GIT_LOCKS_STORE=/Users/Shared/git-locks/workstation.git git locks with --job "agent-$(uuidgen)" --holder claude-echo-2df23d48 \
  --ttl $((DL+600)) --wait 1800 --note "Claude M0/E1: $NOTE (guard 15GiB, scratch $S)" host/heavy-work -- \
  python3 -I "$S/guard.py" "$S" 15 "$DL" "$@"
