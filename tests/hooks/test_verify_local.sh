#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.." || exit 1

PASS=0
FAIL=0

pass() {
  echo "  PASS: $1"
  PASS=$((PASS + 1))
}

fail() {
  echo "  FAIL: $1"
  FAIL=$((FAIL + 1))
}

run_detect() {
  local tmp
  tmp="$(mktemp)"
  printf '%s\n' "$@" >"$tmp"
  VERIFY_CHANGED_FILES_FILE="$tmp" scripts/verify-local.sh detect
  rm -f "$tmp"
}

run_detect_pre_commit() {
  local tmp
  tmp="$(mktemp)"
  printf '%s\n' "$@" >"$tmp"
  VERIFY_CHANGED_FILES_FILE="$tmp" VERIFY_STAMP_SUBJECT="test-index-tree" scripts/verify-local.sh detect-pre-commit
  rm -f "$tmp"
}

run_fixture_detect() {
  local mode="$1"
  local tmp
  tmp="$(mktemp -d)"

  mkdir -p "$tmp/scripts" "$tmp/crates/warp-core/src"
  cp scripts/verify-local.sh "$tmp/scripts/verify-local.sh"
  chmod +x "$tmp/scripts/verify-local.sh"
  cat >"$tmp/rust-toolchain.toml" <<'EOF'
[toolchain]
channel = "1.90.0"
EOF

  (
    cd "$tmp"
    git init -q
    git config user.name "verify-local-test"
    git config user.email "verify-local-test@example.com"
    git config commit.gpgsign false
    git branch -M main
    printf '%s\n' 'pub fn anchor() {}' > crates/warp-core/src/lib.rs
    git add rust-toolchain.toml crates/warp-core/src/lib.rs
    git commit -qm "base"

    case "$mode" in
      branch-delete)
        git checkout -qb feat/delete
        git rm -q crates/warp-core/src/lib.rs
        git commit -qm "delete critical file"
        ./scripts/verify-local.sh detect
        ;;
      pre-commit-delete)
        git checkout -qb feat/delete
        git rm -q crates/warp-core/src/lib.rs
        ./scripts/verify-local.sh detect-pre-commit
        ;;
      *)
        echo "unknown fixture mode: $mode" >&2
        exit 1
        ;;
    esac
  )

  rm -rf "$tmp"
}

run_fake_verify() {
  local mode="$1"
  local changed_file="$2"
  local lane_mode="${3:-parallel}"
  local use_nextest="${4:-0}"
  local tmp
  tmp="$(mktemp -d)"

  mkdir -p "$tmp/scripts/hooks" "$tmp/bin" "$tmp/.git" "$tmp/.githooks" "$tmp/tests/hooks"
  mkdir -p "$tmp/crates/warp-core/src" "$tmp/crates/bin-only/src"
  cp scripts/verify-local.sh "$tmp/scripts/verify-local.sh"
  chmod +x "$tmp/scripts/verify-local.sh"

  cat >"$tmp/rust-toolchain.toml" <<'EOF'
[toolchain]
channel = "1.90.0"
EOF

  cat >"$tmp/crates/warp-core/Cargo.toml" <<'EOF'
[package]
name = "warp-core"
version = "0.0.0"
edition = "2021"
EOF
  printf '%s\n' 'pub fn anchor() {}' >"$tmp/crates/warp-core/src/lib.rs"

  cat >"$tmp/crates/bin-only/Cargo.toml" <<'EOF'
[package]
name = "bin-only"
version = "0.0.0"
edition = "2021"
EOF
  cat >"$tmp/crates/bin-only/src/main.rs" <<'EOF'
fn main() {}
EOF

  cat >"$tmp/bin/cargo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s|%s\n' "${CARGO_TARGET_DIR:-}" "$*" >>"${VERIFY_FAKE_CARGO_LOG}"
exit 0
EOF
  cat >"$tmp/bin/cargo-nextest" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exit 0
EOF
  cat >"$tmp/bin/rustup" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "toolchain" && "${2:-}" == "list" ]]; then
  printf '1.90.0-aarch64-apple-darwin (default)\n'
  exit 0
fi
exit 0
EOF
  cat >"$tmp/bin/rg" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exit 1
EOF
  cat >"$tmp/bin/npx" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exit 0
EOF
  cat >"$tmp/bin/git" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "rev-parse" && "${2:-}" == "HEAD" ]]; then
  printf '%s\n' "${VERIFY_FAKE_GIT_HEAD:-test-head}"
  exit 0
