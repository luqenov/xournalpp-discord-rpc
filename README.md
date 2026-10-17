# xournalpp-discord-rpc

Discord Rich Presence for [Xournal++](https://xournalpp.github.io/) on Linux, written in Rust.

Shows `Studying (file name)` and how long Xournal++ has been open.

## Requirements

- Linux with the [niri](https://github.com/YaLTeR/niri) compositor (the document title is read with `niri msg`)
- Discord (or Vesktop) running, with "Share detected activity" enabled
- Rust and Cargo, only if you want to build it yourself

Without niri the presence still works, but it only shows `Studying` without the file name.

## Build

```bash
git clone https://github.com/luqenov/xournalpp-discord-rpc.git
cd xournalpp-discord-rpc
cargo build --release
```

The binary will be at `target/release/xournal-rpc`.

## Run

```bash
./target/release/xournal-rpc
```

Open Xournal++ and the status appears in about 15 seconds.

## Autostart with niri

Add this to `~/.config/niri/config.kdl`:

```kdl
spawn-at-startup "/path/to/xournal-rpc"
```

## Notes

- The file name is visible to anyone who can see your profile.
- Discord installed through Flatpak or Snap may need a symlink to the IPC socket, for example:
  `ln -sf $XDG_RUNTIME_DIR/app/com.discordapp.Discord/discord-ipc-0 $XDG_RUNTIME_DIR/discord-ipc-0`

## Using your own Discord app

By default it uses the author's Discord application. To use your own, create one at https://discord.com/developers/applications, upload an art asset named `xournalpp`, and set the Application ID with either:

```bash
mkdir -p ~/.config/xournal-rpc
echo "YOUR_APPLICATION_ID" > ~/.config/xournal-rpc/client_id
```

or the environment variable `XOURNAL_RPC_CLIENT_ID`.
