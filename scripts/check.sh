#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

check_log=$(mktemp "${TMPDIR:-/tmp}/xournal-rpc-check.XXXXXX")
spinner_pid=
command_pid=
completed=0
interactive=false
if [ -t 1 ] && [ "${TERM:-dumb}" != dumb ] && [ -z "${CI:-}" ]; then
    interactive=true
fi

cleanup() {
    if [ -n "$spinner_pid" ]; then
        kill "$spinner_pid" 2>/dev/null || :
        wait "$spinner_pid" 2>/dev/null || :
    fi
    if [ -n "$command_pid" ]; then
        kill "$command_pid" 2>/dev/null || :
        wait "$command_pid" 2>/dev/null || :
    fi
    rm -f "$check_log"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

check() {
    label=$1
    shift
    printf '  %s... ' "$label"
    "$@" >"$check_log" 2>&1 &
    command_pid=$!
    if "$interactive"; then
        (
            while :; do
                for frame in '⠋' '⠙' '⠹' '⠸' '⠼' '⠴' '⠦' '⠧' '⠇' '⠏'; do
                    printf '\r\033[K  %s %s' "$frame" "$label"
                    sleep 0.1
                done
            done
        ) &
        spinner_pid=$!
    fi
    result=0
    wait "$command_pid" || result=$?
    command_pid=
    if [ -n "$spinner_pid" ]; then
        kill "$spinner_pid" 2>/dev/null || :
        wait "$spinner_pid" 2>/dev/null || :
        spinner_pid=
        printf '\r\033[K  %s... ' "$label"
    fi
    if [ "$result" -ne 0 ]; then
        printf 'FAILED\n\n'
        cat "$check_log"
        printf '\nStopped after %s/5 successful checks (exit %s).\n' "$completed" "$result"
        exit "$result"
    fi
    completed=$((completed + 1))
    printf 'OK\n'
}

printf 'Checking xournal-rpc\n'
check 'Release script tests' python3 -m unittest discover -s tests -v
check 'Rust formatting' cargo fmt --check
check 'Clippy' cargo clippy --all-targets --locked -- -D warnings
check 'Rust tests' cargo test --locked
check 'Release build' cargo build --release --locked
printf '\nAll %s checks passed.\n' "$completed"
