#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
set -euo pipefail
root=$(git rev-parse --show-toplevel)
work=$(mktemp -d "${TMPDIR:-/tmp}/echo-audit-lockfiles.XXXXXX")
trap 'rm -rf "$work"' EXIT
mkdir "$work/bin"
cat > "$work/bin/cargo" <<'MOCK'
#!/usr/bin/env bash
printf '%s\n' "$*" >> "$ECHO_AUDIT_CAPTURE"
if [[ "${ECHO_AUDIT_FAIL_EXPERIMENT:-0}" == 1 && "$*" == *experiments/echo-keep/Cargo.lock* ]]; then exit 23; fi
exit 0
MOCK
chmod +x "$work/bin/cargo"
PATH="$work/bin:$PATH" ECHO_AUDIT_CAPTURE="$work/calls" bash "$root/scripts/run_cargo_audit.sh"
if ! grep -q -- "--file $root/experiments/echo-keep/Cargo.lock" "$work/calls"; then
  echo "FAIL: experimental lockfile was not audited" >&2
  exit 1
fi
grep -q -- "--file $root/Cargo.lock" "$work/calls"
if PATH="$work/bin:$PATH" ECHO_AUDIT_CAPTURE="$work/failing-calls" ECHO_AUDIT_FAIL_EXPERIMENT=1 bash "$root/scripts/run_cargo_audit.sh"; then
  echo 'FAIL: experimental audit failure was accepted' >&2
  exit 1
fi
printf '%s\n' 'PASS: both lockfiles are audited and an experimental audit failure blocks'
