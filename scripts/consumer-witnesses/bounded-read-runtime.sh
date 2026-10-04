#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
set -euo pipefail

[[ -f /.dockerenv ]] || { echo 'Use the COPY-based bounded-read-publication Dockerfile' >&2; exit 1; }
echo_root="${ECHO_READ_WORKSPACE:-/echo}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/read-build}"
export CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=4
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0

# One worker/target, one sequential campaign. This is a measured soft budget,
# not a filesystem quota. The operator also checks host free space before use.
exec 9>/tmp/echo-validation.lock
flock -n 9 || { echo 'Echo validation worker is busy' >&2; exit 1; }
budget_check() {
    local used=0 path size available
    for path in "$CARGO_TARGET_DIR" "${CARGO_HOME:-/usr/local/cargo}/registry" "${CARGO_HOME:-/usr/local/cargo}/git"; do
        if [[ -d "$path" ]]; then
            size="$(du -sk "$path" | cut -f1)"
            used=$((used + size))
        fi
    done
    available="$(df -Pk "$echo_root" | awk 'NR == 2 {print $4}')"
    if (( used >= 20 * 1024 * 1024 || available < 50 * 1024 * 1024 )); then
        echo "Resource guard refused: cache=${used}KiB available=${available}KiB" >&2
        exit 1
    fi
    echo "READ_WITNESS_CACHE_KIB=$used FREE_KIB=$available"
}

[[ ! -e /read-provider && ! -e /read-evidence ]] || {
    echo 'Export previous witness evidence and remove its owned /read-provider and /read-evidence before rerunning.' >&2
    exit 1
}
budget_check
cd /edict
cargo +1.95.0 build --locked -p edict-cli
budget_check
cd "$echo_root"
cargo +1.90.0 run --locked -p echo-wesley-gen --example ordered_publication_witness -- /read-provider
budget_check
export EDICT_READ_COMPILER="$CARGO_TARGET_DIR/debug/edict"
python3 /bounded-read-publication.py
(( $(du -sk /read-evidence /read-provider | awk '{sum += $1} END {print sum}') < 1024 * 1024 )) || {
    echo 'Read witness evidence exceeded its 1 GiB budget' >&2; exit 1;
}
export EDICT_READ_OUTPUT_ROOT=/read-evidence
cargo +1.90.0 test --locked -p warp-core --features trusted_runtime --test edict_node_read_tests
budget_check
echo BOUNDED_READ_PUBLIC_COMPILER_TO_REAL_FRONTIER_ACCEPTED
