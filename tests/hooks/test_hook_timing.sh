#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.." || exit 1

PASS=0
FAIL=0
CLEANUP_DIRS=()

pass() {
  echo "  PASS: $1"
  PASS=$((PASS + 1))
}

fail() {
  echo "  FAIL: $1"
  FAIL=$((FAIL + 1))
}

cleanup() {
  local dir
  for dir in "${CLEANUP_DIRS[@]}"; do
    rm -rf "$dir" 2>/dev/null || true
  done
}
trap cleanup EXIT

assert_csv_recorded() {
  local csv_file="$1"
  local expected_exit="$2"
  local label="$3"

  if [[ ! -f "$csv_file" ]]; then
    fail "${label} should create ${csv_file}"
    return
  fi

  if [[ "$(head -n1 "$csv_file")" == "timestamp_utc,elapsed_ms,exit_code,pid" ]]; then
    pass "${label} writes the CSV header"
  else
    fail "${label} should write the CSV header"
    cat "$csv_file"
  fi

  if tail -n1 "$csv_file" | awk -F, -v expected="$expected_exit" 'NF == 4 && $2 ~ /^[0-9]+$/ && $3 == expected && $4 ~ /^[0-9]+$/ { found=1 } END { exit found ? 0 : 1 }'; then
    pass "${label} appends a timing row with exit code ${expected_exit}"
  else
    fail "${label} should append a timing row with exit code ${expected_exit}"
    cat "$csv_file"
  fi
}

fixture_root() {
  local tmp
  tmp="$(mktemp -d)"
  CLEANUP_DIRS+=("$tmp")
  mkdir -p "$tmp/.githooks" "$tmp/scripts" "$tmp/bin"
  cp .githooks/_timing.sh "$tmp/.githooks/_timing.sh"
  chmod +x "$tmp/.githooks/_timing.sh"
  printf '%s\n' "$tmp"
}

install_fake_pre_push_workspace() {
  local tmp="$1"
  mkdir -p \
    "$tmp/crates/warp-core" \
    "$tmp/crates/warp-geom" \
    "$tmp/crates/warp-wasm" \
    "$tmp/scripts"

  cat >"$tmp/rust-toolchain.toml" <<'EOF'
[toolchain]
channel = "1.90.0"
EOF

  cat >"$tmp/crates/warp-core/Cargo.toml" <<'EOF'
[package]
name = "warp-core"
version = "0.1.0"
edition = "2021"
EOF

  cat >"$tmp/crates/warp-geom/Cargo.toml" <<'EOF'
[package]
name = "warp-geom"
version = "0.1.0"
edition = "2021"
EOF

  cat >"$tmp/crates/warp-wasm/Cargo.toml" <<'EOF'
[package]
name = "warp-wasm"
version = "0.1.0"
edition = "2021"
EOF

  cat >"$tmp/bin/cargo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
log_file="${DX_TEST_CARGO_LOG:?}"
args=("$@")
if [[ "${args[0]:-}" == +* ]]; then
  args=("${args[@]:1}")
fi
printf '%s\n' "${args[*]}" >>"$log_file"
case "${args[0]:-}" in
  fmt|clippy|test|doc)
    exit 0
    ;;
  nextest)
    if [[ "${args[1]:-}" == "run" ]]; then
      exit 0
    fi
    ;;
esac
echo "unexpected cargo invocation: ${args[*]}" >&2
exit 1
EOF

  cat >"$tmp/bin/rustup" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "toolchain" && "${2:-}" == "list" ]]; then
  printf '%s\n' "1.90.0-x86_64-apple-darwin (default)"
  exit 0
fi
echo "unexpected rustup invocation: $*" >&2
exit 1
EOF

  cat >"$tmp/bin/rg" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exit 1
EOF

  cat >"$tmp/scripts/check_spdx.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exit 0
EOF

  cat >"$tmp/scripts/ban-nondeterminism.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exit 0