fi
if [[ "${1:-}" == "rev-parse" && "${2:-}" == "HEAD^{tree}" ]]; then
  printf '%s\n' "${VERIFY_FAKE_GIT_TREE:-test-tree}"
  exit 0
fi
if [[ "${1:-}" == "write-tree" ]]; then
  printf '%s\n' "${VERIFY_FAKE_GIT_TREE:-test-tree}"
  exit 0
fi
if [[ "${1:-}" == "rev-parse" && "${2:-}" == "--short" && "${3:-}" == "HEAD" ]]; then
  printf '%.12s\n' "${VERIFY_FAKE_GIT_HEAD:-test-head}"
  exit 0
fi
exit 0
EOF
  chmod +x "$tmp/bin/cargo" "$tmp/bin/cargo-nextest" "$tmp/bin/rustup" "$tmp/bin/rg" "$tmp/bin/npx" "$tmp/bin/git"

  cat >"$tmp/tests/hooks/test_verify_local.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "fake hook coverage"
EOF
  cat >"$tmp/.githooks/pre-push" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "fake canonical pre-push"
EOF
  cat >"$tmp/scripts/hooks/pre-commit" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "fake legacy pre-commit shim"
EOF
  chmod +x "$tmp/tests/hooks/test_verify_local.sh" "$tmp/.githooks/pre-push" "$tmp/scripts/hooks/pre-commit"

  local changed
  changed="$(mktemp)"
  printf '%s\n' "$changed_file" >"$changed"
  local cargo_log
  cargo_log="$(mktemp)"

  local output
  output="$(
    cd "$tmp" && \
    PATH="$tmp/bin:$PATH" \
    VERIFY_FORCE=1 \
    VERIFY_USE_NEXTEST="$use_nextest" \
    VERIFY_LANE_MODE="$lane_mode" \
    VERIFY_STAMP_SUBJECT="test-head" \
    VERIFY_CHANGED_FILES_FILE="$changed" \
    VERIFY_FAKE_CARGO_LOG="$cargo_log" \
    ./scripts/verify-local.sh "$mode"
  )"

  printf '%s\n' "$output"
  echo "--- cargo-log ---"
  cat "$cargo_log"

  rm -f "$changed" "$cargo_log"
  rm -rf "$tmp"
}

run_fake_full_stamp_sequence() {
  local tmp
  tmp="$(mktemp -d)"

  mkdir -p "$tmp/scripts/hooks" "$tmp/bin" "$tmp/.git" "$tmp/.githooks" "$tmp/tests/hooks"
  cp scripts/verify-local.sh "$tmp/scripts/verify-local.sh"
  chmod +x "$tmp/scripts/verify-local.sh"

  cat >"$tmp/rust-toolchain.toml" <<'EOF'
[toolchain]
channel = "1.90.0"
EOF

  cat >"$tmp/bin/cargo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s|%s\n' "${CARGO_TARGET_DIR:-}" "$*" >>"${VERIFY_FAKE_CARGO_LOG}"
exit 0
EOF
  cat >"$tmp/bin/rustup" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "toolchain" && "${2:-}" == "list" ]]; then
  printf '1.90.0-aarch64-apple-darwin (default)\n'
  exit 0
fi
exit 0
EOF
  cat >"$tmp/bin/rg" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exit 1
EOF
  cat >"$tmp/bin/npx" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exit 0
EOF
  cat >"$tmp/bin/git" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "rev-parse" && "${2:-}" == "HEAD" ]]; then
  printf '%s\n' "${VERIFY_FAKE_GIT_HEAD:-test-head}"
  exit 0
fi
if [[ "${1:-}" == "rev-parse" && "${2:-}" == "HEAD^{tree}" ]]; then
  printf '%s\n' "${VERIFY_FAKE_GIT_TREE:-test-tree}"
  exit 0
fi
if [[ "${1:-}" == "rev-parse" && "${2:-}" == "--short" && "${3:-}" == "HEAD" ]]; then
  printf '%.12s\n' "${VERIFY_FAKE_GIT_HEAD:-test-head}"
  exit 0
fi
exit 0
EOF
  chmod +x "$tmp/bin/cargo" "$tmp/bin/rustup" "$tmp/bin/rg" "$tmp/bin/npx" "$tmp/bin/git"

  cat >"$tmp/tests/hooks/test_verify_local.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "fake hook coverage"
