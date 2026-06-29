#!/usr/bin/env sh
set -eu

REPO="${PX_REPO:-BahiHabash/px-cli}"
TAG="${PX_VERSION:-v2}"
INSTALL_DIR="${PX_INSTALL_DIR:-$HOME/.local/bin}"

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Darwin)
    case "$arch" in
      arm64|aarch64) asset="px-mac-silicon" ;;
      x86_64|amd64) asset="px-mac-intel" ;;
      *) echo "Unsupported macOS architecture: $arch" >&2; exit 1 ;;
    esac
    ;;
  Linux)
    case "$arch" in
      x86_64|amd64) asset="px-linux" ;;
      *) echo "Unsupported Linux architecture: $arch" >&2; exit 1 ;;
    esac
    ;;
  *)
    echo "Unsupported OS: $os" >&2
    exit 1
    ;;
esac

url="https://github.com/$REPO/releases/download/$TAG/$asset"
tmp="${TMPDIR:-/tmp}/px-install-$$"

mkdir -p "$INSTALL_DIR"
trap 'rm -f "$tmp"' EXIT INT TERM

echo "Downloading $url"
curl -fsSL "$url" -o "$tmp"
chmod +x "$tmp"
mv "$tmp" "$INSTALL_DIR/px"

echo "Installed px to $INSTALL_DIR/px"
if command -v px >/dev/null 2>&1; then
  px --version
else
  echo "Add $INSTALL_DIR to your PATH, then run: px --version"
fi
