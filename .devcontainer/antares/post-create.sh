#!/usr/bin/env bash
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel 2>/dev/null)" || { echo "Codespace did not open at the repository root." >&2; exit 1; }
cd "$ROOT"

echo "[1/6] Installing Lapce build dependencies..."
sudo apt-get update
sudo apt-get install -y --no-install-recommends \
  clang pkg-config libssl-dev libfontconfig1-dev libfreetype6-dev \
  libxkbcommon-dev libxkbcommon-x11-dev libvulkan-dev libwayland-dev \
  libx11-xcb-dev libxcb-shape0-dev libxcb-xfixes0-dev xorg-dev

if [[ -f rust-toolchain.toml ]]; then
  echo "[2/6] Activating repository-pinned Rust toolchain..."
  rustup toolchain install 1.98.0 --profile minimal --component rustfmt --component clippy
  rustup override set 1.98.0
fi

mkdir -p "$HOME/.local/bin"
if ! grep -q 'HOME/.local/bin' "$HOME/.bashrc" 2>/dev/null; then
  echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.bashrc"
fi
export PATH="$HOME/.local/bin:$PATH"

echo "[3/6] Installing pinned Octomind sidecar..."
bash scripts/install-octomind.sh

echo "[4/6] Validating Antares scaffold..."
bash scripts/scaffold-check.sh

echo "[5/6] Resolving Antares crate and Cargo.lock..."
cargo check -p antares-agent

echo "[6/6] Running Antares crate checks..."
bash scripts/antares-ci.sh fast

printf '\nCodespace is ready. Review and commit Cargo.lock if it changed:\n'
git status --short
printf '\nNext: open docs/12-AGENT-MASTER-PROMPT.md in Antigravity and execute Phase 0.\n'
