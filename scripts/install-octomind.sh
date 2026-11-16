#!/usr/bin/env bash
set -euo pipefail

VERSION="0.72.0"
DEST="${OCTOMIND_INSTALL_DIR:-$HOME/.local/bin}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

case "$(uname -s):$(uname -m)" in
  Linux:x86_64)
    ASSET="octomind-${VERSION}-x86_64-unknown-linux-musl.tar.gz"
    SHA="20fe3d901ebd21387065e448f067177c36daf515361bb36079657cf1de066913"
    ;;
  Linux:aarch64|Linux:arm64)
    ASSET="octomind-${VERSION}-aarch64-unknown-linux-musl.tar.gz"
    SHA="df71324133f2e32b7b6e40893345795764ec6305e20dec2732e8e0f16bbd3fd7"
    ;;
  *)
    echo "Automatic installer currently supports Linux x86_64/aarch64 only." >&2
    echo "See docs/04-OCTOMIND-ACP.md for pinned release information." >&2
    exit 2
    ;;
esac

URL="https://github.com/Muvon/octomind/releases/download/${VERSION}/${ASSET}"
curl --fail --location --retry 3 "$URL" --output "$TMP/$ASSET"
printf '%s  %s\n' "$SHA" "$TMP/$ASSET" | sha256sum --check --status || {
  echo "Octomind archive checksum mismatch; refusing installation." >&2
  exit 1
}
mkdir -p "$TMP/unpacked" "$DEST"
tar -xzf "$TMP/$ASSET" -C "$TMP/unpacked"
BIN="$(find "$TMP/unpacked" -type f -name octomind -perm -u+x -print -quit)"
if [[ -z "$BIN" ]]; then
  echo "The pinned archive did not contain an executable named octomind." >&2
  exit 1
fi
install -m 0755 "$BIN" "$DEST/octomind"
printf 'Installed Octomind %s to %s\n' "$VERSION" "$DEST/octomind"
printf 'Ensure %s is on PATH, then run: octomind --help\n' "$DEST"
