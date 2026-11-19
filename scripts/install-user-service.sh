#!/bin/sh
set -eu

binary=$(command -v xournal-rpc) || {
    echo 'xournal-rpc is not on PATH. Install it and add its bin directory to PATH first.' >&2
    exit 1
}
case "$binary" in
    /*) ;;
    *) echo 'Expected an absolute executable path for xournal-rpc.' >&2; exit 1 ;;
esac
case "$binary" in
    *'"'*|*\\*|*'$'*|*'
'*)
        echo 'The executable path contains characters unsupported by systemd ExecStart.' >&2
        exit 1
        ;;
esac

script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
unit_dir=${XDG_CONFIG_HOME:-"$HOME/.config"}/systemd/user
# Escape systemd specifiers, then sed replacement syntax.
escaped_binary=$(printf '%s\n' "$binary" |
    sed 's/%/%%/g' |
    sed 's/[\\&|]/\\&/g')
mkdir -p "$unit_dir"
sed "s|^ExecStart=.*|ExecStart=\"$escaped_binary\"|" \
    "$script_dir/../packaging/xournal-rpc.service" > "$unit_dir/xournal-rpc.service"
printf 'Installed %s using %s\n' "$unit_dir/xournal-rpc.service" "$binary"
printf '%s\n' 'Run systemctl --user daemon-reload && systemctl --user enable --now xournal-rpc'
printf '%s\n' 'If the service is already running, run systemctl --user restart xournal-rpc'
