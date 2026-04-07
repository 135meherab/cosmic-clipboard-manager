#!/usr/bin/env bash
set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo " clipmgr — .deb package builder"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Ensure cargo is available
source "$HOME/.cargo/env" 2>/dev/null || true

# Install cargo-deb if not already present
if ! cargo deb --version &>/dev/null 2>&1; then
    echo "→ Installing cargo-deb…"
    cargo install cargo-deb
fi

# Install required system build dependencies
echo "→ Checking system build dependencies…"
MISSING=()
for pkg in libdbus-1-dev pkg-config libwayland-dev libxkbcommon-dev libvulkan-dev libgl-dev libegl-dev; do
    dpkg -s "$pkg" &>/dev/null || MISSING+=("$pkg")
done

if [ ${#MISSING[@]} -gt 0 ]; then
    echo "→ Installing missing packages: ${MISSING[*]}"
    sudo apt install -y "${MISSING[@]}"
fi

# Build the .deb
echo "→ Building release binary + .deb package…"
cargo deb

# Find the output file
DEB=$(ls target/debian/clipmgr_*.deb | tail -1)
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✓ Package built: $DEB"
echo ""
echo "Install on this machine:"
echo "  sudo dpkg -i $DEB"
echo ""
echo "Share with others — they install with:"
echo "  sudo dpkg -i clipmgr_*.deb"
echo ""
echo "Uninstall anytime:"
echo "  sudo apt remove clipmgr"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
