#!/usr/bin/env bash
set -euo pipefail

echo "[devcontainer] Installing MSRV toolchain (1.68.0) and respecting rust-toolchain.toml..."
if ! command -v rustup >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 --retry 10 --retry-connrefused --location --silent --show-error --fail https://sh.rustup.rs | sh -s -- --default-toolchain none -y
  export PATH="$HOME/.cargo/bin:$PATH"
fi

rustup toolchain install 1.68.0 --profile minimal
# Do not override default; let rust-toolchain.toml control toolchain selection for this repo.
# Install optional newer toolchain for local convenience (kept as non-default).
rustup toolchain install 1.90.0 --profile minimal || true
# Ensure components/targets are available for the active (rust-toolchain.toml) toolchain.
rustup component add rustfmt clippy || true
rustup target add wasm32-unknown-unknown || true

echo "[devcontainer] Priming cargo registry cache (optional)..."
cargo fetch || true

echo "[devcontainer] Done. Run 'cargo test -p rmg-core' or 'make ci-local' to validate."
