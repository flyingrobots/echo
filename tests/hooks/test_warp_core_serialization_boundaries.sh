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

if scripts/check-warp-core-serialization-boundaries.sh >/dev/null 2>&1; then
  pass "checked-in warp-core obeys boundary-only serialization rules"
else
  fail "checked-in warp-core should obey boundary-only serialization rules"
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp"/{scripts/lib,crates/warp-core/src,crates/warp-core/tests}
cp scripts/check-warp-core-serialization-boundaries.sh \
  "$tmp/scripts/check-warp-core-serialization-boundaries.sh"
cp scripts/lib/determinism-scan.sh "$tmp/scripts/lib/determinism-scan.sh"
chmod +x "$tmp/scripts/check-warp-core-serialization-boundaries.sh"
cat >"$tmp/crates/warp-core/Cargo.toml" <<'EOF'
[package]
name = "warp-core"
version = "0.0.0"

[dependencies]
serde = { version = "1", features = ["derive"] }
EOF
printf '%s\n' 'pub fn ok() {}' >"$tmp/crates/warp-core/src/lib.rs"

if (cd "$tmp" && scripts/check-warp-core-serialization-boundaries.sh >/dev/null 2>&1); then
  fail "guard should reject direct serde manifest entries"
else
  pass "guard rejects direct serde manifest entries"
fi

cat >"$tmp/crates/warp-core/Cargo.toml" <<'EOF'
[package]
name = "warp-core"
version = "0.0.0"

[dependencies.serde]
version = "1"
features = ["derive"]
EOF

if (cd "$tmp" && scripts/check-warp-core-serialization-boundaries.sh >/dev/null 2>&1); then
  fail "guard should reject table-form serde manifest entries"
else
  pass "guard rejects table-form serde manifest entries"
fi

cat >"$tmp/crates/warp-core/Cargo.toml" <<'EOF'
[package]
name = "warp-core"
version = "0.0.0"

[dependencies]
EOF
cat >"$tmp/crates/warp-core/src/lib.rs" <<'EOF'
#[derive(serde::Serialize, serde::Deserialize)]
pub struct LeakyCoreDto {
    pub id: u64,
}
EOF

if (cd "$tmp" && scripts/check-warp-core-serialization-boundaries.sh >/dev/null 2>&1); then
  fail "guard should reject serde derives in warp-core source"
else
  pass "guard rejects serde derives in warp-core source"
fi

cat >"$tmp/crates/warp-core/src/lib.rs" <<'EOF'
pub fn not_a_boundary(value: &u64) {
    let _ = echo_wasm_abi::encode_cbor(value);
}
EOF

if (cd "$tmp" && scripts/check-warp-core-serialization-boundaries.sh >/dev/null 2>&1); then
  fail "guard should reject canonical serialization calls outside boundary modules"
else
  pass "guard rejects canonical serialization calls outside boundary modules"
fi

cat >"$tmp/crates/warp-core/src/lib.rs" <<'EOF'
use echo_wasm_abi::encode_cbor;

pub fn not_a_boundary(value: &u64) {
    let _ = encode_cbor(value);
}
EOF

if (cd "$tmp" && scripts/check-warp-core-serialization-boundaries.sh >/dev/null 2>&1); then
  fail "guard should reject direct serializer imports outside boundary modules"
else
  pass "guard rejects direct serializer imports outside boundary modules"
fi

cat >"$tmp/crates/warp-core/src/lib.rs" <<'EOF'
use echo_wasm_abi::{
    encode_cbor,
    pack_intent_v1,
};

pub fn not_a_boundary(value: &u64) {
    let _ = encode_cbor(value);
}
EOF

if (cd "$tmp" && scripts/check-warp-core-serialization-boundaries.sh >/dev/null 2>&1); then
  fail "guard should reject multiline serializer imports outside boundary modules"
else
  pass "guard rejects multiline serializer imports outside boundary modules"
fi

cat >"$tmp/crates/warp-core/src/lib.rs" <<'EOF'
use echo_wasm_abi as abi;

pub fn not_a_boundary(value: &u64) {
    let _ = abi::encode_cbor(value);
}
EOF

if (cd "$tmp" && scripts/check-warp-core-serialization-boundaries.sh >/dev/null 2>&1); then
  fail "guard should reject serializer aliases outside boundary modules"
else
  pass "guard rejects serializer aliases outside boundary modules"
fi

cat >"$tmp/crates/warp-core/src/lib.rs" <<'EOF'
pub fn ok() {}
EOF
cat >"$tmp/crates/warp-core/src/observation.rs" <<'EOF'
pub fn boundary(value: &u64) {
    let _ = echo_wasm_abi::encode_cbor(value);
}
EOF

if (cd "$tmp" && scripts/check-warp-core-serialization-boundaries.sh >/dev/null 2>&1); then
  pass "guard allows canonical serialization calls in boundary modules"
else
  fail "guard should allow canonical serialization calls in boundary modules"
fi

if (
  cd "$tmp" &&
    DETERMINISM_FORCE_NO_RG=1 scripts/check-warp-core-serialization-boundaries.sh >/dev/null 2>&1
); then
  pass "guard fallback works without ripgrep"
else
  fail "guard fallback should work without ripgrep"
fi

echo "warp-core serialization boundary hook tests: ${PASS} passed, ${FAIL} failed"
if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
