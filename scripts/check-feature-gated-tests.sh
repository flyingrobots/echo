#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#
# Every integration test file whose whole body sits behind an inner
# `#![cfg(... feature = "X" ...)]` must (1) declare those features as
# `required-features` in its crate manifest, so a plain `cargo test -p <crate>`
# skips it instead of reporting "running 0 tests ... ok", and (2) be run with
# those features by some CI command, so skipping it never hides it (#867).
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
ci_files=(.github/workflows/*.yml)
failures=0

manifest_required_features() {
  # Prints the required-features of [[test]] `$2` in manifest `$1`, one per line.
  awk -v name="$2" '
    /^\[\[test\]\]/ { in_test = 1; matched = 0; next }
    /^\[/ { in_test = 0; matched = 0 }
    in_test && $0 ~ "^name *= *\"" name "\"" { matched = 1; next }
    in_test && matched && /^required-features/ {
      line = $0; sub(/^[^[]*\[/, "", line); sub(/\].*$/, "", line)
      n = split(line, parts, ",")
      for (i = 1; i <= n; i++) { f = parts[i]; gsub(/[ "]/, "", f); if (f != "") print f }
    }
  ' "$1"
}

for test_file in crates/*/tests/*.rs; do
  inner_cfg="$(grep -m1 -E '^#!\[cfg\(' "$test_file" || true)"
  [[ -z "$inner_cfg" ]] && continue
  features="$(printf '%s\n' "$inner_cfg" | grep -oE 'feature *= *"[^"]+"' | sed -E 's/.*"([^"]+)"/\1/' | sort -u)"
  [[ -z "$features" ]] && continue
  # `not(feature = ...)` gates exclude rather than require a feature.
  if printf '%s\n' "$inner_cfg" | grep -q 'not('; then continue; fi
  crate_dir="${test_file%/tests/*}"
  crate="$(sed -n 's/^name *= *"\(.*\)"/\1/p' "$crate_dir/Cargo.toml" | head -1)"
  target="$(basename "$test_file" .rs)"
  declared="$(manifest_required_features "$crate_dir/Cargo.toml" "$target" | sort -u)"
  for feature in $features; do
    if ! printf '%s\n' "$declared" | grep -qx "$feature"; then
      echo "MISSING required-features: $crate test \`$target\` gates on \`$feature\` (add it to [[test]] in $crate_dir/Cargo.toml)"
      failures=$((failures + 1))
    fi
  done
  run_line="$(grep -hE -- "-p $crate .*--test $target( |\$|\\\\)" "${ci_files[@]}" || true)"
  run_line+="$(grep -hE -- "--test $target( |\$|\\\\)" "${ci_files[@]}" | grep -- "-p $crate" || true)"
  covered=1
  for feature in $features; do
    if ! printf '%s\n' "$run_line" | grep -q -- "$feature"; then covered=0; fi
  done
  if [[ "$covered" -eq 0 ]]; then
    echo "UNSCHEDULED: no CI command runs $crate test \`$target\` with features: $(echo $features | tr '\n' ' ')"
    failures=$((failures + 1))
  fi
done

if [[ "$failures" -gt 0 ]]; then
  echo "check-feature-gated-tests: $failures problem(s)" >&2
  exit 1
fi
echo "check-feature-gated-tests: every feature-gated test target is declared and scheduled"
