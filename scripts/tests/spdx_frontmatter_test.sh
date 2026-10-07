#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
set -euo pipefail
root=$(git rev-parse --show-toplevel)
cd "$root"
checker=${1:-scripts/ensure_spdx.sh}
case "${SPDX_CASE:-all}" in
  all|unclosed|header_first|displaced|prose|thematic|malformed|indented_keys|indented_comments|quoted_keys|spaced_keys|duplicate_headers|explicit_keys|unclosed_license|quoted_type|crlf|delimiterless) ;;
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
if [[ "${SPDX_CASE:-all}" == all || "${SPDX_CASE:-all}" == thematic ]]; then
for separator_count in 1 2; do
  {
    printf '%s\n' '<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->'
    printf '%s\n' '<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->'
    printf '\n---\n# Body heading\nA normal paragraph.\n'
    if [[ "$separator_count" = 2 ]]; then printf '\n---\nMore body.\n'; fi
  } > "$work/thematic-$separator_count.md"
  cp "$work/thematic-$separator_count.md" "$work/thematic-$separator_count.original"
  bash "$checker" --check "$work/thematic-$separator_count.md"
  bash "$checker" "$work/thematic-$separator_count.md"
  cmp "$work/thematic-$separator_count.original" "$work/thematic-$separator_count.md"
done
printf '%s\n' '---' '# Body heading' 'A normal paragraph.' > "$work/unlicensed-break.md"
cp "$work/unlicensed-break.md" "$work/unlicensed-break.original"
if bash "$checker" --check "$work/unlicensed-break.md"; then exit 1; fi
if bash "$checker" "$work/unlicensed-break.md"; then exit 1; else test "$?" = 1; fi
tail -n +3 "$work/unlicensed-break.md" > "$work/unlicensed-break.body"
cmp "$work/unlicensed-break.original" "$work/unlicensed-break.body"
bash "$checker" --check "$work/unlicensed-break.md"
cat > "$work/note-body.md" <<'DOC'
<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

---
Note: this section explains metadata.

A normal body paragraph.
---
DOC
cp "$work/note-body.md" "$work/note-body.original"
bash "$checker" --check "$work/note-body.md"
bash "$checker" "$work/note-body.md"
cmp "$work/note-body.original" "$work/note-body.md"
fi
if [[ "${SPDX_CASE:-all}" == all || "${SPDX_CASE:-all}" == malformed ]]; then
cat > "$work/malformed-comment.md" <<'DOC'
---
id: S01
---
<!-- SPDX-License-Identifier Apache-2.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Task
DOC
if bash "$checker" "$work/malformed-comment.md"; then exit 1; else test "$?" = 1; fi
bash "$checker" --check "$work/malformed-comment.md"
test "$(grep -c 'SPDX-License-Identifier' "$work/malformed-comment.md")" = 1
grep -qx '# Task' "$work/malformed-comment.md"
cat > "$work/equals-comment.md" <<'DOC'
---
id: S01
---
<!-- SPDX-License-Identifier = MIT -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Task
DOC
if bash "$checker" "$work/equals-comment.md"; then exit 1; else test "$?" = 1; fi
bash "$checker" --check "$work/equals-comment.md"
test "$(grep -c 'SPDX-License-Identifier' "$work/equals-comment.md")" = 1
grep -qx '# Task' "$work/equals-comment.md"
fi
if [[ "${SPDX_CASE:-all}" == all || "${SPDX_CASE:-all}" == indented_keys ]]; then
cat > "$work/indented-keys.md" <<'DOC'
<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

---
Example: a task-card code sample follows.

    id: S01
    type: Feature
---
DOC
cp "$work/indented-keys.md" "$work/indented-keys.original"
bash "$checker" --check "$work/indented-keys.md"
bash "$checker" "$work/indented-keys.md"
cmp "$work/indented-keys.original" "$work/indented-keys.md"
fi
if [[ "${SPDX_CASE:-all}" == all || "${SPDX_CASE:-all}" == indented_comments ]]; then
cat > "$work/indented-comments.md" <<'DOC'
---
id: S01
---

    <!-- SPDX-License-Identifier: MIT -->
    <!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

