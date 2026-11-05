# xournalpp-discord-rpc

<div align="center">

[![CI](https://github.com/luqenov/xournalpp-discord-rpc/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/luqenov/xournalpp-discord-rpc/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/luqenov/xournalpp-discord-rpc)](https://github.com/luqenov/xournalpp-discord-rpc/releases)
[![License](https://img.shields.io/github/license/luqenov/xournalpp-discord-rpc)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.71%2B-orange?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-linux-4f46e5?logo=linux&logoColor=white)](#requirements)

</div>

Discord Rich Presence for Xournal++ on Linux. Shows **Studying** in your Discord profile while Xournal++ is open, with the current document name and session time when available.

## Features

- Shows `Studying` with the active document name
- Session timer based on when Xournal++ started
- Supports niri, Hyprland, Sway/i3 and X11
- Prefers the focused Xournal++ window when several are open
- Optional document name sharing
- CLI, config file and environment variable configuration
- Reconnects automatically when Discord restarts

## Requirements

- Linux
- Xournal++ installed
- Discord, Vesktop or another compatible client running with activity sharing enabled
- One of:
  - niri (`niri`)
  - Hyprland (`hyprctl`)
  - Sway/i3 (`swaymsg` or `i3-msg`)
  - X11 (`xprop`)

Rust 1.71+ is only required when building from source.

## Installation

### Prebuilt binary

Download the archive for your architecture from [Releases](https://github.com/luqenov/xournalpp-discord-rpc/releases), verify it against `SHA256SUMS.txt`, and install:

```bash
tar -xzf xournal-rpc-v*-linux-x86_64.tar.gz
sudo install -Dm755 xournal-rpc /usr/local/bin/xournal-rpc
```

Use the `aarch64` archive on ARM64.

### Cargo

```bash
cargo install --git https://github.com/luqenov/xournalpp-discord-rpc --locked
```

### Arch Linux

A `PKGBUILD` is provided in [`packaging/`](packaging/):

```bash
git clone https://github.com/luqenov/xournalpp-discord-rpc.git
cd xournalpp-discord-rpc/packaging
makepkg -si
systemctl --user enable --now xournal-rpc
```

### Nix

```bash
nix run github:luqenov/xournalpp-discord-rpc
```

Or use the overlay:

```nix
{
  inputs.xournalpp-discord-rpc.url = "github:luqenov/xournalpp-discord-rpc";
}
```

### systemd user service

A unit file is available at [`packaging/xournal-rpc.service`](packaging/xournal-rpc.service). For a manual install:

```bash
mkdir -p ~/.config/systemd/user
cp packaging/xournal-rpc.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now xournal-rpc
```

If the binary is not at `/usr/bin/xournal-rpc`, update `ExecStart` first.

## Usage and troubleshooting

Usage, configuration and troubleshooting guides are available in the [Wiki](https://github.com/luqenov/xournalpp-discord-rpc/wiki).

## Privacy

The presence is sent only to the local Discord client over its IPC socket. Document names are visible to anyone who can see your Discord profile while the presence is active. Use `--no-document` or `show_document=false` to hide them. This project collects no telemetry and makes no other network requests.

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md).