EOF
  chmod +x "$tmp/tests/hooks/test_verify_local.sh"

  local changed
  changed="$(mktemp)"
  printf '%s\n' 'scripts/verify-local.sh' >"$changed"
  local cargo_log
  cargo_log="$(mktemp)"

  local first_output second_output third_output
  first_output="$(
    cd "$tmp" && \
    PATH="$tmp/bin:$PATH" \
    VERIFY_CHANGED_FILES_FILE="$changed" \
    VERIFY_FAKE_CARGO_LOG="$cargo_log" \
    VERIFY_FAKE_GIT_HEAD="commit-a" \
    VERIFY_FAKE_GIT_TREE="tree-aaaaaaaaaaaa" \
    ./scripts/verify-local.sh full
  )"
  second_output="$(
    cd "$tmp" && \
    PATH="$tmp/bin:$PATH" \
    VERIFY_CHANGED_FILES_FILE="$changed" \
    VERIFY_FAKE_CARGO_LOG="$cargo_log" \
    VERIFY_FAKE_GIT_HEAD="commit-b" \
    VERIFY_FAKE_GIT_TREE="tree-aaaaaaaaaaaa" \
    ./scripts/verify-local.sh full
  )"
  third_output="$(
    cd "$tmp" && \
    PATH="$tmp/bin:$PATH" \
    VERIFY_CHANGED_FILES_FILE="$changed" \
    VERIFY_FAKE_CARGO_LOG="$cargo_log" \
    VERIFY_FAKE_GIT_HEAD="commit-c" \
    VERIFY_FAKE_GIT_TREE="tree-bbbbbbbbbbbb" \
    ./scripts/verify-local.sh full
  )"

  printf '%s\n' "$first_output"
  echo "--- second ---"
  printf '%s\n' "$second_output"
  echo "--- third ---"
  printf '%s\n' "$third_output"
  echo "--- cargo-log ---"
  cat "$cargo_log"

  rm -f "$changed" "$cargo_log"
  rm -rf "$tmp"
}

echo "=== verify-local classification ==="

docs_output="$(run_detect docs/plans/adr-0008-and-0009.md docs/ROADMAP/backlog/tooling-misc.md)"
if printf '%s\n' "$docs_output" | grep -q '^classification=docs$'; then
  pass "docs-only changes stay in docs mode"
else
  fail "docs-only changes should classify as docs"
  printf '%s\n' "$docs_output"
fi
if printf '%s\n' "$docs_output" | grep -q '^stamp_suite=docs$'; then
  pass "docs-only changes use the shared docs stamp suite"
else
  fail "docs-only changes should map to the docs stamp suite"
  printf '%s\n' "$docs_output"
fi

reduced_output="$(run_detect \
  crates/warp-cli/src/main.rs \
  crates/warp-cli/src/main.rs \
  crates/echo-app-core/src/lib.rs \
)"
if printf '%s\n' "$reduced_output" | grep -q '^classification=reduced$'; then
  pass "non-critical crate changes use reduced mode"
else
  fail "non-critical crate changes should classify as reduced"
  printf '%s\n' "$reduced_output"
fi
if printf '%s\n' "$reduced_output" | grep -q '^stamp_suite=reduced$'; then
  pass "non-critical crate changes use the shared reduced stamp suite"
else
  fail "non-critical crate changes should map to the reduced stamp suite"
  printf '%s\n' "$reduced_output"
fi
if printf '%s\n' "$reduced_output" | grep -q '^changed_crates=echo-app-core,warp-cli$'; then
  pass "changed crate list is deduplicated and sorted"
else
  fail "changed crate list should be sorted and deduplicated"
  printf '%s\n' "$reduced_output"
fi

full_output="$(run_detect crates/warp-core/src/lib.rs)"
if printf '%s\n' "$full_output" | grep -q '^classification=full$'; then
  pass "warp-core changes force full verification"
else
  fail "warp-core changes should classify as full"
  printf '%s\n' "$full_output"
fi
if printf '%s\n' "$full_output" | grep -q '^stamp_suite=full$'; then
  pass "critical changes use the shared full stamp suite"
else
  fail "critical changes should map to the full stamp suite"
  printf '%s\n' "$full_output"
fi
if printf '%s\n' "$full_output" | grep -q '^stamp_context=full$'; then
  pass "full verification uses the shared full stamp context"
else
  fail "full verification should normalize to the shared full stamp context"
  printf '%s\n' "$full_output"
fi

workflow_output="$(run_detect .github/workflows/ci.yml)"
if printf '%s\n' "$workflow_output" | grep -q '^classification=full$'; then
  pass "workflow changes force full verification"
