# cosmic-clipboard-manager

A lightweight, fast clipboard history manager built in Rust for Pop OS / COSMIC and other Linux desktops.

[![Release](https://img.shields.io/github/v/release/135meherab/cosmic-clipboard-manager)](https://github.com/135meherab/cosmic-clipboard-manager/releases/latest)
## Features

- **Text & image history** — captures everything you copy
- **Pinned clips** — pin important entries so they never expire
- **Search** — instantly filter through your history
- **System tray icon** — lives quietly in your panel
- **Global hotkey** — open with Super+V from anywhere
- **100-entry history** stored in a local SQLite database
- **Dark theme** UI built with [iced](https://github.com/iced-rs/iced)

## Install

### Option 1 — Download `.deb` (easiest)

Go to the [Releases page](https://github.com/135meherab/cosmic-clipboard-manager/releases/latest) and download the latest `.deb`, then:

```bash
sudo apt install ./clipmgr_v*.deb
```

clipmgr will start automatically on your next login. To start it now:

```bash
clipmgr &
```

### Option 2 — Build from source

**Prerequisites:**
```bash
sudo apt install -y \
  libdbus-1-dev pkg-config \
  libwayland-dev libxkbcommon-dev \
  libvulkan-dev libgl-dev libegl-dev \
  libxi-dev libxtst-dev libx11-dev
```

**Build & install:**
```bash
git clone https://github.com/135meherab/cosmic-clipboard-manager.git
cd cosmic-clipboard-manager
./build-deb.sh
sudo apt install ./target/debian/clipmgr_*.deb
```

## Usage

| Action | How |
|--------|-----|
| Open clipboard window | Click the tray icon in the panel |
| Open with keyboard | **Super+V** |
| Open from terminal | `clipmgr --show` |
| Copy an entry | Click **Copy** next to any item |
| Pin an entry | Click **Pin** — pinned items never expire |
| Delete an entry | Click **✕** |
| Clear all history | Click **Clear All** in the window header |

### Super+V on COSMIC Wayland

If the global hotkey doesn't register automatically (Wayland security restriction), add it manually:

> **Settings → Keyboard → Custom Shortcuts → Add**
> - Name: `Clipboard Manager`
> - Command: `clipmgr --show`
> - Shortcut: `Super+V`

## Uninstall

```bash
sudo apt remove clipmgr
```

## System requirements

- Ubuntu 22.04+ / Pop OS 22.04+ (amd64)
- Wayland or X11

## Contributing

Pull requests are welcome. For major changes, please open an issue first.

```bash
git clone https://github.com/135meherab/cosmic-clipboard-manager.git
cd cosmic-clipboard-manager
source ~/.cargo/env
cargo check
cargo run
```
