#!/usr/bin/env bash
set -euo pipefail

echo "[devcontainer] Installing MSRV toolchain (1.68.0) and common targets..."
if ! command -v rustup >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 --retry 10 --retry-connrefused --location --silent --show-error --fail https://sh.rustup.rs | sh -s -- --default-toolchain none -y
  export PATH="$HOME/.cargo/bin:$PATH"
fi

rustup toolchain install 1.68.0 --profile minimal
rustup default stable
rustup component add rustfmt clippy
rustup target add wasm32-unknown-unknown

echo "[devcontainer] Priming cargo registry cache (optional)..."
cargo fetch || true

echo "[devcontainer] Done. Run 'cargo test -p rmg-core' or 'make ci-local' to validate."

