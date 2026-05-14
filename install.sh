#!/usr/bin/env sh
set -e

REPO="kodaskills/zellig"
BIN="zellig"
if [ -n "${VERSION:-}" ]; then
  RELEASES="https://github.com/${REPO}/releases/download/v${VERSION}"
else
  RELEASES="https://github.com/${REPO}/releases/latest/download"
fi

# ── Detect OS ────────────────────────────────────────────────────────────────
os=$(uname -s)
case "$os" in
  Linux)  os_name="linux"  ;;
  Darwin) os_name="macos"  ;;
  *)
    echo "Unsupported OS: $os"
    echo "Download manually: https://github.com/${REPO}/releases/latest"
    exit 1
    ;;
esac

# ── Detect arch ──────────────────────────────────────────────────────────────
arch=$(uname -m)
case "$arch" in
  x86_64)          arch_name="x86_64"  ;;
  arm64 | aarch64) arch_name="aarch64" ;;
  *)
    echo "Unsupported architecture: $arch"
    echo "Download manually: https://github.com/${REPO}/releases/latest"
    exit 1
    ;;
esac

# ── Resolve install dir ───────────────────────────────────────────────────────
if [ -n "${INSTALL_DIR:-}" ]; then
  install_dir="$INSTALL_DIR"
  mkdir -p "$install_dir"
elif [ -w /usr/local/bin ]; then
  install_dir="/usr/local/bin"
elif [ -d "$HOME/.local/bin" ]; then
  install_dir="$HOME/.local/bin"
else
  install_dir="$HOME/.local/bin"
  mkdir -p "$install_dir"
fi

# ── Download & install ────────────────────────────────────────────────────────
artifact="${BIN}-${os_name}-${arch_name}.tar.gz"
url="${RELEASES}/${artifact}"
checksum_url="${RELEASES}/${artifact}.sha256"
tmp=$(mktemp -d)

download() {
  if command -v curl > /dev/null 2>&1; then
    curl -fsSL "$1" -o "$2"
  elif command -v wget > /dev/null 2>&1; then
    wget -q "$1" -O "$2"
  else
    echo "curl or wget required"
    exit 1
  fi
}

echo "Downloading ${artifact}..."
download "$url" "${tmp}/${artifact}"
download "$checksum_url" "${tmp}/${artifact}.sha256"

echo "Verifying checksum..."
expected=$(cat "${tmp}/${artifact}.sha256" | awk '{print $1}')
if command -v sha256sum > /dev/null 2>&1; then
  actual=$(sha256sum "${tmp}/${artifact}" | awk '{print $1}')
elif command -v shasum > /dev/null 2>&1; then
  actual=$(shasum -a 256 "${tmp}/${artifact}" | awk '{print $1}')
else
  echo "Warning: sha256sum/shasum not found, skipping checksum verification"
  actual="$expected"
fi
if [ "$actual" != "$expected" ]; then
  echo "Checksum mismatch! Expected: $expected  Got: $actual"
  rm -rf "$tmp"
  exit 1
fi
echo "Checksum OK"

tar -xzf "${tmp}/${artifact}" -C "$tmp"
install -m 755 "${tmp}/${BIN}" "${install_dir}/${BIN}"
rm -rf "$tmp"

echo "Installed ${BIN} to ${install_dir}/${BIN}"

# ── Linux runtime dep check ───────────────────────────────────────────────────
if [ "$os_name" = "linux" ]; then
  if ! (ldconfig -p 2>/dev/null | grep -q "libgomp") && \
     ! find /usr/lib /usr/lib64 /lib /lib64 -name "libgomp.so*" 2>/dev/null | grep -q .; then
    echo ""
    echo "Warning: libgomp.so.1 not found (required by the local translation backend)."
    echo "Install it with:"
    echo "  Debian/Ubuntu:  sudo apt-get install libgomp1"
    echo "  RHEL/Fedora:    sudo dnf install libgomp"
    echo "  Arch:           sudo pacman -S gcc-libs"
  fi
fi

# ── PATH hint if needed ───────────────────────────────────────────────────────
case ":$PATH:" in
  *":${install_dir}:"*) ;;
  *)
    echo ""
    echo "Add ${install_dir} to your PATH:"
    echo "  export PATH=\"${install_dir}:\$PATH\""
    ;;
esac

echo ""
${install_dir}/${BIN} --version
