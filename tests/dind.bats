#!/usr/bin/env bats

setup() {
    ECHO_ROOT="$(cd "$(dirname "$BATS_TEST_FILENAME")/.." && pwd)"
    # Build the harness once (or ensure it's built)
    (cd "$ECHO_ROOT" && cargo build -p echo-dind-harness --quiet)
    
    # Path to the binary
    export HARNESS="$ECHO_ROOT/target/debug/echo-dind-harness"
    
    # Test data paths
    export DATA_DIR="$ECHO_ROOT/testdata/dind"
    export OUT_DIR="test-results/dind/bats"
    mkdir -p "$OUT_DIR"
}

@test "DIND: Smoke Test (Dense Rewrite) - Deterministic Run" {
    run $HARNESS run "$DATA_DIR/010_dense_rewrite_seed0001.eintlog" \
        --golden "$DATA_DIR/010_dense_rewrite_seed0001.hashes.json"
    
    [ "$status" -eq 0 ]
    [[ "$output" =~ "DIND: OK" ]]
}

@test "DIND: Error Determinism - Stable Failure Modes" {
    run $HARNESS run "$DATA_DIR/030_error_determinism.eintlog" \
        --golden "$DATA_DIR/030_error_determinism.hashes.json"
        
    [ "$status" -eq 0 ]
    [[ "$output" =~ "DIND: OK" ]]
}

@test "DIND: Torture Mode (Short) - 5 Runs" {
    run $HARNESS torture "$DATA_DIR/010_dense_rewrite_seed0001.eintlog" --runs 5
    
    [ "$status" -eq 0 ]
    [[ "$output" =~ "Torture complete. 5 runs identical." ]]
}

@test "DIND: Repro Bundle Generation on Failure" {
    # Create a fake golden file with a bad hash to force failure
    local fake_golden="$OUT_DIR/bad_golden.json"
    cp "$DATA_DIR/010_dense_rewrite_seed0001.hashes.json" "$fake_golden"
    # Sed replacement to corrupt the first hash (works on GNU and BSD sed)
    sed -e 's/e0c2acac/deadbeef/g' "$fake_golden" > "$fake_golden.tmp" && mv "$fake_golden.tmp" "$fake_golden"
    
    local repro_dir="$OUT_DIR/repro_test"
    rm -rf "$repro_dir"
    
    run $HARNESS run "$DATA_DIR/010_dense_rewrite_seed0001.eintlog" \
        --golden "$fake_golden" \
        --emit-repro "$repro_dir"
        
    [ "$status" -eq 1 ]
    [[ "$output" =~ "Repro bundle emitted" ]]
    
    # Verify artifacts exist
    [ -f "$repro_dir/scenario.eintlog" ]
    [ -f "$repro_dir/actual.hashes.json" ]
    [ -f "$repro_dir/expected.hashes.json" ]
    [ -f "$repro_dir/diff.txt" ]
}
