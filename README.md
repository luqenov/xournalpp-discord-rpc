# xournalpp-discord-rpc

<div align="center">

[Releases](https://codeberg.org/luqenov/xournalpp-discord-rpc/releases) · [MIT License](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.71%2B-orange?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-linux-4f46e5?logo=linux&logoColor=white)](#requirements)

</div>

Discord Rich Presence for Xournal++ on Linux. Shows a customizable message (default: **Studying**) in your Discord profile while Xournal++ is open, with the current document name and session time when available.

## Features

- Optional custom activity message (`--no-message` hides it) with the active document name
- Optional session timer based on when Xournal++ started
- Supports niri, Hyprland, Sway/i3 and X11
- Prefers the focused Xournal++ window when several are open
- Optional document name sharing and Codeberg button
- Optional activity hiding while Xournal++ is in the background
- Automatic configuration reload and a preview with `--dry-run`
- Commented TOML configuration generated with `--init-config`
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

Download the archive for your architecture from [Releases](https://codeberg.org/luqenov/xournalpp-discord-rpc/releases), download `SHA256SUMS.txt` into the same directory, verify the checksum, and install:

```bash
sha256sum --ignore-missing -c SHA256SUMS.txt
tar -xzf xournal-rpc-v*-linux-x86_64.tar.gz
sudo install -Dm755 xournal-rpc /usr/local/bin/xournal-rpc
```

Use the `aarch64` archive on ARM64.

### Cargo

```bash
cargo install --git https://codeberg.org/luqenov/xournalpp-discord-rpc --locked
```

### Arch Linux

A `PKGBUILD` is provided in [`packaging/`](packaging/):

```bash
git clone https://codeberg.org/luqenov/xournalpp-discord-rpc.git
cd xournalpp-discord-rpc/packaging
makepkg -si
systemctl --user enable --now xournal-rpc
```

### Nix

```bash
nix run git+https://codeberg.org/luqenov/xournalpp-discord-rpc
```

For a persistent installation, including use with the systemd service:

```bash
nix profile install git+https://codeberg.org/luqenov/xournalpp-discord-rpc
```

Or use the overlay:

```nix
{
  inputs.xournalpp-discord-rpc.url = "git+https://codeberg.org/luqenov/xournalpp-discord-rpc";
}
```

### systemd user service

A unit file is available at [`packaging/xournal-rpc.service`](packaging/xournal-rpc.service).
The Arch package installs it automatically. For other installation methods, run
the following from a checkout of this repository after installing the binary:

```bash
sh scripts/install-user-service.sh
systemctl --user daemon-reload
systemctl --user enable --now xournal-rpc
```

The installer sets `ExecStart` to the absolute path returned by
`command -v xournal-rpc`, covering `/usr/local/bin`, Cargo and Nix profile
installations without depending on systemd's `PATH`. Ensure the installed binary's
directory is in your shell's `PATH` first. `nix run` alone does not install a
persistent executable for the service.

If you move the binary or change installation methods, rerun the installer and
`systemctl --user daemon-reload`, then `systemctl --user restart xournal-rpc`.

## Usage and troubleshooting

See the [usage and troubleshooting guide](docs/usage.md).

## Privacy

The presence is sent only to the local Discord client over its IPC socket. Document names are visible to anyone who can see your Discord profile while the presence is active. Use `--no-document` or `show_document=false` to hide them. This project collects no telemetry and makes no other network requests.

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md).
