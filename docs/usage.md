# Usage and troubleshooting

```sh
xournal-rpc --help
xournal-rpc --dry-run --verbose
xournal-rpc --no-document
xournal-rpc --no-codeberg
```

`--dry-run` prints detection results without connecting to Discord. `--verbose`
prints backend and connection errors. Run without flags for normal operation.

## Configuration

Create `$XDG_CONFIG_HOME/xournal-rpc/config.toml` (normally
`~/.config/xournal-rpc/config.toml`):

```toml
interval = 15
show_document = false
show_codeberg = false
verbose = false
# client_id = "YOUR_DISCORD_APPLICATION_ID"
```

If you used the old extensionless `config` file, rename it to `config.toml`
and quote string values such as `client_id`.

Precedence, from highest to lowest: CLI flags, environment variables, config
file, legacy `xournal-rpc/client_id` file, built-in defaults. Environment variables
are `XOURNAL_RPC_CLIENT_ID`, `XOURNAL_RPC_INTERVAL`,
`XOURNAL_RPC_SHOW_DOCUMENT`, `XOURNAL_RPC_SHOW_CODEBERG` and `XOURNAL_RPC_VERBOSE`. The minimum interval is
2 seconds. Use TOML booleans `true` or `false` in the config file. Environment
variables also accept `yes/no`, `on/off` or `1/0`.

The Codeberg button is shown by default. Hide it with `--no-codeberg`,
`show_codeberg = false`, or `XOURNAL_RPC_SHOW_CODEBERG=false`. Use `--codeberg`
to show it even when disabled in the config file or environment.

## Troubleshooting

- No activity: enable activity sharing in your Discord client and check
  `xournal-rpc --verbose` for IPC connection failures.
- No document title: run `xournal-rpc --dry-run --verbose` in your desktop session.
  Ensure the matching command (`niri`, `hyprctl`, `swaymsg`, `i3-msg` or `xprop`)
  is installed and can access the compositor. X11 title detection currently
  requires the Xournal++ window to be focused.
- User service failure: inspect `journalctl --user -u xournal-rpc` and verify the
  unit's `ExecStart` matches the installed binary. Run
  `systemctl --user daemon-reload` after editing the unit.
- Missing compositor environment in systemd: import the relevant variables from
  your graphical session with `systemctl --user import-environment VARIABLE`.
- Document privacy: use `--no-document` or `show_document = false` before sharing
  your activity. Document titles may contain personal information.
