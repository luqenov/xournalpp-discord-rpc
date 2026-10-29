# xournalpp-discord-rpc

<div align="center">

[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Linux](https://img.shields.io/badge/Platform-Linux-4f46e5)](https://www.kernel.org/)
[![niri](https://img.shields.io/badge/Compositor-niri-0ea5e9)](https://github.com/YaLTeR/niri)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

</div>

Discord Rich Presence for Xournal++ on Linux.

This project shows your current activity in Discord as `Studying` and, when available, the active document name and session duration. It is designed for Linux desktops, with native support for `niri` window detection.

## Features

- Shows `Studying` in Discord when Xournal++ is open
- Displays the current document name when available
- Tracks elapsed time from the moment Xournal++ started
- Uses the Xournal++ window title exposed by `niri`
- Lightweight and runs in the background without extra dependencies

## Requirements

- Linux
- Xournal++ installed
- Discord or Vesktop running with "Share detected activity" enabled
- Rust and Cargo if you want to build it from source
- `niri` is preferred for reading the active document title

Without `niri`, the RPC still works, but the document name may not be available and the status falls back to `Studying`.

## Installation

```bash
git clone https://github.com/luqenov/xournalpp-discord-rpc.git
cd xournalpp-discord-rpc
cargo build --release
```

The binary will be available at:

```bash
target/release/xournal-rpc
```

## Usage

Run the binary:

```bash
./target/release/xournal-rpc
```

Then open Xournal++. The Discord status updates after a short delay, usually within about 15 seconds.

## niri autostart

Add this to your `~/.config/niri/config.kdl`:

```kdl
spawn-at-startup "/path/to/xournal-rpc"
```

## Notes

- The Xournal++ document name is visible to anyone who can see your Discord profile.
- If Discord is installed through Flatpak or Snap, you may need to create a symlink to the IPC socket, for example:

```bash
ln -sf "$XDG_RUNTIME_DIR/app/com.discordapp.Discord/discord-ipc-0" "$XDG_RUNTIME_DIR/discord-ipc-0"
```

## Custom Discord application

By default, this project uses the author's Discord application ID. If you want to use your own application, create one in the Discord developer portal, upload an asset named `xournalpp`, and then set a custom client ID.

Create a config file:

```bash
mkdir -p ~/.config/xournal-rpc
printf '%s\n' "YOUR_APPLICATION_ID" > ~/.config/xournal-rpc/client_id
```

You can also override it at runtime with:

```bash
export XOURNAL_RPC_CLIENT_ID="YOUR_APPLICATION_ID"
```

## License

This project is licensed under the MIT License. See the repository files for the full license text.
