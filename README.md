# xournalpp-discord-rpc

<div align="center">

[![CI](https://github.com/luqenov/xournalpp-discord-rpc/actions/workflows/ci.yml/badge.svg?branch=dev)](https://github.com/luqenov/xournalpp-discord-rpc/actions/workflows/ci.yml)
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

### Arch Linux (AUR)

```bash
git clone https://aur.archlinux.org/xournal-rpc.git
cd xournal-rpc
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

## Usage

```bash
xournal-rpc
```

Open Xournal++ and the presence updates automatically.

### Options

| Option | Description |
| --- | --- |
| `-h, --help` | Show help |
| `-V, --version` | Show version |
| `-v, --verbose` | Print diagnostic messages |
| `--once` | Run one update cycle and exit |
| `--dry-run` | Print detected status without connecting to Discord |
| `--no-document` | Hide the document name |
| `--document` | Show the document name |
| `--client-id <ID>` | Use a custom Discord application |
| `--interval <SECS>` | Polling interval in seconds (default 15, minimum 2) |

### Configuration

Settings are applied in this order, with later sources overriding earlier ones:

1. `~/.config/xournal-rpc/client_id` (legacy)
2. `~/.config/xournal-rpc/config`
3. Environment variables
4. Command-line flags

`XDG_CONFIG_HOME` is respected.

Example `~/.config/xournal-rpc/config`:

```ini
client_id=1561027274977972367
interval=15
show_document=true
verbose=false
```

Environment variables: `XOURNAL_RPC_CLIENT_ID`, `XOURNAL_RPC_INTERVAL`, `XOURNAL_RPC_SHOW_DOCUMENT`, `XOURNAL_RPC_VERBOSE`.

### Custom Discord application

Create an application in the [Discord Developer Portal](https://discord.com/developers/applications), upload an asset named `xournalpp`, then set your client ID:

```bash
xournal-rpc --client-id YOUR_APPLICATION_ID
```

Or persist it in `~/.config/xournal-rpc/config`:

```ini
client_id=YOUR_APPLICATION_ID
```

## Troubleshooting

### Presence does not appear

- Enable **Share detected activity** in Discord settings.
- Restart Discord after installing or updating.
- Run `xournal-rpc --verbose` and check the output.
- Make sure only one instance is running.
- Flatpak and Snap clients are detected through their own IPC socket directories. If detection still fails, confirm Discord is running and the socket exists.

### Document name is missing

- Confirm your compositor tool is installed: `niri`, `hyprctl`, `swaymsg`/`i3-msg` or `xprop`.
- Unsaved documents are intentionally omitted.
- Use `--no-document` if you prefer to hide names entirely.

### systemd service cannot detect the compositor

The service needs the compositor environment. Start it from a session that exports `NIRI_SOCKET`, `HYPRLAND_INSTANCE_SIGNATURE`, `SWAYSOCK`, `I3SOCK` or `DISPLAY`, or add the relevant variables to the unit.

### Invalid client ID

Client IDs must be 17-20 digits. Validate one with:

```bash
xournal-rpc --dry-run --client-id YOUR_APPLICATION_ID
```

## Privacy

The presence is sent only to the local Discord client over its IPC socket. Document names are visible to anyone who can see your Discord profile while the presence is active. Use `--no-document` or `show_document=false` to hide them. This project collects no telemetry and makes no other network requests.

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[MIT](LICENSE)
