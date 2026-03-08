#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
set -euo pipefail

# Read the pinned channel from rust-toolchain.toml (e.g. "1.90.0")
TOOLCHAIN=$(grep '^channel' rust-toolchain.toml | sed 's/.*"\(.*\)".*/\1/')

echo "[devcontainer] Installing default toolchain (${TOOLCHAIN} via rust-toolchain.toml)..."
if ! command -v rustup >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 --retry 10 --retry-connrefused --location --silent --show-error --fail https://sh.rustup.rs | sh -s -- --default-toolchain none -y
  export PATH="$HOME/.cargo/bin:$PATH"
fi

rustup toolchain install "${TOOLCHAIN}" --profile minimal
# Do not override default; let rust-toolchain.toml control selection for this repo.
rustup component add --toolchain "${TOOLCHAIN}" rustfmt clippy || true
rustup target add --toolchain "${TOOLCHAIN}" wasm32-unknown-unknown || true

echo "[devcontainer] Priming cargo registry cache (optional)..."
cargo fetch || true

echo "[devcontainer] Done. Run 'cargo test -p warp-core' or 'make ci-local' to validate."
if [ -f Makefile ]; then
  echo "[devcontainer] Installing git hooks (make hooks)"
  make hooks || true
fi