else
  fail "workflow changes should classify as full"
  printf '%s\n' "$workflow_output"
fi

exact_output="$(run_detect Cargo.toml)"
if printf '%s\n' "$exact_output" | grep -q '^classification=full$'; then
  pass "exact critical paths force full verification"
else
  fail "exact critical paths should classify as full"
  printf '%s\n' "$exact_output"
fi
if printf '%s\n' "$exact_output" | grep -q '^stamp_suite=full$'; then
  pass "exact critical paths use the shared full stamp suite"
else
  fail "exact critical paths should map to the full stamp suite"
  printf '%s\n' "$exact_output"
fi

pre_commit_output="$(run_detect_pre_commit crates/warp-core/src/lib.rs)"
if printf '%s\n' "$pre_commit_output" | grep -q '^classification=full$'; then
  pass "pre-commit classification uses staged files"
else
  fail "pre-commit detection should classify staged critical paths as full"
  printf '%s\n' "$pre_commit_output"
fi
if printf '%s\n' "$pre_commit_output" | grep -q '^stamp_context=pre-commit$'; then
  pass "pre-commit uses the index-backed stamp context"
else
  fail "pre-commit detection should report the pre-commit stamp context"
  printf '%s\n' "$pre_commit_output"
fi

deleted_branch_output="$(run_fixture_detect branch-delete)"
if printf '%s\n' "$deleted_branch_output" | grep -q '^classification=full$'; then
  pass "deleting a critical path still forces full branch verification"
else
  fail "critical-path deletions should classify as full in branch detection"
  printf '%s\n' "$deleted_branch_output"
fi

deleted_pre_commit_output="$(run_fixture_detect pre-commit-delete)"
if printf '%s\n' "$deleted_pre_commit_output" | grep -q '^classification=full$'; then
  pass "staged deletion of a critical path forces full pre-commit verification"
else
  fail "critical-path staged deletions should classify as full in pre-commit detection"
  printf '%s\n' "$deleted_pre_commit_output"
fi
if printf '%s\n' "$deleted_pre_commit_output" | grep -q '^stamp_context=pre-commit$'; then
  pass "critical-path staged deletions keep the pre-commit stamp context"
else
  fail "critical-path staged deletions should report the pre-commit stamp context"
  printf '%s\n' "$deleted_pre_commit_output"
fi

if rg -q 'scripts/verify-local\.sh pre-commit' .githooks/pre-commit; then
  pass "canonical pre-commit hook delegates staged crate verification to verify-local"
else
  fail "canonical pre-commit hook should delegate staged crate verification to verify-local"
fi

coverage_output="$(python3 - <<'PY'
from pathlib import Path
import re

text = Path("scripts/verify-local.sh").read_text()

def parse_array(name: str) -> list[str]:
    match = re.search(rf'readonly {name}=\((.*?)\n\)', text, re.S)
    if not match:
        raise SystemExit(f"missing array: {name}")
    items: list[str] = []
    for line in match.group(1).splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        items.append(line.strip('"'))
    return items

critical_crates = {
    prefix[len("crates/"):-1]
    for prefix in parse_array("FULL_CRITICAL_PREFIXES")
    if prefix.startswith("crates/")
}
full_packages = set(parse_array("FULL_LOCAL_PACKAGES"))
full_clippy_core = set(parse_array("FULL_LOCAL_CLIPPY_CORE_PACKAGES"))
full_clippy_support = set(parse_array("FULL_LOCAL_CLIPPY_SUPPORT_PACKAGES"))
full_clippy_bins = set(parse_array("FULL_LOCAL_CLIPPY_BIN_ONLY_PACKAGES"))
full_test_packages = set(parse_array("FULL_LOCAL_TEST_PACKAGES"))
fast_lib_only = set(parse_array("FAST_CLIPPY_LIB_ONLY_PACKAGES"))

missing_build = sorted(critical_crates - full_packages)
missing_clippy = sorted(critical_crates - (full_clippy_core | full_clippy_support | full_clippy_bins))
print("missing_build=" + ",".join(missing_build))
print("missing_clippy=" + ",".join(missing_clippy))
print("ttd_browser_tested=" + str("ttd-browser" in full_test_packages).lower())
print("warp_core_fast_lib_only=" + str("warp-core" in fast_lib_only).lower())
PY
)"
if printf '%s\n' "$coverage_output" | grep -q '^missing_build=$'; then
  pass "every full-critical crate is included in the full build/clippy package set"