EOF

  chmod +x \
    "$tmp/bin/cargo" \
    "$tmp/bin/rustup" \
    "$tmp/bin/rg" \
    "$tmp/scripts/check_spdx.sh" \
    "$tmp/scripts/ban-nondeterminism.sh"
}

assert_cargo_invoked() {
  local log_file="$1"
  local expected="$2"
  local label="$3"

  if grep -q "^${expected}\b" "$log_file"; then
    pass "${label} runs cargo ${expected}"
  else
    fail "${label} should run cargo ${expected}"
    cat "$log_file"
  fi
}

echo "=== Hook timing instrumentation ==="
echo

for hook in commit-msg pre-commit pre-push pre-push-parallel pre-push-sequential pre-rebase; do
  if rg -q "hook_timing_prepare \"\\\$REPO_ROOT\" \"$hook\"" ".githooks/$hook"; then
    pass ".githooks/$hook enables hook timing"
  else
    fail ".githooks/$hook should enable hook timing"
  fi
done

if rg -q '^\.dx-debug/$' .gitignore; then
  pass ".gitignore ignores .dx-debug timing artifacts"
else
  fail ".gitignore should ignore .dx-debug timing artifacts"
fi
if rg -q '^blog/$' .gitignore; then
  pass ".gitignore ignores blog drafts recursively"
else
  fail ".gitignore should ignore blog drafts recursively"
fi

tmp="$(fixture_root)"
cp .githooks/commit-msg "$tmp/.githooks/commit-msg"
chmod +x "$tmp/.githooks/commit-msg"
printf 'feat: timed commit-msg hook\n' >"$tmp/COMMIT_MSG"
(
  cd "$tmp"
  ./.githooks/commit-msg COMMIT_MSG
)
assert_csv_recorded "$tmp/.dx-debug/commit-msg-times.csv" 0 "commit-msg"
rm -rf "$tmp"

tmp="$(fixture_root)"
cp .githooks/pre-rebase "$tmp/.githooks/pre-rebase"
chmod +x "$tmp/.githooks/pre-rebase"
if [[ -x "$tmp/.githooks/pre-rebase" ]]; then
  pass "pre-rebase hook fixture is executable"
else
  fail "pre-rebase hook fixture should be executable"
fi
if (
  cd "$tmp"
  set +e
  ./.githooks/pre-rebase >/dev/null 2>&1
  rc=$?
  set -e
  [[ "$rc" -eq 1 ]]
); then
  pass "pre-rebase exits non-zero as expected"
else
  fail "pre-rebase should exit 1"
fi
assert_csv_recorded "$tmp/.dx-debug/pre-rebase-times.csv" 1 "pre-rebase"
rm -rf "$tmp"

tmp="$(fixture_root)"
cp .githooks/pre-push "$tmp/.githooks/pre-push"
chmod +x "$tmp/.githooks/pre-push"
cat >"$tmp/scripts/verify-local.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" > .verify-local-invocation
EOF
chmod +x "$tmp/scripts/verify-local.sh"
(
  cd "$tmp"
  ./.githooks/pre-push
)
if [[ "$(<"$tmp/.verify-local-invocation")" == "pre-push" ]]; then
  pass "pre-push still delegates to verify-local"
else
  fail "pre-push should delegate to verify-local with pre-push mode"
  cat "$tmp/.verify-local-invocation"
fi
assert_csv_recorded "$tmp/.dx-debug/pre-push-times.csv" 0 "pre-push"
rm -rf "$tmp"

tmp="$(fixture_root)"
cp .githooks/pre-commit "$tmp/.githooks/pre-commit"
chmod +x "$tmp/.githooks/pre-commit"
cat >"$tmp/rust-toolchain.toml" <<'EOF'
[toolchain]
channel = "1.90.0"
EOF
cat >"$tmp/bin/git" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "diff" && "${2:-}" == "--cached" ]]; then
  exit 0
fi
echo "unexpected git invocation: $*" >&2
exit 1
EOF
cat >"$tmp/bin/cargo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "fmt" ]]; then
  exit 0
