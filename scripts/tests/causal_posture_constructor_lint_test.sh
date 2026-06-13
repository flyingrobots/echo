#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#
# Regression coverage for PLATFORM-0027 constructor posture lint.

set -euo pipefail

script_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "${script_root}/../.." && pwd)"
guard="${repo_root}/scripts/check-causal-posture-constructors.sh"

passed=0
failed=0

assert() {
  local label="$1"
  shift
  if (set +e; "$@" >/dev/null 2>&1); then
    echo "  PASS: ${label}"
    ((passed++)) || true
  else
    echo "  FAIL: ${label}"
    ((failed++)) || true
  fi
}

assert_not() {
  local label="$1"
  shift
  if (set +e; "$@" >/dev/null 2>&1); then
    echo "  FAIL: ${label}"
    ((failed++)) || true
  else
    echo "  PASS: ${label}"
    ((passed++)) || true
  fi
}

echo "=== Causal posture constructor lint tests ==="
echo ""

echo "1. Guard exists"
assert "constructor posture guard exists" \
  test -x "${guard}"

echo ""
echo "2. Raw posture construction and default regressions are rejected"
tmpdir="$(mktemp -d "${TMPDIR:-/tmp}/causal-posture-lint.XXXXXX")"
trap 'rm -rf "${tmpdir}"' EXIT
retention_fixture="${tmpdir}/bad-retention.rs"
cat >"${retention_fixture}" <<'RS'
fn bad_fixture() {
    let _posture = RetentionPosture {
        causal_posture,
        posture_derivation,
        authority,
        retention_contract,
        admission_scope,
    };
}
RS

assert_not "raw RetentionPosture literal is rejected" \
  env CAUSAL_POSTURE_LINT_PATHS="${retention_fixture}" "${guard}"

session_fixture="${tmpdir}/bad-session.rs"
cat >"${session_fixture}" <<'RS'
fn bad_fixture() {
    let _session = SessionContext {
        session_id,
        origin_id,
        actor_id,
        author_domain,
        authority_binding,
        seal_strength,
        default_posture,
        default_admission_scope,
        retention_contract,
    };
}
RS

assert_not "raw SessionContext literal is rejected" \
  env CAUSAL_POSTURE_LINT_PATHS="${session_fixture}" "${guard}"

tail_fixture="${tmpdir}/bad-tail.rs"
cat >"${tail_fixture}" <<'RS'
fn bad_fixture() -> RetentionPosture {
    RetentionPosture {
        causal_posture,
        posture_derivation,
        authority,
        retention_contract,
        admission_scope,
    }
}
RS

assert_not "tail-expression RetentionPosture literal is rejected" \
  env CAUSAL_POSTURE_LINT_PATHS="${tail_fixture}" "${guard}"

return_fixture="${tmpdir}/bad-return.rs"
cat >"${return_fixture}" <<'RS'
fn bad_fixture() -> SessionContext {
    return SessionContext {
        session_id,
        origin_id,
        actor_id,
        author_domain,
        authority_binding,
        seal_strength,
        default_posture,
        default_admission_scope,
        retention_contract,
    };
}
RS

assert_not "return SessionContext literal is rejected" \
  env CAUSAL_POSTURE_LINT_PATHS="${return_fixture}" "${guard}"

bracket_fixture="${tmpdir}/bad-bracket.rs"
cat >"${bracket_fixture}" <<'RS'
fn bad_fixture() {
    let _postures = vec![RetentionPosture {
        causal_posture,
        posture_derivation,
        authority,
        retention_contract,
        admission_scope,
    }];
}
RS

assert_not "bracketed RetentionPosture literal is rejected" \
  env CAUSAL_POSTURE_LINT_PATHS="${bracket_fixture}" "${guard}"

match_fixture="${tmpdir}/bad-match.rs"
cat >"${match_fixture}" <<'RS'
fn bad_fixture() -> SessionContext {
    match posture {
        _ => SessionContext {
            session_id,
            origin_id,
            actor_id,
            author_domain,
            authority_binding,
            seal_strength,
            default_posture,
            default_admission_scope,
            retention_contract,
        },
    }
}
RS

assert_not "match-arm SessionContext literal is rejected" \
  env CAUSAL_POSTURE_LINT_PATHS="${match_fixture}" "${guard}"

default_call_fixture="${tmpdir}/bad-default-call.rs"
cat >"${default_call_fixture}" <<'RS'
fn bad_fixture() {
    let _posture = CausalPosture::default();
}
RS

assert_not "CausalPosture::default call is rejected" \
  env CAUSAL_POSTURE_LINT_PATHS="${default_call_fixture}" "${guard}"

impl_default_fixture="${tmpdir}/bad-impl-default.rs"
cat >"${impl_default_fixture}" <<'RS'
impl Default for CausalPosture {
    fn default() -> Self {
        Self::AuthorOnly
    }
}
RS

assert_not "impl Default for CausalPosture is rejected" \
  env CAUSAL_POSTURE_LINT_PATHS="${impl_default_fixture}" "${guard}"

derive_default_fixture="${tmpdir}/bad-derive-default.rs"
cat >"${derive_default_fixture}" <<'RS'
#[derive(Debug, Default)]
pub enum CausalPosture {
    #[default]
    AuthorOnly,
}
RS

assert_not "derive Default on CausalPosture is rejected" \
  env CAUSAL_POSTURE_LINT_PATHS="${derive_default_fixture}" "${guard}"

echo ""
echo "3. Valid constructor calls are accepted"
valid_retention_fixture="${tmpdir}/valid-retention.rs"
cat >"${valid_retention_fixture}" <<'RS'
fn valid_fixture() {
    let _posture = RetentionPosture::new(
        causal_posture,
        posture_derivation,
        authority,
        retention_contract,
        admission_scope,
    );
}
RS

assert "RetentionPosture::new is allowed" \
  env CAUSAL_POSTURE_LINT_PATHS="${valid_retention_fixture}" "${guard}"

valid_session_fixture="${tmpdir}/valid-session.rs"
cat >"${valid_session_fixture}" <<'RS'
fn valid_fixture() {
    let _session = SessionContext::new(
        session_id,
        origin_id,
        actor_id,
        author_domain,
        authority_binding,
        seal_strength,
        default_posture,
        default_admission_scope,
        retention_contract,
    );
}
RS

assert "SessionContext::new is allowed" \
  env CAUSAL_POSTURE_LINT_PATHS="${valid_session_fixture}" "${guard}"

echo ""
echo "=== Results: ${passed} passed, ${failed} failed ==="

if [ "${failed}" -gt 0 ]; then
  exit 1
fi