else
  fail "full-critical crates must all be present in FULL_LOCAL_PACKAGES"
  printf '%s\n' "$coverage_output"
fi
if printf '%s\n' "$coverage_output" | grep -q '^missing_clippy=$'; then
  pass "every full-critical crate is covered by one of the curated local clippy lanes"
else
  fail "full-critical crates must all be present in the local clippy lane package sets"
  printf '%s\n' "$coverage_output"
fi
if printf '%s\n' "$coverage_output" | grep -q '^ttd_browser_tested=true$'; then
  pass "ttd-browser is covered by the full local test lane"
else
  fail "ttd-browser must be exercised by the full local test lane"
  printf '%s\n' "$coverage_output"
fi
if printf '%s\n' "$coverage_output" | grep -q '^warp_core_fast_lib_only=true$'; then
  pass "warp-core uses the narrowed fast local clippy scope"
else
  fail "warp-core should stay in the narrowed fast local clippy package set"
  printf '%s\n' "$coverage_output"
fi

if grep -q '^verify-ultra-fast:' Makefile; then
  pass "Makefile exposes an ultra-fast edit-loop lane"
else
  fail "Makefile should expose verify-ultra-fast for the shortest local loop"
fi

if grep -q '^verify-full-sequential:' Makefile; then
  pass "Makefile exposes a sequential fallback for the parallel full verifier"
else
  fail "Makefile should expose verify-full-sequential as a fallback path"
fi

fake_full_output="$(run_fake_verify full crates/warp-core/src/lib.rs)"
if printf '%s\n' "$fake_full_output" | grep -q '\[verify-local\] full: launching '; then
  pass "full verification fans out into explicit parallel lanes"
else
  fail "full verification should launch the curated local lane set"
  printf '%s\n' "$fake_full_output"
fi
if printf '%s\n' "$fake_full_output" | grep -q 'critical local gate (targeted-rust)'; then
  pass "critical crate changes use the targeted-rust full scope"
else
  fail "critical crate changes should use the targeted-rust full scope"
  printf '%s\n' "$fake_full_output"
fi
if printf '%s\n' "$fake_full_output" | grep -q 'target/verify-lanes/full-clippy-core'; then
  pass "full verification isolates clippy into its own target dir"
else
  fail "full verification should route clippy through an isolated target dir"
  printf '%s\n' "$fake_full_output"
fi
if printf '%s\n' "$fake_full_output" | grep -q 'target/verify-lanes/full-tests-warp-core'; then
  pass "full verification isolates warp-core tests into their own target dir"
else
  fail "full verification should route warp-core tests through an isolated target dir"
  printf '%s\n' "$fake_full_output"
fi

fake_full_seq_output="$(run_fake_verify full crates/warp-core/src/lib.rs sequential)"
if printf '%s\n' "$fake_full_seq_output" | grep -q '\[verify-local\] full: launching '; then
  fail "sequential fallback should not launch parallel lanes"
  printf '%s\n' "$fake_full_seq_output"
else
  pass "sequential fallback dispatches through the non-parallel runner"
fi
if printf '%s\n' "$fake_full_output" | grep -q -- '--test invariant_property_tests'; then
  fail "local warp-core full verification should stay on the smoke suite"
  printf '%s\n' "$fake_full_output"
else
  pass "local warp-core full verification stays on the smoke suite"
fi

fake_full_stamp_output="$(run_fake_full_stamp_sequence)"
if printf '%s\n' "$fake_full_stamp_output" | grep -q 'reusing cached full verification for tree tree-aaaaaaa'; then
  pass "full verification stamp reuse keys off the committed tree instead of HEAD"
else
  fail "full verification should reuse the cache for a different commit with the same tree"
  printf '%s\n' "$fake_full_stamp_output"
fi
if [[ "$(printf '%s\n' "$fake_full_stamp_output" | awk '/--- cargo-log ---/{flag=1; next} flag && NF {count++} END {print count+0}')" == "2" ]]; then
  pass "same-tree cache reuse suppresses the duplicate full rerun"
else
  fail "same-tree cache reuse should skip the duplicate full cargo invocation"
  printf '%s\n' "$fake_full_stamp_output"
fi

fake_fast_output="$(run_fake_verify fast crates/warp-core/src/lib.rs)"
if printf '%s\n' "$fake_fast_output" | grep -q 'clippy -p warp-core --lib -- -D warnings -D missing_docs'; then
  pass "fast verification uses the narrowed warp-core clippy scope"
else
  fail "fast verification should run warp-core clippy on the narrowed local target set"
  printf '%s\n' "$fake_fast_output"
