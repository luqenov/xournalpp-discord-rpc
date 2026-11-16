# Usage and troubleshooting

```sh
xournal-rpc --help
xournal-rpc --init-config
xournal-rpc --dry-run
xournal-rpc --message "Reviewing mathematics" --no-timer
xournal-rpc --no-document --no-codeberg
xournal-rpc --only-when-focused
```

`--dry-run` previews the message, sharing settings, focus state and activity payload
without connecting to Discord. It also reports whether the activity would be
visible. `--verbose` reports startup, configuration path, window detection, reasons for
hiding activity, IPC socket attempts and server acknowledgements or rejections.
Activity updates are confirmed only after the client acknowledges `SET_ACTIVITY`.
Each IPC response has a five-second timeout; failed connections are retried with
backoff. An acknowledgement confirms local client acceptance, not profile visibility.

## Configuration

Run `xournal-rpc --init-config` to create a commented configuration. It never
overwrites an existing file. The path is `$XDG_CONFIG_HOME/xournal-rpc/config.toml`
(normally `~/.config/xournal-rpc/config.toml`). You can also create it manually:

```toml
message = "Reviewing mathematics"
interval = 15
show_document = true
show_codeberg = false
show_timer = true
only_when_focused = false
verbose = false
# client_id = "YOUR_DISCORD_APPLICATION_ID"
```

The defaults are `message = "Studying"`, `interval = 15`, all three `show_*`
settings enabled, and `only_when_focused` and `verbose` disabled. Messages must
contain 2-128 characters and cannot be blank. The minimum interval is 2 seconds.
Use TOML booleans `true` or `false` and quote strings, including `client_id`.
Invalid syntax, unknown settings and incorrect value types report an error.
If you used the old extensionless `config`, rename it to `config.toml` and quote
string values.

Settings reload automatically at the next update, without restarting. The current
polling interval (or connection retry delay) determines when changes take effect.
An invalid edit keeps the last valid settings and prints an error; fixing the file
resumes automatic updates. Removing the file restores defaults, with any remaining
environment variables and CLI flags still applied.

Precedence, from highest to lowest: CLI flags, environment variables, config file,
legacy `xournal-rpc/client_id` file, built-in defaults. CLI flags continue to override
the file after reloading. Environment variables are read from the running process;
changing a variable in another shell does not affect an already running instance.

| Config key | Environment variable | CLI override |
| --- | --- | --- |
| `message` | `XOURNAL_RPC_MESSAGE` | `--message "text"` |
| `interval` | `XOURNAL_RPC_INTERVAL` | `--interval 15` |
| `show_document` | `XOURNAL_RPC_SHOW_DOCUMENT` | `--document` / `--no-document` |
| `show_codeberg` | `XOURNAL_RPC_SHOW_CODEBERG` | `--codeberg` / `--no-codeberg` |
| `show_timer` | `XOURNAL_RPC_SHOW_TIMER` | `--timer` / `--no-timer` |
| `only_when_focused` | `XOURNAL_RPC_ONLY_WHEN_FOCUSED` | `--only-when-focused` / `--always` |
| `verbose` | `XOURNAL_RPC_VERBOSE` | `--verbose` |
| `client_id` | `XOURNAL_RPC_CLIENT_ID` | `--client-id ID` |

Boolean environment variables also accept `yes/no`, `on/off` or `1/0`.

## Focus and session time

Background hiding is optional and disabled by default. Enable it with
`only_when_focused = true` or `--only-when-focused`. The activity clears when
Xournal++ loses focus and returns when a Xournal++ window gains focus. With
multiple windows, the focused one is preferred. If focus cannot be detected,
the activity stays hidden while this option is enabled. Focus changes are checked
at each update, so hiding or showing can take up to the polling interval.

The timer measures time since Xournal++ started, including time spent in the
background. It does not reset when focus changes. Hide it with `show_timer = false`
or `--no-timer`.

## Troubleshooting

- IPC failures: the client must expose a `discord-ipc-*` Unix socket in the same
  user session (Equibop uses the same IPC protocol). WebSocket port 6463 alone is
  insufficient. `--verbose` prints the socket being tried and handshake errors.
  Discovery checks `XDG_RUNTIME_DIR`, temporary directories, `/run/user/<uid>` and
  common Flatpak/Snap paths; unavailable sockets do not block other candidates.
  If the desktop client runs in a sandbox, ensure the socket is accessible from
  the environment running `xournal-rpc`.
- No activity: enable activity sharing in your Discord client and check
  `xournal-rpc --verbose` for IPC connection failures. If background hiding is
  enabled, focus a Xournal++ window or use `--always`.
- No document title or focus: run `xournal-rpc --dry-run --verbose` in your desktop
  session. Ensure the matching command (`niri`, `hyprctl`, `swaymsg`, `i3-msg` or
  `xprop`) is installed and can access the compositor. X11 title detection requires
  the Xournal++ window to be focused.
- Invalid configuration: check the reported file path and TOML error. Quote string
  values, use unquoted booleans, and check setting names. `--help`, `--version`
  and `--init-config` work even when the config is invalid.
- User service failure: inspect `journalctl --user -u xournal-rpc` and verify the
  unit's `ExecStart` matches the installed binary (`~/.cargo/bin/xournal-rpc` for
  Cargo installs). Run `systemctl --user daemon-reload` after editing the unit.
- Missing compositor environment in systemd: import the relevant variables from
  your graphical session with `systemctl --user import-environment VARIABLE`.
- Document privacy: use `--no-document` or `show_document = false` before sharing
  your activity. Document titles may contain personal information.
