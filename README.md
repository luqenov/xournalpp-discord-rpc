# xournalpp-discord-rpc

<div align="center">

[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Linux](https://img.shields.io/badge/Platform-Linux-4f46e5)](https://www.kernel.org/)
[![niri](https://img.shields.io/badge/Compositor-niri-0ea5e9)](https://github.com/YaLTeR/niri)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

</div>

Discord Rich Presence for Xournal++ on Linux.

This project updates your Discord status to show when you are studying in Xournal++, including the active document name and elapsed session time when available.

## Features

- Shows `Studying` while Xournal++ is open
- Displays the current document name when supported
- Tracks how long the session has been active
- Uses `niri` window metadata for document detection
- Minimal runtime footprint and simple setup

## Requirements

- Linux
- Xournal++ installed
- Discord or Vesktop running with “Share detected activity” enabled
- Rust and Cargo if you want to build from source
- `niri` recommended for reading the active document title

Without `niri`, the status still works, but the document name may not be available and the presence falls back to `Studying`.

## Installation

```bash
git clone https://github.com/luqenov/xournalpp-discord-rpc.git
cd xournalpp-discord-rpc
cargo build --release
```

The binary will be created at:

```bash
target/release/xournal-rpc
```

## Usage

```bash
./target/release/xournal-rpc
```

Open Xournal++ and the Discord status will update automatically after a short delay.

## Autostart with niri

Add this to your `~/.config/niri/config.kdl`:

```kdl
spawn-at-startup "/path/to/xournal-rpc"
```

## Notes

- The document name is visible to anyone who can see your Discord profile.
- If Discord is installed via Flatpak or Snap, you may need a symlink to the IPC socket:

```bash
ln -sf "$XDG_RUNTIME_DIR/app/com.discordapp.Discord/discord-ipc-0" "$XDG_RUNTIME_DIR/discord-ipc-0"
```

## Custom Discord app

By default this project uses the author’s Discord application ID. If you want to use your own app, create one in the Discord developer portal, upload an asset named `xournalpp`, and set a custom client ID.

Create a config file:

```bash
mkdir -p ~/.config/xournal-rpc
printf '%s\n' "YOUR_APPLICATION_ID" > ~/.config/xournal-rpc/client_id
```

You can also override it at runtime:

```bash
export XOURNAL_RPC_CLIENT_ID="YOUR_APPLICATION_ID"
```
