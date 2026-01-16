# ADR-000Y: No Global State in Echo — Dependency Injection Only

- **Status:** Accepted
- **Date:** 2026-01-14

## Context

Global mutable state undermines determinism, testability, and provenance. It creates hidden initialization dependencies (“did `install_*` run?”), makes behavior environment-dependent, and complicates WASM vs native parity. Rust patterns like `OnceLock`/`LazyLock` are safe but still encode hidden global wiring.

Echo’s architecture benefits from explicit dependency graphs: runtime/kernel as values, ports as components, and deterministic construction.

## Decision

### 1) Global state is banned

Forbidden in Echo/WARP runtime and core crates:
- `OnceLock`, `LazyLock`, `lazy_static!`, `once_cell`, `thread_local!`
- `static mut`
- process-wide “install_*” patterns for runtime dependencies

Allowed:
- `const` and immutable `static` data (tables, magic bytes, version tags)
- pure functions and types

### 2) Dependencies are injected explicitly

All runtime dependencies must be carried by structs and passed explicitly:

- `EchoKernel { engine, registry, ingress, bus, … }`
- `MaterializationBus` lives inside the runtime
- Registry/codec providers are fields or type parameters (compile-time wiring preferred)

### 3) Enforced by tooling

We add a CI ban script that fails builds if forbidden patterns appear in protected crates. Exceptions require explicit allowlisting and justification.

## Consequences

- Determinism audits become simpler (no hidden wiring)
- Tests construct runtimes with explicit dependencies
- WASM and native implementations share the same dependency model
- Eliminates global init ordering bugs and accidental shared state across tools
  
## Appendix: Ban Global State

```bash
#!/usr/bin/env bash
set -euo pipefail

# Ban global state patterns in Echo/WARP core crates.
# Goal: no hidden wiring, no init-order dependency, no mutable process-wide state.
#
# Allowed:
#   - const
#   - immutable static data (e.g. magic bytes, lookup tables)
#
# Forbidden:
#   - OnceLock/LazyLock/lazy_static/once_cell/thread_local/static mut
#   - "install_*" global init patterns (heuristic)
#
# Usage:
#   ./scripts/ban-globals.sh
#
# Optional env:
#   BAN_GLOBALS_PATHS="crates/warp-core crates/warp-wasm crates/flyingrobots-echo-wasm"
#   BAN_GLOBALS_ALLOWLIST=".ban-globals-allowlist"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

PATHS_DEFAULT="crates/warp-core crates/warp-wasm crates/flyingrobots-echo-wasm crates/echo-wasm-abi"
PATHS="${BAN_GLOBALS_PATHS:-$PATHS_DEFAULT}"

ALLOWLIST="${BAN_GLOBALS_ALLOWLIST:-.ban-globals-allowlist}"

# ripgrep is fast and consistent
if ! command -v rg >/dev/null 2>&1; then
  echo "ERROR: ripgrep (rg) is required." >&2
  exit 2
fi

# Patterns are conservative on purpose.
# If you truly need an exception, add an allowlist line with a justification.
PATTERNS=(
  '\bOnceLock\b'
  '\bLazyLock\b'
  '\blazy_static!\b'
  '\bonce_cell\b'
  '\bthread_local!\b'
  '\bstatic mut\b'
  '\bunsafe\s*\{'              # optional: uncomment if you want "no unsafe" too
  '\binstall_[a-zA-Z0-9_]*\b'  # heuristic: discourages "install_registry" style globals
)

# You may want to allow `unsafe` in some crates; if so, delete the unsafe pattern above.

echo "ban-globals: scanning paths:"
for p in $PATHS; do echo "  - $p"; done
echo

# Build rg args
RG_ARGS=(--hidden --no-ignore --glob '!.git/*' --glob '!target/*' --glob '!**/node_modules/*')

# Apply allowlist as inverted matches (each line is a regex or fixed substring)
# Allowlist format:
#   <pattern>\t<justification>
# or:
#   <pattern>
ALLOW_RG_EXCLUDES=()
if [[ -f "$ALLOWLIST" ]]; then
  # Read first column (pattern) per line, ignore comments
  while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    [[ "$line" =~ ^# ]] && continue
    pat="${line%%$'\t'*}"
    pat="${pat%% *}" # also allow space-separated
    [[ -z "$pat" ]] && continue
    # Exclude lines matching allowlisted pattern
    ALLOW_RG_EXCLUDES+=(--glob "!$pat")
  done < "$ALLOWLIST"
fi

violations=0

for pat in "${PATTERNS[@]}"; do
  echo "Checking: $pat"
  # We can't "glob exclude by line"; allowlist is file-level. Keep it simple:
  # If you need surgical exceptions, prefer moving code or refactoring.
  if rg "${RG_ARGS[@]}" -n -S "$pat" $PATHS; then
    echo
    violations=$((violations+1))
  else
    echo "  OK"
  fi
  echo
done

if [[ $violations -ne 0 ]]; then
  echo "ban-globals: FAILED ($violations pattern group(s) matched)."
  echo "Fix the code or (rarely) justify an exception in $ALLOWLIST."
  exit 1
fi

echo "ban-globals: PASSED."
```

### Optional: Allow List File

```text
# Keep this tiny. If it grows, you’re lying to yourself.

# Example (file-level exclusion):
# crates/some-crate/src/vendor/*    vendored code, cannot change
```

### CI Script

```bash
./scripts/ban-globals.sh
```

## Appendix B: README Brag

```markdown
## Determinism Doctrine: No Global State

Echo forbids global mutable state. No init-once singletons, no hidden wiring, no process-wide “install_*” hooks.
All dependencies (registry, codecs, ports, buses) are injected explicitly via runtime structs.

Why: global state breaks provenance, complicates replay, and creates “it depends how you booted it” bugs.

Enforcement: `./scripts/ban-globals.sh` runs in CI and rejects forbidden patterns (`OnceLock`, `LazyLock`, `lazy_static`, `thread_local`, `static mut`, etc.).
See ADR-000Y: **No Global State**.
```