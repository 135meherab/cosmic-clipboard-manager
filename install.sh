#!/usr/bin/env bash
set -e

echo "→ Building clipmgr (release)…"
cargo build --release

echo "→ Installing binary to ~/.local/bin/clipmgr"
mkdir -p "$HOME/.local/bin"
cp target/release/clipmgr "$HOME/.local/bin/clipmgr"

echo "→ Installing desktop entry…"
mkdir -p "$HOME/.local/share/applications"
cp desktop/clipmgr.desktop "$HOME/.local/share/applications/"

echo "→ Installing autostart entry…"
mkdir -p "$HOME/.config/autostart"
cp desktop/clipmgr-autostart.desktop "$HOME/.config/autostart/"

# Make sure ~/.local/bin is in PATH
if ! echo "$PATH" | grep -q "$HOME/.local/bin"; then
    echo ""
    echo "⚠  Add ~/.local/bin to your PATH by adding this line to ~/.bashrc or ~/.profile:"
    echo "   export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

echo ""
echo "✓ Done! clipmgr is installed."
echo ""
echo "To start now:    clipmgr &"
echo "To open window:  clipmgr --show"
echo ""
echo "On COSMIC Wayland, if Super+V hotkey doesn't register automatically:"
echo "  Settings → Keyboard → Custom Shortcuts → Add:"
echo "    Name: Clipboard Manager"
echo "    Command: clipmgr --show"
echo "    Shortcut: Super+V"
