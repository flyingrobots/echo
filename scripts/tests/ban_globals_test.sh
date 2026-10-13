#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
set -euo pipefail
root=$(git rev-parse --show-toplevel)
cd "$root"
checker=${1:-scripts/ban-globals.sh}
# The checker has two search backends. A runner without ripgrep uses the
# perl one, so every case runs under both. A case that only passes under one
# backend is a failure of the gate, not of the fixture.
#
# Backend 0 means "ripgrep if present". Without ripgrep it silently becomes a
# second perl run, so the ripgrep path would go untested. CI installs ripgrep
# and sets BAN_GLOBALS_TEST_REQUIRE_RG=1 to make that a failure.
if ! command -v rg >/dev/null 2>&1; then
  if [[ "${BAN_GLOBALS_TEST_REQUIRE_RG:-0}" == "1" ]]; then
    echo 'ban-globals test: ripgrep is required here and is not installed' >&2
    exit 1
  fi
  echo 'ban-globals test: ripgrep not found; only the perl backend is exercised' >&2
fi
work=$(mktemp -d "${TMPDIR:-/tmp}/echo-ban-globals-test.XXXXXX")
trap 'rm -rf "$work"' EXIT

run() {
  # run <backend> <allowlist> <paths...>
  local backend="$1" allowlist="$2"
  shift 2
  DETERMINISM_FORCE_NO_RG="$backend" BAN_GLOBALS_ALLOWLIST="$allowlist" \
    BAN_GLOBALS_PATHS="$*" bash "$checker" >"$work/out.txt" 2>&1
}

fail() {
  echo "ban-globals test: $1" >&2
  sed 's/^/    /' "$work/out.txt" >&2
  exit 1
}

mkdir -p "$work/clean" "$work/empty"
cat > "$work/clean/lib.rs" <<'RS'
const LIMIT: usize = 4;
static MAGIC: [u8; 4] = *b"ECHO";
struct Installer;
impl Installer {
    fn install_package(&mut self) {}
}
RS
: > "$work/none.allow"

write_case() {
  mkdir -p "$work/$1"
  printf '%s\n' "$2" > "$work/$1/lib.rs"
}
write_case once-lock 'static TABLE: std::sync::OnceLock<u8> = std::sync::OnceLock::new();'
write_case lazy-lock 'static TABLE: std::sync::LazyLock<u8> = std::sync::LazyLock::new(|| 1);'
write_case lazy-static 'lazy_static! { static ref TABLE: u8 = 1; }'
write_case once-cell 'use once_cell::sync::Lazy;'
write_case thread-local 'thread_local! { static DEPTH: u8 = const { 0 }; }'
write_case static-mut 'static mut COUNTER: u64 = 0;'
write_case static-interior 'static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);'

for backend in 0 1; do
  run "$backend" "$work/none.allow" "$work/clean" || fail "clean source refused (backend $backend)"
  run "$backend" "$work/none.allow" "$work/empty" || fail "empty path refused (backend $backend)"

  for rule in once-lock lazy-lock lazy-static once-cell thread-local static-mut static-interior; do
    if run "$backend" "$work/none.allow" "$work/$rule"; then
      fail "[$rule] violation passed (backend $backend)"
    fi
    grep -q "$work/$rule/lib.rs" "$work/out.txt" \
      || fail "[$rule] violation did not name its file (backend $backend)"

    # A waiver for the rule and path admits it.
    printf '%s %s fixture\n' "$rule" "$work/$rule/lib.rs" > "$work/match.allow"
    run "$backend" "$work/match.allow" "$work/$rule" \
      || fail "[$rule] exact waiver did not admit (backend $backend)"

    # A waiver for another path must not hide it. A non-empty allowlist that
    # matches nothing is the case that once made the perl backend pass.
    printf '%s %s fixture\n' "$rule" "$work/elsewhere.rs" > "$work/other.allow"
    if run "$backend" "$work/other.allow" "$work/$rule"; then
      fail "[$rule] violation passed beside an unrelated waiver (backend $backend)"
    fi

    # A waiver for another rule on the same path must not hide it.
    printf 'no-such-rule %s fixture\n' "$work/$rule/lib.rs" > "$work/wrong.allow"
    if run "$backend" "$work/wrong.allow" "$work/$rule"; then
      fail "[$rule] violation passed under a waiver for a different rule (backend $backend)"
    fi
  done
done

# The repository itself must pass under both backends with its own waivers.
for backend in 0 1; do
  DETERMINISM_FORCE_NO_RG="$backend" bash "$checker" >"$work/out.txt" 2>&1 \
    || fail "repository scan failed (backend $backend)"
done

echo "ban-globals test: PASSED"
