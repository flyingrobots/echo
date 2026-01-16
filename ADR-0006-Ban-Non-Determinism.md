<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
## T2000 on 'em

We already have the **ban-globals** drill sergeant. Now we add the rest of the “you will cry” suite: 

- **ban nondeterministic APIs**
- **ban unordered containers in ABI-ish structs**
- **ban time/rand/JSON**
- **fail CI hard**.

Below is a clean, repo-friendly setup.

```bash
#!/usr/bin/env bash
set -euo pipefail

# Determinism Drill Sergeant: ban nondeterministic APIs and patterns
#
# Usage:
#   ./scripts/ban-nondeterminism.sh
#
# Optional env:
#   DETERMINISM_PATHS="crates/warp-core crates/warp-wasm crates/flyingrobots-echo-wasm"
#   DETERMINISM_ALLOWLIST=".ban-nondeterminism-allowlist"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if ! command -v rg >/dev/null 2>&1; then
  echo "ERROR: ripgrep (rg) is required." >&2
  exit 2
fi

PATHS_DEFAULT="crates/warp-core crates/warp-wasm crates/flyingrobots-echo-wasm crates/echo-wasm-abi"
PATHS="${DETERMINISM_PATHS:-$PATHS_DEFAULT}"

ALLOWLIST="${DETERMINISM_ALLOWLIST:-.ban-nondeterminism-allowlist}"

RG_ARGS=(--hidden --no-ignore --glob '!.git/*' --glob '!target/*' --glob '!**/node_modules/*')

# You can allow file-level exceptions via allowlist (keep it tiny).
ALLOW_GLOBS=()
if [[ -f "$ALLOWLIST" ]]; then
  while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    [[ "$line" =~ ^# ]] && continue
    pat="${line%%$'\t'*}"
    pat="${pat%% *}"
    [[ -z "$pat" ]] && continue
    ALLOW_GLOBS+=(--glob "!$pat")
  done < "$ALLOWLIST"
fi

# Patterns: conservative and intentionally annoying.
# If you hit a false positive, refactor; don't immediately allowlist.
PATTERNS=(
  # Time / entropy (core determinism killers)
  '\bstd::time::SystemTime\b'
  '\bSystemTime::now\b'
  '\bstd::time::Instant\b'
  '\bInstant::now\b'
  '\bstd::thread::sleep\b'
  '\b(tokio|async_std)::time::sleep\b'

  # Randomness
  '\brand::\b'
  '\bgetrandom::\b'
  '\bfastrand::\b'

  # Unordered containers that will betray you if they cross a boundary
  '\bstd::collections::HashMap\b'
  '\bstd::collections::HashSet\b'
  '\bhashbrown::HashMap\b'
  '\bhashbrown::HashSet\b'

  # JSON & “helpful” serialization in core paths
  '\bserde_json::\b'
  '\bserde_wasm_bindgen::\b'

  # Float nondeterminism hotspots (you can tune these)
  '\b(f32|f64)::NAN\b'
  '\b(f32|f64)::INFINITY\b'
  '\b(f32|f64)::NEG_INFINITY\b'
  '\.sin\('
  '\.cos\('
  '\.tan\('
  '\.sqrt\('
  '\.pow[f]?\('

  # Host/environment variability
  '\bstd::env::\b'
  '\bstd::fs::\b'
  '\bstd::process::\b'

  # Concurrency primitives (optional—uncomment if you want core to be single-thread-only)
  # '\bstd::sync::Mutex\b'
  # '\bstd::sync::RwLock\b'
  # '\bstd::sync::atomic::\b'
)

echo "ban-nondeterminism: scanning paths:"
for p in $PATHS; do echo "  - $p"; done
echo

violations=0
for pat in "${PATTERNS[@]}"; do
  echo "Checking: $pat"
  if rg "${RG_ARGS[@]}" "${ALLOW_GLOBS[@]}" -n -S "$pat" $PATHS; then
    echo
    violations=$((violations+1))
  else
    echo "  OK"
  fi
  echo
done

if [[ $violations -ne 0 ]]; then
  echo "ban-nondeterminism: FAILED ($violations pattern group(s) matched)."
  echo "Fix the code or (rarely) justify an exception in $ALLOWLIST."
  exit 1
fi

echo "ban-nondeterminism: PASSED."
##
```

### Optional Allow-List

```text
# Example:
# crates/some-crate/tests/*    tests can use time/rand, not core
```

## ABI Police

### `scripts/ban-unordered-abi.sh`

This one is narrower: 

- **ban HashMap/HashSet inside anything that looks like ABI/codec/message structs**.

It’s opinionated: if it’s named like it crosses a boundary, it must be ordered.

```bash
#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if ! command -v rg >/dev/null 2>&1; then
  echo "ERROR: ripgrep (rg) is required." >&2
  exit 2
fi

# Adjust these to your repo conventions
ABI_HINTS=(
  "abi"
  "codec"
  "message"
  "frame"
  "packet"
  "envelope"
  "dto"
  "wire"
)

RG_ARGS=(--hidden --no-ignore --glob '!.git/*' --glob '!target/*' --glob '!**/node_modules/*')

# Find Rust files likely involved in ABI/wire formats.
files=$(rg "${RG_ARGS[@]}" -l -g'*.rs' "$(printf '%s|' "${ABI_HINTS[@]}")" crates/ || true)

if [[ -z "${files}" ]]; then
  echo "ban-unordered-abi: no ABI-ish files found (by heuristic). OK."
  exit 0
fi

echo "ban-unordered-abi: scanning ABI-ish Rust files..."
violations=0

# HashMap/HashSet are not allowed in ABI-ish types. Use Vec<(K,V)> sorted, BTreeMap, IndexMap with explicit canonicalization, etc.
if rg "${RG_ARGS[@]}" -n -S '\b(HashMap|HashSet)\b' $files; then
  violations=$((violations+1))
fi

if [[ $violations -ne 0 ]]; then
  echo "ban-unordered-abi: FAILED. Unordered containers found in ABI-ish code."
  echo "Fix by using Vec pairs + sorting, or BTreeMap + explicit canonical encode ordering."
  exit 1
fi

echo "ban-unordered-abi: PASSED."
```

## CI Wire It In

```bash
./scripts/ban-globals.sh
./scripts/ban-nondeterminism.sh
./scripts/ban-unordered-abi.sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## README Brag

```markdown
## Determinism Drill Sergeant (Non-Negotiable)

Echo is a deterministic system. We enforce this with automated bans.

- **No global state**: no `OnceLock`, `LazyLock`, `lazy_static`, `thread_local`, `static mut`, or `install_*` singletons.
  - Enforced by: `./scripts/ban-globals.sh`
- **No nondeterminism in core**: no time, randomness, JSON convenience layers, unordered ABI containers, or host-environment dependencies in protected crates.
  - Enforced by: `./scripts/ban-nondeterminism.sh` and `./scripts/ban-unordered-abi.sh`

If your change trips these scripts, the fix is not “add an allowlist line.”
The fix is **refactor the design** so determinism is true by construction.
```