fi
if printf '%s\n' "$fake_fast_output" | grep -vq 'clippy -p warp-core --all-targets'; then
  pass "fast verification no longer uses warp-core all-targets clippy"
else
  fail "fast verification must not fall back to warp-core all-targets clippy"
  printf '%s\n' "$fake_fast_output"
fi

fake_fast_nextest_output="$(run_fake_verify fast crates/bin-only/src/main.rs parallel 1)"
if printf '%s\n' "$fake_fast_nextest_output" | grep -q 'nextest run -p bin-only --bins'; then
  pass "nextest uses target-aware args for bin-only crates"
else
  fail "nextest should use --bins for bin-only crates instead of hardcoded lib/test flags"
  printf '%s\n' "$fake_fast_nextest_output"
fi
if printf '%s\n' "$fake_fast_nextest_output" | grep -q 'nextest run -p bin-only --lib --tests'; then
  fail "nextest must not hardcode --lib --tests for bin-only crates"
  printf '%s\n' "$fake_fast_nextest_output"
else
  pass "nextest avoids invalid lib/test flags for bin-only crates"
fi

fake_ultra_fast_output="$(run_fake_verify ultra-fast crates/warp-core/src/coordinator.rs)"
if printf '%s\n' "$fake_ultra_fast_output" | grep -q 'ultra-fast verification for changed Rust crates: warp-core'; then
  pass "ultra-fast reports the narrowed changed Rust crate set"
else
  fail "ultra-fast should report the narrowed changed Rust crate set"
  printf '%s\n' "$fake_ultra_fast_output"
fi
if printf '%s\n' "$fake_ultra_fast_output" | grep -q 'cargo check -p warp-core'; then
  pass "ultra-fast runs cargo check on changed Rust crates"
else
  fail "ultra-fast should run cargo check on changed Rust crates"
  printf '%s\n' "$fake_ultra_fast_output"
fi
if printf '%s\n' "$fake_ultra_fast_output" | grep -q -- '--test inbox'; then
  pass "ultra-fast still pulls targeted runtime smoke for critical warp-core changes"
else
  fail "ultra-fast should keep targeted runtime smoke for critical warp-core changes"
  printf '%s\n' "$fake_ultra_fast_output"
fi
if printf '%s\n' "$fake_ultra_fast_output" | grep -q 'clippy -p warp-core'; then
  fail "ultra-fast should skip clippy to stay compile-first"
  printf '%s\n' "$fake_ultra_fast_output"
else
  pass "ultra-fast skips clippy"
fi
if printf '%s\n' "$fake_ultra_fast_output" | grep -q 'doc -p warp-core'; then
  fail "ultra-fast should skip rustdoc gates"
  printf '%s\n' "$fake_ultra_fast_output"
else
  pass "ultra-fast skips rustdoc gates"
fi

fake_ultra_fast_warp_wasm_output="$(run_fake_verify ultra-fast crates/warp-wasm/src/warp_kernel.rs)"
if printf '%s\n' "$fake_ultra_fast_warp_wasm_output" | grep -q -- 'test -p warp-wasm --features engine --lib'; then
  pass "ultra-fast preserves warp-wasm engine smoke selection"
else
  fail "ultra-fast should preserve warp-wasm engine smoke selection"
  printf '%s\n' "$fake_ultra_fast_warp_wasm_output"
fi

fake_ultra_fast_readme_output="$(run_fake_verify ultra-fast crates/warp-wasm/README.md)"
if printf '%s\n' "$fake_ultra_fast_readme_output" | grep -q 'cargo check -p warp-wasm'; then
  fail "ultra-fast should not wake Rust cargo for non-Rust critical crate docs"
  printf '%s\n' "$fake_ultra_fast_readme_output"
else
  pass "ultra-fast keeps non-Rust critical crate docs off Rust cargo"
fi

fake_ultra_fast_tooling_output="$(run_fake_verify ultra-fast scripts/verify-local.sh)"
if printf '%s\n' "$fake_ultra_fast_tooling_output" | grep -q '\[verify-local\]\[ultra-fast\] tooling smoke'; then
  pass "ultra-fast tooling changes stay on the tooling smoke lane"
else
  fail "ultra-fast tooling changes should stay on the tooling smoke lane"
  printf '%s\n' "$fake_ultra_fast_tooling_output"
fi
if printf '%s\n' "$fake_ultra_fast_tooling_output" | grep -q 'hook regression coverage'; then
  fail "ultra-fast tooling changes should not inherit the full hook regression suite"
  printf '%s\n' "$fake_ultra_fast_tooling_output"