Body.
DOC
tail -n +4 "$work/indented-comments.md" > "$work/indented-comments.expected"
if bash "$checker" "$work/indented-comments.md"; then exit 1; else test "$?" = 1; fi
bash "$checker" --check "$work/indented-comments.md"
tail -n +6 "$work/indented-comments.md" > "$work/indented-comments.actual"
cmp "$work/indented-comments.expected" "$work/indented-comments.actual"
fi
if [[ "${SPDX_CASE:-all}" == all || "${SPDX_CASE:-all}" == quoted_keys ]]; then
for key in '"title"' "'title'"; do
printf -- '---\n%s: Example\n---\n# Body\n' "$key" > "$work/quoted-keys.md"
head -n 3 "$work/quoted-keys.md" > "$work/quoted-keys.metadata"
if bash "$checker" "$work/quoted-keys.md"; then exit 1; else test "$?" = 1; fi
bash "$checker" --check "$work/quoted-keys.md"
cp "$work/quoted-keys.md" "$work/quoted-keys.once"
bash "$checker" "$work/quoted-keys.md"
cmp "$work/quoted-keys.once" "$work/quoted-keys.md"
head -n 3 "$work/quoted-keys.md" > "$work/quoted-keys.actual"
cmp "$work/quoted-keys.metadata" "$work/quoted-keys.actual"
done
fi
if [[ "${SPDX_CASE:-all}" == all || "${SPDX_CASE:-all}" == spaced_keys ]]; then
cat > "$work/spaced-keys.md" <<'DOC'
---
title : Example
---
# Body
DOC
head -n 3 "$work/spaced-keys.md" > "$work/spaced-keys.expected"
if bash "$checker" "$work/spaced-keys.md"; then exit 1; else test "$?" = 1; fi
bash "$checker" --check "$work/spaced-keys.md"
head -n 3 "$work/spaced-keys.md" > "$work/spaced-keys.actual"
cmp "$work/spaced-keys.expected" "$work/spaced-keys.actual"
fi
if [[ "${SPDX_CASE:-all}" == all || "${SPDX_CASE:-all}" == duplicate_headers ]]; then
cat > "$work/duplicate-header.md" <<'DOC'
---
id: S01
---
<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
<!-- SPDX-License-Identifier: MIT -->
# Body
DOC
if bash "$checker" --check "$work/duplicate-header.md"; then exit 1; else test "$?" = 1; fi
if bash "$checker" "$work/duplicate-header.md"; then exit 1; else test "$?" = 1; fi
bash "$checker" --check "$work/duplicate-header.md"
test "$(grep -c SPDX-License-Identifier "$work/duplicate-header.md")" = 1
fi
if [[ "${SPDX_CASE:-all}" == all || "${SPDX_CASE:-all}" == explicit_keys ]]; then
cat > "$work/explicit-keys.md" <<'DOC'
---
? title
: Example
---
# Body
DOC
head -n 4 "$work/explicit-keys.md" > "$work/explicit-keys.expected"
if bash "$checker" "$work/explicit-keys.md"; then exit 1; else test "$?" = 1; fi
bash "$checker" --check "$work/explicit-keys.md"
head -n 4 "$work/explicit-keys.md" > "$work/explicit-keys.actual"
cmp "$work/explicit-keys.expected" "$work/explicit-keys.actual"
fi
if [[ "${SPDX_CASE:-all}" == all || "${SPDX_CASE:-all}" == unclosed_license ]]; then
cat > "$work/unclosed-license.md" <<'DOC'
---
id: S01
---
<!-- SPDX-License-Identifier: MIT
# Body
DOC
cp "$work/unclosed-license.md" "$work/unclosed-license.original"
if bash "$checker" "$work/unclosed-license.md"; then exit 1; else test "$?" = 1; fi
cmp "$work/unclosed-license.original" "$work/unclosed-license.md"
if bash "$checker" --check "$work/unclosed-license.md"; then exit 1; else test "$?" = 1; fi
fi
if [[ "${SPDX_CASE:-all}" == all || "${SPDX_CASE:-all}" == quoted_type ]]; then
cat > "$work/quoted-type.md" <<'DOC'
<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
---
id: S01
type: "Feature"
---
# Body
DOC
if bash "$checker" --check "$work/quoted-type.md"; then exit 1; else test "$?" = 1; fi
if bash "$checker" "$work/quoted-type.md"; then exit 1; else test "$?" = 1; fi
bash "$checker" --check "$work/quoted-type.md"
test "$(head -n 1 "$work/quoted-type.md")" = '---'
fi
if [[ "${SPDX_CASE:-all}" == all || "${SPDX_CASE:-all}" == crlf ]]; then
printf -- '---\r\nid: S01\r\n---\r\n# Body\r\n' > "$work/crlf.md"
head -n 3 "$work/crlf.md" > "$work/crlf.metadata"
tail -n +4 "$work/crlf.md" > "$work/crlf.body"
if bash "$checker" "$work/crlf.md"; then exit 1; else test "$?" = 1; fi
bash "$checker" --check "$work/crlf.md"
head -n 3 "$work/crlf.md" > "$work/crlf.actual-metadata"
tail -n +6 "$work/crlf.md" > "$work/crlf.actual-body"
cmp "$work/crlf.metadata" "$work/crlf.actual-metadata"
cmp "$work/crlf.body" "$work/crlf.actual-body"
python3 - "$work/crlf.md" <<'CHECK'
from pathlib import Path
import sys
raw=Path(sys.argv[1]).read_bytes()
assert b"\n" not in raw.replace(b"\r\n", b"")
CHECK
cp "$work/crlf.md" "$work/crlf.once"
bash "$checker" "$work/crlf.md"
cmp "$work/crlf.once" "$work/crlf.md"
fi
if [[ "${SPDX_CASE:-all}" == all || "${SPDX_CASE:-all}" == delimiterless ]]; then
cat > "$work/delimiterless.md" <<'DOC'
---
id: S01
---
<!-- SPDX-License-Identifier -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Body
DOC
if bash "$checker" "$work/delimiterless.md"; then exit 1; else test "$?" = 1; fi
bash "$checker" --check "$work/delimiterless.md"
test "$(grep -c SPDX-License-Identifier "$work/delimiterless.md")" = 1
fi
printf '%s\n' 'PASS: metadata placement, unclosed repair refusal, displaced license removal, and prose preservation'
