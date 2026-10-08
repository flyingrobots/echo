#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
set -euo pipefail
root=$(git rev-parse --show-toplevel)
work=$(mktemp -d "${TMPDIR:-/tmp}/echo-dependency-step.XXXXXX")
trap 'rm -rf "$work"' EXIT
mkdir "$work/bin"
cat > "$work/bin/cargo" <<'MOCK'
#!/usr/bin/env bash
if [[ "$1" == tree ]]; then exit 23; fi
exit 0
MOCK
cat > "$work/bin/rustup" <<'MOCK'
#!/usr/bin/env bash
exit 0
MOCK
chmod +x "$work/bin/cargo" "$work/bin/rustup"
body=$(awk '
  /name: Preserve the default CAS toolchain and dependency boundary/ { step = 1; next }
  step && /^[[:space:]]*run: \|/ { command = 1; next }
  command { sub(/^          /, ""); print }
' "$root/.github/workflows/echo-keep-experimental.yml")
test -n "$body"
if PATH="$work/bin:$PATH" bash -e -c "$body"; then
  echo 'FAIL: cargo tree failure was accepted' >&2
  exit 1
fi
printf '%s\n' 'PASS: failing dependency-tree command blocks the exact workflow step'