else
  pass "ultra-fast tooling changes avoid the full hook regression suite"
fi

fake_ultra_fast_hook_output="$(run_fake_verify ultra-fast .githooks/pre-push)"
if printf '%s\n' "$fake_ultra_fast_hook_output" | grep -q '\[verify-local\]\[ultra-fast\] bash -n \.githooks/pre-push'; then
  pass "ultra-fast syntax-checks changed canonical hook entrypoints"
else
  fail "ultra-fast should syntax-check changed canonical hook entrypoints"
  printf '%s\n' "$fake_ultra_fast_hook_output"
fi

fake_ultra_fast_hook_readme_output="$(run_fake_verify ultra-fast scripts/hooks/README.md)"
if printf '%s\n' "$fake_ultra_fast_hook_readme_output" | grep -q '\[verify-local\]\[ultra-fast\] bash -n scripts/hooks/README.md'; then
  fail "ultra-fast should skip non-shell files in hook directories"
  printf '%s\n' "$fake_ultra_fast_hook_readme_output"
else
  pass "ultra-fast skips non-shell files in hook directories"
fi
if printf '%s\n' "$fake_ultra_fast_hook_readme_output" | grep -q '\[verify-local\]\[ultra-fast\] no changed shell tooling files'; then
  pass "non-shell hook docs do not fabricate shell smoke targets"
else
  fail "non-shell hook docs should not appear as shell tooling files"
  printf '%s\n' "$fake_ultra_fast_hook_readme_output"
fi

fake_warp_core_default_output="$(run_fake_verify full crates/warp-core/src/provenance_store.rs)"
if printf '%s\n' "$fake_warp_core_default_output" | grep -q 'test -p warp-core --lib'; then
  pass "warp-core default smoke keeps the lib test lane"
else
  fail "warp-core default smoke should keep the lib test lane"
  printf '%s\n' "$fake_warp_core_default_output"
fi
if printf '%s\n' "$fake_warp_core_default_output" | grep -q -- '--test inbox'; then
  fail "warp-core default smoke should not always pull inbox"
  printf '%s\n' "$fake_warp_core_default_output"
else
  pass "warp-core default smoke avoids inbox when the file family does not need it"
fi

fake_warp_core_runtime_output="$(run_fake_verify full crates/warp-core/src/coordinator.rs)"
if printf '%s\n' "$fake_warp_core_runtime_output" | grep -q -- '--test inbox'; then
  pass "runtime-facing warp-core changes pull the inbox smoke test"
else
  fail "runtime-facing warp-core changes should pull the inbox smoke test"
  printf '%s\n' "$fake_warp_core_runtime_output"
fi

fake_warp_core_playback_output="$(run_fake_verify full crates/warp-core/src/playback.rs)"
if printf '%s\n' "$fake_warp_core_playback_output" | grep -q -- '--test playback_cursor_tests'; then
  pass "playback changes pull the playback cursor smoke test"
else
  fail "playback changes should pull the playback cursor smoke test"
  printf '%s\n' "$fake_warp_core_playback_output"
fi
if printf '%s\n' "$fake_warp_core_playback_output" | grep -q -- '--test outputs_playback_tests'; then
  pass "playback changes pull the outputs playback smoke test"
else
  fail "playback changes should pull the outputs playback smoke test"
  printf '%s\n' "$fake_warp_core_playback_output"
fi

fake_warp_core_prng_output="$(run_fake_verify full crates/warp-core/src/math/prng.rs)"
if printf '%s\n' "$fake_warp_core_prng_output" | grep -q -- '--features golden_prng --test prng_golden_regression'; then
  pass "PRNG changes pull the golden regression smoke test"
else
  fail "PRNG changes should pull the golden regression smoke test"
  printf '%s\n' "$fake_warp_core_prng_output"
fi

fake_warp_wasm_lib_output="$(run_fake_verify full crates/warp-wasm/src/lib.rs)"
if printf '%s\n' "$fake_warp_wasm_lib_output" | grep -q 'test -p warp-wasm --lib'; then
  pass "warp-wasm lib changes use the plain lib smoke lane"
else
  fail "warp-wasm lib changes should use the plain lib smoke lane"
  printf '%s\n' "$fake_warp_wasm_lib_output"
fi
if printf '%s\n' "$fake_warp_wasm_lib_output" | grep -q -- '--features engine --lib'; then
  fail "warp-wasm lib changes should not force the engine smoke lane"
  printf '%s\n' "$fake_warp_wasm_lib_output"