fi
echo "unexpected cargo invocation: $*" >&2
exit 1
EOF
chmod +x "$tmp/bin/git" "$tmp/bin/cargo"
cat >"$tmp/scripts/verify-local.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exit 0
EOF
cat >"$tmp/scripts/ensure_spdx.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exit 0
EOF
chmod +x "$tmp/scripts/verify-local.sh" "$tmp/scripts/ensure_spdx.sh"
(
  cd "$tmp"
  PATH="$tmp/bin:/usr/bin:/bin:/usr/sbin:/sbin" ./.githooks/pre-commit
)
assert_csv_recorded "$tmp/.dx-debug/pre-commit-times.csv" 0 "pre-commit"
rm -rf "$tmp"

tmp="$(fixture_root)"
cp .githooks/pre-push-sequential "$tmp/.githooks/pre-push-sequential"
chmod +x "$tmp/.githooks/pre-push-sequential"
install_fake_pre_push_workspace "$tmp"
(
  cd "$tmp"
  DX_TEST_CARGO_LOG="$tmp/cargo-sequential.log" \
    PATH="$tmp/bin:/usr/bin:/bin:/usr/sbin:/sbin" \
    ./.githooks/pre-push-sequential
)
assert_csv_recorded "$tmp/.dx-debug/pre-push-sequential-times.csv" 0 "pre-push-sequential"
assert_cargo_invoked "$tmp/cargo-sequential.log" "fmt" "pre-push-sequential"
assert_cargo_invoked "$tmp/cargo-sequential.log" "clippy" "pre-push-sequential"
assert_cargo_invoked "$tmp/cargo-sequential.log" "test" "pre-push-sequential"
assert_cargo_invoked "$tmp/cargo-sequential.log" "doc" "pre-push-sequential"
rm -rf "$tmp"

tmp="$(fixture_root)"
cp .githooks/pre-push-parallel "$tmp/.githooks/pre-push-parallel"
chmod +x "$tmp/.githooks/pre-push-parallel"
install_fake_pre_push_workspace "$tmp"
(
  cd "$tmp"
  DX_TEST_CARGO_LOG="$tmp/cargo-parallel.log" \
    PATH="$tmp/bin:/usr/bin:/bin:/usr/sbin:/sbin" \
    ./.githooks/pre-push-parallel
)
assert_csv_recorded "$tmp/.dx-debug/pre-push-parallel-times.csv" 0 "pre-push-parallel"
assert_cargo_invoked "$tmp/cargo-parallel.log" "fmt" "pre-push-parallel"
assert_cargo_invoked "$tmp/cargo-parallel.log" "clippy" "pre-push-parallel"
assert_cargo_invoked "$tmp/cargo-parallel.log" "test" "pre-push-parallel"
assert_cargo_invoked "$tmp/cargo-parallel.log" "doc" "pre-push-parallel"
rm -rf "$tmp"

tmp="$(fixture_root)"
mkdir -p "$tmp/bin"
cat >"$tmp/bin/python3" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "5000000000"
EOF
chmod +x "$tmp/bin/python3"
(
  cd "$tmp"
  PATH="$tmp/bin:/bin:/usr/sbin:/sbin" /bin/bash -c '
    source ./.githooks/_timing.sh
    hook_timing_prepare "$PWD" "cached-method"
    rm -f "$PWD/bin/python3"
    hook_timing_append 0
  '
)
assert_csv_recorded "$tmp/.dx-debug/cached-method-times.csv" 0 "cached timing method"
if tail -n1 "$tmp/.dx-debug/cached-method-times.csv" | awk -F, 'NF == 4 && $2 == 0 { found=1 } END { exit found ? 0 : 1 }'; then
  pass "cached timing method avoids mixed-clock deltas when python disappears"
else
  fail "cached timing method should yield a zero delta when python disappears mid-hook"
  cat "$tmp/.dx-debug/cached-method-times.csv"
fi
rm -rf "$tmp"

echo
echo "PASS: $PASS"
echo "FAIL: $FAIL"

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
