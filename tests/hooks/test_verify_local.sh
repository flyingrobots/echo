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
  local tmp
  tmp="$(mktemp -d)"

  mkdir -p "$tmp/scripts" "$tmp/bin" "$tmp/.git"
  mkdir -p "$tmp/crates/warp-core/src"
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
  printf 'test-head\n'
  exit 0
fi
if [[ "${1:-}" == "rev-parse" && "${2:-}" == "--short" && "${3:-}" == "HEAD" ]]; then
  printf 'test-head\n'
  exit 0
fi
exit 0
EOF
  chmod +x "$tmp/bin/cargo" "$tmp/bin/rustup" "$tmp/bin/rg" "$tmp/bin/npx" "$tmp/bin/git"

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

if grep -q '^verify-full-sequential:' Makefile; then
  pass "Makefile exposes a sequential fallback for the parallel full verifier"
else
  fail "Makefile should expose verify-full-sequential as a fallback path"
fi

fake_full_output="$(run_fake_verify full crates/warp-core/src/lib.rs)"
if printf '%s\n' "$fake_full_output" | grep -q '\[verify-local\] full: launching 9 local lanes'; then
  pass "full verification fans out into explicit parallel lanes"
else
  fail "full verification should launch the curated local lane set"
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

echo "PASS: $PASS"
echo "FAIL: $FAIL"

if [[ $FAIL -gt 0 ]]; then
  exit 1
fi