else
  pass "warp-wasm lib changes avoid the engine smoke lane"
fi

fake_warp_wasm_kernel_output="$(run_fake_verify full crates/warp-wasm/src/warp_kernel.rs)"
if printf '%s\n' "$fake_warp_wasm_kernel_output" | grep -q -- 'test -p warp-wasm --features engine --lib'; then
  pass "warp-kernel changes use the engine-enabled lib smoke lane"
else
  fail "warp-kernel changes should use the engine-enabled lib smoke lane"
  printf '%s\n' "$fake_warp_wasm_kernel_output"
fi

fake_echo_wasm_abi_kernel_port_output="$(run_fake_verify full crates/echo-wasm-abi/src/kernel_port.rs)"
if printf '%s\n' "$fake_echo_wasm_abi_kernel_port_output" | grep -q -- 'test -p echo-wasm-abi --lib'; then
  pass "echo-wasm-abi kernel-port changes keep the lib smoke lane"
else
  fail "echo-wasm-abi kernel-port changes should keep the lib smoke lane"
  printf '%s\n' "$fake_echo_wasm_abi_kernel_port_output"
fi

fake_echo_wasm_abi_canonical_output="$(run_fake_verify full crates/echo-wasm-abi/src/canonical.rs)"
if printf '%s\n' "$fake_echo_wasm_abi_canonical_output" | grep -q -- '--test canonical_vectors'; then
  pass "canonical ABI changes pull canonical vector coverage"
else
  fail "canonical ABI changes should pull canonical vector coverage"
  printf '%s\n' "$fake_echo_wasm_abi_canonical_output"
fi
if printf '%s\n' "$fake_echo_wasm_abi_canonical_output" | grep -q -- '--test non_canonical_floats'; then
  pass "canonical ABI changes pull non-canonical float coverage"
else
  fail "canonical ABI changes should pull non-canonical float coverage"
  printf '%s\n' "$fake_echo_wasm_abi_canonical_output"
fi
if printf '%s\n' "$fake_echo_wasm_abi_canonical_output" | grep -q -- 'test -p echo-wasm-abi --lib'; then
  fail "canonical ABI changes should not always force the lib smoke lane"
  printf '%s\n' "$fake_echo_wasm_abi_canonical_output"
else
  pass "canonical ABI changes avoid the generic lib smoke lane"
fi

fake_warp_wasm_readme_output="$(run_fake_verify full crates/warp-wasm/README.md)"
if printf '%s\n' "$fake_warp_wasm_readme_output" | grep -q 'critical local gate (tooling-only)'; then
  pass "non-rust critical crate docs stay off the Rust smoke lanes"
else
  fail "non-rust critical crate docs should stay off the Rust smoke lanes"
  printf '%s\n' "$fake_warp_wasm_readme_output"
fi
if printf '%s\n' "$fake_warp_wasm_readme_output" | grep -q 'tests-runtime'; then
  fail "non-rust critical crate docs should not launch runtime smoke lanes"
  printf '%s\n' "$fake_warp_wasm_readme_output"
else
  pass "non-rust critical crate docs skip runtime smoke lanes"
fi

fake_tooling_output="$(run_fake_verify full scripts/verify-local.sh)"
if printf '%s\n' "$fake_tooling_output" | grep -q 'critical local gate (tooling-only)'; then
  pass "tooling-only full verification uses the tooling-only scope"
else
  fail "tooling-only full verification should stay in tooling-only scope"
  printf '%s\n' "$fake_tooling_output"
fi
if printf '%s\n' "$fake_tooling_output" | grep -q 'fmt' \
  && printf '%s\n' "$fake_tooling_output" | grep -q 'guards' \
  && printf '%s\n' "$fake_tooling_output" | grep -q 'hook-tests'; then
  pass "tooling-only full verification runs hook regression coverage"
else
  fail "tooling-only full verification should run hook regression coverage"
  printf '%s\n' "$fake_tooling_output"
fi
if printf '%s\n' "$fake_tooling_output" | grep -q 'target/verify-lanes/full-clippy-core'; then
  fail "tooling-only full verification should not launch core Rust lanes"
  printf '%s\n' "$fake_tooling_output"
else
  pass "tooling-only full verification skips core Rust lanes"
fi

echo "PASS: $PASS"
echo "FAIL: $FAIL"

if [[ $FAIL -gt 0 ]]; then
  exit 1
fi
