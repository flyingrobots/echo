#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
set -euo pipefail
root=$(git rev-parse --show-toplevel)
cd "$root"
checker=${1:-scripts/ensure_spdx.sh}
case "${SPDX_CASE:-all}" in
  all|unclosed|header_first|displaced|prose) ;;
  *) echo 'unknown SPDX regression case' >&2; exit 2 ;;
esac
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
if [[ "${SPDX_CASE:-all}" == all || "${SPDX_CASE:-all}" == unclosed ]]; then
cp "$work/unclosed.md" "$work/unclosed.original"
if bash "$checker" "$work/unclosed.md"; then
  echo 'unclosed repair passed' >&2
  exit 1
else
  test "$?" = 1
fi
cmp "$work/unclosed.original" "$work/unclosed.md"
fi
if [[ "${SPDX_CASE:-all}" == all || "${SPDX_CASE:-all}" == header_first ]]; then
cat > "$work/header-first.md" <<'DOC'
<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

---
id: S01
type: Feature
---
# Task
DOC
if bash "$checker" --check "$work/header-first.md"; then
  echo 'displaced metadata passed' >&2
  exit 1
fi
if bash "$checker" "$work/header-first.md"; then
  echo 'metadata relocation must report a change' >&2
  exit 1
else
  test "$?" = 1
fi
bash "$checker" --check "$work/header-first.md"
head -n 4 "$work/valid.md" > "$work/metadata.expected"
head -n 4 "$work/header-first.md" > "$work/metadata.actual"
cmp "$work/metadata.expected" "$work/metadata.actual"
fi
if [[ "${SPDX_CASE:-all}" == all || "${SPDX_CASE:-all}" == displaced ]]; then
cat > "$work/displaced.md" <<'DOC'
---
id: S01
---


<!-- SPDX-License-Identifier: MIT -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Task
DOC
if bash "$checker" "$work/displaced.md"; then
  echo 'displaced license must report a change' >&2
  exit 1
else
  test "$?" = 1
fi
bash "$checker" --check "$work/displaced.md"
test "$(grep -c '^<!-- SPDX-License-Identifier:' "$work/displaced.md")" = 1
if grep -q 'SPDX-License-Identifier: MIT' "$work/displaced.md"; then
  echo 'conflicting displaced license survived repair' >&2
  exit 1
fi
fi
if [[ "${SPDX_CASE:-all}" == all || "${SPDX_CASE:-all}" == prose ]]; then
for spacer in 0 1; do
  {
    printf '%s\n' '---' 'id: S01' '---'
    if [[ "$spacer" = 1 ]]; then printf '\n'; fi
    printf '%s\n' 'SPDX-License-Identifier is the field this guide explains.'
    printf '%s\n' 'James Ross and FLYING ROBOTS appear in this source sentence.'
  } > "$work/prose-$spacer.md"
  if bash "$checker" "$work/prose-$spacer.md"; then
    echo 'body fixture repair must report a change' >&2
    exit 1
  else
    test "$?" = 1
  fi
  grep -qx 'SPDX-License-Identifier is the field this guide explains.' "$work/prose-$spacer.md"
  grep -qx 'James Ross and FLYING ROBOTS appear in this source sentence.' "$work/prose-$spacer.md"
  bash "$checker" --check "$work/prose-$spacer.md"
done
fi
printf '%s\n' 'PASS: metadata placement, unclosed repair refusal, displaced license removal, and prose preservation'
