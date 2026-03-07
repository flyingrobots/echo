#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
# Determinism repro script: run DIND torture with 100 reruns per scenario.
#
# Usage:
#   scripts/torture-100-reruns.sh                    # all PR-tagged scenarios
#   scripts/torture-100-reruns.sh --tags smoke       # only smoke scenarios
#   scripts/torture-100-reruns.sh --runs 200         # override run count
#
# Exit code 0 = all scenarios reproduced identically across all reruns.
# Exit code 1 = at least one divergence detected.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

RUNS=100
TAGS="pr"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --runs)  RUNS="$2"; shift 2 ;;
        --tags)  TAGS="$2"; shift 2 ;;
        *)       echo "Unknown arg: $1" >&2; exit 1 ;;
    esac
done

echo "=== DETERMINISM REPRO: ${RUNS} reruns per scenario (tags: ${TAGS}) ==="
echo ""

# Build the harness first (once).
cargo build -p echo-dind-harness --release --quiet 2>/dev/null || \
    cargo build -p echo-dind-harness --quiet

HARNESS="${PROJECT_ROOT}/target/release/echo-dind-harness"
if [[ ! -x "$HARNESS" ]]; then
    HARNESS="${PROJECT_ROOT}/target/debug/echo-dind-harness"
fi

MANIFEST="${PROJECT_ROOT}/testdata/dind/MANIFEST.json"
if [[ ! -f "$MANIFEST" ]]; then
    echo "ERROR: MANIFEST.json not found at ${MANIFEST}" >&2
    exit 1
fi

# Parse scenarios matching the requested tags from MANIFEST.json.
SCENARIOS=$(node -e "
    const m = require('${MANIFEST}');
    const tags = '${TAGS}'.split(',').map(t => t.trim());
    const hits = m.scenarios.filter(s =>
        tags.some(t => (s.tags || []).includes(t))
    );
    hits.forEach(s => console.log(s.file));
" 2>/dev/null)

if [[ -z "$SCENARIOS" ]]; then
    echo "No scenarios matched tags: ${TAGS}" >&2
    exit 1
fi

PASS=0
FAIL=0
TOTAL=0
RESULTS=""

for SCENARIO_FILE in $SCENARIOS; do
    SCENARIO_PATH="${PROJECT_ROOT}/testdata/dind/${SCENARIO_FILE}"
    if [[ ! -f "$SCENARIO_PATH" ]]; then
        echo "SKIP: ${SCENARIO_FILE} (file not found)"
        continue
    fi

    TOTAL=$((TOTAL + 1))
    echo -n "  ${SCENARIO_FILE} (${RUNS} runs)... "

    if "$HARNESS" torture "$SCENARIO_PATH" --runs "$RUNS" > /dev/null 2>&1; then
        echo "PASS"
        PASS=$((PASS + 1))
        RESULTS="${RESULTS}\n  PASS  ${SCENARIO_FILE}"
    else
        echo "FAIL (divergence detected)"
        FAIL=$((FAIL + 1))
        RESULTS="${RESULTS}\n  FAIL  ${SCENARIO_FILE}"
    fi
done

echo ""
echo "=== RESULTS: ${PASS}/${TOTAL} passed, ${FAIL} failed ==="
echo -e "$RESULTS"
echo ""

if [[ "$FAIL" -gt 0 ]]; then
    echo "DETERMINISM REPRO FAILED: ${FAIL} scenario(s) diverged in ${RUNS} reruns."
    exit 1
fi

# Produce a receipt hash of the full run for auditability.
RECEIPT=$(echo "${RUNS}|${TAGS}|${PASS}/${TOTAL}|$(date -u +%Y-%m-%dT%H:%M:%SZ)" | shasum -a 256 | cut -d' ' -f1)
echo "Receipt: sha256:${RECEIPT}"
echo "All ${TOTAL} scenarios reproduced identically across ${RUNS} reruns."
