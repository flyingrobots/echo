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
  mkdir -p "$tmp/.githooks" "$tmp/scripts" "$tmp/bin"
  cp .githooks/_timing.sh "$tmp/.githooks/_timing.sh"
  chmod +x "$tmp/.githooks/_timing.sh"
  printf '%s\n' "$tmp"
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

if rg -q '^\.dx-debug/\*$' .gitignore; then
  pass ".gitignore ignores .dx-debug timing artifacts"
else
  fail ".gitignore should ignore .dx-debug timing artifacts"
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
(
  cd "$tmp"
  if ./.githooks/pre-rebase; then
    exit 1
  fi
)
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

echo
echo "PASS: $PASS"
echo "FAIL: $FAIL"

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
