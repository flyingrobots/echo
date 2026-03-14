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
full_test_packages = set(parse_array("FULL_LOCAL_TEST_PACKAGES"))

missing_build = sorted(critical_crates - full_packages)
print("missing_build=" + ",".join(missing_build))
print("ttd_browser_tested=" + str("ttd-browser" in full_test_packages).lower())
PY
)"
if printf '%s\n' "$coverage_output" | grep -q '^missing_build=$'; then
  pass "every full-critical crate is included in the full build/clippy package set"
else
  fail "full-critical crates must all be present in FULL_LOCAL_PACKAGES"
  printf '%s\n' "$coverage_output"
fi
if printf '%s\n' "$coverage_output" | grep -q '^ttd_browser_tested=true$'; then
  pass "ttd-browser is covered by the full local test lane"
else
  fail "ttd-browser must be exercised by the full local test lane"
  printf '%s\n' "$coverage_output"
fi

echo "PASS: $PASS"
echo "FAIL: $FAIL"

if [[ $FAIL -gt 0 ]]; then
  exit 1
fi
