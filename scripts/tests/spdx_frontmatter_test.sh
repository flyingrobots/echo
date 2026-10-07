#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
set -euo pipefail
root=$(git rev-parse --show-toplevel)
cd "$root"
checker=${1:-scripts/ensure_spdx.sh}
work=$(mktemp -d "${TMPDIR:-/tmp}/echo-spdx-test.XXXXXX")
trap 'rm -rf "$work"' EXIT
cat > "$work/valid.md" <<'DOC'
---
id: S01
type: Feature
---
<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Task
DOC
bash "$checker" --check "$work/valid.md"
cat > "$work/missing.md" <<'DOC'
---
id: S01
---
# Task
DOC
if bash "$checker" --check "$work/missing.md"; then
  echo 'missing license passed' >&2
  exit 1
fi
if bash "$checker" "$work/missing.md"; then
  echo 'repair must report changed files' >&2
  exit 1
else
  test "$?" = 1
fi
bash "$checker" --check "$work/missing.md"
test "$(head -n 1 "$work/missing.md")" = '---'
test "$(sed -n '2p' "$work/missing.md")" = 'id: S01'
sed 's/Apache-2.0 OR LicenseRef-MIND-UCAL-1.0/MIT/' "$work/valid.md" > "$work/wrong.md"
if bash "$checker" "$work/wrong.md"; then
  echo 'repair must report changed files' >&2
  exit 1
else
  test "$?" = 1
fi
cmp "$work/valid.md" "$work/wrong.md"
cat > "$work/unclosed.md" <<'DOC'
---
id: S01
<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
DOC
if bash "$checker" --check "$work/unclosed.md"; then
  echo 'unclosed frontmatter passed' >&2
  exit 1
fi
printf '%s\n' 'PASS: valid metadata license, missing license refusal, unclosed metadata refusal'
